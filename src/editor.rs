/// # Static / Editor Mode
///
/// Controls (editor mode active):
///
/// | Action                     | Input                                |
/// |----------------------------|--------------------------------------|
/// | **Free-look rotate**       | Alt + Left-drag                      |
/// | **Pan** (slide camera)     | Middle-drag                          |
/// | **Zoom**                   | Scroll wheel                         |
/// | **Focus** on selection     | F key                                |
/// | **Select** object          | Left-click (no modifier)             |
/// | **Go up to parent**        | G key (walks up one hierarchy level) |
/// | **Drag** selected axis     | Click gizmo tip sphere → drag        |
/// | **WASD movement**          | W/A/S/D (editor handles internally) |

use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use winit::keyboard::KeyCode;

use crate::camera::Camera;
use crate::geometry::Geometry;
use crate::mesh::{BakedMesh, MeshData, Vertex};
use crate::objects::Object;
use crate::transform::Transform;
use crate::world::World;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InspectorData {
    pub id: usize,
    pub name: String,
    pub str_id: String,
    pub position: [f32; 3],
    pub rotation_deg: [f32; 3],
    pub scale: [f32; 3],
    pub color: [f32; 4],
    pub geometry_type: Option<String>,
}

impl InspectorData {
    pub fn from_object(id: usize, obj: &Object) -> Self {
        Self {
            id,
            name: obj.name.clone(),
            str_id: obj.str_id.clone(),
            position: obj.transform.position,
            rotation_deg: obj.transform.rotation,
            scale: obj.transform.scale,
            color: obj.color,
            geometry_type: obj.geometry.as_ref().map(geometry_type_name),
        }
    }
}

fn geometry_type_name(g: &Geometry) -> String {
    match g {
        Geometry::Cube { .. }    => "Cube",
        Geometry::Box { .. }     => "Box",
        Geometry::Plane { .. }   => "Plane",
        Geometry::Pyramid { .. } => "Pyramid",
        Geometry::Capsule { .. } => "Capsule",
        Geometry::Sphere { .. }  => "Sphere",
    }.to_string()
}

#[derive(Debug, Clone, Default)]
pub struct Inspector {
    pub selected: Option<InspectorData>,
}

impl Inspector {
    pub fn clear(&mut self) { self.selected = None; }
    pub fn has_selection(&self) -> bool { self.selected.is_some() }
}

// Drag

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DragAxis { X, Y, Z }

#[derive(Debug, Clone)]
pub struct DragState {
    pub object_id: usize,
    pub axis: DragAxis,
}

// EditorEvent

#[derive(Debug, Clone)]
pub enum EditorEvent {
    MouseMotionDelta { dx: f32, dy: f32 },
    CursorMoved { x: f32, y: f32 },
    MouseButton { left: Option<bool>, middle: Option<bool>, right: Option<bool> },
    Scroll { delta: f32 },
    ModifiersChanged { alt: bool },
    FocusKey,
    KeyPressed(KeyCode),
    KeyReleased(KeyCode),
}

// Input state 
#[derive(Debug, Default)]
pub struct EditorInput {
    pub left_down:   bool,
    pub middle_down: bool,
    pub right_down:  bool,
    pub alt_held:    bool,
    pub cursor_x:    f32,
    pub cursor_y:    f32,
}

// EditorState 
pub struct EditorState {
    pub inspector:       Inspector,
    pub input:           EditorInput,
    /// World-space pivot for zoom / pan reference.
    pub pivot:           [f32; 3],
    /// IDs of world objects that must not appear in the inspector.
    pub gizmo_ids:       HashSet<usize>,
    pub viewport_width:  f32,
    pub viewport_height: f32,
    pub drag:            Option<DragState>,
    /// Pre-baked skybox mesh (created once in `enable_editor_mode`).
    pub skybox:          Option<BakedMesh>,
    /// Keys currently held - used for per-frame WASD movement.
    pub pressed_keys:    HashSet<KeyCode>,
    /// Camera movement speed in world units per second (default: 5.0).
    pub camera_speed:    f32,
}

impl EditorState {
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        Self {
            inspector:       Inspector::default(),
            input:           EditorInput::default(),
            pivot:           [0.0, 0.0, 0.0],
            gizmo_ids:       HashSet::new(),
            viewport_width,
            viewport_height,
            drag:            None,
            skybox:          None,
            pressed_keys:    HashSet::new(),
            camera_speed:    5.0,
        }
    }

    pub fn set_viewport_size(&mut self, w: f32, h: f32) {
        self.viewport_width  = w;
        self.viewport_height = h;
    }

    /// No-op: gizmos are now rendered as a see-through overlay mesh each frame
    /// rather than as world objects.  Kept for API compatibility.
    pub fn spawn_gizmos(&mut self, _world: &mut World) {}
    
    /// Build the gizmo overlay mesh for the currently-selected object.
    /// Returns `None` when nothing is selected.
    /// Scale matches the object's bounding sphere radius.
    pub fn gizmo_overlay_for_selection(&self, world: &World) -> Option<(Vec<Vertex>, Vec<u32>)> {
        let sel     = self.inspector.selected.as_ref()?;
        let world_t = compute_world_transform(world, sel.id);
        let center  = world_t.position;
        let geom    = world.objects.get(&sel.id).and_then(|o| o.geometry.clone());
        let scale   = approx_radius(&geom, &world_t).max(0.4) * 1.3;

        // Gizmo arrows
        let (mut verts, mut indices) = build_gizmo_mesh_data(center, scale);

        // Selection box (golden wireframe cage around the object's AABB)
        let half = approx_half_extents(&geom, &world_t);
        let (box_v, box_i) = build_selection_box(center, half);
        let offset = verts.len() as u32;
        verts.extend(box_v);
        indices.extend(box_i.into_iter().map(|i| i + offset));

        Some((verts, indices))
    }

    // Event dispatch
    pub fn process(&mut self, camera: &mut Camera, world: &mut World, event: EditorEvent) {
        match event {
            EditorEvent::CursorMoved { x, y } => {
                self.input.cursor_x = x;
                self.input.cursor_y = y;
            }

            EditorEvent::ModifiersChanged { alt } => {
                self.input.alt_held = alt;
            }

            EditorEvent::KeyPressed(code) => {
                self.pressed_keys.insert(code);
                // G = go up one level in the object hierarchy
                if code == KeyCode::KeyG {
                    let maybe_parent = self.inspector.selected.as_ref()
                        .and_then(|sel| world.objects.get(&sel.id))
                        .and_then(|obj| obj.parent);
                    if let Some(pid) = maybe_parent {
                        self.inspector.selected = world.objects.get(&pid)
                            .map(|o| InspectorData::from_object(pid, o));
                    }
                }
            }

            EditorEvent::KeyReleased(code) => {
                self.pressed_keys.remove(&code);
            }

            EditorEvent::MouseButton { left, middle, right } => {
                if let Some(p) = right  { self.input.right_down  = p; }
                if let Some(p) = middle { self.input.middle_down = p; }
                if let Some(pressed) = left {
                    if !pressed {
                        self.input.left_down = false;
                        self.drag = None;
                    } else {
                        self.input.left_down = true;
                        if !self.input.alt_held {
                            let (sx, sy) = (self.input.cursor_x, self.input.cursor_y);
                            let gizmo_hit = self.inspector.selected.as_ref().and_then(|sel| {
                                let id = sel.id;
                                let wt  = compute_world_transform(world, id);
                                let c   = wt.position;
                                let geom = world.objects.get(&id).and_then(|o| o.geometry.clone());
                                let scale = approx_radius(&geom, &wt).max(0.4) * 1.3;
                                let hit_r = scale * 0.28;
                                let (ro, rd) = self.screen_to_ray(camera, sx, sy);
                                let x_tip = [c[0]+scale, c[1],       c[2]      ];
                                let y_tip = [c[0],       c[1]+scale, c[2]      ];
                                let z_tip = [c[0],       c[1],       c[2]+scale];
                                if       ray_sphere(ro, rd, x_tip, hit_r).is_some() { Some((id, DragAxis::X)) }
                                else if  ray_sphere(ro, rd, y_tip, hit_r).is_some() { Some((id, DragAxis::Y)) }
                                else if  ray_sphere(ro, rd, z_tip, hit_r).is_some() { Some((id, DragAxis::Z)) }
                                else { None }
                            });
                            if let Some((oid, axis)) = gizmo_hit {
                                self.drag = Some(DragState { object_id: oid, axis });
                            } else {
                                // Direct object pick
                                // Select exactly the object that was hit.
                                // Press G to walk up to the parent.
                                let hit = self.pick(camera, world, sx, sy);
                                if let Some(hit_id) = hit {
                                    self.inspector.selected = world.objects.get(&hit_id)
                                        .map(|o| InspectorData::from_object(hit_id, o));
                                } else {
                                    self.inspector.selected = None;
                                }
                            }
                        }
                    }
                }
            }

            EditorEvent::Scroll { delta } => {
                self.zoom(camera, delta);
            }

            EditorEvent::MouseMotionDelta { dx, dy } => {
                // Gizmo drag has the highest priority
                if self.drag.is_some() {
                    self.apply_drag(world, camera, dx, dy);
                    return;
                }
                // Alt + drag = free-look rotation
                if self.input.alt_held {
                    camera.rotate(dx * 0.1, dy * 0.1, false);
                    // Keep pivot in front of camera at same distance
                    let dist = v3_len(v3_sub(camera.eye, self.pivot)).max(0.001);
                    let fwd  = v3_norm(v3_sub(camera.target, camera.eye));
                    self.pivot = v3_add(camera.eye, [fwd[0]*dist, fwd[1]*dist, fwd[2]*dist]);
                } else if self.input.middle_down {
                    self.pan(camera, dx, dy);
                }
            }

            EditorEvent::FocusKey => {
                // Recompute the live world-space position so the focus
                // is correct even after the parent has moved.
                if let Some(sel) = self.inspector.selected.as_ref() {
                    let id = sel.id;
                    let wt = compute_world_transform(world, id);
                    self.focus_on(camera, wt.position);
                }
            }
        }
    }

    // Per-frame update (WASD movement)
    /// Apply WASD camera movement for this frame.
    /// Call this once per frame from `Scene::update_editor`.
    pub fn update(&mut self, camera: &mut Camera, dt: f32) {
        let (fwd, right) = camera.get_directions();
        let mut dir = [0.0_f32; 3];
        if self.pressed_keys.contains(&KeyCode::KeyW) {
            dir[0] += fwd[0]; dir[1] += fwd[1]; dir[2] += fwd[2];
        }
        if self.pressed_keys.contains(&KeyCode::KeyS) {
            dir[0] -= fwd[0]; dir[1] -= fwd[1]; dir[2] -= fwd[2];
        }
        if self.pressed_keys.contains(&KeyCode::KeyD) {
            dir[0] += right[0]; dir[1] += right[1]; dir[2] += right[2];
        }
        if self.pressed_keys.contains(&KeyCode::KeyA) {
            dir[0] -= right[0]; dir[1] -= right[1]; dir[2] -= right[2];
        }
        if dir[0] != 0.0 || dir[1] != 0.0 || dir[2] != 0.0 {
            camera.move_by(dir, self.camera_speed * dt);
            // Keep the pivot under the camera so zoom / orbit stays consistent
            let dist = v3_len(v3_sub(camera.eye, self.pivot)).max(0.001);
            let fwd2 = v3_norm(v3_sub(camera.target, camera.eye));
            self.pivot = v3_add(camera.eye, [fwd2[0]*dist, fwd2[1]*dist, fwd2[2]*dist]);
        }
    }

    // Axis drag
    fn apply_drag(&mut self, world: &mut World, camera: &Camera, dx: f32, dy: f32) {
        let (object_id, axis) = match &self.drag {
            Some(d) => (d.object_id, d.axis),
            None    => return,
        };

        let world_t = compute_world_transform(world, object_id);
        let center  = world_t.position;

        let axis_dir: [f32; 3] = match axis {
            DragAxis::X => [1.0, 0.0, 0.0],
            DragAxis::Y => [0.0, 1.0, 0.0],
            DragAxis::Z => [0.0, 0.0, 1.0],
        };

        // Project center and center+axis into NDC, convert to screen pixels
        let vp    = camera.build_view_projection_matrix();
        let c_ndc = vp.project_point(center);
        let a_ndc = vp.project_point([
            center[0]+axis_dir[0], center[1]+axis_dir[1], center[2]+axis_dir[2],
        ]);
        let ax_px = (a_ndc[0] - c_ndc[0]) * self.viewport_width  * 0.5;
        let ay_px = (a_ndc[1] - c_ndc[1]) * self.viewport_height * 0.5;
        let len   = (ax_px*ax_px + ay_px*ay_px).sqrt();
        if len < 0.5 { return; } // axis edge-on to view

        // Dot(mouse_delta, screen_axis) gives signed travel along axis
        // Screen Y is downward; NDC Y is upward → negate dy
        let alignment = (dx * ax_px + (-dy) * ay_px) / len;

        // Perspective-correct: world units per screen pixel
        let cam_dist = v3_len(v3_sub(center, camera.eye)).max(0.001);
        let wpp = cam_dist * (camera.fov.to_radians() * 0.5).tan() * 2.0 / self.viewport_height;
        let delta_world = alignment * wpp;

        if let Some(obj) = world.objects.get_mut(&object_id) {
            obj.transform.position[0] += axis_dir[0] * delta_world;
            obj.transform.position[1] += axis_dir[1] * delta_world;
            obj.transform.position[2] += axis_dir[2] * delta_world;
        }
        // Sync inspector snapshot
        if let Some(sel) = &mut self.inspector.selected {
            if sel.id == object_id {
                if let Some(obj) = world.objects.get(&object_id) {
                    sel.position = obj.transform.position;
                }
            }
        }
    }

    // Camera helpers
    pub fn zoom(&mut self, camera: &mut Camera, delta: f32) {
        let off  = v3_sub(camera.eye, self.pivot);
        let dist = v3_len(off).max(0.001);
        let new_dist = (dist - delta * dist * 0.15).max(0.3);
        let scale    = new_dist / dist;
        camera.eye = [
            self.pivot[0] + off[0] * scale,
            self.pivot[1] + off[1] * scale,
            self.pivot[2] + off[2] * scale,
        ];
    }

    pub fn pan(&mut self, camera: &mut Camera, dx: f32, dy: f32) {
        let dist  = v3_len(v3_sub(camera.eye, self.pivot)).max(0.001);
        let speed = dist * 0.0012;
        let (_, right) = camera.get_directions();
        let up    = camera.up;
        let delta = [
            (-dx * right[0] + dy * up[0]) * speed,
            (-dx * right[1] + dy * up[1]) * speed,
            (-dx * right[2] + dy * up[2]) * speed,
        ];
        camera.eye    = v3_add(camera.eye,    delta);
        camera.target = v3_add(camera.target, delta);
        self.pivot    = v3_add(self.pivot,    delta);
    }

    pub fn focus_on(&mut self, camera: &mut Camera, point: [f32; 3]) {
        let off    = v3_sub(camera.eye, self.pivot);
        self.pivot = point;
        camera.eye    = v3_add(point, off);
        camera.target = point;
    }

    // Picking
    pub fn pick(&self, camera: &Camera, world: &World, sx: f32, sy: f32) -> Option<usize> {
        let (ro, rd) = self.screen_to_ray(camera, sx, sy);
        let mut best_id   = None;
        let mut best_dist = f32::MAX;

        for (&id, obj) in &world.objects {
            if self.gizmo_ids.contains(&id) { continue; }
            if obj.geometry.is_none()      { continue; }
            let wt = compute_world_transform(world, id);
            let r  = approx_radius(&obj.geometry, &wt);
            if let Some(t) = ray_sphere(ro, rd, wt.position, r) {
                if t > 0.0 && t < best_dist { best_dist = t; best_id = Some(id); }
            }
        }
        best_id
    }

    fn screen_to_ray(&self, camera: &Camera, sx: f32, sy: f32) -> ([f32; 3], [f32; 3]) {
        let x_ndc = 2.0 * sx / self.viewport_width  - 1.0;
        let y_ndc = 1.0 - 2.0 * sy / self.viewport_height;
        let fwd   = v3_norm(v3_sub(camera.target, camera.eye));
        let right = v3_norm([
            camera.up[1]*fwd[2] - camera.up[2]*fwd[1],
            camera.up[2]*fwd[0] - camera.up[0]*fwd[2],
            camera.up[0]*fwd[1] - camera.up[1]*fwd[0],
        ]);
        let up = [
            fwd[1]*right[2] - fwd[2]*right[1],
            fwd[2]*right[0] - fwd[0]*right[2],
            fwd[0]*right[1] - fwd[1]*right[0],
        ];
        let hv = (camera.fov.to_radians() * 0.5).tan();
        let hh = hv * camera.aspect;
        let dir = [
            fwd[0] + x_ndc * hh * right[0] + y_ndc * hv * up[0],
            fwd[1] + x_ndc * hh * right[1] + y_ndc * hv * up[1],
            fwd[2] + x_ndc * hh * right[2] + y_ndc * hv * up[2],
        ];
        (camera.eye, v3_norm(dir))
    }
}

// Gizmo mesh generation
/// Build the three-axis gizmo mesh centred at `center` with arm length `scale`.
pub fn build_gizmo_mesh_data(center: [f32; 3], scale: f32) -> (Vec<Vertex>, Vec<u32>) {
    let mut mesh = MeshData::new();
    let shaft_h  = scale * 0.07; // half-thickness
    let tip_r    = scale * 0.14;
    let dot_r    = scale * 0.09;
    let len      = scale;

    let t = |pos: [f32; 3]| Transform::from_position(pos[0], pos[1], pos[2]);

    Geometry::Sphere { radius: dot_r, subdivisions: 8 }
        .generate_mesh_data(&mut mesh, &t(center), [0.9, 0.9, 0.9, 1.0]);

    let cx = center[0]; let cy = center[1]; let cz = center[2];

    // X (red)
    Geometry::Box { width: len, height: shaft_h*2.0, depth: shaft_h*2.0 }
        .generate_mesh_data(&mut mesh, &t([cx + len*0.5, cy, cz]), [0.95, 0.15, 0.15, 1.0]);
    Geometry::Sphere { radius: tip_r, subdivisions: 8 }
        .generate_mesh_data(&mut mesh, &t([cx + len, cy, cz]), [0.95, 0.15, 0.15, 1.0]);

    // Y (green)
    Geometry::Box { width: shaft_h*2.0, height: len, depth: shaft_h*2.0 }
        .generate_mesh_data(&mut mesh, &t([cx, cy + len*0.5, cz]), [0.15, 0.95, 0.15, 1.0]);
    Geometry::Sphere { radius: tip_r, subdivisions: 8 }
        .generate_mesh_data(&mut mesh, &t([cx, cy + len, cz]), [0.15, 0.95, 0.15, 1.0]);

    // Z (blue)
    Geometry::Box { width: shaft_h*2.0, height: shaft_h*2.0, depth: len }
        .generate_mesh_data(&mut mesh, &t([cx, cy, cz + len*0.5]), [0.15, 0.15, 0.95, 1.0]);
    Geometry::Sphere { radius: tip_r, subdivisions: 8 }
        .generate_mesh_data(&mut mesh, &t([cx, cy, cz + len]), [0.15, 0.15, 0.95, 1.0]);

    (mesh.vertices, mesh.indices)
}

/// Build the skybox mesh — a large box visible from inside (rendered with
/// the overlay pipeline which has `cull_mode: None`).
pub fn build_skybox_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let mut mesh = MeshData::new();
    let s = 450.0_f32;

    let top = [0.08, 0.12, 0.22, 1.0_f32]; // midnight blue top
    let mid = [0.10, 0.13, 0.20, 1.0];     // blue-grey horizon
    let bot = [0.05, 0.06, 0.09, 1.0];     // near-black ground

    mesh.push_quad([[-s,s,-s],[s,s,-s],[s,s,s],[-s,s,s]], top); // top
    mesh.push_quad([[-s,-s,s],[s,-s,s],[s,-s,-s],[-s,-s,-s]], bot); // bottom
    mesh.push_quad([[-s,s,-s],[-s,-s,-s],[s,-s,-s],[s,s,-s]], mid); // front (-Z)
    mesh.push_quad([[s,s,s],[s,-s,s],[-s,-s,s],[-s,s,s]], mid); // back (+Z)
    mesh.push_quad([[s,s,-s],[s,-s,-s],[s,-s,s],[s,s,s]], mid); // right (+X)
    mesh.push_quad([[-s,s,s],[-s,-s,s],[-s,-s,-s],[-s,s,-s]], mid); // left (-X)

    (mesh.vertices, mesh.indices)
}

// Hierarchy helper
pub(crate) fn compute_world_transform(world: &World, id: usize) -> Transform {
    if let Some(obj) = world.objects.get(&id) {
        match obj.parent {
            None            => obj.transform.clone(),
            Some(parent_id) => compute_world_transform(world, parent_id).combine(&obj.transform),
        }
    } else {
        Transform::default()
    }
}

// Math helpers
#[inline] pub(crate) fn v3_len(v: [f32;3]) -> f32 { (v[0]*v[0]+v[1]*v[1]+v[2]*v[2]).sqrt() }
#[inline] fn v3_sub(a:[f32;3], b:[f32;3]) -> [f32;3] { [a[0]-b[0],a[1]-b[1],a[2]-b[2]] }
#[inline] fn v3_add(a:[f32;3], b:[f32;3]) -> [f32;3] { [a[0]+b[0],a[1]+b[1],a[2]+b[2]] }
#[inline] fn v3_norm(v:[f32;3]) -> [f32;3] { let l=v3_len(v).max(1e-6); [v[0]/l,v[1]/l,v[2]/l] }

pub(crate) fn approx_radius(geom: &Option<Geometry>, t: &Transform) -> f32 {
    let base = match geom {
        Some(Geometry::Sphere  { radius, .. })           => *radius,
        Some(Geometry::Cube    { size })                 => *size * 0.5,
        Some(Geometry::Box     { width, height, depth }) => width.max(*height).max(*depth) * 0.5,
        Some(Geometry::Plane   { size })                 => *size * 0.5,
        Some(Geometry::Pyramid { base_size, height })    => base_size.max(*height) * 0.5,
        Some(Geometry::Capsule { radius, height, .. })   => radius + height * 0.5,
        None                                             => 0.5,
    };
    base * t.scale[0].max(t.scale[1]).max(t.scale[2])
}

fn ray_sphere(o:[f32;3], d:[f32;3], c:[f32;3], r:f32) -> Option<f32> {
    let oc = [o[0]-c[0], o[1]-c[1], o[2]-c[2]];
    let a  = d[0]*d[0]+d[1]*d[1]+d[2]*d[2];
    let b  = 2.0*(oc[0]*d[0]+oc[1]*d[1]+oc[2]*d[2]);
    let cc = oc[0]*oc[0]+oc[1]*oc[1]+oc[2]*oc[2]-r*r;
    let dis = b*b-4.0*a*cc;
    if dis < 0.0 { return None; }
    let t1 = (-b-dis.sqrt())/(2.0*a);
    let t2 = (-b+dis.sqrt())/(2.0*a);
    if t1 > 0.0 { Some(t1) } else if t2 > 0.0 { Some(t2) } else { None }
}

/// Axis-aligned half-extents of `geom` in world space (accounts for world scale).
fn approx_half_extents(geom: &Option<Geometry>, t: &Transform) -> [f32; 3] {
    let base: [f32; 3] = match geom {
        Some(Geometry::Sphere  { radius, .. })           => [*radius; 3],
        Some(Geometry::Cube    { size })                 => [*size * 0.5; 3],
        Some(Geometry::Box     { width, height, depth }) => [*width * 0.5, *height * 0.5, *depth * 0.5],
        Some(Geometry::Plane   { size })                 => [*size * 0.5, 0.01, *size * 0.5],
        Some(Geometry::Pyramid { base_size, height })    => [*base_size * 0.5, *height * 0.5, *base_size * 0.5],
        Some(Geometry::Capsule { radius, height, .. })   => [*radius, *height * 0.5 + *radius, *radius],
        None                                             => [0.5; 3],
    };
    [
        (base[0] * t.scale[0]).max(0.05),
        (base[1] * t.scale[1]).max(0.05),
        (base[2] * t.scale[2]).max(0.05),
    ]
}

/// Build a golden wireframe bounding-box cage (12 edges as thin rectangular prisms).
/// Rendered via the overlay pipeline so it is always visible through objects.
pub fn build_selection_box(center: [f32; 3], half: [f32; 3]) -> (Vec<Vertex>, Vec<u32>) {
    let mut mesh = MeshData::new();
    let color = [1.0_f32, 0.85, 0.1, 1.0]; // golden yellow

    // Pad the box slightly so it sits just outside the object surface
    let pad = (half[0] + half[1] + half[2]) / 3.0 * 0.06;
    let [hx, hy, hz] = [half[0] + pad, half[1] + pad, half[2] + pad];
    let [cx, cy, cz] = center;

    // Edge thickness proportional to average half-extent
    let tk = ((hx + hy + hz) / 3.0 * 0.045).max(0.02);

    let tr = |px: f32, py: f32, pz: f32| Transform::from_position(px, py, pz);

    // Bottom 4 edges (y = cy − hy)
    Geometry::Box { width: hx*2.0, height: tk, depth: tk }
        .generate_mesh_data(&mut mesh, &tr(cx,      cy - hy, cz - hz), color);
    Geometry::Box { width: hx*2.0, height: tk, depth: tk }
        .generate_mesh_data(&mut mesh, &tr(cx,      cy - hy, cz + hz), color);
    Geometry::Box { width: tk,     height: tk, depth: hz*2.0 }
        .generate_mesh_data(&mut mesh, &tr(cx - hx, cy - hy, cz),      color);
    Geometry::Box { width: tk,     height: tk, depth: hz*2.0 }
        .generate_mesh_data(&mut mesh, &tr(cx + hx, cy - hy, cz),      color);

    // Top 4 edges (y = cy + hy)
    Geometry::Box { width: hx*2.0, height: tk, depth: tk }
        .generate_mesh_data(&mut mesh, &tr(cx,      cy + hy, cz - hz), color);
    Geometry::Box { width: hx*2.0, height: tk, depth: tk }
        .generate_mesh_data(&mut mesh, &tr(cx,      cy + hy, cz + hz), color);
    Geometry::Box { width: tk,     height: tk, depth: hz*2.0 }
        .generate_mesh_data(&mut mesh, &tr(cx - hx, cy + hy, cz),      color);
    Geometry::Box { width: tk,     height: tk, depth: hz*2.0 }
        .generate_mesh_data(&mut mesh, &tr(cx + hx, cy + hy, cz),      color);

    // 4 vertical edges 
    Geometry::Box { width: tk, height: hy*2.0, depth: tk }
        .generate_mesh_data(&mut mesh, &tr(cx - hx, cy, cz - hz), color);
    Geometry::Box { width: tk, height: hy*2.0, depth: tk }
        .generate_mesh_data(&mut mesh, &tr(cx + hx, cy, cz - hz), color);
    Geometry::Box { width: tk, height: hy*2.0, depth: tk }
        .generate_mesh_data(&mut mesh, &tr(cx - hx, cy, cz + hz), color);
    Geometry::Box { width: tk, height: hy*2.0, depth: tk }
        .generate_mesh_data(&mut mesh, &tr(cx + hx, cy, cz + hz), color);

    (mesh.vertices, mesh.indices)
}
