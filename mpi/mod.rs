//! MPI-based distributed computing support

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::{MessageRouter, MpiMessageChannel, MpiMessageChannel as MpiChannel};
use crate::Error;

/// MPI universe and communicator management
pub struct MpiUniverse {
    universe: mpi::initialize::Universe,
    world: mpi::topology::SystemCommunicator,
    rank: i32,
    size: i32,
}

impl MpiUniverse {
    pub fn new() -> Result<Self, Error> {
        let universe = mpi::initialize().map_err(Error::Mpi)?;
        let world = universe.world();
        let rank = world.rank();
        let size = world.size();

        Ok(Self {
            universe,
            world,
            rank,
            size,
        })
    }

    pub fn rank(&self) -> i32 {
        self.rank
    }

    pub fn size(&self) -> i32 {
        self.size
    }

    pub fn world(&self) -> &mpi::topology::SystemCommunicator {
        &self.world
    }
}

/// Distributed computation context
pub struct DistributedContext {
    universe: Arc<MpiUniverse>,
    message_router: Arc<MessageRouter>,
    local_data: RwLock<HashMap<String, Vec<u8>>>,
}

impl DistributedContext {
    pub fn new(message_router: Arc<MessageRouter>) -> Result<Self, Error> {
        let universe = Arc::new(MpiUniverse::new()?);

        Ok(Self {
            universe,
            message_router,
            local_data: RwLock::new(HashMap::new()),
        })
    }

    pub fn rank(&self) -> i32 {
        self.universe.rank()
    }

    pub fn size(&self) -> i32 {
        self.universe.size()
    }

    /// Broadcast data from root to all ranks
    pub async fn broadcast<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
        data: &T,
        root: i32,
    ) -> Result<T, Error> {
        let serialized = bincode::serialize(data)
            .map_err(Error::Serialization)?;

        let mut buffer = if self.rank() == root {
            serialized
        } else {
            vec![0u8; 1024 * 1024] // Pre-allocate reasonable size
        };

        self.universe.world().process_at_rank(root).broadcast_into(&mut buffer);

        if self.rank() == root {
            Ok(data.clone())
        } else {
            bincode::deserialize(&buffer)
                .map_err(Error::Serialization)
        }
    }

    /// All-to-all data exchange
    pub async fn all_to_all<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
        send_data: &[T],
    ) -> Result<Vec<T>, Error> {
        // Simplified implementation - in practice would use MPI_Alltoallv
        let mut results = Vec::with_capacity(self.size() as usize);

        for rank in 0..self.size() {
            if rank == self.rank() {
                results.push(send_data[rank as usize].clone());
            } else {
                // Send to rank and receive from rank
                let data = self.send_receive(send_data[rank as usize].clone(), rank).await?;
                results.push(data);
            }
        }

        Ok(results)
    }

    /// Send data to specific rank and receive from another
    pub async fn send_receive<T: serde::Serialize + serde::de::DeserializeOwned>(
        &self,
        send_data: T,
        dest: i32,
    ) -> Result<T, Error> {
        let serialized = bincode::serialize(&send_data)
            .map_err(Error::Serialization)?;

        let world = self.universe.world();
        world.process_at_rank(dest).send(&serialized);

        // Receive response (simplified - assumes response comes back)
        let mut buffer = vec![0u8; 1024 * 1024];
        let (msg, _status) = world.receive_into(&mut buffer);

        bincode::deserialize(&buffer)
            .map_err(Error::Serialization)
    }

    /// Reduce operation across all ranks
    pub async fn reduce<T, F>(&self, local_value: T, op: F, root: i32) -> Result<Option<T>, Error>
    where
        T: serde::Serialize + serde::de::DeserializeOwned + Clone,
        F: Fn(T, T) -> T,
    {
        // Simplified reduction - in practice would use MPI_Reduce
        if self.rank() == root {
            let mut result = local_value;
            for rank in 0..self.size() {
                if rank != root {
                    let remote_value: T = self.receive_from(rank).await?;
                    result = op(result, remote_value);
                }
            }
            Ok(Some(result))
        } else {
            self.send_to(local_value, root).await?;
            Ok(None)
        }
    }

    /// Send data to specific rank
    pub async fn send_to<T: serde::Serialize>(&self, data: T, dest: i32) -> Result<(), Error> {
        let serialized = bincode::serialize(&data)
            .map_err(Error::Serialization)?;

        self.universe.world().process_at_rank(dest).send(&serialized);
        Ok(())
    }

    /// Receive data from specific rank
    pub async fn receive_from<T: serde::de::DeserializeOwned>(&self, source: i32) -> Result<T, Error> {
        let mut buffer = vec![0u8; 1024 * 1024];
        let (_msg, _status) = self.universe.world().process_at_rank(source).receive_into(&mut buffer);

        bincode::deserialize(&buffer)
            .map_err(Error::Serialization)
    }

    /// Barrier synchronization
    pub async fn barrier(&self) -> Result<(), Error> {
        self.universe.world().barrier();
        Ok(())
    }

    /// Store local data for distributed operations
    pub async fn store_local(&self, key: String, data: Vec<u8>) {
        self.local_data.write().await.insert(key, data);
    }

    /// Retrieve local data
    pub async fn get_local(&self, key: &str) -> Option<Vec<u8>> {
        self.local_data.read().await.get(key).cloned()
    }
}

/// Distributed data partitioning utilities
pub struct DataPartitioner;

impl DataPartitioner {
    /// Partition 1D array across ranks
    pub fn partition_1d(total_size: usize, rank: i32, size: i32) -> (usize, usize) {
        let base_size = total_size / size as usize;
        let remainder = total_size % size as usize;

        let start = rank as usize * base_size + (rank as usize).min(remainder);
        let extra = if rank as usize < remainder { 1 } else { 0 };
        let local_size = base_size + extra;

        (start, local_size)
    }

    /// Partition 2D array across ranks
    pub fn partition_2d(
        total_rows: usize,
        total_cols: usize,
        rank: i32,
        size: i32,
    ) -> ((usize, usize), (usize, usize)) {
        // Simple row-based partitioning
        let (row_start, local_rows) = Self::partition_1d(total_rows, rank, size);
        ((row_start, 0), (local_rows, total_cols))
    }

    /// Calculate global index from local index
    pub fn global_index(local_idx: usize, start_offset: usize) -> usize {
        local_idx + start_offset
    }

    /// Check if index is owned by this rank
    pub fn owns_index(local_idx: usize, local_size: usize) -> bool {
        local_idx < local_size
    }
}

/// Load balancing utilities
pub struct LoadBalancer;

impl LoadBalancer {
    /// Calculate optimal work distribution
    pub fn balance_workload(total_work: usize, num_workers: i32) -> Vec<(usize, usize)> {
        let mut distribution = Vec::new();
        let base_work = total_work / num_workers as usize;
        let remainder = total_work % num_workers as usize;

        let mut offset = 0;
        for i in 0..num_workers {
            let extra = if i as usize < remainder { 1 } else { 0 };
            let work_size = base_work + extra;
            distribution.push((offset, work_size));
            offset += work_size;
        }

        distribution
    }

    /// Redistribute work based on performance metrics
    pub fn rebalance(
        current_distribution: &[(usize, usize)],
        performance_metrics: &[f64],
    ) -> Vec<(usize, usize)> {
        // Simplified rebalancing - in practice would use more sophisticated algorithms
        let total_work: usize = current_distribution.iter().map(|(_, size)| size).sum();
        let avg_performance: f64 = performance_metrics.iter().sum::<f64>() / performance_metrics.len() as f64;

        let mut new_distribution = Vec::new();
        let mut offset = 0;

        for &perf in performance_metrics {
            let work_factor = perf / avg_performance;
            let work_size = ((total_work as f64 * work_factor) as usize).max(1);
            new_distribution.push((offset, work_size));
            offset += work_size;
        }

        // Adjust the last partition to ensure total work is preserved
        if let Some((_, ref mut last_size)) = new_distribution.last_mut() {
            *last_size = total_work - offset + *last_size;
        }

        new_distribution
    }
}
