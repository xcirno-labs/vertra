//! [`EditorState`] — the main editor subsystem struct and its impl.

use std::collections::HashSet;
use winit::keyboard::KeyCode;

use crate::camera::Camera;
use crate::mesh::{BakedMesh, Vertex};
use crate::world::World;

use super::gizmo::{
    GIZMO_SCREEN_PX,
    build_gizmo_mesh_data, build_rotate_gizmo_mesh_data,
    build_scale_gizmo_mesh_data, build_selection_box,
};
use super::math::{
    v3_len, v3_sub, v3_add, v3_norm,
    approx_radius, approx_half_extents,
    ray_sphere, ray_aabb, ray_ring,
    compute_world_transform, collect_descendants,
    filter_top_level_ids, combined_aabb,
};
use super::types::{
    DragAxis, DragKind, DragState, EditorEvent, GizmoMode,
    InspectorData, Inspector, EditorInput,
    LabelInspectorData, LabelDragState, LabelDragKind,
};

/// All runtime state for the static scene editor.
///
/// Attach to a scene with [`crate::scene::Scene::enable_editor_mode`] and
/// detach with [`crate::scene::Scene::disable_editor_mode`].
pub struct EditorState {
    pub inspector:       Inspector,
    pub input:           EditorInput,
    /// World-space pivot for zoom / pan / orbit reference.
    pub pivot:           [f32; 3],
    /// IDs of world objects that must not appear in the inspector.
    pub gizmo_ids:       HashSet<usize>,
    pub viewport_width:  f32,
    pub viewport_height: f32,
    pub drag:            Option<DragState>,
    /// Active gizmo mode: Translate, Rotate, or Scale (T / R / E keys).
    pub gizmo_mode:      GizmoMode,
    /// Pre-baked skybox mesh (created once in `enable_editor_mode`).
    pub skybox:          Option<BakedMesh>,
    /// Keys currently held — used for per-frame WASD movement.
    pub pressed_keys:    HashSet<KeyCode>,
    /// Camera movement speed in world units per second (default: `5.0`).
    pub camera_speed:    f32,
    /// All IDs in the current group expansion (root + descendants, G key).
    pub group_ids:       Vec<usize>,
    /// All individually-selected object IDs (via Ctrl+Click).
    pub multi_selected:  Vec<usize>,
    /// Currently-selected text label (if any) in the label editor.
    pub selected_label:  Option<LabelInspectorData>,
    /// In-progress label drag (move or resize).
    pub label_drag:      Option<LabelDragState>,
}

impl EditorState {
    /// Create a new editor state for a viewport of the given pixel dimensions.
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        Self {
            inspector: Inspector::default(),
            input: EditorInput::default(),
            pivot: [0.0, 0.0, 0.0],
            gizmo_ids: HashSet::new(),
            viewport_width,
            viewport_height,
            drag: None,
            gizmo_mode: GizmoMode::default(),
            skybox: None,
            pressed_keys: HashSet::new(),
            camera_speed: 5.0,
            group_ids: Vec::new(),
            multi_selected: Vec::new(),
            selected_label: None,
            label_drag: None,
        }
    }

    /// Update the viewport pixel dimensions (call on window resize).
    pub fn set_viewport_size(&mut self, w: f32, h: f32) {
        self.viewport_width = w;
        self.viewport_height = h;
    }

    /// No-op: gizmos are rendered as an overlay mesh each frame rather than
    /// as world objects.  Kept for API compatibility.
    pub fn spawn_gizmos(&mut self, _world: &mut World) {}

    /// Returns `(centre, gizmo_arm_len, aabb_half_extents)` for the current
    /// selection, or `None` when nothing is selected.
    fn selection_geometry(&self, world: &World) -> Option<([f32; 3], f32, [f32; 3])> {
        self.inspector.selected.as_ref()?;

        // Priority 1: multi-selection (Ctrl+Click)
        if self.multi_selected.len() > 1 {
            if let Some((mn, mx)) = combined_aabb(world, &self.multi_selected) {
                let center = [(mn[0] + mx[0]) * 0.5, (mn[1] + mx[1]) * 0.5, (mn[2] + mx[2]) * 0.5];
                let half = [((mx[0] - mn[0]) * 0.5).max(0.05), ((mx[1] - mn[1]) * 0.5).max(0.05), ((mx[2] - mn[2]) * 0.5).max(0.05)];
                let scale = half[0].max(half[1]).max(half[2]) * 1.3;
                return Some((center, scale, half));
            }
        }
        // Priority 2: group expansion (G key)
        if !self.group_ids.is_empty() {
            if let Some((mn, mx)) = combined_aabb(world, &self.group_ids) {
                let center = [(mn[0] + mx[0]) * 0.5, (mn[1] + mx[1]) * 0.5, (mn[2] + mx[2]) * 0.5];
                let half = [((mx[0] - mn[0]) * 0.5).max(0.05), ((mx[1] - mn[1]) * 0.5).max(0.05), ((mx[2] - mn[2]) * 0.5).max(0.05)];
                let scale = half[0].max(half[1]).max(half[2]) * 1.3;
                return Some((center, scale, half));
            }
        }
        // Priority 3: single selection
        let sel = self.inspector.selected.as_ref().unwrap();
        let wt = compute_world_transform(world, sel.id);
        let geom = world.objects.get(&sel.id).and_then(|o| o.geometry.clone());
        let scale = approx_radius(&geom, &wt).max(0.4) * 1.3;
        let half = approx_half_extents(&geom, &wt);
        Some((wt.position, scale, half))
    }

    /// Build the gizmo overlay mesh for the current selection.
    /// Returns `None` when nothing is selected.
    pub fn gizmo_overlay_for_selection(
        &self, world: &World, camera: &Camera,
    ) -> Option<(Vec<Vertex>, Vec<u32>)> {
        let (center, _, half) = self.selection_geometry(world)?;
        let gizmo_scale = self.gizmo_scale(camera, center);
        let (mut verts, mut indices) = match self.gizmo_mode {
            GizmoMode::Translate => build_gizmo_mesh_data(center, gizmo_scale),
            GizmoMode::Rotate => build_rotate_gizmo_mesh_data(center, gizmo_scale),
            GizmoMode::Scale => build_scale_gizmo_mesh_data(center, gizmo_scale),
        };
        let (box_v, box_i) = build_selection_box(center, half);
        let offset = verts.len() as u32;
        verts.extend(box_v);
        indices.extend(box_i.into_iter().map(|i| i + offset));
        Some((verts, indices))
    }

    /// Returns the world-space arm length that makes the gizmo appear
    /// [`GIZMO_SCREEN_PX`] pixels tall regardless of camera distance.
    fn gizmo_scale(&self, camera: &Camera, center: [f32; 3]) -> f32 {
        let dist = v3_len(v3_sub(center, camera.eye)).max(0.001);
        dist * (camera.fov.to_radians() * 0.5).tan() * 2.0
            / self.viewport_height * GIZMO_SCREEN_PX
    }

    /// Process a single [`EditorEvent`].  Called by the window loop every
    /// time a relevant platform event arrives while editor mode is active.
    pub fn process(&mut self, camera: &mut Camera, world: &mut World, event: EditorEvent) {
        match event {
            EditorEvent::CursorMoved { x, y } => {
                self.input.cursor_x = x;
                self.input.cursor_y = y;
            }

            EditorEvent::ModifiersChanged { alt, ctrl } => {
                self.input.alt_held = alt;
                self.input.ctrl_held = ctrl;
            }

            EditorEvent::KeyPressed(code) => {
                self.pressed_keys.insert(code);
                if code == KeyCode::KeyG {
                    if let Some(sel) = &self.inspector.selected {
                        let root_id = sel.id;
                        let mut ids = Vec::new();
                        collect_descendants(world, root_id, &mut ids);
                        self.group_ids = ids;
                    }
                }
                if code == KeyCode::KeyT { self.gizmo_mode = GizmoMode::Translate; }
                if code == KeyCode::KeyR { self.gizmo_mode = GizmoMode::Rotate; }
                if code == KeyCode::KeyE { self.gizmo_mode = GizmoMode::Scale; }
            }

            EditorEvent::KeyReleased(code) => {
                self.pressed_keys.remove(&code);
            }

            EditorEvent::MouseButton { left, middle, right } => {
                if let Some(p) = right { self.input.right_down = p; }
                if let Some(p) = middle { self.input.middle_down = p; }
                if let Some(pressed) = left {
                    if !pressed {
                        self.input.left_down = false;
                        self.drag = None;
                    } else {
                        self.input.left_down = true;
                        if !self.input.alt_held {
                            let (sx, sy) = (self.input.cursor_x, self.input.cursor_y);
                            let sel_geom = self.selection_geometry(world);
                            let sel_id = self.inspector.selected.as_ref().map(|s| s.id);
                            let gizmo_hit = if let (Some((c, _, _)), Some(id)) = (sel_geom, sel_id) {
                                let gs = self.gizmo_scale(camera, c);
                                let (ro, rd) = self.screen_to_ray(camera, sx, sy);
                                match self.gizmo_mode {
                                    GizmoMode::Translate | GizmoMode::Scale => {
                                        let hit_r = gs * 0.28;
                                        let xt = [c[0] + gs, c[1], c[2]];
                                        let yt = [c[0], c[1] + gs, c[2]];
                                        let zt = [c[0], c[1], c[2] + gs];
                                        if ray_sphere(ro, rd, xt, hit_r).is_some() { Some((id, c, DragAxis::X)) } else if ray_sphere(ro, rd, yt, hit_r).is_some() { Some((id, c, DragAxis::Y)) } else if ray_sphere(ro, rd, zt, hit_r).is_some() { Some((id, c, DragAxis::Z)) } else { None }
                                    }
                                    GizmoMode::Rotate => {
                                        let hw = gs * 0.25;
                                        let axes = [
                                            (DragAxis::X, [1.0_f32, 0.0, 0.0]),
                                            (DragAxis::Y, [0.0_f32, 1.0, 0.0]),
                                            (DragAxis::Z, [0.0_f32, 0.0, 1.0]),
                                        ];
                                        let mut best: Option<(DragAxis, f32)> = None;
                                        for (da, n) in &axes {
                                            if let Some(t) = ray_ring(ro, rd, c, *n, gs, hw) {
                                                if best.map_or(true, |(_, bt)| t < bt) {
                                                    best = Some((*da, t));
                                                }
                                            }
                                        }
                                        best.map(|(da, _)| (id, c, da))
                                    }
                                }
                            } else { None };

                            if let Some((oid, center, axis)) = gizmo_hit {
                                let kind = match self.gizmo_mode {
                                    GizmoMode::Translate => DragKind::Translate,
                                    GizmoMode::Rotate => DragKind::Rotate,
                                    GizmoMode::Scale => DragKind::Scale,
                                };
                                self.drag = Some(DragState { object_id: oid, axis, center, kind });
                            } else {
                                let hit = self.pick(camera, world, sx, sy);
                                if self.input.ctrl_held {
                                    self.group_ids.clear();
                                    if let Some(hit_id) = hit {
                                        if let Some(pos) = self.multi_selected.iter().position(|&id| id == hit_id) {
                                            self.multi_selected.remove(pos);
                                            if self.inspector.selected.as_ref().map(|s| s.id) == Some(hit_id) {
                                                self.inspector.selected = self.multi_selected.last()
                                                    .and_then(|&id| world.objects.get(&id)
                                                        .map(|o| InspectorData::from_object(id, o)));
                                            }
                                        } else {
                                            self.multi_selected.push(hit_id);
                                            self.inspector.selected = world.objects.get(&hit_id)
                                                .map(|o| InspectorData::from_object(hit_id, o));
                                        }
                                    }
                                } else {
                                    self.group_ids.clear();
                                    self.multi_selected.clear();
                                    if let Some(hit_id) = hit {
                                        self.multi_selected.push(hit_id);
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
            }

            EditorEvent::Scroll { delta } => { self.zoom(camera, delta); }

            EditorEvent::MouseMotionDelta { dx, dy } => {
                if self.drag.is_some() {
                    self.apply_drag(world, camera, dx, dy);
                    return;
                }
                if self.input.alt_held {
                    camera.rotate(dx * 0.1, dy * 0.1, false);
                    let dist = v3_len(v3_sub(camera.eye, self.pivot)).max(0.001);
                    let fwd = v3_norm(v3_sub(camera.target, camera.eye));
                    self.pivot = v3_add(camera.eye, [fwd[0] * dist, fwd[1] * dist, fwd[2] * dist]);
                } else if self.input.middle_down {
                    self.pan(camera, dx, dy);
                }
            }

            EditorEvent::FocusKey => {
                if let Some((center, _, _)) = self.selection_geometry(world) {
                    self.focus_on(camera, center);
                }
            }
        }
    }

    /// Apply WASD camera movement for this frame.
    /// Call once per frame from [`crate::scene::Scene::update_editor`].
    pub fn update(&mut self, camera: &mut Camera, dt: f32) {
        let (fwd, right) = camera.get_directions();
        let mut dir = [0.0_f32; 3];
        if self.pressed_keys.contains(&KeyCode::KeyW) {
            dir[0] += fwd[0];
            dir[1] += fwd[1];
            dir[2] += fwd[2];
        }
        if self.pressed_keys.contains(&KeyCode::KeyS) {
            dir[0] -= fwd[0];
            dir[1] -= fwd[1];
            dir[2] -= fwd[2];
        }
        if self.pressed_keys.contains(&KeyCode::KeyD) {
            dir[0] += right[0];
            dir[1] += right[1];
            dir[2] += right[2];
        }
        if self.pressed_keys.contains(&KeyCode::KeyA) {
            dir[0] -= right[0];
            dir[1] -= right[1];
            dir[2] -= right[2];
        }
        if dir[0] != 0.0 || dir[1] != 0.0 || dir[2] != 0.0 {
            let shift = self.pressed_keys.contains(&KeyCode::ShiftLeft)
                || self.pressed_keys.contains(&KeyCode::ShiftRight);
            let speed = self.camera_speed * if shift { 3.0 } else { 1.0 };
            camera.move_by(dir, speed * dt);
            let dist = v3_len(v3_sub(camera.eye, self.pivot)).max(0.001);
            let fwd2 = v3_norm(v3_sub(camera.target, camera.eye));
            self.pivot = v3_add(camera.eye, [fwd2[0] * dist, fwd2[1] * dist, fwd2[2] * dist]);
        }
    }

    fn apply_drag(&mut self, world: &mut World, camera: &Camera, dx: f32, dy: f32) {
        let (object_id, axis, center, kind) = match &self.drag {
            Some(d) => (d.object_id, d.axis, d.center, d.kind),
            None => return,
        };
        let axis_dir: [f32; 3] = match axis {
            DragAxis::X => [1., 0., 0.],
            DragAxis::Y => [0., 1., 0.],
            DragAxis::Z => [0., 0., 1.]
        };
        let axis_idx: usize = match axis {
            DragAxis::X => 0,
            DragAxis::Y => 1,
            DragAxis::Z => 2
        };

        let vp = camera.build_view_projection_matrix();
        let c_ndc = vp.project_point(center);
        let a_ndc = vp.project_point([center[0] + axis_dir[0], center[1] + axis_dir[1], center[2] + axis_dir[2]]);
        let ax_px = (a_ndc[0] - c_ndc[0]) * self.viewport_width * 0.5;
        let ay_px = (a_ndc[1] - c_ndc[1]) * self.viewport_height * 0.5;
        let len = (ax_px * ax_px + ay_px * ay_px).sqrt();

        if len < 0.5 && kind != DragKind::Rotate { return; }
        let alignment = if len >= 0.5 { (dx * ax_px + (-dy) * ay_px) / len } else { 0.0 };

        let raw_ids: Vec<usize> = if !self.group_ids.is_empty() { self.group_ids.clone() } else if self.multi_selected.len() > 1 { self.multi_selected.clone() } else { vec![object_id] };
        let top_ids = filter_top_level_ids(world, &raw_ids);

        match kind {
            DragKind::Translate => {
                let wpp = v3_len(v3_sub(center, camera.eye)).max(0.001)
                    * (camera.fov.to_radians() * 0.5).tan() * 2.0 / self.viewport_height;
                let delta = alignment * wpp;
                for &id in &top_ids {
                    if let Some(obj) = world.objects.get_mut(&id) {
                        obj.transform.position[0] += axis_dir[0] * delta;
                        obj.transform.position[1] += axis_dir[1] * delta;
                        obj.transform.position[2] += axis_dir[2] * delta;
                    }
                }
            }
            DragKind::Rotate => {
                let cam_dir = v3_norm(v3_sub(center, camera.eye));
                let dot = axis_dir[0] * cam_dir[0] + axis_dir[1] * cam_dir[1] + axis_dir[2] * cam_dir[2];
                let abs_dot = dot.abs();
                let cx_px = (c_ndc[0] + 1.0) * self.viewport_width * 0.5;
                let cy_px = (1.0 - c_ndc[1]) * self.viewport_height * 0.5;
                let vx = self.input.cursor_x - cx_px;
                let vy = self.input.cursor_y - cy_px;
                let vlen = (vx * vx + vy * vy).sqrt().max(0.001);
                let circ = (dx * (-vy) + dy * vx) / vlen * if dot < 0.0 { -1.0 } else { 1.0 };
                let perp = if len > 0.1 { (dx * (-ay_px) + (-dy) * ax_px) / len } else { 0.0 };
                let angle = (perp * (1.0 - abs_dot) + circ * abs_dot) * 0.5;
                for &id in &top_ids {
                    if let Some(obj) = world.objects.get_mut(&id) {
                        obj.transform.rotation[axis_idx] += angle;
                    }
                }
            }
            DragKind::Scale => {
                let ds = alignment / GIZMO_SCREEN_PX * 0.5;
                for &id in &top_ids {
                    if let Some(obj) = world.objects.get_mut(&id) {
                        obj.transform.scale[axis_idx] = (obj.transform.scale[axis_idx] + ds).max(0.01);
                    }
                }
            }
        }
        if let Some(sel) = &mut self.inspector.selected {
            if let Some(obj) = world.objects.get(&sel.id) {
                sel.position = obj.transform.position;
                sel.rotation_deg = obj.transform.rotation;
                sel.scale = obj.transform.scale;
            }
        }
    }

    /// Zoom the camera towards / away from the pivot point.
    pub fn zoom(&mut self, camera: &mut Camera, delta: f32) {
        let off = v3_sub(camera.eye, self.pivot);
        let dist = v3_len(off);
        if dist < 0.001 {
            let (fwd, _) = camera.get_directions();
            self.pivot = v3_add(camera.eye, fwd);
            return;
        }
        let delta = delta.clamp(-4.0, 4.0);
        let new_dist = (dist * (1.0 - delta * 0.12)).max(0.3);
        let scale = new_dist / dist;
        let new_eye = [self.pivot[0] + off[0] * scale, self.pivot[1] + off[1] * scale, self.pivot[2] + off[2] * scale];
        let d = v3_sub(new_eye, camera.eye);
        camera.eye = new_eye;
        camera.target = v3_add(camera.target, d);
    }

    /// Pan the camera (slide parallel to the view plane).
    pub fn pan(&mut self, camera: &mut Camera, dx: f32, dy: f32) {
        let dist = v3_len(v3_sub(camera.eye, self.pivot)).max(0.001);
        let speed = dist * 0.0012;
        let (_, right) = camera.get_directions();
        let up = camera.up;
        let delta = [
            (-dx * right[0] + dy * up[0]) * speed,
            (-dx * right[1] + dy * up[1]) * speed,
            (-dx * right[2] + dy * up[2]) * speed,
        ];
        camera.eye = v3_add(camera.eye, delta);
        camera.target = v3_add(camera.target, delta);
        self.pivot = v3_add(self.pivot, delta);
    }

    /// Move the orbit pivot (and camera) to focus on `point`.
    pub fn focus_on(&mut self, camera: &mut Camera, point: [f32; 3]) {
        let off = v3_sub(camera.eye, self.pivot);
        self.pivot = point;
        camera.eye = v3_add(point, off);
        camera.target = point;
    }

    /// Cast a ray from screen pixel `(sx, sy)` and return the nearest object ID.
    pub fn pick(&self, camera: &Camera, world: &World, sx: f32, sy: f32) -> Option<usize> {
        let (ro, rd) = self.screen_to_ray(camera, sx, sy);
        let mut best_id = None;
        let mut best_dist = f32::MAX;
        for (&id, obj) in &world.objects {
            if self.gizmo_ids.contains(&id) || obj.geometry.is_none() { continue; }
            let wt = compute_world_transform(world, id);
            // Use a per-axis AABB test so that scaling one axis only enlarges
            // the hit volume on that axis, not in every direction.
            let half = approx_half_extents(&obj.geometry, &wt);
            if let Some(t) = ray_aabb(ro, rd, wt.position, half) {
                if t < best_dist {
                    best_dist = t;
                    best_id = Some(id);
                }
            }
        }
        best_id
    }

    fn screen_to_ray(&self, camera: &Camera, sx: f32, sy: f32) -> ([f32; 3], [f32; 3]) {
        // ...existing code...
        let x_ndc = 2.0 * sx / self.viewport_width - 1.0;
        let y_ndc = 1.0 - 2.0 * sy / self.viewport_height;
        let fwd = v3_norm(v3_sub(camera.target, camera.eye));
        let right = v3_norm([
            camera.up[1] * fwd[2] - camera.up[2] * fwd[1],
            camera.up[2] * fwd[0] - camera.up[0] * fwd[2],
            camera.up[0] * fwd[1] - camera.up[1] * fwd[0],
        ]);
        let up = [
            fwd[1] * right[2] - fwd[2] * right[1],
            fwd[2] * right[0] - fwd[0] * right[2],
            fwd[0] * right[1] - fwd[1] * right[0],
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

    // ── Text-label editor ──────────────────────────────────────────────────

    /// Returns the approximate pixel bounding box `(x, y, w, h)` of a label.
    /// Uses the actual rasterized dimensions when available, otherwise falls
    /// back to a rough `0.6 * font_size` per-character width estimate.
    fn label_bounds(label: &crate::text_label::TextLabel) -> (f32, f32, f32, f32) {
        let (w, h) = if label.rasterized_w > 0 && label.rasterized_h > 0 {
            (label.rasterized_w as f32, label.rasterized_h as f32)
        } else {
            let w = (label.text.chars().count() as f32 * label.font_size * 0.6).max(label.font_size);
            let h = label.font_size * 1.4;
            (w, h)
        };
        (label.x, label.y, w, h)
    }

    /// Returns the pixel rect `(x, y, w, h)` of the resize handle at the
    /// bottom-right corner of the label bounding box.
    fn label_resize_handle(label: &crate::text_label::TextLabel) -> (f32, f32, f32, f32) {
        let (lx, ly, lw, lh) = Self::label_bounds(label);
        let sz = (label.font_size * 0.35).max(10.0).min(20.0);
        (lx + lw - sz * 0.5, ly + lh - sz * 0.5, sz, sz)
    }

    /// Returns `true` if screen point `(px, py)` is inside the rect `(x, y, w, h)`.
    fn point_in_rect(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32) -> bool {
        px >= x && px <= x + w && py >= y && py <= y + h
    }

    /// Process a mouse/cursor event for the text-label editor layer.
    ///
    /// Must be called *before* [`Self::process`] so that `self.input` cursor
    /// position is still the previous value (used for hit-testing).
    ///
    /// Returns an [`EditorStateEvent`] when a label-editor state change occurred,
    /// or `None` when the event was not consumed by the label editor.
    pub fn process_overlay(
        &mut self,
        overlay: &mut crate::text_overlay::TextOverlay,
        event: &EditorEvent,
    ) -> Option<super::types::EditorStateEvent> {
        use super::types::EditorStateEvent;
        match event {
            // ── Use raw OS motion delta for label dragging ──────────────────────
            // `MouseMotionDelta` is a 1-to-1 physical/CSS-pixel delta straight
            // from the OS / browser, so labels always track the cursor exactly
            // regardless of any DPR / canvas-scale mismatch on absolute coords.
            EditorEvent::MouseMotionDelta { dx, dy } => {
                let drag = match self.label_drag.clone() {
                    Some(d) => d,
                    None => return None,
                };
                if let Some(label) = overlay.labels.get_mut(&drag.label_id) {
                    match drag.kind {
                        LabelDragKind::Move => {
                            label.x += dx;
                            label.y += dy;
                            label.dirty = true;
                            if let Some(sel) = &mut self.selected_label {
                                if sel.id == drag.label_id {
                                    sel.x = label.x;
                                    sel.y = label.y;
                                }
                            }
                            return self.selected_label.clone().map(EditorStateEvent::LabelMoved);
                        }
                        LabelDragKind::Resize => {
                            let new_size = (label.font_size + dx * 0.5).max(4.0);
                            label.font_size = new_size;
                            label.dirty = true;
                            if let Some(sel) = &mut self.selected_label {
                                if sel.id == drag.label_id {
                                    sel.font_size = new_size;
                                }
                            }
                            return self.selected_label.clone().map(EditorStateEvent::LabelResized);
                        }
                    }
                }
                None
            }

            // CursorMoved only tracks position for hit-testing;
            // label movement is driven by MouseMotionDelta above.
            EditorEvent::CursorMoved { .. } => None,

            EditorEvent::MouseButton { left: Some(true), .. } => {
                let (cx, cy) = (self.input.cursor_x, self.input.cursor_y);

                // Resize handle takes priority (only when a label is selected).
                if let Some(sel_id) = self.selected_label.as_ref().map(|l| l.id) {
                    if let Some(label) = overlay.labels.get(&sel_id) {
                        let (rx, ry, rw, rh) = Self::label_resize_handle(label);
                        if Self::point_in_rect(cx, cy, rx, ry, rw, rh) {
                            self.label_drag = Some(LabelDragState {
                                label_id: sel_id,
                                kind: LabelDragKind::Resize,
                                start_cursor: [cx, cy],
                                start_pos: [label.x, label.y],
                                start_size: label.font_size,
                            });
                            return None; // drag started, selection unchanged
                        }
                    }
                }

                // Hit-test all visible labels (sorted front-to-back by zindex).
                let mut candidates: Vec<_> = overlay.labels.values()
                    .filter(|l| l.visible)
                    .collect();
                candidates.sort_by(|a, b| b.zindex.cmp(&a.zindex));

                let mut hit_id: Option<usize> = None;
                for label in &candidates {
                    let (lx, ly, lw, lh) = Self::label_bounds(label);
                    if Self::point_in_rect(cx, cy, lx, ly, lw, lh) {
                        hit_id = Some(label.id);
                        break;
                    }
                }

                if let Some(id) = hit_id {
                    if let Some(label) = overlay.labels.get(&id) {
                        self.selected_label = Some(LabelInspectorData::from_label(label));
                        self.label_drag = Some(LabelDragState {
                            label_id: id,
                            kind: LabelDragKind::Move,
                            start_cursor: [cx, cy],
                            start_pos: [label.x, label.y],
                            start_size: label.font_size,
                        });
                    }
                    return Some(EditorStateEvent::LabelSelectionChanged(self.selected_label.clone()));
                }
                None
            }

            EditorEvent::MouseButton { left: Some(false), .. } => {
                self.label_drag = None;
                None
            }

            _ => None,
        }
    }

    /// Returns the pixel bounding box `(x, y, w, h)` of the currently-selected
    /// label, or `None` when nothing is selected.
    pub fn selected_label_bounds(
        &self,
        overlay: &crate::text_overlay::TextOverlay,
    ) -> Option<(f32, f32, f32, f32)> {
        let id = self.selected_label.as_ref()?.id;
        let label = overlay.labels.get(&id)?;
        Some(Self::label_bounds(label))
    }

    /// Returns the pixel rect of the resize handle for the currently-selected
    /// label, or `None` when nothing is selected.
    pub fn selected_label_resize_handle(
        &self,
        overlay: &crate::text_overlay::TextOverlay,
    ) -> Option<(f32, f32, f32, f32)> {
        let id = self.selected_label.as_ref()?.id;
        let label = overlay.labels.get(&id)?;
        Some(Self::label_resize_handle(label))
    }

    /// Build a screen-space overlay mesh (vertices + indices) that draws:
    /// * a 1-px cyan border around the selected label bounding box, and
    /// * a filled white square at the bottom-right as a resize handle.
    ///
    /// Returns `None` when no label is selected or it no longer exists.
    pub fn label_selection_overlay(
        &self,
        overlay: &crate::text_overlay::TextOverlay,
    ) -> Option<(Vec<crate::mesh::Vertex>, Vec<u32>)> {
        use crate::mesh::Vertex;
        let id = self.selected_label.as_ref()?.id;
        let label = overlay.labels.get(&id)?;
        let (lx, ly, lw, lh) = Self::label_bounds(label);
        let (rx, ry, rw, rh) = Self::label_resize_handle(label);

        let border = 1.5_f32; // px thickness of the selection border
        let cyan = [0.0_f32, 1.0, 1.0];
        let white = [1.0_f32, 1.0, 1.0];

        let mut verts: Vec<Vertex> = Vec::new();
        let mut idx: Vec<u32> = Vec::new();

        // Helper: push a solid colour screen-space rect (x, y, w, h).
        let mut push_rect = |verts: &mut Vec<Vertex>, idx: &mut Vec<u32>,
                             x: f32, y: f32, w: f32, h: f32, col: [f32; 3]| {
            let base = verts.len() as u32;
            verts.extend_from_slice(&[
                Vertex { position: [x, y, 0.0], color: col, uv: [0.0, 0.0] },
                Vertex { position: [x + w, y, 0.0], color: col, uv: [1.0, 0.0] },
                Vertex { position: [x + w, y + h, 0.0], color: col, uv: [1.0, 1.0] },
                Vertex { position: [x, y + h, 0.0], color: col, uv: [0.0, 1.0] },
            ]);
            idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        };

        // Border: 4 thin rects along each edge (top, bottom, left, right).
        push_rect(&mut verts, &mut idx, lx, ly, lw, border, cyan); // top
        push_rect(&mut verts, &mut idx, lx, ly + lh, lw, border, cyan); // bottom
        push_rect(&mut verts, &mut idx, lx, ly, border, lh, cyan); // left
        push_rect(&mut verts, &mut idx, lx + lw, ly, border, lh, cyan); // right

        // Resize handle (solid white square at bottom-right corner).
        push_rect(&mut verts, &mut idx, rx, ry, rw, rh, white);

        Some((verts, idx))
    }
}

