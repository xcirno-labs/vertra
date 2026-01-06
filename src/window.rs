use winit::{
    window::WindowBuilder,
    dpi::PhysicalSize
};
use crate::event::{Event, EventLoopWindowTarget, EventLoop, WindowEvent};
use crate::pipeline::{Pipeline};

use std::sync::Arc;
use crate::camera::Camera;
use crate::mesh::Mesh;
use crate::scene::Scene;
use crate::constants::window;

type UpdateCallback = Box<dyn FnMut(f32)>;
type DrawCallback = Box<dyn FnMut(&mut Scene)>;
type EventCallback = Box<dyn FnMut(Event<()>, &EventLoopWindowTarget<()>)>;
type CloseCallback = Box<dyn FnMut(WindowEvent, &EventLoopWindowTarget<()>)>;

pub struct WindowConfig {
    pub title: String,
    pub height: u32,
    pub width: u32,
    pub minimum_dimension: [u32; 2],
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "untitled".into(),
            width: window::DEFAULT_WIDTH,
            height: window::DEFAULT_HEIGHT,
            minimum_dimension: window::MIN_DIMENSION,
        }
    }
}

pub struct Window {
    pub handle: Option<Arc<winit::window::Window>>,
    config: WindowConfig,
    event_handler: Option<EventCallback>,
    on_window_close_fn: CloseCallback,
    on_update_fn: Option<UpdateCallback>,
    on_draw_requested_fn: Option<DrawCallback>,
    camera: Option<Camera>,
}

impl Window {
    pub fn new() -> Self {
        Self {
            handle: None,
            config: WindowConfig::default(),
            event_handler: None,
            on_update_fn: None,
            on_window_close_fn: Box::new(|_event: WindowEvent, elwt: &EventLoopWindowTarget<()>| {
                elwt.exit();
            }),
            on_draw_requested_fn: None,
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

    // Setters wrap the input in a Box automatically
    pub fn with_script<F>(mut self, function: F) -> Self
    where F: FnMut(Event<()>, &EventLoopWindowTarget<()>) + 'static {
        self.event_handler = Some(Box::new(function));
        self
    }

    pub fn on_update<F>(mut self, function: F) -> Self
    where F: FnMut(f32) + 'static {
        self.on_update_fn = Some(Box::new(function));
        self
    }

    pub fn on_draw_request<F>(mut self, function: F) -> Self
    where F: FnMut(&mut Scene) + 'static {
        self.on_draw_requested_fn = Some(Box::new(function));
        self
    }
    pub fn on_window_close<F>(mut self, function: F) -> Self
    where F: FnMut(WindowEvent, &EventLoopWindowTarget<()>) + 'static {
        self.on_window_close_fn = Box::new(function);
        self
    }

    pub fn create(mut self) {
        let event_loop = EventLoop::new().unwrap();

        let winit_window = WindowBuilder::new()
            .with_inner_size(PhysicalSize::new(self.config.width, self.config.height))
            .with_min_inner_size(PhysicalSize::new(
                self.config.minimum_dimension[0], self.config.minimum_dimension[1]
            ))
            .with_title(self.config.title)
            .build(&event_loop)
            .unwrap();


        let mesh = Mesh::new();
        let window_handle = Arc::new(winit_window);
        let pipeline = Pipeline::initialize(Arc::clone(&window_handle));

        self.handle = Some(Arc::clone(&window_handle));

        let mut last_update_inst = std::time::Instant::now();
        let camera = self.camera.unwrap_or_else(|| {
            Camera::new().with_aspect(self.config.width as f32 / self.config.height as f32)
        });
        let mut scene = Scene {
            pipeline,
            mesh,
            camera,
        };
        event_loop.run(move |event, elwt| {
            if let Some(update_fn) = &mut self.on_update_fn {
                let now = std::time::Instant::now();
                let dt = now.duration_since(last_update_inst).as_secs_f32();
                last_update_inst = now;
                update_fn(dt);
            }

            // Handle all events (including AboutToWait)
            if let Some(event_handler) = &mut self.event_handler {
                event_handler(event.clone(), elwt);
            }
            window_handle.request_redraw();
            match event {
                Event::WindowEvent { event: window_event, .. } => {
                    match window_event {
                        WindowEvent::CloseRequested => (self.on_window_close_fn)(window_event, elwt),
                        WindowEvent::RedrawRequested => {
                            if let Some(handler) = &mut self.on_draw_requested_fn {
                                handler(&mut scene);
                            }
                            scene.pipeline.render(&scene.mesh, &scene.camera);
                        }
                        WindowEvent::Resized(new_size) => {
                            scene.pipeline.resize(new_size);
                            scene.camera.aspect = new_size.width as f32 / new_size.height as f32;
                        }
                        _ => ()
                    }
                }
                _ => (),
            }}).unwrap();
    }
}
