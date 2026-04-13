//! Gizmo and selection-box mesh builders.

use crate::geometry::Geometry;
use crate::mesh::{MeshData, Vertex};
use crate::transform::Transform;

/// Target arm length of the gizmo in screen pixels.
/// The gizmo will always appear this size regardless of camera distance.
pub(crate) const GIZMO_SCREEN_PX: f32 = 80.0;

/// Build the three-axis **translate** gizmo: thin box shafts ending in cone tips.
///
/// * X axis (red)   — cone pointing +X
/// * Y axis (green) — cone pointing +Y
/// * Z axis (blue)  — cone pointing +Z
pub fn build_gizmo_mesh_data(center: [f32; 3], scale: f32) -> (Vec<Vertex>, Vec<u32>) {
    let mut mesh    = MeshData::new();
    let shaft_h     = scale * 0.025;
    let cone_r      = scale * 0.09;
    let cone_h      = scale * 0.28;
    let shaft_len   = scale - cone_h;
    let dot_r       = scale * 0.07;
    let t = |pos: [f32; 3]| Transform::from_position(pos[0], pos[1], pos[2]);
    let [cx, cy, cz] = center;

    Geometry::Sphere { radius: dot_r, subdivisions: 8 }
        .generate_mesh_data(&mut mesh, &t(center), [0.9, 0.9, 0.9, 1.0]);

    // X (red)
    Geometry::Box { width: shaft_len, height: shaft_h*2.0, depth: shaft_h*2.0 }
        .generate_mesh_data(&mut mesh, &t([cx+shaft_len*0.5, cy, cz]), [0.95,0.15,0.15,1.0]);
    push_cone(&mut mesh,
        [cx+scale, cy, cz], [cx+scale-cone_h, cy, cz],
        [0.0,1.0,0.0], [0.0,0.0,1.0], cone_r, [0.95,0.15,0.15,1.0]);

    // Y (green)
    Geometry::Box { width: shaft_h*2.0, height: shaft_len, depth: shaft_h*2.0 }
        .generate_mesh_data(&mut mesh, &t([cx, cy+shaft_len*0.5, cz]), [0.15,0.95,0.15,1.0]);
    push_cone(&mut mesh,
        [cx, cy+scale, cz], [cx, cy+scale-cone_h, cz],
        [1.0,0.0,0.0], [0.0,0.0,1.0], cone_r, [0.15,0.95,0.15,1.0]);

    // Z (blue)
    Geometry::Box { width: shaft_h*2.0, height: shaft_h*2.0, depth: shaft_len }
        .generate_mesh_data(&mut mesh, &t([cx, cy, cz+shaft_len*0.5]), [0.15,0.15,0.95,1.0]);
    push_cone(&mut mesh,
        [cx, cy, cz+scale], [cx, cy, cz+scale-cone_h],
        [1.0,0.0,0.0], [0.0,1.0,0.0], cone_r, [0.15,0.15,0.95,1.0]);

    (mesh.vertices, mesh.indices)
}

/// Build the **rotate** gizmo: three coloured ring-tubes, one per axis.
///
/// * X ring (red)   — YZ plane, rotates around +X
/// * Y ring (green) — XZ plane, rotates around +Y
/// * Z ring (blue)  — XY plane, rotates around +Z
pub fn build_rotate_gizmo_mesh_data(center: [f32; 3], scale: f32) -> (Vec<Vertex>, Vec<u32>) {
    let mut mesh = MeshData::new();
    let ring_r   = scale;
    let tube_r   = scale * 0.045;
    let dot_r    = scale * 0.07;
    let t = |pos: [f32; 3]| Transform::from_position(pos[0], pos[1], pos[2]);

    Geometry::Sphere { radius: dot_r, subdivisions: 8 }
        .generate_mesh_data(&mut mesh, &t(center), [0.9, 0.9, 0.9, 1.0]);

    push_ring_tube(&mut mesh, center, ring_r, tube_r,
        [0.0,1.0,0.0], [0.0,0.0,1.0], [0.95,0.15,0.15,1.0]); // X (red)
    push_ring_tube(&mut mesh, center, ring_r, tube_r,
        [0.0,0.0,1.0], [1.0,0.0,0.0], [0.15,0.95,0.15,1.0]); // Y (green)
    push_ring_tube(&mut mesh, center, ring_r, tube_r,
        [1.0,0.0,0.0], [0.0,1.0,0.0], [0.15,0.15,0.95,1.0]); // Z (blue)

    (mesh.vertices, mesh.indices)
}

/// Build the **scale** gizmo: three axis shafts ending in coloured cubes.
///
/// * X axis (red)   — cube tip along +X
/// * Y axis (green) — cube tip along +Y
/// * Z axis (blue)  — cube tip along +Z
pub fn build_scale_gizmo_mesh_data(center: [f32; 3], scale: f32) -> (Vec<Vertex>, Vec<u32>) {
    let mut mesh  = MeshData::new();
    let shaft_h   = scale * 0.025;
    let cube_hs   = scale * 0.09;
    let shaft_len = scale - cube_hs * 2.0;
    let dot_r     = scale * 0.07;
    let t = |pos: [f32; 3]| Transform::from_position(pos[0], pos[1], pos[2]);
    let [cx, cy, cz] = center;

    Geometry::Sphere { radius: dot_r, subdivisions: 8 }
        .generate_mesh_data(&mut mesh, &t(center), [0.9, 0.9, 0.9, 1.0]);

    // X (red)
    Geometry::Box { width: shaft_len, height: shaft_h*2.0, depth: shaft_h*2.0 }
        .generate_mesh_data(&mut mesh, &t([cx+shaft_len*0.5, cy, cz]), [0.95,0.15,0.15,1.0]);
    Geometry::Box { width: cube_hs*2.0, height: cube_hs*2.0, depth: cube_hs*2.0 }
        .generate_mesh_data(&mut mesh, &t([cx+scale, cy, cz]), [0.95,0.15,0.15,1.0]);

    // Y (green)
    Geometry::Box { width: shaft_h*2.0, height: shaft_len, depth: shaft_h*2.0 }
        .generate_mesh_data(&mut mesh, &t([cx, cy+shaft_len*0.5, cz]), [0.15,0.95,0.15,1.0]);
    Geometry::Box { width: cube_hs*2.0, height: cube_hs*2.0, depth: cube_hs*2.0 }
        .generate_mesh_data(&mut mesh, &t([cx, cy+scale, cz]), [0.15,0.95,0.15,1.0]);

    // Z (blue)
    Geometry::Box { width: shaft_h*2.0, height: shaft_h*2.0, depth: shaft_len }
        .generate_mesh_data(&mut mesh, &t([cx, cy, cz+shaft_len*0.5]), [0.15,0.15,0.95,1.0]);
    Geometry::Box { width: cube_hs*2.0, height: cube_hs*2.0, depth: cube_hs*2.0 }
        .generate_mesh_data(&mut mesh, &t([cx, cy, cz+scale]), [0.15,0.15,0.95,1.0]);

    (mesh.vertices, mesh.indices)
}

/// Build a golden wireframe bounding-box cage (12 edges as thin box prisms).
/// Rendered via the overlay pipeline so it is always visible through objects.
pub fn build_selection_box(center: [f32; 3], half: [f32; 3]) -> (Vec<Vertex>, Vec<u32>) {
    let mut mesh = MeshData::new();
    let color    = [1.0_f32, 0.85, 0.1, 1.0];
    let pad      = (half[0]+half[1]+half[2]) / 3.0 * 0.06;
    let [hx, hy, hz] = [half[0]+pad, half[1]+pad, half[2]+pad];
    let [cx, cy, cz] = center;
    let tk = ((hx+hy+hz) / 3.0 * 0.045).max(0.02);
    let tr = |px: f32, py: f32, pz: f32| Transform::from_position(px, py, pz);

    // Bottom 4 edges
    Geometry::Box { width: hx*2.0, height: tk, depth: tk }
        .generate_mesh_data(&mut mesh, &tr(cx,      cy-hy, cz-hz), color);
    Geometry::Box { width: hx*2.0, height: tk, depth: tk }
        .generate_mesh_data(&mut mesh, &tr(cx,      cy-hy, cz+hz), color);
    Geometry::Box { width: tk, height: tk, depth: hz*2.0 }
        .generate_mesh_data(&mut mesh, &tr(cx-hx,   cy-hy, cz),    color);
    Geometry::Box { width: tk, height: tk, depth: hz*2.0 }
        .generate_mesh_data(&mut mesh, &tr(cx+hx,   cy-hy, cz),    color);
    // Top 4 edges
    Geometry::Box { width: hx*2.0, height: tk, depth: tk }
        .generate_mesh_data(&mut mesh, &tr(cx,      cy+hy, cz-hz), color);
    Geometry::Box { width: hx*2.0, height: tk, depth: tk }
        .generate_mesh_data(&mut mesh, &tr(cx,      cy+hy, cz+hz), color);
    Geometry::Box { width: tk, height: tk, depth: hz*2.0 }
        .generate_mesh_data(&mut mesh, &tr(cx-hx,   cy+hy, cz),    color);
    Geometry::Box { width: tk, height: tk, depth: hz*2.0 }
        .generate_mesh_data(&mut mesh, &tr(cx+hx,   cy+hy, cz),    color);
    // 4 vertical edges
    Geometry::Box { width: tk, height: hy*2.0, depth: tk }
        .generate_mesh_data(&mut mesh, &tr(cx-hx,   cy, cz-hz), color);
    Geometry::Box { width: tk, height: hy*2.0, depth: tk }
        .generate_mesh_data(&mut mesh, &tr(cx+hx,   cy, cz-hz), color);
    Geometry::Box { width: tk, height: hy*2.0, depth: tk }
        .generate_mesh_data(&mut mesh, &tr(cx-hx,   cy, cz+hz), color);
    Geometry::Box { width: tk, height: hy*2.0, depth: tk }
        .generate_mesh_data(&mut mesh, &tr(cx+hx,   cy, cz+hz), color);

    (mesh.vertices, mesh.indices)
}

/// Build the skybox mesh — a large box visible from inside, rendered with
/// the overlay pipeline (`cull_mode: None`).
pub fn build_skybox_mesh() -> (Vec<Vertex>, Vec<u32>) {
    let mut mesh = MeshData::new();
    let s   = 450.0_f32;
    let top = [0.08, 0.12, 0.22, 1.0_f32];
    let mid = [0.10, 0.13, 0.20, 1.0];
    let bot = [0.05, 0.06, 0.09, 1.0];

    mesh.push_quad([[-s,s,-s],[s,s,-s],[s,s,s],[-s,s,s]], top);
    mesh.push_quad([[-s,-s,s],[s,-s,s],[s,-s,-s],[-s,-s,-s]], bot);
    mesh.push_quad([[-s,s,-s],[-s,-s,-s],[s,-s,-s],[s,s,-s]], mid);
    mesh.push_quad([[s,s,s],[s,-s,s],[-s,-s,s],[-s,s,s]], mid);
    mesh.push_quad([[s,s,-s],[s,-s,-s],[s,-s,s],[s,s,s]], mid);
    mesh.push_quad([[-s,s,s],[-s,-s,s],[-s,-s,-s],[-s,s,-s]], mid);

    (mesh.vertices, mesh.indices)
}

/// Append a torus-tube ring segment into `mesh`.
fn push_ring_tube(
    mesh:   &mut MeshData,
    center: [f32; 3],
    ring_r: f32,
    tube_r: f32,
    perp1:  [f32; 3],
    perp2:  [f32; 3],
    color:  [f32; 4],
) {
    const SEGS: usize = 40;
    let ax = [
        perp1[1]*perp2[2] - perp1[2]*perp2[1],
        perp1[2]*perp2[0] - perp1[0]*perp2[2],
        perp1[0]*perp2[1] - perp1[1]*perp2[0],
    ];
    let step = std::f32::consts::PI * 2.0 / SEGS as f32;
    for i in 0..SEGS {
        let (c1,s1) = ((i as f32*step).cos(), (i as f32*step).sin());
        let (c2,s2) = (((i+1) as f32*step).cos(), ((i+1) as f32*step).sin());
        let mp1 = [center[0]+(perp1[0]*c1+perp2[0]*s1)*ring_r,
                   center[1]+(perp1[1]*c1+perp2[1]*s1)*ring_r,
                   center[2]+(perp1[2]*c1+perp2[2]*s1)*ring_r];
        let mp2 = [center[0]+(perp1[0]*c2+perp2[0]*s2)*ring_r,
                   center[1]+(perp1[1]*c2+perp2[1]*s2)*ring_r,
                   center[2]+(perp1[2]*c2+perp2[2]*s2)*ring_r];
        let o1  = [perp1[0]*c1+perp2[0]*s1, perp1[1]*c1+perp2[1]*s1, perp1[2]*c1+perp2[2]*s1];
        let o2  = [perp1[0]*c2+perp2[0]*s2, perp1[1]*c2+perp2[1]*s2, perp1[2]*c2+perp2[2]*s2];
        let a1v = [mp1[0]+ax[0]*tube_r+o1[0]*tube_r, mp1[1]+ax[1]*tube_r+o1[1]*tube_r, mp1[2]+ax[2]*tube_r+o1[2]*tube_r];
        let b1v = [mp1[0]+ax[0]*tube_r-o1[0]*tube_r, mp1[1]+ax[1]*tube_r-o1[1]*tube_r, mp1[2]+ax[2]*tube_r-o1[2]*tube_r];
        let c1v = [mp1[0]-ax[0]*tube_r-o1[0]*tube_r, mp1[1]-ax[1]*tube_r-o1[1]*tube_r, mp1[2]-ax[2]*tube_r-o1[2]*tube_r];
        let d1v = [mp1[0]-ax[0]*tube_r+o1[0]*tube_r, mp1[1]-ax[1]*tube_r+o1[1]*tube_r, mp1[2]-ax[2]*tube_r+o1[2]*tube_r];
        let a2v = [mp2[0]+ax[0]*tube_r+o2[0]*tube_r, mp2[1]+ax[1]*tube_r+o2[1]*tube_r, mp2[2]+ax[2]*tube_r+o2[2]*tube_r];
        let b2v = [mp2[0]+ax[0]*tube_r-o2[0]*tube_r, mp2[1]+ax[1]*tube_r-o2[1]*tube_r, mp2[2]+ax[2]*tube_r-o2[2]*tube_r];
        let c2v = [mp2[0]-ax[0]*tube_r-o2[0]*tube_r, mp2[1]-ax[1]*tube_r-o2[1]*tube_r, mp2[2]-ax[2]*tube_r-o2[2]*tube_r];
        let d2v = [mp2[0]-ax[0]*tube_r+o2[0]*tube_r, mp2[1]-ax[1]*tube_r+o2[1]*tube_r, mp2[2]-ax[2]*tube_r+o2[2]*tube_r];
        mesh.push_quad([a1v, a2v, b2v, b1v], color);
        mesh.push_quad([d1v, c1v, c2v, d2v], color);
        mesh.push_quad([a1v, d1v, d2v, a2v], color);
        mesh.push_quad([b2v, c2v, c1v, b1v], color);
    }
}

/// Append a cone into `mesh`.
fn push_cone(
    mesh: &mut MeshData,
    tip: [f32; 3], base: [f32; 3],
    perp1: [f32; 3], perp2: [f32; 3],
    radius: f32, color: [f32; 4],
) {
    const SEGS: usize = 10;
    let step = std::f32::consts::PI * 2.0 / SEGS as f32;
    for i in 0..SEGS {
        let (c1,s1) = (((i   as f32)*step).cos()*radius, (( i   as f32)*step).sin()*radius);
        let (c2,s2) = ((((i+1) as f32)*step).cos()*radius, (((i+1) as f32)*step).sin()*radius);
        let v1 = [base[0]+perp1[0]*c1+perp2[0]*s1, base[1]+perp1[1]*c1+perp2[1]*s1, base[2]+perp1[2]*c1+perp2[2]*s1];
        let v2 = [base[0]+perp1[0]*c2+perp2[0]*s2, base[1]+perp1[1]*c2+perp2[1]*s2, base[2]+perp1[2]*c2+perp2[2]*s2];
        mesh.push_triangle([v1, tip, v2], color);
        mesh.push_triangle([base, v2, v1], color);
    }
}

