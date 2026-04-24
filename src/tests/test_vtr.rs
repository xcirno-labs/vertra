/// Tests for the VTR binary scene format (vtr.rs).
///
/// All tests operate on in-memory buffers (`Vec<u8>` / `std::io::Cursor`) so
/// no filesystem I/O is required and the suite runs fully offline.
///
/// Coverage:
///   - round-trip: empty scene, single object, full hierarchy
///   - every Geometry variant
///   - camera field fidelity
///   - root ordering preservation
///   - next_id continuity after load (no ID collision on spawn)
///   - deterministic / idempotent output
///   - header-only reads (`read_header`)
///   - `VtrHeader::engine_version_string`
///   - error path: bad magic bytes
///   - error path: unsupported format version
///   - error path: unknown geometry tag
///   - error path: truncated data (unexpected EOF)
///   - UTF-8 object names (including multibyte characters)
///   - object with long names (including a 300-byte name)
///   - deeply nested hierarchy (3 levels)
///   - multiple root objects, order preserved
///   - object with no geometry but non-default transform and color
///   - object deleted after load doesn't affect sibling IDs

use std::io::Cursor;
use crate::camera::Camera;
use crate::geometry::Geometry;
use crate::objects::{Object, ObjectConstructor};
use crate::transform::Transform;
use crate::vtr::{
    self, ENGINE_VERSION_MAJOR, ENGINE_VERSION_MINOR, ENGINE_VERSION_PATCH, FORMAT_VERSION, MAGIC,
};
use crate::world::World;

// helpers
/// Build a default Camera for tests.
fn test_camera() -> Camera {
    Camera::new()
}

/// Build a Camera with non-default values so we can verify all fields survive.
fn custom_camera() -> Camera {
    Camera {
        eye: [1.0, 2.0, 3.0],
        target: [4.0, 5.0, 6.0],
        up: [0.0, 1.0, 0.0],
        aspect: 16.0 / 9.0,
        fov: 60.0,
        znear: 0.01,
        zfar: 500.0,
        lr_rot: 45.0,
        ud_rot: -15.0,
    }
}

/// Serialize (camera, world) → Vec<u8> via the in-memory write path.
fn serialize(camera: &Camera, world: &World) -> Vec<u8> {
    let mut buf = Vec::new();
    vtr::write(&mut buf, camera, world).expect("write failed");
    buf
}

/// Deserialize a byte slice back into SceneData.
fn deserialize(bytes: &[u8]) -> vtr::SceneData {
    let mut cur = Cursor::new(bytes);
    vtr::read(&mut cur).expect("read failed")
}

/// Round-trip helper: write then read, returning SceneData.
fn roundtrip(camera: &Camera, world: &World) -> vtr::SceneData {
    let bytes = serialize(camera, world);
    deserialize(&bytes)
}

/// Assert two cameras are field-for-field equal.
fn assert_cameras_eq(a: &Camera, b: &Camera) {
    assert_eq!(a.eye, b.eye, "eye mismatch");
    assert_eq!(a.target, b.target, "target mismatch");
    assert_eq!(a.up, b.up, "up mismatch");
    assert_eq!(a.aspect, b.aspect, "aspect mismatch");
    assert_eq!(a.fov, b.fov, "fov mismatch");
    assert_eq!(a.znear, b.znear, "znear mismatch");
    assert_eq!(a.zfar, b.zfar, "zfar mismatch");
    assert_eq!(a.lr_rot, b.lr_rot, "lr_rot mismatch");
    assert_eq!(a.ud_rot, b.ud_rot, "ud_rot mismatch");
}

/// Assert two objects are field-for-field equal.
fn assert_objects_eq(a: &Object, b: &Object) {
    assert_eq!(a.name, b.name, "name mismatch");
    assert_eq!(a.transform, b.transform, "transform mismatch");
    assert_eq!(a.geometry, b.geometry, "geometry mismatch");
    assert_eq!(a.color, b.color, "color mismatch");
    assert_eq!(a.parent, b.parent, "parent mismatch");
    // Sort children before comparing - insertion order may differ on reload.
    let mut ca = a.children.clone();
    let mut cb = b.children.clone();
    ca.sort_unstable();
    cb.sort_unstable();
    assert_eq!(ca, cb, "children mismatch");
}

// header tests
#[test]
fn header_magic_and_version() {
    let world = World::new();
    let bytes = serialize(&test_camera(), &world);

    // First 4 bytes must be the magic constant.
    assert_eq!(&bytes[0..4], &MAGIC, "magic bytes wrong");

    // Bytes [4..6] are format_version in little-endian.
    let fv = u16::from_le_bytes([bytes[4], bytes[5]]);
    assert_eq!(fv, FORMAT_VERSION);
}

#[test]
fn header_engine_version_fields() {
    let world = World::new();
    let bytes = serialize(&test_camera(), &world);

    let major = u16::from_le_bytes([bytes[6], bytes[7]]);
    let minor = u16::from_le_bytes([bytes[8], bytes[9]]);
    let patch = u16::from_le_bytes([bytes[10], bytes[11]]);

    assert_eq!(major, ENGINE_VERSION_MAJOR);
    assert_eq!(minor, ENGINE_VERSION_MINOR);
    assert_eq!(patch, ENGINE_VERSION_PATCH);
}

#[test]
fn header_flags_reserved_zero() {
    let world = World::new();
    let bytes = serialize(&test_camera(), &world);

    let flags = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
    assert_eq!(flags, 0, "reserved flags must be zero");
}

#[test]
fn read_header_only() {
    let mut world = World::new();
    world.spawn_object(Object::default(), None);
    world.spawn_object(Object::default(), None);

    let bytes = serialize(&test_camera(), &world);
    let mut cur = Cursor::new(&bytes[..]);
    let hdr = vtr::read_header(&mut cur).expect("read_header failed");

    assert_eq!(hdr.format_version, FORMAT_VERSION);
    assert_eq!(hdr.engine_major, ENGINE_VERSION_MAJOR);
    assert_eq!(hdr.engine_minor, ENGINE_VERSION_MINOR);
    assert_eq!(hdr.engine_patch, ENGINE_VERSION_PATCH);
    assert_eq!(hdr.object_count, 2);
}

#[test]
fn header_engine_version_string() {
    let hdr = vtr::VtrHeader {
        format_version: FORMAT_VERSION,
        engine_major: 1,
        engine_minor: 2,
        engine_patch: 3,
        object_count: 0,
    };
    assert_eq!(hdr.engine_version_string(), "1.2.3");
}

// empty scene
#[test]
fn empty_scene_roundtrip() {
    let camera = custom_camera();
    let world = World::new();
    let data = roundtrip(&camera, &world);

    assert_cameras_eq(&camera, &data.camera);
    assert!(data.world.objects.is_empty());
    assert!(data.world.roots.is_empty());
}

#[test]
fn empty_scene_minimum_size() {
    // header(20) + camera(60) + roots_count(4) = 84 bytes minimum
    let bytes = serialize(&test_camera(), &World::new());
    assert_eq!(bytes.len(), 84, "minimum file size should be 84 bytes");
}

// camera round-trip
#[test]
fn camera_all_fields_preserved() {
    let camera = custom_camera();
    let data = roundtrip(&camera, &World::new());
    assert_cameras_eq(&camera, &data.camera);
}

#[test]
fn camera_negative_values() {
    let camera = Camera {
        eye: [-100.5, -0.001, -999.9],
        target: [-1.0, -2.0, -3.0],
        up: [0.0, -1.0, 0.0],
        aspect: 0.5625,
        fov: 120.0,
        znear: 0.001,
        zfar: 10_000.0,
        lr_rot: -180.0,
        ud_rot: -89.0,
    };
    let data = roundtrip(&camera, &World::new());
    assert_cameras_eq(&camera, &data.camera);
}

// single object
#[test]
fn single_object_no_geometry() {
    let mut world = World::new();
    let id = world.spawn_object(
        Object::new(ObjectConstructor {
            name: "Bare Box".to_string(),
            transform: Some(Transform {
                position: [1.0, 2.0, 3.0],
                rotation: [10.0, 20.0, 30.0],
                scale: [2.0, 0.5, 1.0],
            }),
            geometry: None,
            color: Some([0.1, 0.2, 0.3, 0.9]),
            str_id: None,
            texture_path: None,
        }),
        None,
    );

    let data = roundtrip(&test_camera(), &world);
    let obj = data.world.objects.get(&id).expect("object missing after roundtrip");

    assert_objects_eq(obj, world.objects.get(&id).unwrap());
    assert_eq!(data.world.roots, vec![id]);
}

#[test]
fn single_object_default() {
    let mut world = World::new();
    let id = world.spawn_object(Object::default(), None);

    let data = roundtrip(&test_camera(), &world);
    let original = world.objects.get(&id).unwrap();
    let loaded = data.world.objects.get(&id).unwrap();

    assert_objects_eq(loaded, original);
}

// geometry variants
fn roundtrip_geometry(geom: Geometry) -> Option<Geometry> {
    let mut world = World::new();
    world.spawn_object(
        Object::new(ObjectConstructor {
            name: "geo test".to_string(),
            transform: None,
            geometry: Some(geom),
            color: None,
            str_id: None,
            texture_path: None,
        }),
        None,
    );
    let data = roundtrip(&test_camera(), &world);
    data.world.objects.into_values().next().unwrap().geometry
}

#[test]
fn geometry_none_roundtrip() {
    let mut world = World::new();
    let id = world.spawn_object(Object::default(), None); // geometry = None by default
    let data = roundtrip(&test_camera(), &world);
    assert!(data.world.objects[&id].geometry.is_none());
}

#[test]
fn geometry_cube_roundtrip() {
    let g = Geometry::Cube { size: 3.14 };
    assert_eq!(roundtrip_geometry(g.clone()), Some(g));
}

#[test]
fn geometry_box_roundtrip() {
    let g = Geometry::Box { width: 1.0, height: 2.5, depth: 0.75 };
    assert_eq!(roundtrip_geometry(g.clone()), Some(g));
}

#[test]
fn geometry_plane_roundtrip() {
    let g = Geometry::Plane { size: 10.0 };
    assert_eq!(roundtrip_geometry(g.clone()), Some(g));
}

#[test]
fn geometry_pyramid_roundtrip() {
    let g = Geometry::Pyramid { base_size: 4.0, height: 6.0 };
    assert_eq!(roundtrip_geometry(g.clone()), Some(g));
}

#[test]
fn geometry_capsule_roundtrip() {
    let g = Geometry::Capsule { radius: 0.5, height: 2.0, subdivisions: 16 };
    assert_eq!(roundtrip_geometry(g.clone()), Some(g));
}

#[test]
fn geometry_sphere_roundtrip() {
    let g = Geometry::Sphere { radius: 1.0, subdivisions: 32 };
    assert_eq!(roundtrip_geometry(g.clone()), Some(g));
}

#[test]
fn geometry_capsule_large_subdivisions() {
    let g = Geometry::Capsule { radius: 1.0, height: 5.0, subdivisions: 256 };
    assert_eq!(roundtrip_geometry(g.clone()), Some(g));
}

// hierarchy tests
#[test]
fn parent_child_roundtrip() {
    let mut world = World::new();
    let parent_id = world.spawn_object(
        Object::new(ObjectConstructor {
            name: "Parent".to_string(),
            transform: None,
            geometry: Some(Geometry::Cube { size: 1.0 }),
            color: Some([1.0, 0.0, 0.0, 1.0]),
            str_id: None,
            texture_path: None,
        }),
        None,
    );
    let child_id = world.spawn_object(
        Object::new(ObjectConstructor {
            name: "Child".to_string(),
            transform: Some(Transform::from_position(5.0, 0.0, 0.0)),
            geometry: Some(Geometry::Sphere { radius: 0.5, subdivisions: 8 }),
            color: Some([0.0, 1.0, 0.0, 1.0]),
            str_id: None,
            texture_path: None,
        }),
        Some(parent_id),
    );

    let data = roundtrip(&test_camera(), &world);

    // Hierarchy links preserved.
    let loaded_parent = &data.world.objects[&parent_id];
    let loaded_child = &data.world.objects[&child_id];

    assert!(loaded_parent.children.contains(&child_id), "child not linked to parent");
    assert_eq!(loaded_child.parent, Some(parent_id), "child's parent pointer wrong");

    // Only the true root appears in roots.
    assert_eq!(data.world.roots, vec![parent_id]);

    assert_objects_eq(loaded_parent, &world.objects[&parent_id]);
    assert_objects_eq(loaded_child, &world.objects[&child_id]);
}

#[test]
fn deep_three_level_hierarchy() {
    let mut world = World::new();
    let sun_id = world.spawn_object(
        Object::new(ObjectConstructor {
            name: "Sun".to_string(),
            transform: None,
            geometry: Some(Geometry::Sphere { radius: 2.0, subdivisions: 32 }),
            color: Some([1.0, 0.9, 0.2, 1.0]),
            str_id: None,
            texture_path: None,
        }),
        None,
    );
    let planet_id = world.spawn_object(
        Object::new(ObjectConstructor {
            name: "Planet".to_string(),
            transform: Some(Transform::from_position(6.0, 0.0, 0.0)),
            geometry: Some(Geometry::Sphere { radius: 0.8, subdivisions: 24 }),
            color: Some([0.2, 0.5, 1.0, 1.0]),
            str_id: None,
            texture_path: None,
        }),
        Some(sun_id),
    );
    let moon_id = world.spawn_object(
        Object::new(ObjectConstructor {
            name: "Moon".to_string(),
            transform: Some(Transform::from_position(1.5, 0.0, 0.0)),
            geometry: Some(Geometry::Sphere { radius: 0.3, subdivisions: 16 }),
            color: Some([0.7, 0.7, 0.7, 1.0]),
            str_id: None,
            texture_path: None,
        }),
        Some(planet_id),
    );

    let data = roundtrip(&test_camera(), &world);

    assert_eq!(data.world.roots, vec![sun_id], "only sun is root");
    assert_eq!(data.world.objects[&sun_id].children, vec![planet_id]);
    assert_eq!(data.world.objects[&planet_id].children, vec![moon_id]);
    assert_eq!(data.world.objects[&moon_id].children, vec![]);
    assert_eq!(data.world.objects[&moon_id].parent, Some(planet_id));
}

#[test]
fn multiple_roots_order_preserved() {
    let mut world = World::new();
    // Spawn 5 roots - the serializer must restore them in the same order.
    let ids: Vec<usize> = (0..5)
        .map(|i| {
            world.spawn_object(
                Object::new(ObjectConstructor {
                    name: format!("Root{i}"),
                    transform: None,
                    geometry: None,
                    color: None,
                    str_id: None,
                    texture_path: None,
                }),
                None,
            )
        })
        .collect();

    let data = roundtrip(&test_camera(), &world);
    assert_eq!(data.world.roots, ids, "root insertion order not preserved");
}

#[test]
fn multiple_roots_with_children() {
    let mut world = World::new();
    let r1 = world.spawn_object(Object::default(), None);
    let r2 = world.spawn_object(Object::default(), None);
    let c1 = world.spawn_object(Object::default(), Some(r1));
    let c2 = world.spawn_object(Object::default(), Some(r2));
    let c3 = world.spawn_object(Object::default(), Some(r1)); // second child of r1

    let data = roundtrip(&test_camera(), &world);

    assert_eq!(data.world.roots, vec![r1, r2]);
    assert!(data.world.objects[&r1].children.contains(&c1));
    assert!(data.world.objects[&r1].children.contains(&c3));
    assert!(data.world.objects[&r2].children.contains(&c2));
    assert_eq!(data.world.objects[&c1].parent, Some(r1));
    assert_eq!(data.world.objects[&c2].parent, Some(r2));
    assert_eq!(data.world.objects[&c3].parent, Some(r1));
}

// ID continuity after load
#[test]
fn next_id_after_load_does_not_collide() {
    let mut world = World::new();
    let existing_ids: Vec<usize> = (0..3).map(|_| world.spawn_object(Object::default(), None)).collect();

    let data = roundtrip(&test_camera(), &world);
    let mut loaded_world = data.world;

    // Spawn a new object after loading - its ID must not collide.
    let new_id = loaded_world.spawn_object(Object::default(), None);
    assert!(
        !existing_ids.contains(&new_id),
        "new ID {new_id} collides with a loaded object ID"
    );
    assert_eq!(
        loaded_world.objects.len(),
        4,
        "should have 3 loaded + 1 new object"
    );
}

#[test]
fn spawn_after_load_links_correctly() {
    let mut world = World::new();
    let root_id = world.spawn_object(Object::default(), None);

    let data = roundtrip(&test_camera(), &world);
    let mut loaded = data.world;

    let child_id = loaded.spawn_object(Object::default(), Some(root_id));

    assert!(loaded.objects[&root_id].children.contains(&child_id));
    assert_eq!(loaded.objects[&child_id].parent, Some(root_id));
}

// object name tests
#[test]
fn empty_name_roundtrip() {
    let mut world = World::new();
    let id = world.spawn_object(
        Object::new(ObjectConstructor {
            name: String::new(),
            transform: None,
            geometry: None,
            color: None,
            str_id: None,
            texture_path: None,
        }),
        None,
    );
    let data = roundtrip(&test_camera(), &world);
    assert_eq!(data.world.objects[&id].name, "");
}

#[test]
fn unicode_name_roundtrip() {
    let name = "太陽 ☀ Soleil 🌍".to_string();
    let mut world = World::new();
    let id = world.spawn_object(
        Object::new(ObjectConstructor {
            name: name.clone(),
            transform: None,
            geometry: None,
            color: None,
            str_id: None,
            texture_path: None,
        }),
        None,
    );
    let data = roundtrip(&test_camera(), &world);
    assert_eq!(data.world.objects[&id].name, name);
}

#[test]
fn long_name_roundtrip() {
    // 300 ASCII chars - well under u16::MAX but exercises the length prefix.
    let name = "x".repeat(300);
    let mut world = World::new();
    let id = world.spawn_object(
        Object::new(ObjectConstructor {
            name: name.clone(),
            transform: None,
            geometry: None,
            color: None,
            str_id: None,
            texture_path: None,
        }),
        None,
    );
    let data = roundtrip(&test_camera(), &world);
    assert_eq!(data.world.objects[&id].name, name);
}

// transform & color fidelity
#[test]
fn transform_all_fields() {
    let t = Transform {
        position: [-12.34, 56.78, -0.001],
        rotation: [180.0, -90.0, 45.0],
        scale: [0.1, 100.0, 3.14159],
    };
    let mut world = World::new();
    let id = world.spawn_object(
        Object::new(ObjectConstructor {
            name: "t".to_string(),
            transform: Some(t.clone()),
            geometry: None,
            color: None,
            str_id: None,
            texture_path: None,
        }),
        None,
    );
    let data = roundtrip(&test_camera(), &world);
    assert_eq!(data.world.objects[&id].transform, t);
}

#[test]
fn color_transparent_black() {
    let color = [0.0_f32, 0.0, 0.0, 0.0];
    let mut world = World::new();
    let id = world.spawn_object(
        Object::new(ObjectConstructor {
            name: "c".to_string(),
            transform: None,
            geometry: None,
            color: Some(color),
            str_id: None,
            texture_path: None,
        }),
        None,
    );
    let data = roundtrip(&test_camera(), &world);
    assert_eq!(data.world.objects[&id].color, color);
}

#[test]
fn color_hdr_values() {
    // HDR colors can exceed 1.0 - verify no clamping.
    let color = [2.5_f32, 0.0, 10.0, 1.0];
    let mut world = World::new();
    let id = world.spawn_object(
        Object::new(ObjectConstructor {
            name: "hdr".to_string(),
            transform: None,
            geometry: None,
            color: Some(color),
            str_id: None,
            texture_path: None,
        }),
        None,
    );
    let data = roundtrip(&test_camera(), &world);
    assert_eq!(data.world.objects[&id].color, color);
}

// determinism
#[test]
fn deterministic_output() {
    // Serializing the same world twice must produce identical bytes.
    let mut world = World::new();
    let r = world.spawn_object(Object::default(), None);
    world.spawn_object(Object::default(), Some(r));
    world.spawn_object(Object::default(), Some(r));

    let bytes1 = serialize(&test_camera(), &world);
    let bytes2 = serialize(&test_camera(), &world);
    assert_eq!(bytes1, bytes2, "serialization must be deterministic");
}

#[test]
fn idempotent_roundtrip() {
    // Serialize -> deserialize -> serialize again: the bytes must be identical.
    let mut world = World::new();
    let r = world.spawn_object(
        Object::new(ObjectConstructor {
            name: "Sphere".to_string(),
            transform: Some(Transform::from_position(1.0, 2.0, 3.0)),
            geometry: Some(Geometry::Sphere { radius: 1.0, subdivisions: 16 }),
            color: Some([0.8, 0.2, 0.4, 1.0]),
            str_id: None,
            texture_path: None,
        }),
        None,
    );
    world.spawn_object(
        Object::new(ObjectConstructor {
            name: "Child".to_string(),
            transform: None,
            geometry: Some(Geometry::Cube { size: 0.5 }),
            color: None,
            str_id: None,
            texture_path: None,
        }),
        Some(r),
    );

    let camera = custom_camera();
    let bytes1 = serialize(&camera, &world);
    let data = deserialize(&bytes1);
    let bytes2 = serialize(&data.camera, &data.world);

    assert_eq!(bytes1, bytes2, "output must be idempotent");
}

// large scene
#[test]
fn many_objects_roundtrip() {
    const N: usize = 200;
    let mut world = World::new();
    let root = world.spawn_object(Object::default(), None);
    for i in 0..N {
        world.spawn_object(
            Object::new(ObjectConstructor {
                name: format!("obj_{i}"),
                transform: Some(Transform::from_position(i as f32, 0.0, 0.0)),
                geometry: Some(Geometry::Cube { size: 1.0 }),
                color: Some([i as f32 / N as f32, 0.5, 1.0, 1.0]),
                str_id: None,
                texture_path: None,
            }),
            Some(root),
        );
    }

    let data = roundtrip(&test_camera(), &world);
    assert_eq!(data.world.objects.len(), N + 1, "all objects present");
    assert_eq!(data.world.objects[&root].children.len(), N);
}

// error paths
#[test]
fn error_bad_magic() {
    let mut bytes = serialize(&test_camera(), &World::new());
    bytes[0] = 0xFF; // corrupt the first magic byte
    let mut cur = Cursor::new(&bytes[..]);
    let result = vtr::read(&mut cur);
    assert!(
        matches!(result, Err(vtr::VtrError::InvalidMagic)),
        "expected InvalidMagic, got {result:?}"
    );
}

#[test]
fn error_wrong_magic_all_zeros() {
    let bytes = vec![0u8; 20];
    let mut cur = Cursor::new(&bytes[..]);
    assert!(matches!(vtr::read(&mut cur), Err(vtr::VtrError::InvalidMagic)));
}

#[test]
fn error_unsupported_version() {
    let mut bytes = serialize(&test_camera(), &World::new());
    // Overwrite format_version field (bytes 4-5) with an unsupported value.
    let bad_ver: u16 = 999;
    let le = bad_ver.to_le_bytes();
    bytes[4] = le[0];
    bytes[5] = le[1];

    let mut cur = Cursor::new(&bytes[..]);
    let result = vtr::read(&mut cur);
    assert!(
        matches!(result, Err(vtr::VtrError::UnsupportedVersion { found: 999 })),
        "expected UnsupportedVersion(999), got {result:?}"
    );
}

#[test]
fn error_unknown_geometry_tag() {
    let mut world = World::new();
    let fixed_sid = "test-uuid-0000-1111-2222-333344445555".to_string(); // 36 chars

    world.spawn_object(Object {
        name: "TAG_TEST".to_string(),
        str_id: fixed_sid.clone(),
        ..Object::default()
    }, None);

    let mut bytes = serialize(&test_camera(), &world);

    let name_bytes = b"TAG_TEST";
    let name_pos = bytes.windows(name_bytes.len())
        .position(|window| window == name_bytes)
        .expect("Could not find object name in binary data");

    // offset = name_start + name_len
    //        + transform(36)
    //        + color(16)
    //        + str_id_len_prefix(2)
    //        + str_id_content(36)
    let tag_offset = name_pos + name_bytes.len() + 36 + 16 + 2 + fixed_sid.len();

    // Verify we are within bounds
    assert!(tag_offset < bytes.len(), "Calculated offset is out of bounds!");

    bytes[tag_offset] = 0xAB;

    let mut cur = Cursor::new(&bytes[..]);
    let result = vtr::read(&mut cur);

    assert!(
        matches!(result, Err(vtr::VtrError::UnknownGeometryTag(0xAB))),
        "Expected UnknownGeometryTag(0xAB), but the parser didn't hit the corrupted byte. Offset might still be wrong."
    );
}

#[test]
fn error_truncated_header() {
    // Only 10 bytes - not enough for the full header (format_version = 2).
    let bytes = vec![0x56, 0x54, 0x52, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00];
    let mut cur = Cursor::new(&bytes[..]);
    let result = vtr::read(&mut cur);
    assert!(
        matches!(result, Err(vtr::VtrError::Io(_))),
        "expected Io error for truncated header, got {result:?}"
    );
}

#[test]
fn error_truncated_camera_block() {
    let bytes = serialize(&test_camera(), &World::new());
    // Truncate in the middle of the camera block.
    let truncated = &bytes[..30];
    let mut cur = Cursor::new(truncated);
    assert!(
        matches!(vtr::read(&mut cur), Err(vtr::VtrError::Io(_))),
        "expected Io error for truncated camera"
    );
}

#[test]
fn error_truncated_object_data() {
    let mut world = World::new();
    world.spawn_object(Object::default(), None);
    let bytes = serialize(&test_camera(), &world);
    // Truncate inside the first object's data.
    let truncated = &bytes[..bytes.len() - 10];
    let mut cur = Cursor::new(truncated);
    assert!(
        matches!(vtr::read(&mut cur), Err(vtr::VtrError::Io(_))),
        "expected Io error for truncated object"
    );
}

// error display
#[test]
fn error_display_invalid_magic() {
    let e = vtr::VtrError::InvalidMagic;
    assert!(e.to_string().contains("magic"), "InvalidMagic display: {e}");
}

#[test]
fn error_display_unsupported_version() {
    let e = vtr::VtrError::UnsupportedVersion { found: 42 };
    let s = e.to_string();
    assert!(s.contains("42"), "should mention found version: {s}");
}

#[test]
fn error_display_unknown_tag() {
    let e = vtr::VtrError::UnknownGeometryTag(0x0F);
    let s = e.to_string();
    assert!(s.contains("0x0f") || s.contains("0x0F") || s.contains("15"), "{s}");
}

// delete-after-load
#[test]
fn delete_after_load_does_not_affect_sibling() {
    let mut world = World::new();
    let r = world.spawn_object(Object::default(), None);
    let c1 = world.spawn_object(Object::default(), Some(r));
    let c2 = world.spawn_object(Object::default(), Some(r));

    let data = roundtrip(&test_camera(), &world);
    let mut w = data.world;

    w.delete(c1);

    assert!(w.objects.contains_key(&c2), "sibling c2 must still exist after deleting c1");
    assert!(!w.objects[&r].children.contains(&c1), "c1 must be unlinked from parent");
    assert!(w.objects[&r].children.contains(&c2), "c2 must still be linked to parent");
}

// full solar system scene (integration)
#[test]
fn solar_system_full_roundtrip() {
    let mut world = World::new();

    let sun = world.spawn_object(
        Object::new(ObjectConstructor {
            name: "Sun".to_string(),
            transform: Some(Transform::from_position(0.0, 0.0, 0.0)),
            geometry: Some(Geometry::Sphere { radius: 2.0, subdivisions: 32 }),
            color: Some([1.0, 0.9, 0.2, 1.0]),
            str_id: None,
            texture_path: None,
        }),
        None,
    );
    let planet = world.spawn_object(
        Object::new(ObjectConstructor {
            name: "Planet".to_string(),
            transform: Some(Transform::from_position(6.0, 0.0, 0.0)),
            geometry: Some(Geometry::Sphere { radius: 0.8, subdivisions: 24 }),
            color: Some([0.2, 0.5, 1.0, 1.0]),
            str_id: None,
            texture_path: None,
        }),
        Some(sun),
    );
    let moon = world.spawn_object(
        Object::new(ObjectConstructor {
            name: "Moon".to_string(),
            transform: Some(Transform::from_position(1.5, 0.0, 0.0)),
            geometry: Some(Geometry::Sphere { radius: 0.3, subdivisions: 16 }),
            color: Some([0.7, 0.7, 0.7, 1.0]),
            str_id: None,
            texture_path: None,
        }),
        Some(planet),
    );
    // An asteroid belt: 10 asteroids orbiting the sun.
    let belt_ids: Vec<usize> = (0..10)
        .map(|i| {
            world.spawn_object(
                Object::new(ObjectConstructor {
                    name: format!("Asteroid {i}"),
                    transform: Some(Transform::from_position(4.0 + i as f32 * 0.2, 0.0, 0.0)),
                    geometry: Some(Geometry::Sphere { radius: 0.05, subdivisions: 4 }),
                    color: Some([0.6, 0.5, 0.4, 1.0]),
                    str_id: None,
                    texture_path: None,
                }),
                Some(sun),
            )
        })
        .collect();

    let camera = Camera::new()
        .with_position([0.0, 8.0, -12.0])
        .with_rotation(90.0, -30.0);

    let data = roundtrip(&camera, &world);

    // Structural checks.
    assert_eq!(data.world.roots, vec![sun]);
    assert_eq!(data.world.objects.len(), 13); // sun + planet + moon + 10 asteroids

    // Sun has planet + 10 asteroids as children.
    let sun_children = &data.world.objects[&sun].children;
    assert!(sun_children.contains(&planet));
    for &aid in &belt_ids {
        assert!(sun_children.contains(&aid), "asteroid {aid} should be a sun child");
    }

    // Planet -> Moon.
    assert_eq!(data.world.objects[&planet].children, vec![moon]);
    assert_eq!(data.world.objects[&moon].parent, Some(planet));

    // Camera preserved.
    assert_cameras_eq(&camera, &data.camera);

    // All names intact.
    assert_eq!(data.world.objects[&sun].name, "Sun");
    assert_eq!(data.world.objects[&planet].name, "Planet");
    assert_eq!(data.world.objects[&moon].name, "Moon");
}

// --- str_id Specific Tests ---

#[test]
fn str_id_roundtrip_preservation() {
    let mut world = World::new();
    let sid = "unique_handle_123".to_string();

    let id = world.spawn_object(
        Object {
            name: "Handle Test".to_string(),
            str_id: sid.clone(),
            ..Object::default()
        },
        None,
    );

    let data = roundtrip(&test_camera(), &world);
    let loaded_obj = data.world.objects.get(&id).expect("Object missing");

    // Verify the string survived the binary serialization
    assert_eq!(loaded_obj.str_id, sid, "str_id mismatch after roundtrip");
}

#[test]
fn str_id_cache_rebuild_on_load() {
    let mut world = World::new();
    let sid = "player_spawn_point".to_string();

    world.spawn_object(
            Object {
                name: "Spawn".to_string(),
                str_id: sid.clone(),
                ..Object::default()
            },
        None,
    );

    let data = roundtrip(&test_camera(), &world);

    // Verify the name_handles HashMap was rebuilt correctly from the loaded objects
    let lookup_id = data.world.get_id(&sid);
    assert!(lookup_id.is_some(), "name_handles cache was not rebuilt on load");

    let obj = data.world.objects.get(&lookup_id.unwrap()).unwrap();
    assert_eq!(obj.name, "Spawn");
}

#[test]
fn str_id_cleanup_on_delete() {
    let mut world = World::new();
    let sid = "temporary_object".to_string();

    let id = world.spawn_object(
        Object {
            name: "Temp".to_string(),
            str_id: sid.clone(),
            ..Object::default()
        },
        None,
    );

    // Ensure it exists first
    assert!(world.get_id(&sid).is_some());

    // Delete the object
    world.delete(id);

    // Verify both the object AND the handle are gone
    assert!(!world.objects.contains_key(&id), "Object still exists in map");
    assert!(world.get_id(&sid).is_none(), "str_id handle was not cleaned up after delete");
}

#[test]
fn str_id_empty_by_default() {
    let mut world = World::new();
    let id = world.spawn_object(Object::default(), None);

    let data = roundtrip(&test_camera(), &world);
    let loaded_obj = &data.world.objects[&id];

    assert!(!loaded_obj.str_id.is_empty(), "str_id should not be empty if UUID fallback is active");
}

#[test]
fn str_id_unicode_handles() {
    let unicode_id = "核心_引擎_01".to_string();
    let mut world = World::new();

    world.spawn_object(
        Object {
            name: "Unicode Test".to_string(),
            str_id: unicode_id.clone(),
            ..Object::default()
        },
        None,
    );

    let data = roundtrip(&test_camera(), &world);
    assert_eq!(data.world.get_id(&unicode_id).is_some(), true);
}

