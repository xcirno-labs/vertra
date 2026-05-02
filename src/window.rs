use winit::{
    window::WindowBuilder,
    dpi::PhysicalSize
};
use std::sync::Arc;
use crate::event::{
    Event, EventLoopWindowTarget, EventLoop, WindowEvent,
    MouseButton, MouseScrollDelta, ElementState, DeviceEvent,
};
use crate::pipeline::Pipeline;
use crate::frame_stats::FrameStats;
use crate::camera::Camera;
use crate::mesh::MeshRegistry;
use crate::scene::Scene;
use crate::editor::{EditorEvent, EditorStateEvent};
use crate::constants::window;
use crate::objects::Object;
use crate::world::World;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowBuilderExtWebSys;
/// Per-frame timing information passed to every callback.
pub struct FrameContext<'a> {
    /// Delta-time in seconds since the previous frame.
    pub dt: f32,
    /// Smoothed frame statistics updated on a sampling window.
    pub stats: &'a FrameStats,
}
type DrawCallback<S>             = Box<dyn for<'a> FnMut(&mut S, &mut Scene, &mut FrameContext<'a>)>;
type EventCallback<S>            = Box<dyn FnMut(&mut S, &mut Scene, Event<()>, &EventLoopWindowTarget<()>)>;
type CloseCallback<S>            = Box<dyn FnMut(&mut S, WindowEvent, &EventLoopWindowTarget<()>)>;
type EditorStateEventCallback<S> = Box<dyn FnMut(&mut S, &mut Scene, EditorStateEvent, Option<Object>)>;

/// Initial window configuration.
///
/// Populated via [`Window`]'s builder methods; you would not normally construct
/// this directly.
pub struct WindowConfig {
    /// OS window title bar text.
    pub title: String,
    /// Initial window height in physical pixels.
    pub height: u32,
    /// Initial window width in physical pixels.
    pub width: u32,
    /// Minimum allowed window dimensions `[width, height]` in physical pixels.
    pub minimum_dimension: [u32; 2],
    /// *(WASM only)* `id` attribute of the `<canvas>` element to render into.
    pub canvas_id: Option<String>,
    /// Sleep time between two frame stats.
    pub stats_sample_window_secs: f32,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "untitled".into(),
            width: window::DEFAULT_WIDTH,
            height: window::DEFAULT_HEIGHT,
            minimum_dimension: window::MIN_DIMENSION,
            canvas_id: None,
            stats_sample_window_secs: 0.5,
        }
    }
}
/// Builder-style window handle that wires together engine systems and
/// user-supplied callbacks before opening the OS window.
///
/// # Lifecycle
///
/// ```text
/// Window::new(state)
///     .on_startup(…)         // called once before the loop starts
///     .on_update(…)          // called every frame  ⚠ suppressed in editor mode
///     .on_fixed_update(…)    // called at a fixed timestep  ⚠ suppressed in editor mode
///     .on_draw_request(…)    // called on RedrawRequested  ⚠ suppressed in editor mode
///     .on_editor_event(…)    // called when editor state changes
///     .on_window_close(…)    // called on CloseRequested
///     .create();             // consumes self, opens the OS window, runs the loop
/// ```
///
/// > **Editor mode:** when [`Scene::enable_editor_mode`](crate::scene::Scene::enable_editor_mode)
/// > is active, the `on_update`, `on_fixed_update`, and `on_draw_request` callbacks
/// > are **suppressed** so that game logic does not interfere with the editor.
/// > Use [`on_editor_event`](Window::on_editor_event) to react to editor state
/// > changes instead.
pub struct Window<S: 'static> {
    pub handle: Option<Arc<winit::window::Window>>,
    state: S,
    config: WindowConfig,
    event_handler: Option<EventCallback<S>>,
    on_window_close_fn: CloseCallback<S>,
    on_update_fn: Option<DrawCallback<S>>,
    on_draw_requested_fn: Option<DrawCallback<S>>,
    on_startup_fn: Option<DrawCallback<S>>,
    on_fixed_update_fn: Option<DrawCallback<S>>,
    on_editor_state_event_fn: Option<EditorStateEventCallback<S>>,
    camera: Option<Camera>,
}
impl<S> Window<S> {
    /// Create a new window builder with the given initial application state.
    pub fn new(initial_state: S) -> Self {
        Self {
            state: initial_state,
            handle: None,
            config: WindowConfig::default(),
            event_handler: None,
            on_update_fn: None,
            on_window_close_fn: Box::new(|_, _event, elwt| {
                elwt.exit();
            }),
            on_draw_requested_fn: None,
            on_startup_fn: None,
            on_fixed_update_fn: None,
            on_editor_state_event_fn: None,
            camera: None,
        }
    }
    /// Set the OS window title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.config.title = title.into();
        self
    }
    /// Set the initial window dimensions in physical pixels.
    pub fn with_dimensions(mut self, width: u32, height: u32) -> Self {
        self.config.width = width;
        self.config.height = height;
        self
    }
    /// Attach a pre-configured [`Camera`].  The aspect ratio is automatically
    /// overridden to match the current window size.
    pub fn with_camera(mut self, camera: Camera) -> Self {
        let camera = camera.with_aspect(
            self.config.width as f32 / self.config.height as f32
        );
        self.camera = Some(camera);
        self
    }
    /// *(WASM only)* Attach the renderer to an existing `<canvas>` element by
    /// its HTML `id` attribute.
    pub fn with_canvas_id(mut self, id: impl Into<String>) -> Self {
        self.config.canvas_id = Some(id.into());
        self
    }
    /// Set the time window (in seconds) over which frame statistics are averaged.
    /// Defaults to 0.5 seconds.
    pub fn with_stats_sample_window(mut self, secs: f32) -> Self {
        self.config.stats_sample_window_secs = secs;
        self
    }
    /// Register a raw winit event handler that receives every [`Event`].
    ///
    /// This callback fires even in editor mode and is intended for advanced use
    /// cases.  Prefer [`on_update`](Self::on_update) for normal game logic.
    pub fn with_event_handler<F>(mut self, function: F) -> Self
    where F: FnMut(&mut S, &mut Scene, Event<()>, &EventLoopWindowTarget<()>) + 'static {
        self.event_handler = Some(Box::new(function));
        self
    }
    /// Register a per-frame game update callback.
    ///
    /// Called once per frame **before** rendering.
    ///
    /// > **Suppressed in editor mode.** Use [`on_editor_event`](Self::on_editor_event)
    /// > to react to editor-side changes.
    pub fn on_update<F>(mut self, function: F) -> Self
    where F: for<'a> FnMut(&mut S, &mut Scene, &mut FrameContext<'a>) + 'static {
        self.on_update_fn = Some(Box::new(function));
        self
    }
    /// Register a callback invoked at a fixed timestep (default 60 Hz).
    ///
    /// Useful for physics or other simulation steps that must be
    /// timestep-independent.
    ///
    /// > **Suppressed in editor mode.**
    pub fn on_fixed_update<F>(mut self, function: F) -> Self
    where F: for<'a> FnMut(&mut S, &mut Scene, &mut FrameContext<'a>) + 'static {
        self.on_fixed_update_fn = Some(Box::new(function));
        self
    }
    /// Register a callback invoked every time the OS requests a redraw,
    /// just before [`Scene::draw_world`](crate::scene::Scene::draw_world) runs.
    ///
    /// > **Suppressed in editor mode.**
    pub fn on_draw_request<F>(mut self, function: F) -> Self
    where F: for<'a> FnMut(&mut S, &mut Scene, &mut FrameContext<'a>) + 'static {
        self.on_draw_requested_fn = Some(Box::new(function));
        self
    }
    /// Register a callback for high-level editor state-change events.
    ///
    /// Fired *after* the editor has processed input and its state has actually
    /// changed. Possible variants:
    ///
    /// * [`EditorStateEvent::GizmoModeChanged`] — T / R / E switched the gizmo mode.
    /// * [`EditorStateEvent::SelectionChanged`] — the editor selection changed.
    /// * [`EditorStateEvent::DragStart`] — user began dragging a gizmo axis handle.
    /// * [`EditorStateEvent::DragEnd`] — user released a gizmo axis drag.
    ///
    /// The second argument is a cloned [`Object`] relevant to the event, or `None`.
    ///
    /// Fires even when [`on_update`](Self::on_update) is suppressed, making it
    /// the correct place for logic that should only run during editing.
    pub fn on_editor_event<F>(mut self, function: F) -> Self
    where F: FnMut(&mut S, &mut Scene, EditorStateEvent, Option<Object>) + 'static {
        self.on_editor_state_event_fn = Some(Box::new(function));
        self
    }
    /// Override the default window-close behaviour.
    ///
    /// By default, closing the window exits the event loop.
    pub fn on_window_close<F>(mut self, function: F) -> Self
    where F: FnMut(&mut S, WindowEvent, &EventLoopWindowTarget<()>) + 'static {
        self.on_window_close_fn = Box::new(function);
        self
    }
    /// Register a one-shot startup callback, called once before the event loop
    /// begins.  Use this to spawn objects, load assets, and optionally call
    /// [`Scene::enable_editor_mode`](crate::scene::Scene::enable_editor_mode).
    pub fn on_startup<F>(mut self, function: F) -> Self
    where F: for<'a> FnMut(&mut S, &mut Scene, &mut FrameContext<'a>) + 'static {
        self.on_startup_fn = Some(Box::new(function));
        self
    }
    /// Consume the builder, open the OS window, and start the event loop.
    ///
    /// Does not return on native targets (blocks until the window is closed).
    /// Returns immediately on WASM (the loop is spawned asynchronously).
    pub fn create(mut self) {
        let event_loop = EventLoop::new().unwrap();
        #[allow(unused_mut)]
        let mut builder = WindowBuilder::new()
            .with_inner_size(PhysicalSize::new(self.config.width, self.config.height))
            .with_min_inner_size(PhysicalSize::new(
                self.config.minimum_dimension[0], self.config.minimum_dimension[1]
            ))
            .with_title(self.config.title.clone());
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(id) = &self.config.canvas_id {
                let canvas = web_sys::window()
                    .and_then(|win| win.document())
                    .and_then(|doc| doc.get_element_by_id(id))
                    .and_then(|ent| ent.dyn_into::<web_sys::HtmlCanvasElement>().ok())
                    .expect("Could not find canvas with the provided ID");
                builder = builder.with_canvas(Some(canvas));
            }
        }
        let winit_window = builder.build(&event_loop).unwrap();
        let window_handle = Arc::new(winit_window);
        self.handle = Some(Arc::clone(&window_handle));
        #[cfg(target_arch = "wasm32")]
        {
            let window_handle_clone = Arc::clone(&window_handle);
            wasm_bindgen_futures::spawn_local(async move {
                let pipeline = Pipeline::initialize(Arc::clone(&window_handle_clone)).await;
                self.run_loop(event_loop, pipeline, window_handle_clone);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let pipeline = pollster::block_on(Pipeline::initialize(Arc::clone(&window_handle)));
            self.run_loop(event_loop, pipeline, window_handle);
        }
    }
    fn run_loop(
        mut self,
        event_loop: EventLoop<()>,
        pipeline: Pipeline,
        window_handle: Arc<winit::window::Window>,
    ) {
        fn make_frame_context<'a>(dt: f32, stats: &'a FrameStats) -> FrameContext<'a> {
            FrameContext { dt, stats }
        }

        let mesh_registry = MeshRegistry::new();
        let mut last_update_inst = web_time::Instant::now();
        let mut frame_stats = FrameStats::new()
            .with_sample_window(self.config.stats_sample_window_secs);
        let camera = self.camera.unwrap_or_else(|| {
            Camera::new().with_aspect(self.config.width as f32 / self.config.height as f32)
        });
        // Box the scene so its heap address is stable from this point forward.
        // on_startup fires before the event loop starts; without Boxing the scene
        // lives on the Rust stack here and is later moved into the main_loop
        // closure (and again when the closure is boxed for spawn_local on WASM).
        // Any raw pointer derived from `&mut scene` during on_startup would
        // therefore dangle after the first move.  With Box::new the contents
        // never move, only the thin pointer does, so the address stays valid
        // for the entire lifetime of the engine.
        let mut scene = Box::new(Scene {
            pipeline,
            mesh_registry,
            camera,
            world: World::new(),
            editor: None,
            textures: std::collections::HashMap::new(),
            snapshot: None,
            script_registry: crate::script::ScriptRegistry::new(),
        });
        if let Some(startup_fn) = &mut self.on_startup_fn {
            startup_fn(&mut self.state, &mut *scene, &mut make_frame_context(0.0, &frame_stats));
        }
        let mut accumulator = 0.0_f32;
        let main_loop = move |event: Event<()>, elwt: &EventLoopWindowTarget<()>| {
            let now = web_time::Instant::now();
            let dt  = now.duration_since(last_update_inst).as_secs_f32();
            last_update_inst = now;

            if scene.editor.is_none() {
                scene.run_scripts(dt);
                if let Some(f) = &mut self.on_update_fn {
                    f(&mut self.state, &mut *scene, &mut make_frame_context(dt, &frame_stats));
                }
            }

            if scene.editor.is_some() {
                scene.update_editor(dt);

                let prev_gizmo_mode   = scene.editor.as_ref().map(|ed| ed.gizmo_mode);
                let prev_drag_active  = scene.editor.as_ref().map_or(false, |ed| ed.drag.is_some());
                let prev_drag_obj_id  = scene.editor.as_ref()
                    .and_then(|ed| ed.drag.as_ref().map(|d| d.object_id));
                let prev_selection_id = scene.editor.as_ref()
                    .and_then(|ed| ed.inspector.selected.as_ref().map(|s| s.id));

                dispatch_editor_event(&mut *scene, &event);

                if self.on_editor_state_event_fn.is_some() {
                    let mut to_fire: Vec<(EditorStateEvent, Option<Object>)> = Vec::new();
                    if let Some(ed) = &scene.editor {
                        if let Some(prev) = prev_gizmo_mode {
                            if prev != ed.gizmo_mode {
                                let mode = ed.gizmo_mode;
                                let obj  = ed.inspector.selected.as_ref()
                                    .and_then(|s| scene.world.objects.get(&s.id).cloned());
                                to_fire.push((EditorStateEvent::GizmoModeChanged(mode), obj));
                            }
                        }

                        if !prev_drag_active {
                            if let Some(drag) = &ed.drag {
                                let axis = drag.axis;
                                let obj  = scene.world.objects.get(&drag.object_id).cloned();
                                to_fire.push((EditorStateEvent::DragStart { axis }, obj));
                            }
                        }

                        if prev_drag_active && ed.drag.is_none() {
                            let obj = prev_drag_obj_id
                                .and_then(|id| scene.world.objects.get(&id).cloned());
                            to_fire.push((EditorStateEvent::DragEnd, obj));
                        }

                        let new_selection_id = ed.inspector.selected.as_ref().map(|s| s.id);
                        if prev_selection_id != new_selection_id {
                            let obj = new_selection_id
                                .and_then(|id| scene.world.objects.get(&id).cloned());
                            to_fire.push((EditorStateEvent::SelectionChanged, obj));
                        }
                    }
                    for (ev, obj) in to_fire {
                        if let Some(f) = &mut self.on_editor_state_event_fn {
                            f(&mut self.state, &mut *scene, ev, obj);
                        }
                    }
                }
            }

            if let Some(f) = &mut self.event_handler {
                f(&mut self.state, &mut *scene, event.clone(), elwt);
            }

            match event {
                Event::AboutToWait => {
                    accumulator += dt;
                    while accumulator >= window::FIXED_DELTA {
                        if scene.editor.is_none() {
                            scene.run_fixed_update_scripts(window::FIXED_DELTA);
                            if let Some(f) = &mut self.on_fixed_update_fn {
                                f(
                                    &mut self.state,
                                    &mut *scene,
                                    &mut make_frame_context(window::FIXED_DELTA, &frame_stats),
                                );
                            }
                        }
                        accumulator -= window::FIXED_DELTA;
                    }
                    window_handle.request_redraw();
                }
                Event::WindowEvent { event: window_event, .. } => {
                    match window_event {
                        WindowEvent::CloseRequested => {
                            (self.on_window_close_fn)(&mut self.state, window_event, elwt);
                        }
                        WindowEvent::RedrawRequested => {
                            if scene.editor.is_none() {
                                if let Some(f) = &mut self.on_draw_requested_fn {
                                    f(&mut self.state, &mut *scene, &mut make_frame_context(dt, &frame_stats));
                                }
                            }
                            let render_stats = scene.draw_world();
                            frame_stats.set_gpu_stats(render_stats.draw_calls, render_stats.triangle_count);
                            frame_stats.tick(dt);
                        }
                        WindowEvent::Resized(new_size) => {
                            scene.pipeline.resize(new_size);
                            scene.camera.aspect = new_size.width as f32 / new_size.height as f32;
                            if let Some(ed) = &mut scene.editor {
                                ed.set_viewport_size(new_size.width as f32, new_size.height as f32);
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        event_loop.run(main_loop).unwrap();
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::EventLoopExtWebSys;
            event_loop.spawn(main_loop);
        }
    }
}
/// Convert winit platform events into [`EditorEvent`]s and dispatch them.
/// No-op when editor mode is inactive.
fn dispatch_editor_event(scene: &mut Scene, event: &Event<()>) {
    use winit::keyboard::{PhysicalKey, KeyCode};
    match event {
        Event::WindowEvent { event: wev, .. } => match wev {
            WindowEvent::CursorMoved { position, .. } => {
                scene.handle_editor_event(EditorEvent::CursorMoved {
                    x: position.x as f32,
                    y: position.y as f32,
                });
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = *state == ElementState::Pressed;
                scene.handle_editor_event(EditorEvent::MouseButton {
                    left:   (*button == MouseButton::Left).then_some(pressed),
                    middle: (*button == MouseButton::Middle).then_some(pressed),
                    right:  (*button == MouseButton::Right).then_some(pressed),
                });
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(p)   => p.y as f32 * 0.1,
                };
                scene.handle_editor_event(EditorEvent::Scroll { delta: scroll });
            }
            WindowEvent::ModifiersChanged(mods) => {
                let s = mods.state();
                scene.handle_editor_event(EditorEvent::ModifiersChanged {
                    alt:  s.alt_key(),
                    ctrl: s.control_key(),
                });
            }
            WindowEvent::KeyboardInput { event: ke, .. } => {
                if let PhysicalKey::Code(code) = ke.physical_key {
                    match ke.state {
                        ElementState::Pressed => {
                            scene.handle_editor_event(EditorEvent::KeyPressed(code));
                            if code == KeyCode::KeyF {
                                scene.handle_editor_event(EditorEvent::FocusKey);
                            }
                        }
                        ElementState::Released => {
                            scene.handle_editor_event(EditorEvent::KeyReleased(code));
                        }
                    }
                }
            }
            _ => {}
        },
        Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta }, .. } => {
            scene.handle_editor_event(EditorEvent::MouseMotionDelta {
                dx: delta.0 as f32,
                dy: delta.1 as f32,
            });
        }
        _ => {}
    }
}