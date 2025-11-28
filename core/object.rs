//! Core object system for scientific data

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for objects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectId(Uuid);

impl ObjectId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ObjectId {
    fn default() -> Self {
        Self::new()
    }
}

/// Object type enumeration - simplified from C++ version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum ObjectType {
    // Basic types
    Unknown = 0,
    Empty = 1,
    Placeholder = 11,

    // Geometric types
    Points = 18,
    Lines = 20,
    Triangles = 22,
    Polygons = 23,
    UnstructuredGrid = 24,
    UniformGrid = 25,
    RectilinearGrid = 26,
    StructuredGrid = 27,
    Quads = 28,

    // Data types
    Vec = 100, // Base for all vector types
}

impl ObjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ObjectType::Unknown => "Unknown",
            ObjectType::Empty => "Empty",
            ObjectType::Placeholder => "Placeholder",
            ObjectType::Points => "Points",
            ObjectType::Lines => "Lines",
            ObjectType::Triangles => "Triangles",
            ObjectType::Polygons => "Polygons",
            ObjectType::UnstructuredGrid => "UnstructuredGrid",
            ObjectType::UniformGrid => "UniformGrid",
            ObjectType::RectilinearGrid => "RectilinearGrid",
            ObjectType::StructuredGrid => "StructuredGrid",
            ObjectType::Quads => "Quads",
            ObjectType::Vec => "Vec",
        }
    }
}

/// Metadata associated with objects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMeta {
    pub block: i32,
    pub num_blocks: i32,
    pub timestep: i32,
    pub num_timesteps: i32,
    pub iteration: i32,
    pub generation: i32,
    pub creator: i32,
    pub real_time: f64,
    pub transform: nalgebra::Matrix4<f32>,
}

impl Default for ObjectMeta {
    fn default() -> Self {
        Self {
            block: 0,
            num_blocks: 1,
            timestep: 0,
            num_timesteps: 1,
            iteration: 0,
            generation: 0,
            creator: 0,
            real_time: 0.0,
            transform: nalgebra::Matrix4::identity(),
        }
    }
}

/// Base trait for all Vistle objects
#[async_trait::async_trait]
pub trait Object: Send + Sync {
    /// Get the object's unique ID
    fn id(&self) -> ObjectId;

    /// Get the object's type
    fn object_type(&self) -> ObjectType;

    /// Get object metadata
    fn meta(&self) -> &ObjectMeta;

    /// Get mutable metadata
    fn meta_mut(&mut self) -> &mut ObjectMeta;

    /// Check if object is complete (all references resolved)
    fn is_complete(&self) -> bool;

    /// Get references to other objects
    fn references(&self) -> Vec<ObjectId>;

    /// Clone the object
    fn clone_object(&self) -> Box<dyn Object>;

    /// Get attribute by key
    fn get_attribute(&self, key: &str) -> Option<&str>;

    /// Set attribute
    fn set_attribute(&mut self, key: String, value: String);

    /// Get all attributes
    fn attributes(&self) -> &HashMap<String, String>;
}

/// Generic object data container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectData {
    pub id: ObjectId,
    pub object_type: ObjectType,
    pub meta: ObjectMeta,
    pub attributes: HashMap<String, String>,
    pub data: ObjectPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectPayload {
    Empty,
    Points {
        coordinates: ndarray::Array2<f32>,
    },
    Lines {
        coordinates: ndarray::Array2<f32>,
        connections: ndarray::Array2<i32>,
    },
    Triangles {
        coordinates: ndarray::Array2<f32>,
        triangles: ndarray::Array2<i32>,
    },
    VecScalar {
        data: ndarray::Array1<f32>,
    },
    VecVec3 {
        data: ndarray::Array2<f32>,
    },
    Custom(Vec<u8>),
}

/// Concrete object implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VistleObject {
    data: ObjectData,
}

impl VistleObject {
    pub fn new(object_type: ObjectType) -> Self {
        Self {
            data: ObjectData {
                id: ObjectId::new(),
                object_type,
                meta: ObjectMeta::default(),
                attributes: HashMap::new(),
                data: ObjectPayload::Empty,
            },
        }
    }

    pub fn with_data(object_type: ObjectType, payload: ObjectPayload) -> Self {
        Self {
            data: ObjectData {
                id: ObjectId::new(),
                object_type,
                meta: ObjectMeta::default(),
                attributes: HashMap::new(),
                data: payload,
            },
        }
    }
}

#[async_trait::async_trait]
impl Object for VistleObject {
    fn id(&self) -> ObjectId {
        self.data.id
    }

    fn object_type(&self) -> ObjectType {
        self.data.object_type
    }

    fn meta(&self) -> &ObjectMeta {
        &self.data.meta
    }

    fn meta_mut(&mut self) -> &mut ObjectMeta {
        &mut self.data.meta
    }

    fn is_complete(&self) -> bool {
        // Simplified: in real implementation, check for unresolved references
        true
    }

    fn references(&self) -> Vec<ObjectId> {
        // Return IDs of referenced objects
        Vec::new()
    }

    fn clone_object(&self) -> Box<dyn Object> {
        Box::new(self.clone())
    }

    fn get_attribute(&self, key: &str) -> Option<&str> {
        self.data.attributes.get(key).map(|s| s.as_str())
    }

    fn set_attribute(&mut self, key: String, value: String) {
        self.data.attributes.insert(key, value);
    }

    fn attributes(&self) -> &HashMap<String, String> {
        &self.data.attributes
    }
}

/// Thread-safe object registry
pub struct ObjectRegistry {
    objects: dashmap::DashMap<ObjectId, Arc<dyn Object>>,
}

impl ObjectRegistry {
    pub fn new() -> Self {
        Self {
            objects: dashmap::DashMap::new(),
        }
    }

    pub fn store(&self, object: Arc<dyn Object>) -> ObjectId {
        let id = object.id();
        self.objects.insert(id, object);
        id
    }

    pub fn get(&self, id: ObjectId) -> Option<Arc<dyn Object>> {
        self.objects.get(&id).map(|r| r.clone())
    }

    pub fn remove(&self, id: ObjectId) -> bool {
        self.objects.remove(&id).is_some()
    }

    pub fn iter(&self) -> dashmap::iter::Iter<ObjectId, Arc<dyn Object>> {
        self.objects.iter()
    }
}

impl Default for ObjectRegistry {
    fn default() -> Self {
        Self::new()
    }
}
