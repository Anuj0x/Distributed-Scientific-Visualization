//! Metadata handling for objects and modules

use serde::{Deserialize, Serialize};
use nalgebra::Matrix4;

/// Metadata structure for objects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub block: i32,
    pub num_blocks: i32,
    pub timestep: i32,
    pub num_timesteps: i32,
    pub iteration: i32,
    pub generation: i32,
    pub creator: i32,
    pub real_time: f64,
    pub transform: Matrix4<f32>,
}

impl Default for Meta {
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
            transform: Matrix4::identity(),
        }
    }
}

impl Meta {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_block(mut self, block: i32, num_blocks: i32) -> Self {
        self.block = block;
        self.num_blocks = num_blocks;
        self
    }

    pub fn with_timestep(mut self, timestep: i32, num_timesteps: i32) -> Self {
        self.timestep = timestep;
        self.num_timesteps = num_timesteps;
        self
    }

    pub fn with_iteration(mut self, iteration: i32) -> Self {
        self.iteration = iteration;
        self
    }

    pub fn with_generation(mut self, generation: i32) -> Self {
        self.generation = generation;
        self
    }

    pub fn with_creator(mut self, creator: i32) -> Self {
        self.creator = creator;
        self
    }

    pub fn with_real_time(mut self, real_time: f64) -> Self {
        self.real_time = real_time;
        self
    }

    pub fn with_transform(mut self, transform: Matrix4<f32>) -> Self {
        self.transform = transform;
        self
    }

    /// Merge metadata from another source
    pub fn merge(&mut self, other: &Meta) {
        // Update fields if they represent more recent data
        if other.generation > self.generation {
            self.generation = other.generation;
        }
        if other.iteration > self.iteration {
            self.iteration = other.iteration;
        }
        if other.real_time > self.real_time {
            self.real_time = other.real_time;
        }
    }
}

/// Module information and status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub category: String,
    pub rank: i32,
    pub size: i32,
    pub status: ModuleStatus,
}

impl ModuleInfo {
    pub fn new(id: u32, name: &str, rank: i32, size: i32) -> Self {
        Self {
            id,
            name: name.to_string(),
            description: String::new(),
            category: "General".to_string(),
            rank,
            size,
            status: ModuleStatus::Initializing,
        }
    }
}

/// Module execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleStatus {
    Initializing,
    Ready,
    Executing,
    Completed,
    Error,
    Cancelled,
}

/// Computation context for modules
#[derive(Debug, Clone)]
pub struct ComputeContext {
    pub module_id: u32,
    pub timestep: i32,
    pub iteration: i32,
    pub rank: i32,
    pub size: i32,
}

impl ComputeContext {
    pub fn new(module_id: u32, rank: i32, size: i32) -> Self {
        Self {
            module_id,
            timestep: 0,
            iteration: 0,
            rank,
            size,
        }
    }

    pub fn with_timestep(mut self, timestep: i32) -> Self {
        self.timestep = timestep;
        self
    }

    pub fn with_iteration(mut self, iteration: i32) -> Self {
        self.iteration = iteration;
        self
    }
}

/// Execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub module_id: u32,
    pub start_time: std::time::SystemTime,
    pub end_time: Option<std::time::SystemTime>,
    pub objects_created: usize,
    pub objects_processed: usize,
    pub errors: Vec<String>,
}

impl ExecutionStats {
    pub fn new(module_id: u32) -> Self {
        Self {
            module_id,
            start_time: std::time::SystemTime::now(),
            end_time: None,
            objects_created: 0,
            objects_processed: 0,
            errors: Vec::new(),
        }
    }

    pub fn complete(mut self) -> Self {
        self.end_time = Some(std::time::SystemTime::now());
        self
    }

    pub fn duration(&self) -> Option<std::time::Duration> {
        self.end_time.and_then(|end| end.duration_since(self.start_time).ok())
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    pub fn increment_created(&mut self) {
        self.objects_created += 1;
    }

    pub fn increment_processed(&mut self) {
        self.objects_processed += 1;
    }
}
