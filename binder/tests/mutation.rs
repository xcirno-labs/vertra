//! Integration tests for the deferred world-mutation system.
//!
//! These tests live in `binder/tests/` (a separate compilation unit) and
//! exercise the public surface of [`vertra_js::internals::mutation`].  They
//! are compiled as an independent crate, so they can only reach items that
//! are explicitly `pub`.
//!
//! # Running
//!
//! Because the binder is wasm-only, run these tests with:
//!
//! ```sh
//! wasm-pack test --node
//! ```
//!
//! # Thread-local isolation
//!
//! The wasm executor runs tests sequentially on a single thread.  The mutation
//! queue and the script-borrow flag are thread-locals, so state can still leak
//! between tests.  Every test **must** call [`reset()`] at its very start.

use vertra_js::internals::mutation::{
    Mutation,
    flush_mutations,
    is_mutation_queue_empty,
    is_script_borrow_active,
    mutation_queue_len,
    queue_mutation,
    reset_test_state,
    script_borrow_enter,
    script_borrow_exit,
};
use vertra::objects::Object as CoreObject;
use vertra::world::World as CoreWorld;
use wasm_bindgen_test::*;

fn reset() {
    reset_test_state();
}

fn make_world() -> CoreWorld {
    CoreWorld::new()
}

fn make_object(str_id: &str) -> CoreObject {
    CoreObject {
        name:   str_id.into(),
        str_id: str_id.into(),
        ..Default::default()
    }
}

#[wasm_bindgen_test]
fn borrow_inactive_by_default() {
    reset();
    assert!(!is_script_borrow_active());
}

#[wasm_bindgen_test]
fn borrow_enter_sets_flag() {
    reset();
    script_borrow_enter();
    assert!(is_script_borrow_active());
    reset();
}

#[wasm_bindgen_test]
fn borrow_exit_clears_flag() {
    reset();
    script_borrow_enter();
    assert!(is_script_borrow_active());
    // Queue is empty -> null pointer is never dereferenced.
    script_borrow_exit(std::ptr::null_mut());
    assert!(!is_script_borrow_active());
}

#[wasm_bindgen_test]
fn borrow_exit_flushes_queue_and_applies_mutation() {
    reset();
    let mut world = make_world();
    let id = world.spawn_object(make_object("target"), None);

    script_borrow_enter();
    queue_mutation(Mutation::Delete(id));
    assert!(!is_mutation_queue_empty(), "queue should hold the deferred delete");

    script_borrow_exit(&mut world as *mut CoreWorld);

    assert!(!is_script_borrow_active(), "flag must be cleared after exit");
    assert!(is_mutation_queue_empty(),  "queue must be drained after exit");
    assert!(
        !world.objects.contains_key(&id),
        "delete must have been applied to the world",
    );
}

#[wasm_bindgen_test]
fn flush_delete_removes_object() {
    reset();
    let mut world = make_world();
    let id = world.spawn_object(make_object("obj"), None);
    assert!(world.objects.contains_key(&id));

    queue_mutation(Mutation::Delete(id));
    flush_mutations(&mut world as *mut CoreWorld);

    assert!(!world.objects.contains_key(&id));
}

#[wasm_bindgen_test]
fn flush_delete_nonexistent_id_is_noop() {
    reset();
    let mut world = make_world();
    // Must not panic even when the ID was never added.
    queue_mutation(Mutation::Delete(999));
    flush_mutations(&mut world as *mut CoreWorld);
}

#[wasm_bindgen_test]
fn flush_delete_removes_all_descendants() {
    reset();
    let mut world     = make_world();
    let parent_id     = world.spawn_object(make_object("parent"), None);
    let child_id      = world.spawn_object(make_object("child"),  Some(parent_id));
    let grandchild_id = world.spawn_object(make_object("gc"),     Some(child_id));

    queue_mutation(Mutation::Delete(parent_id));
    flush_mutations(&mut world as *mut CoreWorld);

    assert!(!world.objects.contains_key(&parent_id));
    assert!(!world.objects.contains_key(&child_id));
    assert!(!world.objects.contains_key(&grandchild_id));
}

#[wasm_bindgen_test]
fn flush_spawn_inserts_object_at_preallocated_id() {
    reset();
    let mut world = make_world();
    let pre_id    = world.alloc_id();
    let obj       = make_object("deferred");

    queue_mutation(Mutation::Spawn { id: pre_id, object: obj, parent: None });
    flush_mutations(&mut world as *mut CoreWorld);

    assert!(world.objects.contains_key(&pre_id));
    assert_eq!(world.objects[&pre_id].str_id, "deferred");
    assert_eq!(world.get_id("deferred"), Some(pre_id));
}

#[wasm_bindgen_test]
fn flush_spawn_at_root_adds_to_roots_list() {
    reset();
    let mut world = make_world();
    let pre_id    = world.alloc_id();

    queue_mutation(Mutation::Spawn {
        id: pre_id, object: make_object("root_obj"), parent: None,
    });
    flush_mutations(&mut world as *mut CoreWorld);

    assert!(world.roots.contains(&pre_id), "root-level spawn must appear in roots");
    assert_eq!(world.objects[&pre_id].parent, None);
}

#[wasm_bindgen_test]
fn flush_spawn_with_parent_links_hierarchy_correctly() {
    reset();
    let mut world    = make_world();
    let parent_id    = world.spawn_object(make_object("parent"), None);
    let child_pre_id = world.alloc_id();

    queue_mutation(Mutation::Spawn {
        id: child_pre_id, object: make_object("child"), parent: Some(parent_id),
    });
    flush_mutations(&mut world as *mut CoreWorld);

    assert_eq!(world.objects[&child_pre_id].parent, Some(parent_id));
    assert!(world.objects[&parent_id].children.contains(&child_pre_id));
}

#[wasm_bindgen_test]
fn consecutive_deferred_spawns_receive_sequential_ids() {
    reset();
    let mut world = make_world();
    let id1 = world.alloc_id();
    let id2 = world.alloc_id();
    assert_eq!(id2, id1 + 1, "pre-allocated IDs must be strictly sequential");

    queue_mutation(Mutation::Spawn { id: id1, object: make_object("first"),  parent: None });
    queue_mutation(Mutation::Spawn { id: id2, object: make_object("second"), parent: None });
    flush_mutations(&mut world as *mut CoreWorld);

    assert!(world.objects.contains_key(&id1));
    assert!(world.objects.contains_key(&id2));
}

#[wasm_bindgen_test]
fn flush_reparent_moves_child_to_new_parent() {
    reset();
    let mut world     = make_world();
    let new_parent_id = world.spawn_object(make_object("parent"), None);
    let child_id      = world.spawn_object(make_object("child"),  None);
    assert_eq!(world.objects[&child_id].parent, None, "child starts at root");

    queue_mutation(Mutation::Reparent { id: child_id, new_parent: Some(new_parent_id) });
    flush_mutations(&mut world as *mut CoreWorld);

    assert_eq!(world.objects[&child_id].parent, Some(new_parent_id));
    assert!(world.objects[&new_parent_id].children.contains(&child_id));
    assert!(!world.roots.contains(&child_id), "child must leave root list");
}

#[wasm_bindgen_test]
fn flush_reparent_to_none_moves_to_root() {
    reset();
    let mut world = make_world();
    let parent_id = world.spawn_object(make_object("parent"), None);
    let child_id  = world.spawn_object(make_object("child"),  Some(parent_id));

    queue_mutation(Mutation::Reparent { id: child_id, new_parent: None });
    flush_mutations(&mut world as *mut CoreWorld);

    assert_eq!(world.objects[&child_id].parent, None);
    assert!(world.roots.contains(&child_id));
    assert!(!world.objects[&parent_id].children.contains(&child_id));
}

#[wasm_bindgen_test]
fn flush_rename_updates_str_id_and_name_handle_cache() {
    reset();
    let mut world = make_world();
    let id        = world.spawn_object(make_object("old_name"), None);

    queue_mutation(Mutation::Rename { id, new_str_id: "new_name".into() });
    flush_mutations(&mut world as *mut CoreWorld);

    assert_eq!(world.objects[&id].str_id, "new_name");
    assert_eq!(world.get_id("new_name"), Some(id), "new handle must resolve");
    assert_eq!(world.get_id("old_name"), None,     "old handle must be removed");
}

#[wasm_bindgen_test]
fn flush_applies_mutations_in_fifo_order() {
    reset();
    let mut world = make_world();
    let a = world.spawn_object(make_object("a"), None);
    let b = world.spawn_object(make_object("b"), None);

    // Enqueue in order: rename a, then delete b.
    queue_mutation(Mutation::Rename { id: a, new_str_id: "a_renamed".into() });
    queue_mutation(Mutation::Delete(b));
    flush_mutations(&mut world as *mut CoreWorld);

    assert_eq!(world.get_id("a_renamed"), Some(a), "rename must be applied");
    assert!(!world.objects.contains_key(&b),        "delete must be applied");
}

#[wasm_bindgen_test]
fn deferred_spawn_then_deferred_delete_nets_to_no_object() {
    reset();
    let mut world = make_world();
    let pre_id    = world.alloc_id();

    queue_mutation(Mutation::Spawn  { id: pre_id, object: make_object("temp"), parent: None });
    queue_mutation(Mutation::Delete(pre_id));
    flush_mutations(&mut world as *mut CoreWorld);

    assert!(
        !world.objects.contains_key(&pre_id),
        "a spawn immediately followed by a delete must leave no object",
    );
}

#[wasm_bindgen_test]
fn flush_empty_queue_does_not_touch_world() {
    reset();
    let mut world = make_world();
    let id = world.spawn_object(make_object("existing"), None);

    // Nothing queued, world must be untouched.
    flush_mutations(&mut world as *mut CoreWorld);

    assert!(world.objects.contains_key(&id));
}

#[wasm_bindgen_test]
fn queue_len_reflects_enqueued_mutations() {
    reset();
    assert_eq!(mutation_queue_len(), 0);
    queue_mutation(Mutation::Delete(1));
    assert_eq!(mutation_queue_len(), 1);
    queue_mutation(Mutation::Delete(2));
    assert_eq!(mutation_queue_len(), 2);
    reset();
}

#[wasm_bindgen_test]
fn is_mutation_queue_empty_reflects_state() {
    reset();
    assert!(is_mutation_queue_empty());
    queue_mutation(Mutation::Delete(42));
    assert!(!is_mutation_queue_empty());
    reset();
}

#[wasm_bindgen_test]
fn mutations_accumulate_while_borrow_active() {
    reset();
    script_borrow_enter();
    // Simulate the binder's deferred-path logic.
    if is_script_borrow_active() {
        queue_mutation(Mutation::Delete(10));
        queue_mutation(Mutation::Delete(11));
    }
    assert_eq!(mutation_queue_len(), 2);
    reset();
}
