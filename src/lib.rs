//! # Vertra
//!
//! A lightweight, cross-platform 3D rendering engine built on [`wgpu`].
//!
//! ## Core modules
//!
//! | Module            | Purpose                                                            |
//! |-------------------|--------------------------------------------------------------------|
//! | [`camera`]        | Perspective camera, view/projection matrix construction            |
//! | [`scene`]         | Root scene container; spawn, texture, and draw APIs                |
//! | [`world`]         | Scene-graph (object hierarchy, events, spatial queries)            |
//! | [`objects`]       | [`objects::Object`] - the fundamental scene-graph node             |
//! | [`geometry`]      | Procedural geometry primitives (cube, sphere, capsule, …)          |
//! | [`transform`]     | Local-space TRS transform and matrix conversion                    |
//! | [`mesh`]          | CPU mesh builder and GPU buffer baking                             |
//! | [`math`]          | Column-major 4×4 matrix for rendering math                        |
//! | [`timer`]         | Simple countdown timer for use in game logic                       |
//! | [`window`]        | Builder-pattern windowing and event-loop host                      |
//! | [`editor`]        | Built-in static scene editor (gizmos, orbit cam, inspector)        |
//! | [`vtr`]           | Binary `.vtr` scene serialization format                           |
//! | [`constants`]     | Engine-wide default constants                                      |
//! | [`event`]         | Re-exports of winit event types used throughout the API            |
pub mod event;
pub mod window;
pub mod pipeline;
pub mod mesh;
pub mod timer;
pub mod transform;
pub mod geometry;
pub mod math;
pub mod camera;
pub mod scene;
pub mod constants;
pub mod world;
pub mod objects;
pub mod editor;

#[cfg(test)]
mod tests;
pub mod vtr;
