use crate::geometry::Geometry;
use crate::transform::Transform;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A node in the scene graph.
///
/// Every visible or logical entity in a scene is represented by an `Object`.
/// Objects are stored in and managed by a [`crate::world::World`]; use
/// [`crate::world::World::spawn_object`] (or [`crate::scene::Scene::spawn`]) to
/// insert them.
///
/// # Hierarchy
/// Parent-child relationships are tracked via the [`Object::parent`] and
/// [`Object::children`] integer-ID lists.  **Do not mutate these directly**:
/// use [`crate::world::World::reparent`] and [`crate::world::World::delete`]
/// to keep the hierarchy consistent.
///
/// # Identity
/// Each object has two identifiers:
/// * An integer `id` assigned by [`crate::world::World`] at spawn time,
///   fast for per-frame lookups.
/// * A stable `str_id` string chosen at construction, human-readable,
///   resolved to an integer via [`crate::world::World::get_id`].
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Object {
    /// Human-readable display name (does **not** need to be unique).
    pub name: String,
    /// Local-space transform (position, rotation, scale) relative to the
    /// parent, or to world origin if at the scene root.
    pub transform: Transform,
    /// Optional procedural geometry attached to this object.  `None` means
    /// the object is invisible (useful for empty pivot nodes).
    pub geometry: Option<Geometry>,
    /// RGBA base color multiplied with the geometry during rendering.
    /// Values outside `[0.0, 1.0]` are currently clamped by the shader.
    pub color: [f32; 4],
    /// Integer IDs of direct children.  Managed by [`crate::world::World`];
    /// do not mutate directly.
    pub children: Vec<usize>,
    /// Integer ID of the parent object, or `None` if this is a root object.
    /// Managed by [`crate::world::World`]; do not mutate directly.
    pub parent: Option<usize>,
    /// Stable string identifier used as a human-friendly handle.  Should be
    /// unique within a world.  Use
    /// [`crate::world::World::rename_str_id`] to change it after spawn.
    pub str_id: String,
    /// Path to a texture image applied to this object's surface.
    pub texture_path: Option<String>,
}

/// Configuration bundle passed to [`Object::new`].
///
/// All fields except `name` are optional and fall back to sensible defaults
/// when `None`.
pub struct ObjectConstructor {
    /// Human-readable display name.
    pub name: String,
    /// Stable string identifier.  A random UUID is used when `None`.
    pub str_id: Option<String>,
    /// Initial local-space transform.  Defaults to identity.
    pub transform: Option<Transform>,
    /// Procedural geometry shape.  `None` = invisible pivot node.
    pub geometry: Option<Geometry>,
    /// RGBA base color.  Defaults to opaque white `[1.0, 1.0, 1.0, 1.0]`.
    pub color: Option<[f32; 4]>,
    /// Optional texture path.
    pub texture_path: Option<String>,
}

impl Default for Object {
    fn default() -> Self {
        Self::new(ObjectConstructor {
            name: "Untitled Object".to_string(),
            transform: None,
            geometry: None,
            color: None,
            str_id: Uuid::new_v4().to_string().into(),
            texture_path: None,
        })
    }
}

impl Object {
    /// Create a new object from an [`ObjectConstructor`] configuration.
    ///
    /// `None` fields fall back to their defaults:
    /// * `transform` -> identity
    /// * `str_id` -> random UUID
    /// * `color` -> opaque white
    /// * `geometry` -> `None` (invisible)
    pub fn new(config: ObjectConstructor) -> Self {
        Self {
            name: config.name,
            transform: config.transform.unwrap_or_default(),
            geometry: config.geometry,
            str_id: config.str_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            color: config.color.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            children: Vec::new(),
            parent: None,
            texture_path: config.texture_path,
        }
    }

    /// Convenience constructor for an object with a known geometry, transform,
    /// and color.
    ///
    /// * `name`     - display name.
    /// * `str_id`   - stable string handle; a random UUID is used if `None`.
    /// * `geometry` - shape attached to the object.
    /// * `transform`- initial local transform.
    /// * `color`    - RGBA base color.
    pub fn from_geometry(
        name: &str,
        str_id: Option<String>,
        geometry: Geometry,
        transform: Transform,
        color: [f32; 4]
    ) -> Self {
        Self {
            name: name.to_string(),
            transform,
            geometry: Some(geometry),
            color,
            children: Vec::new(),
            parent: None,
            str_id: str_id.unwrap_or_else(|| Uuid::new_v4().to_string()).into(),
            texture_path: None,
        }
    }
}