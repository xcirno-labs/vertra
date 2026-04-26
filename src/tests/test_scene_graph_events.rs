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
    log.borrow_mut().clear();
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
    assert!(log.borrow().is_empty());
}

#[test]
fn delete_removes_entire_subtree() {
    let (mut world, log) = world_with_log();
    //   root
    //   └── child
    //       └── grandchild
    let root  = world.spawn_object(default_object("R", "r"), None);
    let child = world.spawn_object(default_object("C", "c"), Some(root));
    let grand = world.spawn_object(default_object("G", "g"), Some(child));
    log.borrow_mut().clear();

    world.delete(root);

    // All three objects must be gone
    assert!(!world.objects.contains_key(&root),  "root still present");
    assert!(!world.objects.contains_key(&child), "child still present");
    assert!(!world.objects.contains_key(&grand), "grandchild still present");
    // Root must be removed from the roots list
    assert!(!world.roots.contains(&root));
    // One ObjectDeleted event fired for the top-level deletion
    let events = log.borrow();
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0], SceneGraphEvent::ObjectDeleted { id } if *id == root));
}

#[test]
fn delete_child_removes_from_parent_children_list() {
    let (mut world, _log) = world_with_log();
    let parent = world.spawn_object(default_object("P", "p"), None);
    let child  = world.spawn_object(default_object("C", "c"), Some(parent));
    world.delete(child);
    assert!(!world.objects[&parent].children.contains(&child));
}

#[test]
fn reparent_fires_object_reparented() {
    let (mut world, log) = world_with_log();
    let parent_a = world.spawn_object(default_object("A", "a"), None);
    let parent_b = world.spawn_object(default_object("B", "b"), None);
    let child    = world.spawn_object(default_object("C", "c"), Some(parent_a));
    log.borrow_mut().clear();

    let ok = world.reparent(child, Some(parent_b));
    assert!(ok);

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

    let ok = world.reparent(child, None);
    assert!(ok);

    let events = log.borrow();
    assert_eq!(events.len(), 1);
    match &events[0] {
        SceneGraphEvent::ObjectReparented { id, old_parent, new_parent } => {
            assert_eq!(*id, child);
            assert_eq!(*old_parent, Some(parent));
            assert_eq!(*new_parent, None);
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

    let ok = world.reparent(child, Some(parent));
    assert!(!ok);
    assert!(log.borrow().is_empty());
}

#[test]
fn reparent_nonexistent_id_fires_no_event() {
    let (mut world, log) = world_with_log();
    log.borrow_mut().clear();
    let ok = world.reparent(9999, None);
    assert!(!ok);
    assert!(log.borrow().is_empty());
}

#[test]
fn reparent_to_nonexistent_parent_is_noop() {
    let (mut world, log) = world_with_log();
    let id = world.spawn_object(default_object("A", "a"), None);
    log.borrow_mut().clear();

    // 9999 does not exist, must be rejected
    let ok = world.reparent(id, Some(9999));
    assert!(!ok, "reparent to nonexistent parent should return false");
    assert!(log.borrow().is_empty(), "no event should fire");
    // Object must still be at root
    assert!(world.roots.contains(&id));
    assert_eq!(world.objects[&id].parent, None);
}

#[test]
fn reparent_self_is_noop() {
    let (mut world, log) = world_with_log();
    let id = world.spawn_object(default_object("A", "a"), None);
    log.borrow_mut().clear();

    let ok = world.reparent(id, Some(id));
    assert!(!ok);
    assert!(log.borrow().is_empty());
}

#[test]
fn reparent_rejects_direct_cycle() {
    let (mut world, log) = world_with_log();
    //   parent -> child
    // Attempt: reparent parent under child -> would create cycle
    let parent = world.spawn_object(default_object("P", "p"), None);
    let child  = world.spawn_object(default_object("C", "c"), Some(parent));
    log.borrow_mut().clear();

    let ok = world.reparent(parent, Some(child));
    assert!(!ok, "direct cycle must be rejected");
    assert!(log.borrow().is_empty());

    // Hierarchy must be unchanged
    assert!(world.roots.contains(&parent));
    assert_eq!(world.objects[&parent].parent, None);
    assert_eq!(world.objects[&child].parent, Some(parent));
}

#[test]
fn reparent_rejects_deep_cycle() {
    let (mut world, log) = world_with_log();
    //   a -> b -> c -> d
    let a = world.spawn_object(default_object("A", "a"), None);
    let b = world.spawn_object(default_object("B", "b"), Some(a));
    let c = world.spawn_object(default_object("C", "c"), Some(b));
    let d = world.spawn_object(default_object("D", "d"), Some(c));
    log.borrow_mut().clear();

    // Reparenting `a` under `d` would make `a` its own descendant
    let ok = world.reparent(a, Some(d));
    assert!(!ok, "deep cycle must be rejected");
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

// TODO: Move these test below somewhere else since it's not related to scene graph events
#[test]
fn rename_str_id_updates_cache() {
    let (mut world, _log) = world_with_log();
    let id = world.spawn_object(default_object("Obj", "old_id"), None);

    let ok = world.rename_str_id(id, "new_id".to_string());

    assert!(ok);
    assert_eq!(world.get_id("new_id"), Some(id));   // new key resolves
    assert_eq!(world.get_id("old_id"), None);         // old key is gone
    assert_eq!(world.objects[&id].str_id, "new_id"); // field is updated
}

#[test]
fn rename_str_id_nonexistent_returns_false() {
    let (mut world, _log) = world_with_log();
    assert!(!world.rename_str_id(9999, "x".to_string()));
}

#[test]
fn direct_field_mutation_desynchronises_cache() {
    // Documents the hazard: writing obj.str_id directly bypasses the cache.
    // rename_str_id is the safe path.
    let (mut world, _log) = world_with_log();
    let id = world.spawn_object(default_object("Obj", "original"), None);

    // UNSAFE direct mutation - bypasses cache
    world.objects.get_mut(&id).unwrap().str_id = "bypassed".to_string();

    // Cache still maps "original" -> id; "bypassed" is unknown
    assert_eq!(world.get_id("original"), Some(id));
    assert_eq!(world.get_id("bypassed"), None);

    // The safe fix: rename_str_id reads the *current* field value ("bypassed")
    // as the stale key, removes it (no-op since it wasn't in the cache), then
    // inserts the correct mapping.
    let ok = world.rename_str_id(id, "fixed".to_string());
    assert!(ok);
    assert_eq!(world.get_id("fixed"), Some(id));
}
