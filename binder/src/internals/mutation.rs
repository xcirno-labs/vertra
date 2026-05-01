//! Internal runtime state for the WASM binder.
//!
//! Everything that must live outside the public WASM API surface lives here:
//!
//! * **Deferred mutation queue**: world-mutating calls made *inside* a script
//!   callback are buffered and replayed immediately after the callback returns,
//!   so the JS author never has to think about the re-entrant borrow problem.
//! * **Script-borrow guard**: a `Cell<bool>` flag that is set for the exact
//!   duration of a JS script callback, used to detect whether a mutation should
//!   be executed immediately or deferred.
//! * **Scene-graph event queue**: structural world events (add / delete /
//!   reparent) are accumulated during a mutation and dispatched to JS only
//!   after the `*mut CoreWorld` borrow has been fully released.

use std::cell::{Cell, RefCell};
use js_sys::Function;
use serde::Serialize;
use vertra::world::{World as CoreWorld, SceneGraphEvent, SceneGraphCallback};
use vertra::objects::Object as CoreObject;
use wasm_bindgen::JsValue;

#[derive(Serialize)]
#[serde(tag = "type", content = "data")]
pub enum SceneGraphModifiedEvent {
    #[serde(rename = "object_added")]
    ObjectAdded { id: usize, parent_id: Option<usize> },
    #[serde(rename = "object_deleted")]
    ObjectDeleted { id: usize },
    #[serde(rename = "object_reparented")]
    ObjectReparented { id: usize, old_parent: Option<usize>, new_parent: Option<usize> },
}

thread_local! {
    static SCENE_GRAPH_CB: RefCell<Option<Function>> = const { RefCell::new(None) };
    /// Pending structural events. Drained after every world-mutation call once
    /// the raw `*mut CoreWorld` borrow has been released.
    static SCENE_GRAPH_QUEUE: RefCell<Vec<SceneGraphModifiedEvent>> =
        const { RefCell::new(Vec::new()) };
}

/// A world mutation that was requested while a script callback was executing.
///
/// Queued via [`queue_mutation`] and replayed against the live world by
/// [`flush_mutations`] immediately after the callback returns.
///
/// # Spawn ID pre-allocation
///
/// `Mutation::Spawn` carries a pre-allocated `id` that was reserved with
/// [`CoreWorld::alloc_id`] before the callback yielded to JS.  This lets the
/// JS caller receive a valid object ID synchronously even though the actual
/// insertion is deferred.  Sequential deferred spawns receive sequential IDs
/// because `alloc_id` bumps the world counter each time.
#[doc(hidden)]
pub enum Mutation {
    /// Insert a new object that was pre-ID-allocated with [`CoreWorld::alloc_id`].
    Spawn {
        id:     usize,
        object: CoreObject,
        parent: Option<usize>,
    },
    /// Remove an object and all its descendants.
    Delete(usize),
    /// Move an object to a new parent (or to the scene root when `None`).
    Reparent { id: usize, new_parent: Option<usize> },
    /// Rename the string identifier of an object.
    Rename { id: usize, new_str_id: String },
}

thread_local! {
    /// `true` for the exact duration of a JS script callback.
    static SCRIPT_BORROW_ACTIVE: Cell<bool> = const { Cell::new(false) };
    /// Mutations deferred because they arrived during a script callback.
    static MUTATION_QUEUE: RefCell<Vec<Mutation>> = const { RefCell::new(Vec::new()) };
}

/// Returns `true` when a script callback is currently executing.
#[doc(hidden)]
#[inline]
pub fn is_script_borrow_active() -> bool {
    SCRIPT_BORROW_ACTIVE.with(|c| c.get())
}

/// Mark the start of a JS script callback.
///
/// Must be paired with exactly one [`script_borrow_exit`] call.
#[doc(hidden)]
#[inline]
pub fn script_borrow_enter() {
    SCRIPT_BORROW_ACTIVE.with(|c| c.set(true));
}

/// Mark the end of a JS script callback, flush all deferred mutations, and
/// dispatch any queued scene-graph events to JS.
///
/// # Safety
///
/// `world_ptr` must point to the same `CoreWorld` that was passed into the
/// script callback and must still be valid (the calling `&mut CoreWorld` is
/// alive for the entire duration of [`JsObjectScript::on_update`] / etc., so
/// this is always the case at the call site in `script.rs`).
///
/// Passing a null pointer is safe **only** when [`MUTATION_QUEUE`] is empty
/// (the pointer will never be dereferenced).
#[doc(hidden)]
pub fn script_borrow_exit(world_ptr: *mut CoreWorld) {
    SCRIPT_BORROW_ACTIVE.with(|c| c.set(false));
    flush_mutations(world_ptr);
    drain_scene_graph_events();
}

/// Push a mutation onto the deferred queue.
#[doc(hidden)]
pub fn queue_mutation(m: Mutation) {
    MUTATION_QUEUE.with(|q| q.borrow_mut().push(m));
}

/// Drain and apply every queued mutation against `world_ptr` in FIFO order.
///
/// This is a no-op when the queue is empty.
///
/// # Safety
///
/// `world_ptr` must be a valid, exclusively-owned pointer.  It is never
/// dereferenced when the queue is empty, so a null pointer is safe in that
/// case.
#[doc(hidden)]
pub fn flush_mutations(world_ptr: *mut CoreWorld) {
    let mutations: Vec<Mutation> =
        MUTATION_QUEUE.with(|q| std::mem::take(&mut *q.borrow_mut()));
    if mutations.is_empty() {
        return;
    }

    // SAFETY: caller guarantees `world_ptr` is valid and exclusively owned.
    let world = unsafe { &mut *world_ptr };

    for m in mutations {
        match m {
            Mutation::Spawn { id, object, parent } => {
                world.insert_spawned(id, object, parent);
            }
            Mutation::Delete(id) => {
                world.delete(id);
            }
            Mutation::Reparent { id, new_parent } => {
                world.reparent(id, new_parent);
            }
            Mutation::Rename { id, new_str_id } => {
                world.rename_str_id(id, new_str_id);
            }
        }
    }
}

/// Returns the number of mutations currently waiting in the deferred queue.
#[doc(hidden)]
pub fn mutation_queue_len() -> usize {
    MUTATION_QUEUE.with(|q| q.borrow().len())
}

/// Returns `true` when the deferred mutation queue is empty.
#[doc(hidden)]
pub fn is_mutation_queue_empty() -> bool {
    MUTATION_QUEUE.with(|q| q.borrow().is_empty())
}

/// Reset all thread-local state to its initial values.
///
/// **Call this at the start of every integration test** that touches the
/// mutation queue or the script-borrow flag, so that thread-local state
/// left over from a previous test cannot affect the current one.
#[doc(hidden)]
pub fn reset_test_state() {
    SCRIPT_BORROW_ACTIVE.with(|c| c.set(false));
    MUTATION_QUEUE.with(|q| q.borrow_mut().clear());
}

pub(crate) fn register_scene_graph_cb(f: Option<Function>) {
    SCENE_GRAPH_CB.with(|c| *c.borrow_mut() = f);
}

/// Push a structural world event onto the pending queue.
///
/// Events are dispatched to JS only after the calling mutation method fully
/// releases its `*mut CoreWorld` borrow, preventing re-entrant aliasing.
pub(crate) fn fire_scene_graph_event(ev: SceneGraphModifiedEvent) {
    SCENE_GRAPH_QUEUE.with(|q| q.borrow_mut().push(ev));
}

/// Dispatch all pending scene-graph events to the registered JS callback and
/// clear the queue.  Must be called after every operation that releases the
/// `*mut CoreWorld` borrow.
pub(crate) fn drain_scene_graph_events() {
    let events: Vec<SceneGraphModifiedEvent> =
        SCENE_GRAPH_QUEUE.with(|q| std::mem::take(&mut *q.borrow_mut()));
    if events.is_empty() {
        return;
    }
    SCENE_GRAPH_CB.with(|c| {
        if let Some(cb) = c.borrow().as_ref() {
            for ev in events {
                if let Ok(js) = serde_wasm_bindgen::to_value(&ev) {
                    let _ = cb.call1(&JsValue::UNDEFINED, &js);
                }
            }
        }
    });
}

/// Install the Rust -> JS scene-graph bridge on `world` so every structural
/// mutation fires the registered JS handler.
pub(crate) fn attach_scene_graph_cb(world: &mut CoreWorld) {
    world.on_scene_graph_modified =
        Some(SceneGraphCallback(Box::new(|ev: SceneGraphEvent| {
            let web_ev = match ev {
                SceneGraphEvent::ObjectAdded { id, parent_id } =>
                    SceneGraphModifiedEvent::ObjectAdded { id, parent_id },
                SceneGraphEvent::ObjectDeleted { id } =>
                    SceneGraphModifiedEvent::ObjectDeleted { id },
                SceneGraphEvent::ObjectReparented { id, old_parent, new_parent } =>
                    SceneGraphModifiedEvent::ObjectReparented { id, old_parent, new_parent },
            };
            fire_scene_graph_event(web_ev);
        })));
}

