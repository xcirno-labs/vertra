use crate::geometry::Geometry;
use crate::transform::Transform;

pub struct Object {
    pub name: String,
    pub transform: Transform,
    pub geometry: Option<Geometry>,
    pub color: [f32; 4],
    pub children: Vec<usize>,
    pub parent: Option<usize>,
}

pub struct ObjectConstructor {
    pub name: String,
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
        })
    }
}

impl Object {
    pub fn new(config: ObjectConstructor) -> Self {
        Self {
            name: config.name,
            transform: config.transform.unwrap_or_default(),
            geometry: config.geometry,
            color: config.color.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            children: Vec::new(),
            parent: None,
        }
    }

    pub fn from_geometry(name: &str, geometry: Geometry, transform: Transform, color: [f32; 4]) -> Self {
        Self {
            name: name.to_string(),
            transform,
            geometry: Some(geometry),
            color,
            children: Vec::new(),
            parent: None,
        }
    }
}