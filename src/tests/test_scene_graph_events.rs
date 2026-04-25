//! Unit tests for `World::on_scene_graph_modified`.
//!
//! These tests exercise the three structural mutations — `spawn_object`,
//! `delete`, and `reparent` — and verify that the correct [`SceneGraphEvent`]
//! variants are fired with the expected payload.
//!
//! No window, GPU pipeline, or async runtime is required; the tests run with
//! plain `cargo test`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::objects::Object;
use crate::world::{SceneGraphCallback, SceneGraphEvent, World};

/// Create a minimal [`World`] with a callback that appends every event to a
/// shared `Vec`.  Returns `(world, event_log)`.
fn world_with_log() -> (World, Rc<RefCell<Vec<SceneGraphEvent>>>) {
    let log: Rc<RefCell<Vec<SceneGraphEvent>>> = Rc::new(RefCell::new(Vec::new()));
    let log_clone = Rc::clone(&log);

    let mut world = World::new();
    world.on_scene_graph_modified = Some(SceneGraphCallback(Box::new(move |ev| {
        log_clone.borrow_mut().push(ev);
    })));

    (world, log)
}

fn default_object(name: &str, str_id: &str) -> Object {
    Object {
        name: name.to_string(),
        str_id: str_id.to_string(),
        ..Default::default()
    }
}

#[test]
fn spawn_fires_object_added_at_root() {
    let (mut world, log) = world_with_log();

    let id = world.spawn_object(default_object("Root", "root"), None);

    let events = log.borrow();
    assert_eq!(events.len(), 1);
    match &events[0] {
        SceneGraphEvent::ObjectAdded { id: fired_id, parent_id } => {
            assert_eq!(*fired_id, id);
            assert_eq!(*parent_id, None);
        }
        other => panic!("Unexpected event: {other:?}"),
    }
}

#[test]
fn spawn_fires_object_added_with_parent() {
    let (mut world, log) = world_with_log();

    let parent_id = world.spawn_object(default_object("Parent", "parent"), None);
    let child_id  = world.spawn_object(default_object("Child",  "child"),  Some(parent_id));

    let events = log.borrow();
    // Two additions: parent then child
    assert_eq!(events.len(), 2);

    match &events[1] {
        SceneGraphEvent::ObjectAdded { id, parent_id: p } => {
            assert_eq!(*id, child_id);
            assert_eq!(*p, Some(parent_id));
        }
        other => panic!("Unexpected event: {other:?}"),
    }
}

#[test]
fn delete_fires_object_deleted() {
    let (mut world, log) = world_with_log();

    let id = world.spawn_object(default_object("ToDelete", "td"), None);
    log.borrow_mut().clear(); // ignore the spawn event

    world.delete(id);

    let events = log.borrow();
    assert_eq!(events.len(), 1);
    match &events[0] {
        SceneGraphEvent::ObjectDeleted { id: fired_id } => {
            assert_eq!(*fired_id, id);
        }
        other => panic!("Unexpected event: {other:?}"),
    }
}

#[test]
fn delete_nonexistent_id_fires_no_event() {
    let (mut world, log) = world_with_log();

    world.delete(9999);
    println!("Events: {:?}", log.borrow());
    assert!(log.borrow().is_empty());
}

#[test]
fn reparent_fires_object_reparented() {
    let (mut world, log) = world_with_log();

    let parent_a = world.spawn_object(default_object("A", "a"), None);
    let parent_b = world.spawn_object(default_object("B", "b"), None);
    let child    = world.spawn_object(default_object("C", "c"), Some(parent_a));
    log.borrow_mut().clear();

    world.reparent(child, Some(parent_b));

    let events = log.borrow();
    assert_eq!(events.len(), 1);
    match &events[0] {
        SceneGraphEvent::ObjectReparented { id, old_parent, new_parent } => {
            assert_eq!(*id, child);
            assert_eq!(*old_parent, Some(parent_a));
            assert_eq!(*new_parent, Some(parent_b));
        }
        other => panic!("Unexpected event: {other:?}"),
    }
}

#[test]
fn reparent_to_root_sets_new_parent_none() {
    let (mut world, log) = world_with_log();

    let parent = world.spawn_object(default_object("P", "p"), None);
    let child  = world.spawn_object(default_object("C", "c"), Some(parent));
    log.borrow_mut().clear();

    world.reparent(child, None);

    let events = log.borrow();
    assert_eq!(events.len(), 1);
    match &events[0] {
        SceneGraphEvent::ObjectReparented { id, old_parent, new_parent } => {
            assert_eq!(*id, child);
            assert_eq!(*old_parent, Some(parent));
            assert_eq!(*new_parent, None);
            // Object should now appear in roots
        }
        other => panic!("Unexpected event: {other:?}"),
    }

    assert!(world.roots.contains(&child));
}

#[test]
fn reparent_same_parent_fires_no_event() {
    let (mut world, log) = world_with_log();

    let parent = world.spawn_object(default_object("P", "p"), None);
    let child  = world.spawn_object(default_object("C", "c"), Some(parent));
    log.borrow_mut().clear();

    world.reparent(child, Some(parent)); // no-op

    assert!(log.borrow().is_empty());
}

#[test]
fn reparent_nonexistent_id_fires_no_event() {
    let (mut world, log) = world_with_log();
    log.borrow_mut().clear();

    world.reparent(9999, None);

    assert!(log.borrow().is_empty());
}

#[test]
fn reparent_updates_parent_children_lists() {
    let (mut world, _log) = world_with_log();

    let a     = world.spawn_object(default_object("A", "a"), None);
    let b     = world.spawn_object(default_object("B", "b"), None);
    let child = world.spawn_object(default_object("C", "c"), Some(a));

    world.reparent(child, Some(b));

    assert!(!world.objects[&a].children.contains(&child), "old parent still owns child");
    assert!( world.objects[&b].children.contains(&child), "new parent does not own child");
    assert_eq!(world.objects[&child].parent, Some(b));
}

