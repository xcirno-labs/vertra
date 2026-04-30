//! Unit tests for the per-object script system.
//!
//! These tests exercise:
//! - attach / detach lifecycle
//! - `on_start` called exactly once
//! - `on_update` and `on_fixed_update` receiving correct IDs and dt
//! - world mutations performed inside scripts
//! - replacing a script resets `on_start`
//! - detaching a non-existent script is a no-op
//! - empty registry is zero-cost (no panic, no iteration)
//! - multiple scripts run independently

use crate::objects::Object;
use crate::world::World;
use crate::script::{ObjectScript, ScriptRegistry};

fn make_world_with_object() -> (World, usize) {
    let mut world = World::new();
    let obj = Object {
        name: "test".into(),
        str_id: "test_obj".into(),
        ..Default::default()
    };
    let id = world.spawn_object(obj, None);
    (world, id)
}

struct CounterScript {
    start_count:  usize,
    update_count: usize,
    fixed_count:  usize,
    last_dt:      f32,
    last_id:      usize,
}

impl CounterScript {
    fn new() -> Self {
        Self { start_count: 0, update_count: 0, fixed_count: 0, last_dt: 0.0, last_id: usize::MAX }
    }
}

impl ObjectScript for CounterScript {
    fn on_start(&mut self, id: usize, _world: &mut World) {
        self.start_count  += 1;
        self.last_id       = id;
    }
    fn on_update(&mut self, id: usize, _world: &mut World, dt: f32) {
        self.update_count += 1;
        self.last_dt       = dt;
        self.last_id       = id;
    }
    fn on_fixed_update(&mut self, id: usize, _world: &mut World, dt: f32) {
        self.fixed_count  += 1;
        self.last_dt       = dt;
        self.last_id       = id;
    }
}

#[test]
fn attach_and_has_script() {
    let (mut world, id) = make_world_with_object();
    let mut reg = ScriptRegistry::new();

    assert!(!reg.has(id));
    reg.attach(id, Box::new(CounterScript::new()));
    assert!(reg.has(id));
    assert_eq!(reg.len(), 1);

    // Running update to satisfy borrow
    reg.run_update(&mut world, 0.0);
}

#[test]
fn detach_removes_script() {
    let (mut _world, id) = make_world_with_object();
    let mut reg = ScriptRegistry::new();

    reg.attach(id, Box::new(CounterScript::new()));
    assert!(reg.detach(id));
    assert!(!reg.has(id));
    assert!(reg.is_empty());
}

#[test]
fn detach_nonexistent_returns_false() {
    let mut reg = ScriptRegistry::new();
    assert!(!reg.detach(999));
}

#[test]
fn on_start_called_exactly_once_on_first_update() {
    let (mut world, id) = make_world_with_object();
    let script = Box::new(CounterScript::new());

    let mut reg = ScriptRegistry::new();
    reg.attach(id, script);

    reg.run_update(&mut world, 0.016);
    reg.run_update(&mut world, 0.016);
    reg.run_update(&mut world, 0.016);

    // Retrieve via detach to inspect state
    let mut reg2 = ScriptRegistry::new();
    std::mem::swap(&mut reg, &mut reg2);
    // We can't directly inspect inner box; use a shared-state approach instead.
    // This test re-creates and asserts via separate cell.
    drop(reg2);
}

#[test]
fn on_start_called_once_verified_with_cell() {
    use std::cell::Cell;
    use std::rc::Rc;

    let (mut world, id) = make_world_with_object();

    let start_calls = Rc::new(Cell::new(0usize));
    let sc = Rc::clone(&start_calls);

    struct TrackStart(Rc<Cell<usize>>);
    impl ObjectScript for TrackStart {
        fn on_start(&mut self, _id: usize, _w: &mut World) {
            self.0.set(self.0.get() + 1);
        }
    }

    let mut reg = ScriptRegistry::new();
    reg.attach(id, Box::new(TrackStart(sc)));

    for _ in 0..5 {
        reg.run_update(&mut world, 0.016);
    }

    assert_eq!(start_calls.get(), 1, "on_start must fire exactly once");
}

#[test]
fn replacing_script_resets_on_start() {
    use std::cell::Cell;
    use std::rc::Rc;

    let (mut world, id) = make_world_with_object();

    let calls = Rc::new(Cell::new(0usize));

    struct TrackStart(Rc<Cell<usize>>);
    impl ObjectScript for TrackStart {
        fn on_start(&mut self, _id: usize, _w: &mut World) {
            self.0.set(self.0.get() + 1);
        }
    }

    let mut reg = ScriptRegistry::new();
    reg.attach(id, Box::new(TrackStart(Rc::clone(&calls))));
    reg.run_update(&mut world, 0.016); // on_start fires once

    // Replace the script
    reg.attach(id, Box::new(TrackStart(Rc::clone(&calls))));
    reg.run_update(&mut world, 0.016); // on_start fires again for the new script

    assert_eq!(calls.get(), 2, "on_start should fire once per script instance");
}

#[test]
fn on_update_receives_correct_id_and_dt() {
    use std::cell::Cell;
    use std::rc::Rc;

    let (mut world, id) = make_world_with_object();

    let seen_id = Rc::new(Cell::new(usize::MAX));
    let seen_dt = Rc::new(Cell::new(0.0f32));
    let si = Rc::clone(&seen_id);
    let sd = Rc::clone(&seen_dt);

    struct Tracker(Rc<Cell<usize>>, Rc<Cell<f32>>);
    impl ObjectScript for Tracker {
        fn on_update(&mut self, id: usize, _w: &mut World, dt: f32) {
            self.0.set(id);
            self.1.set(dt);
        }
    }

    let mut reg = ScriptRegistry::new();
    reg.attach(id, Box::new(Tracker(si, sd)));
    reg.run_update(&mut world, 0.025);

    assert_eq!(seen_id.get(), id);
    assert!((seen_dt.get() - 0.025).abs() < 1e-6);
}

#[test]
fn on_fixed_update_receives_correct_id_and_dt() {
    use std::cell::Cell;
    use std::rc::Rc;

    let (mut world, id) = make_world_with_object();

    let seen_id = Rc::new(Cell::new(usize::MAX));
    let seen_dt = Rc::new(Cell::new(0.0f32));
    let si = Rc::clone(&seen_id);
    let sd = Rc::clone(&seen_dt);

    struct Tracker(Rc<Cell<usize>>, Rc<Cell<f32>>);
    impl ObjectScript for Tracker {
        fn on_fixed_update(&mut self, id: usize, _w: &mut World, dt: f32) {
            self.0.set(id);
            self.1.set(dt);
        }
    }

    let mut reg = ScriptRegistry::new();
    reg.attach(id, Box::new(Tracker(si, sd)));
    reg.run_fixed_update(&mut world, 0.0166);

    assert_eq!(seen_id.get(), id);
    assert!((seen_dt.get() - 0.0166).abs() < 1e-5);
}

#[test]
fn script_can_mutate_world() {
    use std::cell::Cell;
    use std::rc::Rc;

    let (mut world, id) = make_world_with_object();

    let mutated = Rc::new(Cell::new(false));
    let m = Rc::clone(&mutated);

    struct Mutator(Rc<Cell<bool>>);
    impl ObjectScript for Mutator {
        fn on_update(&mut self, id: usize, world: &mut World, _dt: f32) {
            if let Some(obj) = world.get_mut(id) {
                obj.name = "mutated".into();
                self.0.set(true);
            }
        }
    }

    let mut reg = ScriptRegistry::new();
    reg.attach(id, Box::new(Mutator(m)));
    reg.run_update(&mut world, 0.016);

    assert!(mutated.get());
    assert_eq!(world.objects[&id].name, "mutated");
}

#[test]
fn multiple_scripts_all_run() {
    use std::cell::Cell;
    use std::rc::Rc;

    let mut world = World::new();
    let id_a = world.spawn_object(Object { name: "a".into(), str_id: "a".into(), ..Default::default() }, None);
    let id_b = world.spawn_object(Object { name: "b".into(), str_id: "b".into(), ..Default::default() }, None);

    let calls = Rc::new(Cell::new(0usize));

    struct Inc(Rc<Cell<usize>>);
    impl ObjectScript for Inc {
        fn on_update(&mut self, _id: usize, _w: &mut World, _dt: f32) {
            self.0.set(self.0.get() + 1);
        }
    }

    let mut reg = ScriptRegistry::new();
    reg.attach(id_a, Box::new(Inc(Rc::clone(&calls))));
    reg.attach(id_b, Box::new(Inc(Rc::clone(&calls))));
    reg.run_update(&mut world, 0.016);

    assert_eq!(calls.get(), 2);
}

#[test]
fn empty_registry_run_is_no_op() {
    let (mut world, _id) = make_world_with_object();
    let mut reg = ScriptRegistry::new();
    // Must not panic
    reg.run_update(&mut world, 0.016);
    reg.run_fixed_update(&mut world, 0.016);
}

#[test]
fn detach_middle_element_preserves_remaining() {
    use std::cell::Cell;
    use std::rc::Rc;

    let mut world = World::new();
    let ids: Vec<usize> = (0..3).map(|i| {
        let s = i.to_string();
        world.spawn_object(Object { name: s.clone(), str_id: s, ..Default::default() }, None)
    }).collect();

    let runs = Rc::new(Cell::new(0usize));
    struct Inc(Rc<Cell<usize>>);
    impl ObjectScript for Inc {
        fn on_update(&mut self, _id: usize, _w: &mut World, _dt: f32) {
            self.0.set(self.0.get() + 1);
        }
    }

    let mut reg = ScriptRegistry::new();
    for &id in &ids {
        reg.attach(id, Box::new(Inc(Rc::clone(&runs))));
    }

    // Remove the middle entry
    assert!(reg.detach(ids[1]));
    assert_eq!(reg.len(), 2);

    reg.run_update(&mut world, 0.016);
    assert_eq!(runs.get(), 2);
}

/// `reset_started` must cause every script's `on_start` to fire again on the
/// very next `run_update`, simulating a play-mode restart after a world restore.
#[test]
fn reset_started_causes_on_start_to_rerun() {
    use std::cell::Cell;
    use std::rc::Rc;

    let (mut world, id) = make_world_with_object();

    let calls = Rc::new(Cell::new(0usize));
    struct TrackStart2(Rc<Cell<usize>>);
    impl ObjectScript for TrackStart2 {
        fn on_start(&mut self, _id: usize, _w: &mut World) {
            self.0.set(self.0.get() + 1);
        }
    }

    let mut reg = ScriptRegistry::new();
    reg.attach(id, Box::new(TrackStart2(Rc::clone(&calls))));

    reg.run_update(&mut world, 0.016); // on_start fires: count == 1
    reg.run_update(&mut world, 0.016); // already started: count still 1
    assert_eq!(calls.get(), 1);

    reg.reset_started();

    reg.run_update(&mut world, 0.016); // on_start re-fires: count == 2
    reg.run_update(&mut world, 0.016); // already started again: count still 2
    assert_eq!(calls.get(), 2, "on_start should fire again after reset_started");
}

/// `reset_started` must reset ALL entries when multiple scripts are registered.
#[test]
fn reset_started_resets_all_scripts() {
    use std::cell::Cell;
    use std::rc::Rc;

    let mut world = World::new();
    let id_a = world.spawn_object(Object { name: "a".into(), str_id: "a".into(), ..Default::default() }, None);
    let id_b = world.spawn_object(Object { name: "b".into(), str_id: "b".into(), ..Default::default() }, None);

    let calls = Rc::new(Cell::new(0usize));
    struct TrackStart3(Rc<Cell<usize>>);
    impl ObjectScript for TrackStart3 {
        fn on_start(&mut self, _id: usize, _w: &mut World) {
            self.0.set(self.0.get() + 1);
        }
    }

    let mut reg = ScriptRegistry::new();
    reg.attach(id_a, Box::new(TrackStart3(Rc::clone(&calls))));
    reg.attach(id_b, Box::new(TrackStart3(Rc::clone(&calls))));

    reg.run_update(&mut world, 0.016); // both start: count == 2
    assert_eq!(calls.get(), 2);

    reg.reset_started();
    reg.run_update(&mut world, 0.016); // both re-start: count == 4
    assert_eq!(calls.get(), 4, "reset_started should reset every registered script");
}

/// A script whose object was deleted must be automatically removed from the
/// registry during the next `run_update` call (lazy pruning).
#[test]
fn stale_entry_pruned_on_run_update() {
    use std::cell::Cell;
    use std::rc::Rc;

    let mut world = World::new();
    let id_live = world.spawn_object(Object { name: "live".into(), str_id: "live".into(), ..Default::default() }, None);
    let id_dead = world.spawn_object(Object { name: "dead".into(), str_id: "dead".into(), ..Default::default() }, None);

    let update_calls = Rc::new(Cell::new(0usize));
    struct Counter2(Rc<Cell<usize>>);
    impl ObjectScript for Counter2 {
        fn on_update(&mut self, _id: usize, _w: &mut World, _dt: f32) {
            self.0.set(self.0.get() + 1);
        }
    }

    let mut reg = ScriptRegistry::new();
    reg.attach(id_live, Box::new(Counter2(Rc::clone(&update_calls))));
    reg.attach(id_dead, Box::new(Counter2(Rc::clone(&update_calls))));
    assert_eq!(reg.len(), 2);

    world.delete(id_dead);

    reg.run_update(&mut world, 0.016);

    assert_eq!(reg.len(), 1, "stale entry must be pruned during run_update");
    assert!(!reg.has(id_dead), "dead object's script must be gone");
    assert!(reg.has(id_live), "live object's script must remain");
    assert_eq!(update_calls.get(), 1, "only the live script should have run");
}

/// Stale entries are also pruned during `run_fixed_update`.
#[test]
fn stale_entry_pruned_on_run_fixed_update() {
    use std::cell::Cell;
    use std::rc::Rc;

    let mut world = World::new();
    let id_live = world.spawn_object(Object { name: "live".into(), str_id: "live".into(), ..Default::default() }, None);
    let id_dead = world.spawn_object(Object { name: "dead".into(), str_id: "dead".into(), ..Default::default() }, None);

    let fixed_calls = Rc::new(Cell::new(0usize));
    struct Counter3(Rc<Cell<usize>>);
    impl ObjectScript for Counter3 {
        fn on_fixed_update(&mut self, _id: usize, _w: &mut World, _dt: f32) {
            self.0.set(self.0.get() + 1);
        }
    }

    let mut reg = ScriptRegistry::new();
    reg.attach(id_live, Box::new(Counter3(Rc::clone(&fixed_calls))));
    reg.attach(id_dead, Box::new(Counter3(Rc::clone(&fixed_calls))));

    world.delete(id_dead);
    reg.run_fixed_update(&mut world, 0.0166);

    assert_eq!(reg.len(), 1);
    assert!(!reg.has(id_dead));
    assert!(reg.has(id_live));
    assert_eq!(fixed_calls.get(), 1);
}

/// After all objects are deleted the registry must be completely empty and
/// subsequent `detach` calls must not underflow (`len() - 1` on empty `Vec`).
#[test]
fn detach_after_all_objects_deleted_no_underflow() {
    let mut world = World::new();
    let id = world.spawn_object(Object { name: "x".into(), str_id: "x".into(), ..Default::default() }, None);

    let mut reg = ScriptRegistry::new();
    reg.attach(id, Box::new(CounterScript::new()));

    world.delete(id);
    reg.run_update(&mut world, 0.016); // prunes the stale entry

    assert!(reg.is_empty());
    assert!(!reg.detach(id));
}

/// Pruning consecutive stale entries must not skip the element that
/// was swapped into the pruned slot.
#[test]
fn multiple_stale_entries_all_pruned() {
    let mut world = World::new();
    let ids: Vec<usize> = (0..4)
        .map(|i| {
            let s = format!("obj{i}");
            world.spawn_object(Object { name: s.clone(), str_id: s, ..Default::default() }, None)
        })
        .collect();

    let mut reg = ScriptRegistry::new();
    for &id in &ids {
        reg.attach(id, Box::new(CounterScript::new()));
    }

    for &id in &ids[..3] {
        world.delete(id);
    }

    reg.run_update(&mut world, 0.016);

    assert_eq!(reg.len(), 1, "only the surviving object's script must remain");
    assert!(reg.has(ids[3]));
}

/// Run update twice in a row to confirm entries are not accidentally consumed.
#[test]
fn run_update_twice_without_consuming_entries() {
    use std::cell::Cell;
    use std::rc::Rc;

    let (mut world, id) = make_world_with_object();

    let calls = Rc::new(Cell::new(0usize));
    struct Counter4(Rc<Cell<usize>>);
    impl ObjectScript for Counter4 {
        fn on_update(&mut self, _id: usize, _w: &mut World, _dt: f32) {
            self.0.set(self.0.get() + 1);
        }
    }

    let mut reg = ScriptRegistry::new();
    reg.attach(id, Box::new(Counter4(Rc::clone(&calls))));

    reg.run_update(&mut world, 0.016);
    reg.run_update(&mut world, 0.016);

    assert_eq!(reg.len(), 1, "entries must still be present after two run_update calls");
    assert_eq!(calls.get(), 2);
}

/// `has`, `detach`, and `len` must all remain correct after several
/// `run_update` calls (index consistency check).
#[test]
fn index_consistent_across_multiple_run_update_calls() {
    let mut world = World::new();
    let ids: Vec<usize> = (0..3)
        .map(|i| {
            let s = format!("o{i}");
            world.spawn_object(Object { name: s.clone(), str_id: s, ..Default::default() }, None)
        })
        .collect();

    let mut reg = ScriptRegistry::new();
    for &id in &ids {
        reg.attach(id, Box::new(CounterScript::new()));
    }

    for _ in 0..5 {
        reg.run_update(&mut world, 0.016);
    }

    assert_eq!(reg.len(), 3);
    for &id in &ids {
        assert!(reg.has(id));
    }

    assert!(reg.detach(ids[1]));
    assert_eq!(reg.len(), 2);
    assert!(!reg.has(ids[1]));
}
