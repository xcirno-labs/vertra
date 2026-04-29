/// Per-object behaviour callbacks.
///
/// Implement this trait to attach custom logic to any scene object.
/// The runtime calls each method at the appropriate point in the frame
/// loop **only while editor mode is inactive** (same suppression rule as
/// [`crate::window::Window::on_update`]).
///
/// # Efficiency
///
/// Scripts are stored in a [`ScriptRegistry`] that is kept entirely
/// separate from the serialisable [`crate::world::World`].  When no
/// scripts are attached the hot paths in the window loop are zero-cost:
/// the empty-check short-circuits before any iteration.  When scripts
/// *are* present the registry temporarily moves its `Vec` out of
/// `self` (an O(1) pointer swap) so that the script closures can hold
/// an exclusive reference to the world simultaneously, with no heap
/// allocation per frame.
pub trait ObjectScript: 'static {
    /// Called once the first time the registry runs its update pass after
    /// the script was attached.  Use it to pre-resolve string IDs into
    /// integer IDs and to initialise per-object state.
    fn on_start(&mut self, id: usize, world: &mut crate::world::World) {
        let _ = (id, world);
    }

    /// Called every frame before rendering (variable delta-time).
    ///
    /// `dt` is elapsed time in seconds since the previous frame.
    fn on_update(&mut self, id: usize, world: &mut crate::world::World, dt: f32) {
        let _ = (id, world, dt);
    }

    /// Called at the fixed timestep (default 60 Hz, independent of frame rate).
    ///
    /// `dt` is the fixed timestep duration in seconds
    /// ([`crate::constants::window::FIXED_DELTA`]).
    fn on_fixed_update(&mut self, id: usize, world: &mut crate::world::World, dt: f32) {
        let _ = (id, world, dt);
    }
}

struct ScriptEntry {
    id:      usize,
    script:  Box<dyn ObjectScript>,
    started: bool,
}

/// Per-scene registry that maps object IDs to their [`ObjectScript`]
/// implementations.
///
/// Stored in [`crate::scene::Scene`] (not in `World`) so scripts never
/// interfere with scene serialisation.
///
/// # Thread safety
///
/// `ScriptRegistry` is `!Send` / `!Sync` because trait objects are
/// heap-allocated with `Box<dyn ObjectScript>`.  This matches the rest of
/// the engine which is single-threaded.
#[derive(Default)]
pub struct ScriptRegistry {
    /// Flat storage – optimised for iteration (hot path).
    entries: Vec<ScriptEntry>,
    /// Index map for O(1) attach / detach lookups.
    index:   std::collections::HashMap<usize, usize>,
}

impl ScriptRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Attach `script` to object `id`.
    ///
    /// If a script is already attached to `id` it is replaced.  `on_start`
    /// will be called for the new script on the next update pass.
    pub fn attach(&mut self, id: usize, script: Box<dyn ObjectScript>) {
        if let Some(&idx) = self.index.get(&id) {
            // Replace in-place; reset started flag so on_start runs again.
            self.entries[idx].script  = script;
            self.entries[idx].started = false;
        } else {
            let idx = self.entries.len();
            self.entries.push(ScriptEntry { id, script, started: false });
            self.index.insert(id, idx);
        }
    }

    /// Detach and drop the script for object `id`.
    ///
    /// Returns `true` if a script existed, `false` if `id` had no script.
    pub fn detach(&mut self, id: usize) -> bool {
        let Some(idx) = self.index.remove(&id) else { return false; };

        // swap_remove is O(1), swap last element into this slot.
        let last_idx = self.entries.len() - 1;
        if idx != last_idx {
            self.entries.swap(idx, last_idx);
            // Update the moved element's index entry.
            let moved_id = self.entries[idx].id;
            self.index.insert(moved_id, idx);
        }
        self.entries.pop();
        true
    }

    /// Returns `true` when object `id` has an attached script.
    pub fn has(&self, id: usize) -> bool {
        self.index.contains_key(&id)
    }

    /// Number of scripts currently registered.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` when no scripts are registered.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    
    /// Run `on_start` (if needed) + `on_update` for every registered script.
    ///
    /// Internally the entry `Vec` is temporarily *moved out* of `self` via
    /// `std::mem::take` (an O(1) pointer swap, zero allocation) so that both
    /// the script and the `&mut World` borrow can be alive concurrently.
    /// After iteration the `Vec` is moved back.
    pub(crate) fn run_update(&mut self, world: &mut crate::world::World, dt: f32) {
        if self.entries.is_empty() { return; }

        let mut entries = std::mem::take(&mut self.entries);
        for entry in &mut entries {
            if !entry.started {
                entry.script.on_start(entry.id, world);
                entry.started = true;
            }
            entry.script.on_update(entry.id, world, dt);
        }
        self.entries = entries;
    }

    /// Run `on_start` (if needed) + `on_fixed_update` for every registered
    /// script.
    pub(crate) fn run_fixed_update(&mut self, world: &mut crate::world::World, dt: f32) {
        if self.entries.is_empty() { return; }

        let mut entries = std::mem::take(&mut self.entries);
        for entry in &mut entries {
            if !entry.started {
                entry.script.on_start(entry.id, world);
                entry.started = true;
            }
            entry.script.on_fixed_update(entry.id, world, dt);
        }
        self.entries = entries;
    }
}

