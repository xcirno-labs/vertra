use winit::{
    window::WindowBuilder,
    dpi::PhysicalSize
};
use std::sync::Arc;

use crate::event::{Event, EventLoopWindowTarget, EventLoop, WindowEvent};
use crate::pipeline::{Pipeline};
use crate::camera::Camera;
use crate::mesh::MeshRegistry;
use crate::scene::Scene;
use crate::constants::window;
use crate::world::World;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast; // Provides .dyn_into()

#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowBuilderExtWebSys; // Provides .with_canvas()

pub struct FrameContext {
    pub dt: f32,
}

type DrawCallback<S> = Box<dyn FnMut(&mut S, &mut Scene, &mut FrameContext)>;
type EventCallback<S> = Box<dyn FnMut(&mut S, &mut Scene, Event<()>, &EventLoopWindowTarget<()>)>;
type CloseCallback<S> = Box<dyn FnMut(&mut S, WindowEvent, &EventLoopWindowTarget<()>)>;

pub struct WindowConfig {
    pub title: String,
    pub height: u32,
    pub width: u32,
    pub minimum_dimension: [u32; 2],
    pub canvas_id: Option<String>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "untitled".into(),
            width: window::DEFAULT_WIDTH,
            height: window::DEFAULT_HEIGHT,
            minimum_dimension: window::MIN_DIMENSION,
            canvas_id: None,
        }
    }
}

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
    camera: Option<Camera>,
}

impl<S> Window<S> {
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
            camera: None
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.config.title = title.into();
        self
    }

    pub fn with_dimensions(mut self, width: u32, height: u32) -> Self {
        self.config.width = width;
        self.config.height = height;
        self
    }

    pub fn with_camera(mut self, camera: Camera) -> Self {
        let camera = camera.with_aspect(
            self.config.width as f32 / self.config.height as f32
        );
        self.camera = Some(camera);
        self
    }

    pub fn with_canvas_id(mut self, id: impl Into<String>) -> Self {
        self.config.canvas_id = Some(id.into());
        self
    }

    // Setters wrap the input in a Box automatically
    pub fn with_event_handler<F>(mut self, function: F) -> Self
    where F: FnMut(&mut S, &mut Scene, Event<()>, &EventLoopWindowTarget<()>) + 'static {
        self.event_handler = Some(Box::new(function));
        self
    }

    pub fn on_update<F>(mut self, function: F) -> Self
    where F: FnMut(&mut S, &mut Scene, &mut FrameContext) + 'static {
        self.on_update_fn = Some(Box::new(function));
        self
    }

    pub fn on_draw_request<F>(mut self, function: F) -> Self
    where F: FnMut(&mut S, &mut Scene, &mut FrameContext) + 'static {
        self.on_draw_requested_fn = Some(Box::new(function));
        self
    }

    pub fn on_window_close<F>(mut self, function: F) -> Self
    where F: FnMut(&mut S, WindowEvent, &EventLoopWindowTarget<()>) + 'static {
        self.on_window_close_fn = Box::new(function);
        self
    }

    pub fn on_startup<F>(mut self, function: F) -> Self
    where F: FnMut(&mut S, &mut Scene, &mut FrameContext) + 'static {
        self.on_startup_fn = Some(Box::new(function));
        self
    }

    pub fn on_fixed_update<F>(mut self, function: F) -> Self
    where F: FnMut(&mut S, &mut Scene, &mut FrameContext) + 'static {
        self.on_fixed_update_fn = Some(Box::new(function));
        self
    }

    pub fn create(mut self) {
        let event_loop = EventLoop::new().unwrap();

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
            // We clone what we need to move into the async block
            let window_handle_clone = Arc::clone(&window_handle);

            wasm_bindgen_futures::spawn_local(async move {
                // 1. Wait for GPU/Pipeline to initialize
                let pipeline = Pipeline::initialize(Arc::clone(&window_handle_clone)).await;

                // 2. Start the loop!
                self.run_loop(event_loop, pipeline, window_handle_clone);
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            // On Desktop, we block locally just for the init, then run the loop
            let pipeline = pollster::block_on(Pipeline::initialize(Arc::clone(&window_handle)));
            self.run_loop(event_loop, pipeline, window_handle);
        }
    }
    fn run_loop(mut self, event_loop: EventLoop<()>, pipeline: Pipeline, window_handle: Arc<winit::window::Window>) {
        let mesh_registry = MeshRegistry::new();
        let mut last_update_inst = web_time::Instant::now();
        let camera = self.camera.unwrap_or_else(|| {
            Camera::new().with_aspect(self.config.width as f32 / self.config.height as f32)
        });
        let mut scene = Scene {
            pipeline,
            mesh_registry,
            camera,
            world: World::new(),
        };
        if let Some(startup_fn) = &mut self.on_startup_fn {
            startup_fn(&mut self.state, &mut scene, &mut FrameContext {dt: 0.0});
        }
        let mut accumulator = 0.0;

        let main_loop = move |event: Event<()>, elwt: &EventLoopWindowTarget<()>| {
            let now = web_time::Instant::now();
            let dt = now.duration_since(last_update_inst).as_secs_f32();
            last_update_inst = now;

            if let Some(update_fn) = &mut self.on_update_fn {
                update_fn(&mut self.state, &mut scene, &mut FrameContext { dt } );
            }

            // Handle all events (including AboutToWait)
            if let Some(event_handler) = &mut self.event_handler {
                event_handler(&mut self.state, &mut scene, event.clone(), elwt);
            }

            match event {
                Event::AboutToWait => {
                    accumulator += dt;
                    while accumulator >= window::FIXED_DELTA {
                        if let Some(fixed_update) = &mut self.on_fixed_update_fn {
                            fixed_update(&mut self.state, &mut scene, &mut FrameContext { dt: window::FIXED_DELTA });
                        }
                        accumulator -= window::FIXED_DELTA;
                    }

                    window_handle.request_redraw();
                }
                Event::WindowEvent { event: window_event, .. } => {
                    match window_event {
                        WindowEvent::CloseRequested => (self.on_window_close_fn)(&mut self.state, window_event, elwt),
                        WindowEvent::RedrawRequested => {
                            if let Some(handler) = &mut self.on_draw_requested_fn {
                                handler(&mut self.state, &mut scene, &mut FrameContext { dt });
                            }
                            scene.draw_world();
                        }
                        WindowEvent::Resized(new_size) => {
                            scene.pipeline.resize(new_size);
                            scene.camera.aspect = new_size.width as f32 / new_size.height as f32;
                        }
                        _ => ()
                    }
                }
                _ => (),
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            event_loop.run(main_loop).unwrap();
        }

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::EventLoopExtWebSys;
            event_loop.spawn(main_loop);
        }
    }
}

