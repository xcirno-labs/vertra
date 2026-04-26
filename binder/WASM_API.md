# Vertra WASM API Reference

The `vertra-binder` crate compiles to a WebAssembly module that exposes Vertra's
scene editor to JavaScript / TypeScript.

---

## Quick start

```bash
# Build the WASM package (requires wasm-pack)
wasm-pack build --target web --out-dir pkg
```

```html
<canvas id="vertra-canvas" width="1280" height="720"></canvas>
<script type="module">
import init, { WebWindow, Camera } from './pkg/vertra_binder.js';

await init();

const cam = new Camera();
cam.set_position(0, 8, -12);
cam.set_rotation(90, -30);

const win = new WebWindow(cam, { /* your state */ });

win.on_startup((state, scene, ctx) => {
    scene.enable_editor_mode();

    const sun = /* build an Object */;
    const sunId = scene.spawn(sun, null);
});

win.on_update((state, scene, ctx) => {
    // per-frame logic
});

win.on_select(data => {
    // data is InspectorData | undefined
    if (data) console.log('Selected:', data.name, data.id);
});

win.start('vertra-canvas');
</script>
```

---

## `WebWindow`

The top-level application controller.

### Constructor

```ts
new WebWindow(camera: Camera, state?: any): WebWindow
```

| Parameter | Type     | Description                                   |
|-----------|----------|-----------------------------------------------|
| `camera`  | `Camera` | Initial camera (position, rotation, FOV, …)  |
| `state`   | `any`    | Arbitrary JS object forwarded to every callback |

### Lifecycle callbacks

| Method                                      | Signature                                    | When called                   |
|---------------------------------------------|----------------------------------------------|-------------------------------|
| `on_startup(f)`                             | `(state, scene, ctx) => void`                | Once before the first frame   |
| `on_update(f)`                              | `(state, scene, ctx) => void`                | Every frame                   |
| `on_draw_request(f)`                        | `(state, scene, ctx) => void`                | When a redraw is needed       |
| `with_event_handler(f)`                     | `(state, scene, event) => void`              | Every browser input event     |
| `on_select(f)`                              | `(data: InspectorData \| undefined) => void` | Inspector selection changes   |

`FrameContext` has a single field:
```ts
interface FrameContext { dt: number; } // seconds since last frame
```

### `start(canvas_id: string)`

Initialises the WebGPU pipeline and begins the render loop against the given
`<canvas>` element.  **Call this last**, after registering all callbacks.

---

## `Scene`

Returned to every callback; wraps the live scene graph.

### Object management

```ts
scene.spawn(object: VertraObject, parent_id?: number): number
```
Adds an object to the scene.  Returns the new integer ID.

### Camera

```ts
scene.camera   // getter → Camera (wraps scene's camera)
```

### World

```ts
scene.world    // getter → World  (wraps scene's world graph)
```

### Editor

```ts
scene.editor   // getter → Editor (wraps editor state)
```

---

## Editor mode

### Enable

```ts
scene.enable_editor_mode(): void
```

Call once from `on_startup`.  Activates orbit / pan / zoom camera controls,
object picking, gizmo overlay, and the golden selection box.

```ts
scene.editor.is_editor_mode(): boolean
```

---

## Selection

All selection APIs live under `scene.editor`.

### Inspector (primary selection)

```ts
scene.editor.inspector(): InspectorData | undefined
scene.editor.clear_inspector(): void
```

`InspectorData` shape:

```ts
interface InspectorData {
    id:           number;
    name:         string;
    str_id:       string;
    position:     [number, number, number];
    rotation_deg: [number, number, number];
    scale:        [number, number, number];
    color:        [number, number, number, number];
    geometry_type: string | null;
    texture_path:  string | null;
}
```

### Multi-selection (Ctrl+Click / programmatic)

| Method                                         | Description                                       |
|------------------------------------------------|---------------------------------------------------|
| `scene.editor.multi_selected_ids(): number[]`  | All IDs currently in the multi-selection         |
| `scene.editor.is_multi_selected(id): boolean`  | True if `id` is in the current multi-selection  |
| `scene.editor.set_multi_selected(ids: number[])` | Replace the multi-selection programmatically  |
| `scene.editor.clear_selection()`               | Clear inspector + multi-select + group expansion  |

```ts
// Programmatically select two objects and move them together
scene.editor.set_multi_selected([sunId, planetId]);
// Now dragging the gizmo moves both objects simultaneously.
```

### Group expansion (G key)

```ts
scene.editor.group_ids(): number[]   // IDs in the current G-key expansion
```

Pressing **G** while an object is selected recursively adds all its children and
grandchildren to the group.  The gold bounding box and gizmo then encompass the
entire sub-tree.  A plain click clears the group.

---

## Camera controls (editor mode)

| Action              | Input                       |
|---------------------|-----------------------------|
| Free-look rotate    | **Alt** + left-drag         |
| Pan                 | Middle-drag                 |
| Zoom                | Scroll wheel                |
| WASD fly            | W / A / S / D               |
| Fast fly            | **Shift** + WASD (3×)       |
| Focus on selection  | **F**                       |
| Expand to children  | **G**                       |

```ts
scene.editor.set_camera_speed(10.0);  // world units / second  (default: 5.0)
scene.editor.set_pivot(x, y, z);      // manually reposition the orbit pivot
scene.editor.get_pivot();             // → [x, y, z] | undefined
```

---

## Forwarding input events

If your canvas is inside a custom UI shell that intercepts raw events, forward
them manually via `scene.editor.editor_event(payload)`.

`payload` must match `EditorEventPayload`:

```ts
type EditorEventPayload =
  | { type: "mouse_motion";  dx: number; dy: number }
  | { type: "cursor_moved";  x: number;  y: number  }
  | { type: "mouse_button";  left?: boolean; middle?: boolean; right?: boolean }
  | { type: "scroll";        delta: number }
  | { type: "modifiers";     alt: boolean; ctrl: boolean }
  | { type: "focus_key" }
  | { type: "key_pressed";   code: string }  // winit KeyCode string, e.g. "KeyW"
  | { type: "key_released";  code: string };
```

### Wiring example (pointer-lock)

```ts
canvas.addEventListener('pointermove', e => {
    scene.editor.editor_event({ type: 'cursor_moved', x: e.clientX, y: e.clientY });
    scene.editor.editor_event({ type: 'mouse_motion', dx: e.movementX, dy: e.movementY });
});

canvas.addEventListener('mousedown', e => {
    scene.editor.editor_event({ type: 'mouse_button', left: e.button === 0 ? true : undefined });
});

canvas.addEventListener('mouseup', e => {
    scene.editor.editor_event({ type: 'mouse_button', left: e.button === 0 ? false : undefined });
});

canvas.addEventListener('wheel', e => {
    scene.editor.editor_event({ type: 'scroll', delta: -e.deltaY * 0.01 });
});

// Send modifiers on EVERY keydown/up so the state is always current
const sendMods = e =>
    scene.editor.editor_event({ type: 'modifiers', alt: e.altKey, ctrl: e.ctrlKey });

window.addEventListener('keydown', e => {
    sendMods(e);
    scene.editor.editor_event({ type: 'key_pressed',  code: e.code });
});
window.addEventListener('keyup', e => {
    sendMods(e);
    scene.editor.editor_event({ type: 'key_released', code: e.code });
});
```

### Supported key codes

Key code strings follow winit's `KeyCode` debug representation:

| Keys                      | Strings                                              |
|---------------------------|------------------------------------------------------|
| Letters                   | `"KeyA"` … `"KeyZ"`                                 |
| Shift                     | `"ShiftLeft"`, `"ShiftRight"`                       |
| Control                   | `"ControlLeft"`, `"ControlRight"`                   |
| Alt                       | `"AltLeft"`, `"AltRight"`                           |
| Arrows                    | `"ArrowUp"`, `"ArrowDown"`, `"ArrowLeft"`, `"ArrowRight"` |
| Misc                      | `"Space"`, `"Escape"`                               |

> **Tip:** `e.code` from browser `KeyboardEvent` already produces the same
> strings (`"KeyW"`, `"ShiftLeft"`, …), so you can pass it directly.

---

## Scene persistence (VTR format)

```ts
// Export
const bytes: Uint8Array = scene.save_vtr();

// Import
await scene.load_vtr(bytes);
```

The VTR binary format preserves the full scene hierarchy, all object transforms,
colours, **texture paths**, and the camera state.

---

## Textures

Textures are identified by a string key that **must match** the `texture_path`
property of the objects that should display them.

### Loading a texture

The browser has no direct file-system access, so you must decode the image
yourself and hand the raw RGBA pixels to the engine:

```ts
// 1. Fetch + decode
const blob    = await fetch('assets/texture.png').then(r => r.blob());
const img     = await createImageBitmap(blob);

// 2. Draw into an OffscreenCanvas to extract raw RGBA bytes
const cvs     = new OffscreenCanvas(img.width, img.height);
const ctx2d   = cvs.getContext('2d')!;
ctx2d.drawImage(img, 0, 0);
const pixels  = ctx2d.getImageData(0, 0, img.width, img.height);

// 3. Upload to the GPU (called from on_startup or any callback)
scene.load_texture_from_rgba(
    'assets/texture.png',   // key — must match VertraObject.texture_path
    img.width,
    img.height,
    pixels.data,            // Uint8ClampedArray: R,G,B,A per pixel
);
```

### Applying a texture to an object

Set `texture_path` on the object **before** spawning it, using the same key
you will later pass to `load_texture_from_rgba`:

```ts
const obj = new VertraObject('My Cube', {
    color:        [1, 1, 1, 1],   // white = texture displayed at full fidelity
    texture_path: 'assets/texture.png',
});
obj.set_geometry(new Geometry('cube', 1.5));
scene.spawn(obj, null);
```

> **Tip:** you can load textures before or after spawning objects — the
> engine resolves `texture_path` keys at draw time, so order doesn't matter.

### Texture API reference

| Method | Signature | Description |
|--------|-----------|-------------|
| `load_texture_from_rgba` | `(key, width, height, data) => void` | Upload raw RGBA8 pixels |
| `unload_texture` | `(key) => boolean` | Remove texture; returns `true` if it existed |
| `has_texture` | `(key) => boolean` | Returns `true` if the key is loaded |

### Full example (startup + texture)

```ts
win.on_startup(async (state, scene, ctx) => {
    // Spawn a textured cube
    const cube = new VertraObject('Cube', {
        color:        [1, 1, 1, 1],
        texture_path: 'assets/texture.png',
    });
    cube.set_geometry(new Geometry('cube', 2.0));
    state.cubeId = scene.spawn(cube, null);

    // Load + upload the texture
    const blob   = await fetch('assets/texture.png').then(r => r.blob());
    const img    = await createImageBitmap(blob);
    const cvs    = new OffscreenCanvas(img.width, img.height);
    const ctx2d  = cvs.getContext('2d')!;
    ctx2d.drawImage(img, 0, 0);
    const pixels = ctx2d.getImageData(0, 0, img.width, img.height);
    scene.load_texture_from_rgba('assets/texture.png', img.width, img.height, pixels.data);

    scene.enable_editor_mode();
});
```

---

## TypeScript definitions

All types above are emitted into `pkg/vertra_binder.d.ts` by `wasm-pack`.
Import them:

```ts
import type { InspectorData, EditorEventPayload } from './pkg/vertra_binder';
```
