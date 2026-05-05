# Vertra

## Project Overview

Vertra is a cross-platform 3D rendering engine (Rust + `wgpu`) with a WASM/JavaScript binder.
The workspace has two crates:
- **`vertra`** (`src/`) – the core engine library (also the native binary via `src/main.rs`)
- **`vertra_js`** (`binder/`) – `wasm-bindgen` wrapper that re-exports the full API to JavaScript

## Build & Test Commands

```powershell
# Native build & tests
cargo build
cargo test

# Run a specific example
cargo run --example solar_system

# Build WASM binder (requires wasm-pack)
cd binder
wasm-pack build --target web

# Run binder integration tests (WASM target)
cd binder
wasm-pack test --headless --firefox
```

## Architecture

### Core Module Responsibilities

| Module | Key types | Notes |
|---|---|---|
| `src/world.rs` | `World`, `SceneGraphEvent` | Scene-graph storage; all hierarchy mutations live here |
| `src/scene.rs` | `Scene` | Thin facade over `World` + `Camera`; entry point for user code |
| `src/objects.rs` | `Object` | Scene node – holds `Transform`, `Geometry`, colour, texture path |
| `src/window.rs` | `Window` | Builder-pattern event-loop host; typed callbacks |
| `src/pipeline.rs` | — | wgpu render pipeline; bakes geometry each frame |
| `src/vtr.rs` | — | Little-endian binary scene format (save/load full hierarchy) |
| `src/script.rs` | `ObjectScript`, `ScriptRegistry` | Per-object behaviour trait; kept separate from `World` |
| `binder/src/internals/mutation.rs` | deferred queue | WASM re-entrancy safety layer |

### Data Flow (one frame)

```
Window event loop
  → on_update callback  (user code mutates World via Scene)
  → ScriptRegistry::tick (ObjectScript::on_update per object)
  → pipeline::render
      → walk World tree, bake MeshData grouped by texture_path
      → upload batched draw calls to GPU
      → editor gizmo overlay (separate pass, only in editor mode)
```

### Coordinate System & Conventions

- **Y-up, left-handed**; default camera looks along +Z.
- Rotation angles are **degrees** (Euler, Y → X → Z order) everywhere.
- Integer IDs are stable handles; **call `world.get_id(str)` once and cache the result** – it performs a HashMap lookup and the string map (`name_handles`) is not updated automatically after rename.
- Geometry is **baked every frame** from CPU `MeshData`; no incremental GPU buffer updates.

### Scene-Graph Mutation Rules

Use `World` methods – never mutate `world.objects` directly:
- `spawn_object(object, parent_id)` → returns `usize` ID
- `delete(id)` → removes subtree
- `reparent(id, new_parent)` → cycle-safe

`SceneGraphEvent` is fired after every structural mutation. In the WASM binder, mutations made *inside* a JS script callback are **deferred** via a queue in `binder/src/internals/mutation.rs` to avoid re-entrant borrow conflicts.

### Scripts (`ObjectScript` trait)

Attach behaviour to objects without polluting `World` (which must remain serialisable):
- `on_start` – called once; pre-resolve string→integer IDs here.
- `on_update(id, world, dt)` – variable frame delta.
- `on_fixed_update(id, world, dt)` – fixed 60 Hz (constant in `src/constants.rs`).
Scripts are suppressed while the built-in editor is active (same rule as `on_update`).

### Built-in Editor

Enabled with `scene.enable_editor_mode()`. While active, `on_update`, `on_fixed_update`, and `on_draw_request` are **suppressed**. Exit with `Escape`. Gizmos: `T` translate, `R` rotate, `E` scale.

### VTR Binary Format

Compact, little-endian binary (~84 bytes minimum for an empty scene). Roundtrips camera + full hierarchy. Use `scene.save_vtr_file` / `scene.load_vtr_file` on native; `vtr::write` / `vtr::read` for any `Write`/`Read` impl.
VTR deserialization uses `World::from_parts` to rebuild without re-triggering `spawn_object` (preserves original IDs).

### `default-fonts` Feature

Drop font files into `src/fonts/` (`sans.ttf`, `serif.ttf`, `mono.ttf`) and enable the `default-fonts` Cargo feature to make `text_overlay::DefaultFont::{Sans, Serif, Mono}` available without a manual `add_font` call.

## Test Layout

Unit/integration tests live in `src/tests/` (gated `#[cfg(test)]`):
- `test_vtr.rs` – VTR roundtrip
- `test_scene_graph_events.rs` – mutation event assertions
- `test_scripts.rs` – `ObjectScript` lifecycle
- `test_snapshot.rs`, `test_frame_stats.rs`, `test_text_overlay.rs`, `test_timer.rs`

Binder-side tests are in `binder/tests/mutation.rs` (run under `wasm-bindgen-test`).

## Key Files to Read First

1. `src/world.rs` – understand the scene graph before touching hierarchy code
2. `src/scene.rs` – public API surface used in every example
3. `binder/src/internals/mutation.rs` – critical if modifying WASM mutation paths
4. `examples/solar_system.rs` – canonical usage pattern

## Important Notes
- Always run tests after making changes to ensure no regressions, especially in the scene graph and VTR serialization logic.
- When adding new features, consider whether they belong in the core `vertra` crate or should be exposed via the `vertra_js` binder.
- Always write production-quality code
- If possible, create tests for new features and edge cases to maintain code quality and prevent future regressions.
- Always write documentation comments for new public types and methods, and update this guide if the architecture or conventions change.
- Internal code belongs to `internals/`, so write any internal code there.