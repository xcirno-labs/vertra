use std::collections::HashMap;
use crate::camera::Camera;
use crate::editor::{EditorEvent, EditorState, InspectorData};
use crate::mesh::{MeshData, MeshRegistry};
use crate::pipeline::Pipeline;
use crate::world::World;
use crate::objects::Object;
use crate::transform::Transform;
use crate::vtr::{self, VtrError};

/// Holds an uploaded GPU texture and its associated bind group.
pub struct TextureEntry {
    #[allow(dead_code)]
    pub texture: wgpu::Texture,
    pub bind_group: wgpu::BindGroup,
}

pub struct Scene {
    pub pipeline:       Pipeline,
    pub mesh_registry:  MeshRegistry,
    pub camera:         Camera,
    pub world:          World,
    /// When `Some`, the engine runs in static editor mode.
    /// Attach with [`Scene::enable_editor_mode`].
    pub editor:         Option<EditorState>,
    /// Per-texture-path GPU resources. Key matches `Object::texture_path`.
    pub textures:       HashMap<String, TextureEntry>,
}

impl Scene {
    pub fn spawn(&mut self, object: Object, parent_id: Option<usize>) -> usize {
        self.world.spawn_object(object, parent_id)
    }

    /// Upload raw RGBA pixel data and register it under `path_key`.
    ///
    /// After this call any object whose `texture_path` equals `path_key` will
    /// have the texture applied during rendering.  Safe to call every frame
    /// (the previous entry is simply replaced).
    pub fn load_texture_from_rgba(
        &mut self,
        path_key: &str,
        width: u32,
        height: u32,
        rgba_data: &[u8],
    ) {
        let (texture, bind_group) = self.pipeline
            .create_texture_bind_group_from_rgba(path_key, width, height, rgba_data);
        self.textures.insert(path_key.to_string(), TextureEntry { texture, bind_group });
    }

    /// Load a PNG / JPEG texture from the file system and register it under its
    /// path.  After this call any object whose `texture_path` equals `path` will
    /// be rendered with the image applied.
    ///
    /// Only available on native targets (not wasm32). On WASM use
    /// [`load_texture_from_rgba`] with bytes fetched via JS.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_texture(&mut self, path: &str) -> Result<(), String> {
        use image::GenericImageView;
        let img = image::open(path).map_err(|e| format!("load_texture(\"{path}\"): {e}"))?;
        let rgba = img.to_rgba8();
        let (width, height) = img.dimensions();
        self.load_texture_from_rgba(path, width, height, &rgba);
        Ok(())
    }

    /// Remove a previously-loaded texture by its key.
    ///
    /// Objects that referenced this key fall back to vertex colour.
    /// Returns `true` if a texture existed under that key and was removed.
    pub fn unload_texture(&mut self, path_key: &str) -> bool {
        self.textures.remove(path_key).is_some()
    }

    /// Returns `true` if a texture has been loaded under `path_key`.
    pub fn has_texture(&self, path_key: &str) -> bool {
        self.textures.contains_key(path_key)
    }

    pub fn draw_world(&mut self) {
        // Group object geometry by texture_path so we minimise bind-group switches.
        let mut groups: HashMap<Option<String>, MeshData> = HashMap::new();
        let identity = Transform::default();
        for &root_id in &self.world.roots {
            collect_by_texture(&self.world, root_id, &identity, &mut groups);
        }

        // Bake each group - collect into Vec so we own the BakedMeshes before
        // taking any references out of `self.pipeline`.
        let baked_groups: Vec<(Option<String>, crate::mesh::BakedMesh)> = groups
            .into_iter()
            .map(|(key, mesh_data)| (key, mesh_data.bake(&self.pipeline)))
            .collect();

        // Pair each baked mesh with the matching bind group (or default white).
        let world_batches: Vec<(&crate::mesh::BakedMesh, &wgpu::BindGroup)> = baked_groups
            .iter()
            .map(|(key, baked)| {
                let bg: &wgpu::BindGroup = key
                    .as_ref()
                    .and_then(|p| self.textures.get(p))
                    .map(|e| &e.bind_group)
                    .unwrap_or(&self.pipeline.default_texture_bind_group);
                (baked, bg)
            })
            .collect();

        // Build gizmo overlay for the selected object (if editor is active).
        let overlay_baked = self.editor.as_ref()
            .and_then(|ed| ed.gizmo_overlay_for_selection(&self.world, &self.camera))
            .map(|(v, i)| self.pipeline.create_baked_mesh(&v, &i));

        let camera = &self.camera;
        let skybox = self.editor.as_ref().and_then(|ed| ed.skybox.as_ref());
        self.pipeline.render_scene(camera, &world_batches, skybox, overlay_baked.as_ref());
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

    /// Exit editor mode and switch to **play mode**.
    ///
    /// Drops all editor state (selection, gizmos, skybox, pivot).
    /// After this call `scene.editor` is `None`, the gizmo overlay is hidden,
    /// and all client-side event handlers begin receiving raw input events again.
    pub fn disable_editor_mode(&mut self) {
        self.editor = None;
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

    /// Feed a platform-agnostic [`EditorEvent`] into the editor.
    ///
    /// **Default keybind — `Escape`:** pressing Escape while editor mode is
    /// active automatically calls [`Self::disable_editor_mode`], switching the
    /// engine to play mode before any further processing occurs.
    pub fn handle_editor_event(&mut self, event: EditorEvent) {
        if self.editor.is_none() { return; }

        // Default keybind: Escape exits editor mode -> play mode
        if matches!(&event, EditorEvent::KeyPressed(winit::keyboard::KeyCode::Escape)) {
            self.editor = None;
            return;
        }

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

/// Traverse the object hierarchy and accumulate each object's mesh geometry
/// into a bucket keyed by `texture_path`.  Objects with no geometry are skipped.
fn collect_by_texture(
    world: &World,
    object_id: usize,
    parent_transform: &Transform,
    groups: &mut HashMap<Option<String>, MeshData>,
) {
    if let Some(obj) = world.objects.get(&object_id) {
        let world_transform = parent_transform.combine(&obj.transform);

        if let Some(geo) = &obj.geometry {
            let entry = groups
                .entry(obj.texture_path.clone())
                .or_insert_with(MeshData::new);
            geo.generate_mesh_data(entry, &world_transform, obj.color);
        }

        for &child_id in &obj.children {
            collect_by_texture(world, child_id, &world_transform, groups);
        }
    }
}

