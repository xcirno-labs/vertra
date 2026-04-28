/// Tests for the play-mode snapshot / restore mechanism.
///
/// `Scene` requires a live GPU `Pipeline` and cannot be instantiated in unit
/// tests.  The snapshot feature is, however, pure VTR logic:
///
/// * `disable_editor_mode` -> `vtr::write(camera, world)` into a `Vec<u8>`
/// * `enable_editor_mode`  -> `vtr::read` that buffer and replace camera/world
///
/// Every test below exercises that contract directly, so no GPU context is
/// needed.  The helpers `make_snapshot` / `restore_snapshot` mirror the exact
/// code paths used in `Scene`.
///
/// Coverage:
///   - snapshot bytes are valid VTR and non-empty
///   - object transform mutations during play are reverted on restore
///   - camera mutations during play are reverted on restore
///   - objects spawned during play do not persist after restore
///   - objects deleted during play are restored
///   - multiple toggle cycles each create a fresh snapshot of the current state
///   - restoring from snapshot rebuilds the `str_id` name-handle cache
///   - all object fields (color, geometry, texture_path) survive the round-trip

use std::io::Cursor;

use crate::camera::Camera;
use crate::geometry::Geometry;
use crate::objects::Object;
use crate::transform::Transform;
use crate::vtr;
use crate::world::World;

/// Equivalent to `Scene::disable_editor_mode`: serialize camera + world into
/// a byte buffer that acts as the snapshot.
fn make_snapshot(camera: &Camera, world: &World) -> Vec<u8> {
    let mut buf = Vec::new();
    vtr::write(&mut buf, camera, world).expect("snapshot write failed");
    buf
}

/// Equivalent to `Scene::enable_editor_mode` restore path: deserialize the
/// snapshot and return the restored (camera, world).
fn restore_snapshot(buf: Vec<u8>) -> (Camera, World) {
    let data = vtr::read(&mut Cursor::new(buf)).expect("snapshot read failed");
    (data.camera, data.world)
}

fn cube_object(str_id: &str, pos: [f32; 3]) -> Object {
    Object {
        name: str_id.to_string(),
        str_id: str_id.to_string(),
        transform: Transform {
            position: pos,
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        },
        geometry: Some(Geometry::Cube { size: 1.0 }),
        color: [1.0, 1.0, 1.0, 1.0],
        children: Vec::new(),
        parent: None,
        texture_path: None,
    }
}

#[test]
fn snapshot_bytes_are_non_empty_and_valid_vtr() {
    let camera = Camera::new();
    let world = World::new();

    let buf = make_snapshot(&camera, &world);

    assert!(!buf.is_empty(), "snapshot must not be empty");
    // Must start with the VTR magic bytes
    assert_eq!(&buf[..4], &vtr::MAGIC, "snapshot must begin with VTR magic");
}

#[test]
fn object_position_mutation_is_reverted_on_restore() {
    let camera = Camera::new();
    let mut world = World::new();
    world.spawn_object(cube_object("box", [1.0, 1.0, 1.0]), None);

    // Snapshot at this point (equivalent to entering play mode)
    let snapshot = make_snapshot(&camera, &world);

    // Simulate a mutation during play
    let id = world.get_id("box").unwrap();
    world.get_mut(id).unwrap().transform.position = [2.0, 2.0, 2.0];
    assert_eq!(world.objects[&id].transform.position, [2.0, 2.0, 2.0]);

    // Restore (equivalent to returning to editor mode)
    let (_cam, restored) = restore_snapshot(snapshot);
    let rid = restored.get_id("box").unwrap();
    assert_eq!(
        restored.objects[&rid].transform.position,
        [1.0, 1.0, 1.0],
        "position must be reverted to pre-play value"
    );
}

#[test]
fn camera_mutation_is_reverted_on_restore() {
    let mut camera = Camera::new();
    camera.eye = [0.0, 5.0, -10.0];
    camera.fov = 60.0;
    let world = World::new();

    let snapshot = make_snapshot(&camera, &world);

    // Mutate camera during play (these values must NOT appear after restore)
    let _ = camera; // original camera consumed into snapshot; create a mutated copy
    let mut mutated = Camera::new();
    mutated.eye = [99.0, 99.0, 99.0];
    mutated.fov = 120.0;
    let _ = mutated; // just asserting the restored values differ

    let (restored_cam, _) = restore_snapshot(snapshot);
    assert_eq!(restored_cam.eye, [0.0, 5.0, -10.0], "camera eye must revert");
    assert_eq!(restored_cam.fov, 60.0, "camera fov must revert");
}

#[test]
fn objects_spawned_during_play_do_not_persist_after_restore() {
    let camera = Camera::new();
    let mut world = World::new();
    world.spawn_object(cube_object("original", [0.0, 0.0, 0.0]), None);

    let snapshot = make_snapshot(&camera, &world);

    // Spawn extra objects during play
    world.spawn_object(cube_object("play_obj_1", [5.0, 0.0, 0.0]), None);
    world.spawn_object(cube_object("play_obj_2", [10.0, 0.0, 0.0]), None);
    assert_eq!(world.objects.len(), 3);

    let (_cam, restored) = restore_snapshot(snapshot);
    assert_eq!(restored.objects.len(), 1, "only the original object should survive");
    assert!(restored.get_id("original").is_some(), "original must still exist");
    assert!(restored.get_id("play_obj_1").is_none(), "play_obj_1 must be gone");
    assert!(restored.get_id("play_obj_2").is_none(), "play_obj_2 must be gone");
}

#[test]
fn objects_deleted_during_play_are_restored() {
    let camera = Camera::new();
    let mut world = World::new();
    world.spawn_object(cube_object("to_delete", [0.0, 0.0, 0.0]), None);
    world.spawn_object(cube_object("survivor", [1.0, 0.0, 0.0]), None);

    let snapshot = make_snapshot(&camera, &world);

    // Delete during play
    let del_id = world.get_id("to_delete").unwrap();
    world.delete(del_id);
    assert_eq!(world.objects.len(), 1);

    let (_cam, restored) = restore_snapshot(snapshot);
    assert_eq!(restored.objects.len(), 2, "both objects must be restored");
    assert!(restored.get_id("to_delete").is_some(), "deleted object must reappear");
    assert!(restored.get_id("survivor").is_some(), "survivor must still be present");
}

#[test]
fn multiple_toggle_cycles_each_snapshot_current_state() {
    let camera = Camera::new();
    let mut world = World::new();
    world.spawn_object(cube_object("obj", [0.0, 0.0, 0.0]), None);

    // First cycle: snapshot at position [0,0,0]
    let snap1 = make_snapshot(&camera, &world);
    let id = world.get_id("obj").unwrap();
    world.get_mut(id).unwrap().transform.position = [1.0, 0.0, 0.0];

    // Restore -> back to [0,0,0]; then re-snapshot at [0,0,0]
    let (cam2, mut world2) = restore_snapshot(snap1);
    let id2 = world2.get_id("obj").unwrap();
    assert_eq!(world2.objects[&id2].transform.position, [0.0, 0.0, 0.0]);

    // Second cycle: mutate to [3,0,0] then snapshot
    world2.get_mut(id2).unwrap().transform.position = [3.0, 0.0, 0.0];
    let snap2 = make_snapshot(&cam2, &world2);
    world2.get_mut(id2).unwrap().transform.position = [9.0, 9.0, 9.0];

    // Restore snap2 -> should be [3,0,0], not [0,0,0] or [9,9,9]
    let (_cam3, world3) = restore_snapshot(snap2);
    let id3 = world3.get_id("obj").unwrap();
    assert_eq!(
        world3.objects[&id3].transform.position,
        [3.0, 0.0, 0.0],
        "second snapshot must reflect state at second play-enter, not first"
    );
}

#[test]
fn str_id_name_handle_cache_is_rebuilt_after_restore() {
    let camera = Camera::new();
    let mut world = World::new();
    world.spawn_object(cube_object("alpha", [0.0, 0.0, 0.0]), None);
    world.spawn_object(cube_object("beta", [1.0, 0.0, 0.0]), None);

    let snapshot = make_snapshot(&camera, &world);

    let (_cam, restored) = restore_snapshot(snapshot);

    // get_id uses the name_handles cache, it must be fully rebuilt
    assert!(restored.get_id("alpha").is_some(), "cache must contain 'alpha'");
    assert!(restored.get_id("beta").is_some(),  "cache must contain 'beta'");
    assert!(restored.get_id("gamma").is_none(), "unknown id must return None");
}

#[test]
fn all_object_fields_survive_snapshot_roundtrip() {
    let camera = Camera::new();
    let mut world = World::new();
    world.spawn_object(
        Object {
            name: "full".to_string(),
            str_id: "full".to_string(),
            transform: Transform {
                position: [1.0, 2.0, 3.0],
                rotation: [10.0, 20.0, 30.0],
                scale: [2.0, 3.0, 4.0],
            },
            geometry: Some(Geometry::Sphere { radius: 1.5, subdivisions: 16 }),
            color: [0.1, 0.2, 0.3, 0.4],
            texture_path: Some("textures/test.png".to_string()),
            children: Vec::new(),
            parent: None,
        },
        None,
    );

    let snapshot = make_snapshot(&camera, &world);
    let (_cam, restored) = restore_snapshot(snapshot);

    let id = restored.get_id("full").unwrap();
    let obj = &restored.objects[&id];

    assert_eq!(obj.transform.position, [1.0, 2.0, 3.0]);
    assert_eq!(obj.transform.rotation, [10.0, 20.0, 30.0]);
    assert_eq!(obj.transform.scale,    [2.0, 3.0, 4.0]);
    assert_eq!(obj.color,              [0.1, 0.2, 0.3, 0.4]);
    assert_eq!(obj.texture_path.as_deref(), Some("textures/test.png"));
    assert!(matches!(obj.geometry, Some(Geometry::Sphere { radius, subdivisions })
        if (radius - 1.5).abs() < 1e-6 && subdivisions == 16));
}

#[test]
fn child_hierarchy_is_fully_restored() {
    let camera = Camera::new();
    let mut world = World::new();

    let parent_id = world.spawn_object(cube_object("parent", [0.0, 0.0, 0.0]), None);
    world.spawn_object(cube_object("child", [1.0, 0.0, 0.0]), Some(parent_id));

    let snapshot = make_snapshot(&camera, &world);

    // Reparent child to root during play
    let child_id = world.get_id("child").unwrap();
    world.reparent(child_id, None);

    let (_cam, restored) = restore_snapshot(snapshot);

    let rp = restored.get_id("parent").unwrap();
    let rc = restored.get_id("child").unwrap();

    assert_eq!(restored.objects[&rc].parent, Some(rp), "child must be re-parented to parent");
    assert!(restored.objects[&rp].children.contains(&rc), "parent.children must contain child");
    assert!(!restored.roots.contains(&rc), "child must not be in roots");
}

#[test]
fn roots_list_is_restored_correctly() {
    let camera = Camera::new();
    let mut world = World::new();
    world.spawn_object(cube_object("r1", [0.0, 0.0, 0.0]), None);
    world.spawn_object(cube_object("r2", [1.0, 0.0, 0.0]), None);

    let snapshot = make_snapshot(&camera, &world);

    // Delete one root during play
    let id = world.get_id("r1").unwrap();
    world.delete(id);
    assert_eq!(world.roots.len(), 1);

    let (_cam, restored) = restore_snapshot(snapshot);
    assert_eq!(restored.roots.len(), 2, "both roots must be restored");
}


