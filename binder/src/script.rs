use wasm_bindgen::prelude::*;
use js_sys::Function;
use vertra::world::World as CoreWorld;
use vertra::script::ObjectScript;

/// A per-object script backed by JavaScript callback functions.
///
/// Construct one with [`JsScript::new`], optionally supply
/// `on_start`, `on_update`, and/or `on_fixed_update` JS functions, then
/// attach it to an object via [`Scene::attach_script`].
///
/// Each callback receives:
/// - `id: number`   - the integer object ID
/// - `world: World` - a live handle to the scene world
/// - `dt?: number`  - elapsed time in seconds (absent in `on_start`)
///
/// ```js
/// const script = new JsScript({
///   on_start(id, world) {
///     console.log('started', id);
///   },
///   on_update(id, world, dt) {
///     const obj = world.get_object(id);
///     if (obj) {
///       const t = obj.transform;
///       t.position = [t.position[0], t.position[1] + dt, t.position[2]];
///       obj.transform = t;
///     }
///   },
/// });
/// scene.attach_script(myId, script);
/// ```
pub struct JsObjectScript {
    on_start_fn:        Option<Function>,
    on_update_fn:       Option<Function>,
    on_fixed_update_fn: Option<Function>,
}

impl JsObjectScript {
    pub fn new(
        on_start_fn:        Option<Function>,
        on_update_fn:       Option<Function>,
        on_fixed_update_fn: Option<Function>,
    ) -> Self {
        Self { on_start_fn, on_update_fn, on_fixed_update_fn }
    }
}

impl ObjectScript for JsObjectScript {
    fn on_start(&mut self, id: usize, world: &mut CoreWorld) {
        let Some(f) = &self.on_start_fn else { return; };
        let world_ptr = world as *mut CoreWorld;

        let world_js  = crate::world::World { inner: world_ptr };
        let world_val = JsValue::from(world_js);
        let id_val    = JsValue::from_f64(id as f64);

        crate::internals::mutation::script_borrow_enter();
        let _ = f.call2(&JsValue::UNDEFINED, &id_val, &world_val);

        // SAFETY: The caller guarantees the underlying memory outlives this function.
        crate::internals::mutation::script_borrow_exit(world_ptr);
    }

    fn on_update(&mut self, id: usize, world: &mut CoreWorld, dt: f32) {
        let Some(f) = &self.on_update_fn else { return; };
        let world_ptr = world as *mut CoreWorld;

        let world_js  = crate::world::World { inner: world_ptr };
        let world_val = JsValue::from(world_js);
        let id_val    = JsValue::from_f64(id as f64);
        let dt_val    = JsValue::from_f64(dt as f64);

        crate::internals::mutation::script_borrow_enter();
        let _ = f.call3(&JsValue::UNDEFINED, &id_val, &world_val, &dt_val);

        crate::internals::mutation::script_borrow_exit(world_ptr);
    }

    fn on_fixed_update(&mut self, id: usize, world: &mut CoreWorld, dt: f32) {
        let Some(f) = &self.on_fixed_update_fn else { return; };
        let world_ptr = world as *mut CoreWorld;

        let world_js  = crate::world::World { inner: world_ptr };
        let world_val = JsValue::from(world_js);
        let id_val    = JsValue::from_f64(id as f64);
        let dt_val    = JsValue::from_f64(dt as f64);

        crate::internals::mutation::script_borrow_enter();
        let _ = f.call3(&JsValue::UNDEFINED, &id_val, &world_val, &dt_val);

        crate::internals::mutation::script_borrow_exit(world_ptr);
    }
}

#[wasm_bindgen(typescript_custom_section)]
const TS_JS_SCRIPT: &'static str = r#"
/** Callbacks for a per-object [`JsScript`]. */
export interface JsScriptOptions {
    /** Called once when the script is first activated. */
    on_start?: (id: number, world: World) => void;
    /** Called every frame (variable dt in seconds). */
    on_update?: (id: number, world: World, dt: number) => void;
    /** Called at the fixed timestep (~60 Hz, fixed dt in seconds). */
    on_fixed_update?: (id: number, world: World, dt: number) => void;
}
"#;

#[wasm_bindgen(typescript_custom_section)]
const TS_SCENE_SCRIPT_METHODS: &'static str = r#"
export interface SceneScriptMethods {
    /**
     * Attach a script to object `id`.
     *
     * Replaces any previously attached script.  `on_start` will be called on
     * the next frame before `on_update`.
     */
    attach_script(id: number, script: JsScript): void;
    /**
     * Detach and drop the script for object `id`.
     *
     * Returns `true` if a script existed and was removed.
     */
    detach_script(id: number): boolean;
    /** Returns `true` when object `id` has a script attached. */
    has_script(id: number): boolean;
}
"#;

/// A script object that can be attached to a scene object.
///
/// Create one with the constructor, supplying up to three callback functions,
/// then attach it to an object ID via [`Scene::attach_script`].
#[wasm_bindgen(js_name = JsScript)]
pub struct WasmScript {
    #[wasm_bindgen(skip)]
    pub on_start_fn:        Option<Function>,
    #[wasm_bindgen(skip)]
    pub on_update_fn:       Option<Function>,
    #[wasm_bindgen(skip)]
    pub on_fixed_update_fn: Option<Function>,
}

#[wasm_bindgen(js_name = JsScript)]
impl WasmScript {
    /// Create a new script from an options object with optional callback fields.
    ///
    /// ```js
    /// const script = new JsScript({
    ///   on_update(id, world, dt) { }, // Do anything inside this callback!
    /// });
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(options: JsValue) -> Self {
        let on_start_fn        = js_sys::Reflect::get(&options, &"on_start".into())
            .ok().and_then(|v| v.dyn_into::<Function>().ok());
        let on_update_fn       = js_sys::Reflect::get(&options, &"on_update".into())
            .ok().and_then(|v| v.dyn_into::<Function>().ok());
        let on_fixed_update_fn = js_sys::Reflect::get(&options, &"on_fixed_update".into())
            .ok().and_then(|v| v.dyn_into::<Function>().ok());
        Self { on_start_fn, on_update_fn, on_fixed_update_fn }
    }

    /// Convert into a boxed `JsObjectScript` suitable for the Rust registry.
    ///
    /// Consumes `self` - call this exactly once when passing to `attach_script`.
    pub(crate) fn into_core_script(self) -> Box<dyn ObjectScript> {
        Box::new(JsObjectScript::new(
            self.on_start_fn,
            self.on_update_fn,
            self.on_fixed_update_fn,
        ))
    }
}

