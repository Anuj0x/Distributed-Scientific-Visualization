//! Safe shared memory management for distributed computing

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::{RwLock, Mutex};
use shared_memory::{Shmem, ShmemConf};
use serde::{Deserialize, Serialize};

use crate::core::{ObjectId, Object};
use crate::Error;

/// Shared memory configuration
#[derive(Debug, Clone)]
pub struct ShmConfig {
    pub size: usize,
    pub name: String,
}

impl Default for ShmConfig {
    fn default() -> Self {
        Self {
            size: 1024 * 1024 * 1024, // 1GB default
            name: format!("vistle_shm_{}", std::process::id()),
        }
    }
}

/// Safe shared memory arena
pub struct SharedArena {
    shmem: Arc<Shmem>,
    objects: RwLock<HashMap<ObjectId, SharedObject>>,
    allocator: Mutex<SharedAllocator>,
}

impl SharedArena {
    /// Create a new shared memory arena
    pub fn new(config: ShmConfig) -> Result<Self, Error> {
        let shmem = Arc::new(
            ShmemConf::new()
                .size(config.size)
                .flink(&config.name)
                .create()
                .map_err(|e| Error::SharedMemory(format!("Failed to create shared memory: {}", e)))?
        );

        Ok(Self {
            shmem,
            objects: RwLock::new(HashMap::new()),
            allocator: Mutex::new(SharedAllocator::new(config.size)),
        })
    }

    /// Attach to existing shared memory arena
    pub fn attach(name: &str) -> Result<Self, Error> {
        let shmem = Arc::new(
            ShmemConf::new()
                .flink(name)
                .open()
                .map_err(|e| Error::SharedMemory(format!("Failed to attach to shared memory: {}", e)))?
        );

        // For simplicity, assume we can reconstruct the allocator state
        // In a real implementation, this would be stored in shared memory
        let size = shmem.len();
        let allocator = Mutex::new(SharedAllocator::new(size));

        Ok(Self {
            shmem,
            objects: RwLock::new(HashMap::new()),
            allocator,
        })
    }

    /// Store an object in shared memory
    pub fn store_object(&self, object: Arc<dyn Object>) -> Result<ObjectId, Error> {
        let id = object.id();

        // Serialize the object
        let data = bincode::serialize(&object.clone_object())
            .map_err(|e| Error::Serialization(e))?;

        // Allocate space in shared memory
        let mut allocator = self.allocator.lock();
        let offset = allocator.allocate(data.len())?;

        // Copy data to shared memory
        unsafe {
            let ptr = self.shmem.as_ptr().add(offset);
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
        }

        // Create shared object entry
        let shared_obj = SharedObject {
            id,
            offset,
            size: data.len(),
            object_type: object.object_type(),
        };

        // Store in registry
        self.objects.write().insert(id, shared_obj);

        Ok(id)
    }

    /// Retrieve an object from shared memory
    pub fn get_object(&self, id: ObjectId) -> Result<Option<Arc<dyn Object>>, Error> {
        let objects = self.objects.read();
        let shared_obj = match objects.get(&id) {
            Some(obj) => obj,
            None => return Ok(None),
        };

        // Read data from shared memory
        let mut data = vec![0u8; shared_obj.size];
        unsafe {
            let ptr = self.shmem.as_ptr().add(shared_obj.offset);
            std::ptr::copy_nonoverlapping(ptr, data.as_mut_ptr(), shared_obj.size);
        }

        // Deserialize the object
        let object: Box<dyn Object> = bincode::deserialize(&data)
            .map_err(|e| Error::Serialization(e))?;

        Ok(Some(Arc::from(object)))
    }

    /// Remove an object from shared memory
    pub fn remove_object(&self, id: ObjectId) -> Result<bool, Error> {
        let mut objects = self.objects.write();
        if let Some(shared_obj) = objects.remove(&id) {
            let mut allocator = self.allocator.lock();
            allocator.deallocate(shared_obj.offset, shared_obj.size)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get shared memory statistics
    pub fn stats(&self) -> ShmStats {
        let allocator = self.allocator.lock();
        let objects = self.objects.read();

        ShmStats {
            total_size: self.shmem.len(),
            used_size: allocator.used(),
            free_size: allocator.free(),
            object_count: objects.len(),
        }
    }
}

/// Shared memory statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShmStats {
    pub total_size: usize,
    pub used_size: usize,
    pub free_size: usize,
    pub object_count: usize,
}

/// Internal representation of a shared object
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SharedObject {
    id: ObjectId,
    offset: usize,
    size: usize,
    object_type: crate::core::ObjectType,
}

/// Simple shared memory allocator
struct SharedAllocator {
    total_size: usize,
    allocations: HashMap<usize, usize>, // offset -> size
    free_blocks: Vec<(usize, usize)>, // (offset, size)
}

impl SharedAllocator {
    fn new(total_size: usize) -> Self {
        Self {
            total_size,
            allocations: HashMap::new(),
            free_blocks: vec![(0, total_size)],
        }
    }

    fn allocate(&mut self, size: usize) -> Result<usize, Error> {
        // Find a suitable free block (first fit strategy)
        for i in 0..self.free_blocks.len() {
            let (offset, block_size) = self.free_blocks[i];
            if block_size >= size {
                // Remove this block
                self.free_blocks.remove(i);

                // If there's leftover space, add it back as a free block
                if block_size > size {
                    self.free_blocks.push((offset + size, block_size - size));
                }

                // Record the allocation
                self.allocations.insert(offset, size);

                return Ok(offset);
            }
        }

        Err(Error::SharedMemory("Insufficient shared memory space".to_string()))
    }

    fn deallocate(&mut self, offset: usize, size: usize) -> Result<(), Error> {
        // Remove the allocation
        if self.allocations.remove(&offset).is_none() {
            return Err(Error::SharedMemory("Invalid deallocation".to_string()));
        }

        // Add to free blocks and merge adjacent blocks
        self.free_blocks.push((offset, size));
        self.coalesce_free_blocks();

        Ok(())
    }

    fn coalesce_free_blocks(&mut self) {
        self.free_blocks.sort_by_key(|&(offset, _)| offset);

        let mut i = 0;
        while i + 1 < self.free_blocks.len() {
            let (offset1, size1) = self.free_blocks[i];
            let (offset2, size2) = self.free_blocks[i + 1];

            if offset1 + size1 == offset2 {
                // Merge blocks
                self.free_blocks[i] = (offset1, size1 + size2);
                self.free_blocks.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }

    fn used(&self) -> usize {
        self.allocations.values().sum()
    }

    fn free(&self) -> usize {
        self.total_size - self.used()
    }
}

/// Global shared memory manager
pub struct ShmManager {
    arenas: RwLock<HashMap<String, Arc<SharedArena>>>,
}

impl ShmManager {
    pub fn new() -> Self {
        Self {
            arenas: RwLock::new(HashMap::new()),
        }
    }

    pub fn create_arena(&self, name: String, config: ShmConfig) -> Result<Arc<SharedArena>, Error> {
        let arena = Arc::new(SharedArena::new(config)?);
        self.arenas.write().insert(name, arena.clone());
        Ok(arena)
    }

    pub fn get_arena(&self, name: &str) -> Option<Arc<SharedArena>> {
        self.arenas.read().get(name).cloned()
    }

    pub fn attach_arena(&self, name: String, shm_name: &str) -> Result<Arc<SharedArena>, Error> {
        let arena = Arc::new(SharedArena::attach(shm_name)?);
        self.arenas.write().insert(name, arena.clone());
        Ok(arena)
    }
}

impl Default for ShmManager {
    fn default() -> Self {
        Self::new()
    }
}
