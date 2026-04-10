use crate::camera::Camera;
use crate::editor::{EditorEvent, EditorState, InspectorData};
use crate::mesh::{MeshRegistry};
use crate::pipeline::Pipeline;
use crate::world::World;
use crate::objects::Object;
use crate::transform::Transform;
use crate::vtr::{self, VtrError};

pub struct Scene {
    pub pipeline:       Pipeline,
    pub mesh_registry:  MeshRegistry,
    pub camera:         Camera,
    pub world:          World,
    /// When `Some`, the engine runs in static editor mode.
    /// Attach with [`Scene::enable_editor_mode`].
    pub editor:         Option<EditorState>,
}

impl Scene {
    pub fn spawn(&mut self, object: Object, parent_id: Option<usize>) -> usize {
        self.world.spawn_object(object, parent_id)
    }

    pub fn draw_world(&mut self) {
        let mut mesh_data = crate::mesh::MeshData::new();
        let identity = Transform::default();

        // Flatten the entire world hierarchy into vertices
        for &root_id in &self.world.roots {
            mesh_data.add_object(&self.world, root_id, &identity);
        }

        // Bake world geometry to the GPU
        let world_baked = mesh_data.bake(&self.pipeline);

        // Build gizmo overlay for the selected object (if editor is active)
        let overlay_baked = self.editor.as_ref()
            .and_then(|ed| ed.gizmo_overlay_for_selection(&self.world))
            .map(|(v, i)| self.pipeline.create_baked_mesh(&v, &i));

        // Render: borrow disjoint fields to satisfy the borrow checker
        let camera  = &self.camera;
        let skybox  = self.editor.as_ref().and_then(|ed| ed.skybox.as_ref());
        self.pipeline.render_scene(camera, &world_baked, skybox, overlay_baked.as_ref());
    }

    /// Switch into static editor mode.
    ///
    /// Spawns the X/Y/Z axis gizmos at the world origin and initialises the
    /// orbit pivot in front of the camera.  Call once from `on_startup`.
    pub fn enable_editor_mode(&mut self) {
        let w = self.pipeline.surface_config.width  as f32;
        let h = self.pipeline.surface_config.height as f32;
        let mut ed = EditorState::new(w, h);
        ed.spawn_gizmos(&mut self.world);

        // Bake the skybox once and store it
        let (sky_v, sky_i) = crate::editor::build_skybox_mesh();
        ed.skybox = Some(self.pipeline.create_baked_mesh(&sky_v, &sky_i));

        // Place pivot at the camera's current look-at target
        ed.pivot = self.camera.target;

        self.editor = Some(ed);
    }

    /// Feed a platform-agnostic [`EditorEvent`] into the editor.
    ///
    /// In most cases you do not call this manually — `window.rs` converts
    /// winit events and calls this automatically when editor mode is active.
    /// Advance per-frame editor logic (WASD camera movement).
    /// Called automatically by the window loop every frame when editor mode is active.
    pub fn update_editor(&mut self, dt: f32) {
        if let Some(ed) = &mut self.editor {
            ed.update(&mut self.camera, dt);
        }
    }

    pub fn handle_editor_event(&mut self, event: EditorEvent) {
        if let Some(ed) = &mut self.editor {
            ed.process(&mut self.camera, &mut self.world, event);
        }
    }

    /// Returns a reference to the currently-selected object's inspector data,
    /// or `None` if nothing is selected or editor mode is inactive.
    pub fn inspector(&self) -> Option<&InspectorData> {
        self.editor.as_ref()?.inspector.selected.as_ref()
    }

    /// Serialize the current camera and world to a `.vtr` binary file.
    ///
    /// Creates or truncates the file at `path`.
    ///
    /// # Errors
    /// Returns a [`VtrError`] on I/O failure or serialization problems.
    pub fn save_vtr_file(&self, path: &std::path::Path) -> Result<(), VtrError> {
        vtr::write_to_file(path, &self.camera, &self.world)
    }

    /// Replace the current camera and world with the contents of a `.vtr` file.
    ///
    /// The GPU pipeline is **not** affected — only the logical scene state
    /// (camera, objects, hierarchy) is replaced.
    ///
    /// # Errors
    /// Returns a [`VtrError`] on I/O failure, bad magic bytes, unsupported
    /// format version, or any other parse error.
    pub fn load_vtr_file(&mut self, path: &std::path::Path) -> Result<(), VtrError> {
        let data = vtr::read_from_file(path)?;
        self.camera = data.camera;
        self.world  = data.world;
        Ok(())
    }
}