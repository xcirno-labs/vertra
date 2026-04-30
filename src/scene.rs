use std::collections::HashMap;
use crate::camera::Camera;
use crate::editor::{EditorEvent, EditorState, InspectorData};
use crate::mesh::{MeshData, MeshRegistry};
use crate::pipeline::Pipeline;
use crate::world::World;
use crate::objects::Object;
use crate::transform::Transform;
use crate::vtr::{self, VtrError};
use crate::script::{ObjectScript, ScriptRegistry};

/// A loaded GPU texture paired with its bind group.
///
/// Stored in [`Scene::textures`] keyed by the `texture_path` string used on
/// objects.  The `texture` field is kept alive so the GPU memory is not freed
/// while the bind group is in use.
pub struct TextureEntry {
    #[allow(dead_code)]
    pub texture: wgpu::Texture,
    /// Bind group that wires the texture to the shader's texture slot.
    pub bind_group: wgpu::BindGroup,
}

/// The root container for a 3D scene.
///
/// `Scene` owns all engine subsystems for a single viewport:
/// * [`Scene::world`]  - the scene-graph (objects, hierarchy).
/// * [`Scene::camera`] - the viewport camera.
/// * [`Scene::pipeline`] - the wgpu render pipeline.
/// * [`Scene::editor`] - optional built-in editor overlay.
/// * [`Scene::textures`] - loaded GPU textures keyed by path.
///
/// A `Scene` is created internally by [`crate::window::Window`] before
/// `on_startup` fires.  You interact with it through the callbacks.
pub struct Scene {
    /// The wgpu render pipeline, surface, and device context.
    pub pipeline:       Pipeline,
    /// Registry tracking the world mesh (primarily used internally).
    pub mesh_registry:  MeshRegistry,
    /// Active viewport camera.
    pub camera:         Camera,
    /// The scene graph containing all objects and their hierarchy.
    pub world:          World,
    /// When `Some`, the engine runs in static editor mode.
    /// Attach with [`Scene::enable_editor_mode`].
    pub editor:         Option<EditorState>,
    /// Per-texture-path GPU resources. Key matches `Object::texture_path`.
    pub textures:       HashMap<String, TextureEntry>,
    /// In-memory VTR snapshot captured the moment play mode is entered.
    ///
    /// Restored automatically when the user returns to editor mode, so that
    /// any mutations that occurred during play (object movement, etc.) are
    /// reverted to the exact state the editor saved.
    pub(crate) snapshot: Option<Vec<u8>>,
    /// Per-object script registry.  Kept separate from `World` so scripts
    /// never affect serialisation.
    pub script_registry: ScriptRegistry,
}

impl Scene {
    /// Spawn `object` into the scene, optionally as a child of `parent_id`.
    ///
    /// This is a thin convenience wrapper around
    /// [`World::spawn_object`](crate::world::World::spawn_object).
    /// If `parent_id` is `Some` but the parent does not exist the object is
    /// placed at root level.
    ///
    /// Returns the unique integer ID assigned to the new object.
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

    /// Traverse the entire scene graph and issue a single batched draw call
    /// per texture group.
    ///
    /// Objects are grouped by their `texture_path` so the number of GPU
    /// bind-group switches is minimised.  The editor gizmo overlay (if any) is
    /// rendered as a separate pass on top.
    ///
    /// Called automatically by [`crate::window::Window`] every frame on
    /// `RedrawRequested`.  You do not normally need to call this manually.
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
    ///
    /// If a play-mode snapshot exists (i.e. the user is returning from play
    /// mode) the world and camera are first restored to the state they were in
    /// when play mode was entered, discarding any mutations that occurred during
    /// play.
    pub fn enable_editor_mode(&mut self) {
        // Restore play-mode snapshot so play-time mutations are discarded.
        if let Some(buf) = self.snapshot.take() {
            match vtr::read(&mut std::io::Cursor::new(buf)) {
                Ok(data) => {
                    self.camera = data.camera;
                    self.world  = data.world;
                }
                Err(e) => eprintln!("enable_editor_mode: failed to restore snapshot: {e}"),
            }
        }

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
    /// Captures an in-memory VTR snapshot of the current world and camera so
    /// that [`Self::enable_editor_mode`] can restore them later.  Drops all
    /// editor state (selection, gizmos, skybox, pivot).  After this call
    /// `scene.editor` is `None`, the gizmo overlay is hidden, and all
    /// client-side event handlers begin receiving raw input events again.
    pub fn disable_editor_mode(&mut self) {
        // Snapshot current state so we can roll back when returning to editor.
        let mut buf = Vec::new();
        match vtr::write(&mut buf, &self.camera, &self.world) {
            Ok(()) => self.snapshot = Some(buf),
            Err(e) => eprintln!("disable_editor_mode: failed to capture snapshot: {e}"),
        }
        self.editor = None;
        // Reset all scripts so on_start re-runs against the fresh world that
        // will be restored when the user returns to editor mode.  Without this,
        // cached IDs / base transforms from a previous play session would be
        // stale after the snapshot is restored.
        self.script_registry.reset_started();
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


        if let Some(ed) = &mut self.editor {
            ed.process(&mut self.camera, &mut self.world, event);
        }
    }

    /// Returns a reference to the currently-selected object's inspector data,
    /// or `None` if nothing is selected or editor mode is inactive.
    pub fn inspector(&self) -> Option<&InspectorData> {
        self.editor.as_ref()?.inspector.selected.as_ref()
    }
    
    /// Attach `script` to object `id`.
    ///
    /// The script's [`ObjectScript::on_start`] will be called on the next
    /// `run_scripts` / `run_fixed_update_scripts` invocation before
    /// [`ObjectScript::on_update`] / [`ObjectScript::on_fixed_update`].
    ///
    /// If the object already had a script it is replaced.  Scripts are
    /// suppressed while editor mode is active, i.e. the window loop does not call
    /// `run_scripts` when `scene.editor.is_some()`.
    pub fn attach_script(&mut self, id: usize, script: Box<dyn ObjectScript>) {
        self.script_registry.attach(id, script);
    }

    /// Detach and drop the script for object `id`.
    ///
    /// Returns `true` if a script existed and was removed.
    pub fn detach_script(&mut self, id: usize) -> bool {
        self.script_registry.detach(id)
    }

    /// Returns `true` when object `id` has a script attached.
    pub fn has_script(&self, id: usize) -> bool {
        self.script_registry.has(id)
    }

    /// Run `on_start` (first call only) + `on_update` for all attached scripts.
    ///
    /// Called automatically by the window loop every frame when not in editor
    /// mode.  You do not normally need to call this manually.
    pub fn run_scripts(&mut self, dt: f32) {
        self.script_registry.run_update(&mut self.world, dt);
    }

    /// Run `on_start` (first call only) + `on_fixed_update` for all attached scripts.
    ///
    /// Called automatically by the window loop at the fixed timestep when not
    /// in editor mode.  You do not normally need to call this manually.
    pub fn run_fixed_update_scripts(&mut self, dt: f32) {
        self.script_registry.run_fixed_update(&mut self.world, dt);
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
        // World has changed, cached script state (IDs, transforms, etc.) is
        // no longer valid for the new world, so force on_start to re-run.
        self.script_registry.reset_started();
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
    // `collect_by_texture` uses `groups.entry(obj.texture_path.clone())`,
    // cloning the (potentially long) texture path string for every object
    // on every frame. This can become a noticeable per-frame allocation cost
    // in large scenes.
    // TODO: Consider grouping by a borrowed key (e.g. Option<&str> via
    //  a two-pass approach) or storing an interned/shared key on Object
    //  (e.g. Arc<str>), so we can hash without allocating each frame.
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
