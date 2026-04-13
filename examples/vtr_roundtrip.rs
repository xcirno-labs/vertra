//! # VTR Round-trip
//!
//! A **headless** (no window) example that exercises the VTR binary scene
//! format end-to-end:
//!
//! 1. Build a small scene (camera + three objects) in memory.
//! 2. Serialize it to a `Vec<u8>` using [`vertra::vtr::write`].
//! 3. Peek at the header with [`vertra::vtr::read_header`].
//! 4. Deserialize back with [`vertra::vtr::read`].
//! 5. Assert that the round-tripped scene matches the original.
//!
//! The same `write` / `read` functions back [`Scene::save_vtr_file`] and
//! [`Scene::load_vtr_file`], so this example demonstrates exactly what those
//! convenience helpers do internally.
//!
//! **Run:**
//! ```sh
//! cargo run --example vtr_roundtrip
//! ```

use std::io::Cursor;

use vertra::camera::Camera;
use vertra::geometry::Geometry;
use vertra::objects::Object;
use vertra::transform::Transform;
use vertra::vtr;
use vertra::world::World;

fn main() {
    // ── 1. Build a scene in memory ────────────────────────────────────────────

    let camera = Camera::new()
        .with_position([0.0, 5.0, -12.0])
        .with_rotation(90.0, -20.0);

    let mut world = World::new();

    // Root object
    let root_id = world.spawn_object(
        Object {
            name: "Root".to_string(),
            str_id: "root".to_string(),
            geometry: Some(Geometry::Cube { size: 1.0 }),
            color: [1.0, 0.4, 0.4, 1.0],
            transform: Transform::from_position(0.0, 0.0, 0.0),
            ..Default::default()
        },
        None,
    );

    // Child of root
    let child_id = world.spawn_object(
        Object {
            name: "Child".to_string(),
            str_id: "child".to_string(),
            geometry: Some(Geometry::Sphere {
                radius: 0.5,
                subdivisions: 16,
            }),
            color: [0.4, 0.8, 0.4, 1.0],
            transform: Transform::from_position(3.0, 0.0, 0.0),
            ..Default::default()
        },
        Some(root_id),
    );

    // Grandchild
    world.spawn_object(
        Object {
            name: "Grandchild".to_string(),
            str_id: "grandchild".to_string(),
            geometry: Some(Geometry::Pyramid {
                base_size: 0.8,
                height: 1.2,
            }),
            color: [0.4, 0.4, 1.0, 1.0],
            transform: Transform::from_position(2.0, 0.0, 0.0),
            ..Default::default()
        },
        Some(child_id),
    );

    println!("Original scene: {} object(s)", world.objects.len());

    // ── 2. Serialize ──────────────────────────────────────────────────────────

    let mut buf: Vec<u8> = Vec::new();
    vtr::write(&mut buf, &camera, &world).expect("serialization failed");
    println!("Serialized to {} byte(s)", buf.len());

    // ── 3. Peek at the header ─────────────────────────────────────────────────

    let header = vtr::read_header(&mut Cursor::new(&buf)).expect("header read failed");
    println!(
        "Header: format v{}, engine v{}, {} object(s)",
        header.format_version,
        header.engine_version_string(),
        header.object_count,
    );

    assert_eq!(header.object_count as usize, world.objects.len());

    // ── 4. Deserialize ────────────────────────────────────────────────────────

    let loaded = vtr::read(&mut Cursor::new(&buf)).expect("deserialization failed");
    println!("Loaded scene:  {} object(s)", loaded.world.objects.len());

    // ── 5. Verify ─────────────────────────────────────────────────────────────

    assert_eq!(
        loaded.world.objects.len(),
        world.objects.len(),
        "object count mismatch"
    );

    // Check that hierarchy is preserved.
    for str_id in ["root", "child", "grandchild"] {
        let orig_id = world.get_id(str_id).expect("str_id missing in original");
        let load_id = loaded
            .world
            .get_id(str_id)
            .expect("str_id missing after round-trip");

        let orig = &world.objects[&orig_id];
        let load = &loaded.world.objects[&load_id];

        assert_eq!(orig.name, load.name, "name mismatch for {str_id}");
        assert_eq!(orig.color, load.color, "color mismatch for {str_id}");
        assert_eq!(
            orig.transform.position, load.transform.position,
            "position mismatch for {str_id}"
        );
        assert_eq!(
            orig.geometry, load.geometry,
            "geometry mismatch for {str_id}"
        );

        println!("  ✓  {str_id:12}  name={:?}", load.name);
    }

    // Camera round-trip
    assert_eq!(camera.eye, loaded.camera.eye, "camera eye mismatch");
    assert_eq!(camera.fov, loaded.camera.fov, "camera fov mismatch");
    println!("  ✓  camera");

    println!("\nRound-trip verified ✓");
}

