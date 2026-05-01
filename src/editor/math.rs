//! Internal math / geometry / ray-test helpers and hierarchy utilities.

use std::collections::HashSet;

use crate::geometry::Geometry;
use crate::transform::Transform;
use crate::world::World;

#[inline] pub(crate) fn v3_len(v: [f32;3]) -> f32 { (v[0]*v[0]+v[1]*v[1]+v[2]*v[2]).sqrt() }
#[inline] pub(crate) fn v3_sub(a:[f32;3], b:[f32;3]) -> [f32;3] { [a[0]-b[0],a[1]-b[1],a[2]-b[2]] }
#[inline] pub(crate) fn v3_add(a:[f32;3], b:[f32;3]) -> [f32;3] { [a[0]+b[0],a[1]+b[1],a[2]+b[2]] }
#[inline] pub(crate) fn v3_norm(v:[f32;3]) -> [f32;3] { let l=v3_len(v).max(1e-6); [v[0]/l,v[1]/l,v[2]/l] }

/// Approximate bounding-sphere radius of `geom` in world space.
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

/// Axis-aligned half-extents of `geom` in world space (accounts for scale).
pub(crate) fn approx_half_extents(geom: &Option<Geometry>, t: &Transform) -> [f32; 3] {
    let base: [f32; 3] = match geom {
        Some(Geometry::Sphere  { radius, .. })           => [*radius; 3],
        Some(Geometry::Cube    { size })                 => [*size * 0.5; 3],
        Some(Geometry::Box     { width, height, depth }) => [*width*0.5, *height*0.5, *depth*0.5],
        Some(Geometry::Plane   { size })                 => [*size*0.5, 0.01, *size*0.5],
        Some(Geometry::Pyramid { base_size, height })    => [*base_size*0.5, *height*0.5, *base_size*0.5],
        Some(Geometry::Capsule { radius, height, .. })   => [*radius, *height*0.5 + *radius, *radius],
        None                                             => [0.5; 3],
    };
    [
        (base[0] * t.scale[0]).max(0.05),
        (base[1] * t.scale[1]).max(0.05),
        (base[2] * t.scale[2]).max(0.05),
    ]
}

/// Ray–AABB intersection using the slab method.
///
/// `center` is the world-space centre of the box; `half` is the per-axis
/// half-extent **already scaled to world space**.  Returns the nearest
/// positive `t` along the ray, or `None` on a miss.
///
/// Unlike [`ray_sphere`], each axis is tested independently, so a box that
/// was scaled only on X is only larger on X, not in every direction.
pub(crate) fn ray_aabb(ro: [f32; 3], rd: [f32; 3], center: [f32; 3], half: [f32; 3]) -> Option<f32> {
    let mut t_min = f32::NEG_INFINITY;
    let mut t_max = f32::INFINITY;

    for i in 0..3 {
        let lo = center[i] - half[i];
        let hi = center[i] + half[i];

        if rd[i].abs() < 1e-6 {
            // Ray is parallel to this slab, miss if origin is outside.
            if ro[i] < lo || ro[i] > hi { return None; }
        } else {
            let inv = 1.0 / rd[i];
            let (t1, t2) = {
                let a = (lo - ro[i]) * inv;
                let b = (hi - ro[i]) * inv;
                if a < b { (a, b) } else { (b, a) }
            };
            t_min = t_min.max(t1);
            t_max = t_max.min(t2);
            if t_max < t_min { return None; }
        }
    }

    if t_max < 0.0 { return None; }           // box is entirely behind the ray
    Some(if t_min >= 0.0 { t_min } else { t_max })
}

pub(crate) fn ray_sphere(o:[f32;3], d:[f32;3], c:[f32;3], r:f32) -> Option<f32> {
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

/// Ray–ring (torus) intersection used for the rotate-gizmo hit test.
pub(crate) fn ray_ring(ro:[f32;3], rd:[f32;3], c:[f32;3], n:[f32;3], r:f32, w:f32) -> Option<f32> {
    let mut best_t = f32::MAX;
    let oc  = [c[0]-ro[0], c[1]-ro[1], c[2]-ro[2]];
    let den = rd[0]*n[0] + rd[1]*n[1] + rd[2]*n[2];

    let mut check_t = |t: f32| {
        if t > 0.0 {
            let p       = [ro[0]+rd[0]*t, ro[1]+rd[1]*t, ro[2]+rd[2]*t];
            let pl      = [p[0]-c[0], p[1]-c[1], p[2]-c[2]];
            let y       = pl[0]*n[0] + pl[1]*n[1] + pl[2]*n[2];
            let pp      = [pl[0]-y*n[0], pl[1]-y*n[1], pl[2]-y*n[2]];
            let d_plane = (pp[0]*pp[0] + pp[1]*pp[1] + pp[2]*pp[2]).sqrt();
            let dist    = ((d_plane-r).powi(2) + y.powi(2)).sqrt();
            if dist <= w && t < best_t { best_t = t; }
        }
    };

    if den.abs() > 1e-6 {
        check_t((oc[0]*n[0] + oc[1]*n[1] + oc[2]*n[2]) / den);
    }
    let tc   = oc[0]*rd[0] + oc[1]*rd[1] + oc[2]*rd[2];
    let b    = -2.0 * tc;
    let cc   = (oc[0]*oc[0] + oc[1]*oc[1] + oc[2]*oc[2]) - (r+w).powi(2);
    let disc = b*b - 4.0*cc;
    if disc >= 0.0 {
        let s  = disc.sqrt();
        let t1 = (-b-s)/2.0;
        let t2 = (-b+s)/2.0;
        const SAMPLES: usize = 12;
        let step = (t2-t1) / SAMPLES as f32;
        for i in 0..=SAMPLES { check_t(t1 + step * i as f32); }
    }
    if best_t < f32::MAX { Some(best_t) } else { None }
}

/// Compute the combined world-space transform of `id` by accumulating parent
/// transforms up the hierarchy.
pub(crate) fn compute_world_transform(world: &World, id: usize) -> Transform {
    if let Some(obj) = world.objects.get(&id) {
        match obj.parent {
            None         => obj.transform.clone(),
            Some(pid)    => compute_world_transform(world, pid).combine(&obj.transform),
        }
    } else {
        Transform::default()
    }
}

/// Recursively collect `id` and every descendant into `out`.
pub(crate) fn collect_descendants(world: &World, id: usize, out: &mut Vec<usize>) {
    out.push(id);
    if let Some(obj) = world.objects.get(&id) {
        for &child_id in &obj.children {
            collect_descendants(world, child_id, out);
        }
    }
}

/// Returns only the "top-level" IDs, those whose ancestor is not also in
/// the set (prevents double-transforming children of a selected parent).
pub(crate) fn filter_top_level_ids(world: &World, ids: &[usize]) -> Vec<usize> {
    let id_set: HashSet<usize> = ids.iter().cloned().collect();
    ids.iter()
        .filter(|&&id| !has_selected_ancestor(world, id, &id_set))
        .cloned()
        .collect()
}

fn has_selected_ancestor(world: &World, id: usize, selected: &HashSet<usize>) -> bool {
    let mut cur = id;
    loop {
        match world.objects.get(&cur).and_then(|o| o.parent) {
            None      => return false,
            Some(pid) => {
                if selected.contains(&pid) { return true; }
                cur = pid;
            }
        }
    }
}

/// Compute the AABB that encloses all objects in `ids`.
/// Returns `None` only when `ids` is empty.
pub(crate) fn combined_aabb(world: &World, ids: &[usize]) -> Option<([f32; 3], [f32; 3])> {
    if ids.is_empty() { return None; }
    let mut mn = [f32::INFINITY;     3];
    let mut mx = [f32::NEG_INFINITY; 3];
    for &id in ids {
        let wt   = compute_world_transform(world, id);
        let geom = world.objects.get(&id).and_then(|o| o.geometry.clone());
        let half = approx_half_extents(&geom, &wt);
        let c    = wt.position;
        for i in 0..3 {
            mn[i] = mn[i].min(c[i] - half[i]);
            mx[i] = mx[i].max(c[i] + half[i]);
        }
    }
    Some((mn, mx))
}

