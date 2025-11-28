//! Message passing system for distributed communication

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::core::ObjectId;

/// Unique message identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(Uuid);

impl MessageId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

/// Message priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Message types for Vistle communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    // Control messages
    Execute {
        module_id: u32,
        timestep: i32,
    },
    CancelExecute {
        module_id: u32,
    },
    Quit,

    // Data messages
    AddObject {
        object_id: ObjectId,
        port_name: String,
    },
    RemoveObject {
        object_id: ObjectId,
    },

    // Parameter messages
    SetParameter {
        module_id: u32,
        param_name: String,
        value: ParameterValue,
    },
    AddParameter {
        module_id: u32,
        param_name: String,
        param_type: ParameterType,
    },

    // Connection messages
    ConnectPorts {
        from_module: u32,
        from_port: String,
        to_module: u32,
        to_port: String,
    },
    DisconnectPorts {
        from_module: u32,
        from_port: String,
        to_module: u32,
        to_port: String,
    },

    // Status messages
    ModuleReady {
        module_id: u32,
    },
    ComputationComplete {
        module_id: u32,
        objects_created: Vec<ObjectId>,
    },
    Error {
        module_id: u32,
        message: String,
    },

    // Custom messages
    Custom {
        type_id: u32,
        data: Vec<u8>,
    },
}

/// Parameter value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterValue {
    Int(i32),
    Float(f32),
    String(String),
    Bool(bool),
    VecInt(Vec<i32>),
    VecFloat(Vec<f32>),
    VecString(Vec<String>),
}

/// Parameter types for dynamic parameter creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    Int { min: Option<i32>, max: Option<i32> },
    Float { min: Option<f32>, max: Option<f32> },
    String,
    Bool,
    VectorInt { min: Option<i32>, max: Option<i32> },
    VectorFloat { min: Option<f32>, max: Option<f32> },
    VectorString,
}

/// Complete message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub sender: u32,      // Module ID of sender
    pub recipient: u32,   // Module ID of recipient (0 for broadcast)
    pub priority: Priority,
    pub message_type: MessageType,
    pub timestamp: std::time::SystemTime,
}

impl Message {
    pub fn new(sender: u32, recipient: u32, message_type: MessageType) -> Self {
        Self {
            id: MessageId::new(),
            sender,
            recipient,
            priority: Priority::Normal,
            message_type,
            timestamp: std::time::SystemTime::now(),
        }
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn is_broadcast(&self) -> bool {
        self.recipient == 0
    }
}

/// Message payload for large data transfers
#[derive(Debug, Clone)]
pub enum MessagePayload {
    None,
    ObjectData(Vec<u8>),
    ParameterData(Vec<u8>),
    Custom(Vec<u8>),
}

/// Complete message envelope
#[derive(Debug)]
pub struct MessageEnvelope {
    pub message: Message,
    pub payload: MessagePayload,
}

/// Async message sender
#[async_trait::async_trait]
pub trait MessageSender: Send + Sync {
    async fn send_message(&self, message: MessageEnvelope) -> Result<(), crate::Error>;
}

/// Async message receiver
#[async_trait::async_trait]
pub trait MessageReceiver: Send + Sync {
    async fn receive_message(&mut self) -> Result<Option<MessageEnvelope>, crate::Error>;
}

/// In-memory message queue for local communication
pub struct MessageQueue {
    sender: mpsc::UnboundedSender<MessageEnvelope>,
    receiver: mpsc::UnboundedReceiver<MessageEnvelope>,
}

impl MessageQueue {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self { sender, receiver }
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<MessageEnvelope> {
        self.sender.clone()
    }
}

#[async_trait::async_trait]
impl MessageSender for MessageQueue {
    async fn send_message(&self, message: MessageEnvelope) -> Result<(), crate::Error> {
        self.sender.send(message)
            .map_err(|_| crate::Error::Module("Failed to send message".to_string()))?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl MessageReceiver for MessageQueue {
    async fn receive_message(&mut self) -> Result<Option<MessageEnvelope>, crate::Error> {
        match self.receiver.try_recv() {
            Ok(msg) => Ok(Some(msg)),
            Err(mpsc::error::TryRecvError::Empty) => Ok(None),
            Err(mpsc::error::TryRecvError::Disconnected) => Ok(None),
        }
    }
}

/// MPI-based distributed message passing
pub struct MpiMessageChannel {
    rank: i32,
    size: i32,
}

impl MpiMessageChannel {
    pub fn new() -> Result<Self, crate::Error> {
        let universe = mpi::initialize().map_err(crate::Error::Mpi)?;
        let world = universe.world();

        Ok(Self {
            rank: world.rank(),
            size: world.size(),
        })
    }

    pub fn rank(&self) -> i32 {
        self.rank
    }

    pub fn size(&self) -> i32 {
        self.size
    }
}

#[async_trait::async_trait]
impl MessageSender for MpiMessageChannel {
    async fn send_message(&self, message: MessageEnvelope) -> Result<(), crate::Error> {
        // Serialize message
        let data = bincode::serialize(&message)
            .map_err(crate::Error::Serialization)?;

        let universe = mpi::initialize().map_err(crate::Error::Mpi)?;
        let world = universe.world();

        // Send to recipient
        if message.message.recipient == 0 {
            // Broadcast to all ranks
            for rank in 0..self.size {
                if rank != self.rank {
                    world.process_at_rank(rank).send(&data);
                }
            }
        } else {
            // Send to specific rank
            world.process_at_rank(message.message.recipient as i32).send(&data);
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl MessageReceiver for MpiMessageChannel {
    async fn receive_message(&mut self) -> Result<Option<MessageEnvelope>, crate::Error> {
        let universe = mpi::initialize().map_err(crate::Error::Mpi)?;
        let world = universe.world();

        // Try to receive message (non-blocking)
        let mut buffer = Vec::new();
        match world.any_process().receive_into(&mut buffer) {
            Ok(_) => {
                let envelope: MessageEnvelope = bincode::deserialize(&buffer)
                    .map_err(crate::Error::Serialization)?;
                Ok(Some(envelope))
            }
            Err(_) => Ok(None), // No message available
        }
    }
}

/// Message router for complex communication patterns
pub struct MessageRouter {
    local_queues: dashmap::DashMap<u32, Arc<MessageQueue>>,
    mpi_channel: Option<MpiMessageChannel>,
    handlers: dashmap::DashMap<MessageId, mpsc::UnboundedSender<MessageEnvelope>>,
}

impl MessageRouter {
    pub fn new() -> Self {
        Self {
            local_queues: dashmap::DashMap::new(),
            mpi_channel: None,
            handlers: dashmap::DashMap::new(),
        }
    }

    pub fn with_mpi(mut self) -> Result<Self, crate::Error> {
        self.mpi_channel = Some(MpiMessageChannel::new()?);
        Ok(self)
    }

    pub fn register_module(&self, module_id: u32) -> Arc<MessageQueue> {
        let queue = Arc::new(MessageQueue::new());
        self.local_queues.insert(module_id, queue.clone());
        queue
    }

    pub async fn route_message(&self, envelope: MessageEnvelope) -> Result<(), crate::Error> {
        let recipient = envelope.message.recipient;

        // Check if it's a local message
        if let Some(queue) = self.local_queues.get(&recipient) {
            queue.send_message(envelope).await?;
            return Ok(());
        }

        // Use MPI for distributed messages
        if let Some(mpi) = &self.mpi_channel {
            mpi.send_message(envelope).await?;
            return Ok(());
        }

        Err(crate::Error::Module(format!("No route to module {}", recipient)))
    }

    pub async fn process_messages(&self) -> Result<(), crate::Error> {
        // Process MPI messages if available
        if let Some(mpi) = &self.mpi_channel {
            if let Some(envelope) = mpi.receive_message().await? {
                self.route_message(envelope).await?;
            }
        }

        Ok(())
    }
}

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}
