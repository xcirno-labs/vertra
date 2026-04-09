use crate::geometry::Geometry;
use crate::transform::Transform;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Object {
    pub name: String,
    pub transform: Transform,
    pub geometry: Option<Geometry>,
    pub color: [f32; 4],
    pub children: Vec<usize>,
    pub parent: Option<usize>,
    pub str_id: String,
}

pub struct ObjectConstructor {
    pub name: String,
    pub str_id: Option<String>,
    pub transform: Option<Transform>,
    pub geometry: Option<Geometry>,
    pub color: Option<[f32; 4]>,
}

impl Default for Object {
    fn default() -> Self {
        Self::new(ObjectConstructor {
            name: "Untitled Object".to_string(),
            transform: None,
            geometry: None,
            color: None,
            str_id: Uuid::new_v4().to_string().into(),
        })
    }
}

impl Object {
    pub fn new(config: ObjectConstructor) -> Self {
        Self {
            name: config.name,
            transform: config.transform.unwrap_or_default(),
            geometry: config.geometry,
            // Since we use UUID to generate
            str_id: config.str_id.unwrap_or_default(),
            color: config.color.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            children: Vec::new(),
            parent: None,
        }
    }

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
        }
    }
}