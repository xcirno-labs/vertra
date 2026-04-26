//! Re-exports of winit event types used throughout the Vertra public API.
//!
//! Import from here rather than directly from `winit` so that the engine's
//! winit version stays in sync with all call sites.

pub use winit::{
    event::{
        DeviceEvent, ElementState, Event, Modifiers, MouseButton,
        MouseScrollDelta, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    keyboard::PhysicalKey,
};
