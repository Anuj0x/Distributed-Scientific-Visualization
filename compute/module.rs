//! Module system for computation and data processing

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::{
    Object, ObjectId, ParameterSet, PortSet, ComputeContext,
    MessageRouter, Message, MessageType, MessageEnvelope, MessagePayload,
    ModuleInfo, ModuleStatus, ExecutionStats, ObjectRegistry,
};

/// Input data for a module port
pub type InputPort = Vec<Arc<dyn Object>>;
/// Output data from a module port
pub type OutputPort = Vec<Arc<dyn Object>>;
/// Collection of input ports
pub type InputPorts = HashMap<String, InputPort>;
/// Collection of output ports
pub type OutputPorts = HashMap<String, OutputPort>;

/// Core module trait that all Vistle modules must implement
#[async_trait::async_trait]
pub trait Module: Send + Sync {
    /// Get module information
    fn info(&self) -> &ModuleInfo;

    /// Get module parameters
    fn parameters(&self) -> &ParameterSet;

    /// Get module ports
    fn ports(&self) -> &PortSet;

    /// Set input data for a specific port
    async fn set_input(&mut self, port_name: &str, objects: InputPort) -> Result<(), crate::Error>;

    /// Execute the module's computation
    async fn compute(&mut self, ctx: &ComputeContext) -> Result<OutputPorts, crate::Error>;

    /// Cancel execution if possible
    async fn cancel(&mut self) -> Result<(), crate::Error> {
        Ok(())
    }

    /// Get execution statistics
    fn stats(&self) -> &ExecutionStats;
}

/// Concrete module implementation
pub struct VistleModule<M: Module> {
    inner: M,
    inputs: RwLock<InputPorts>,
    status: RwLock<ModuleStatus>,
    stats: RwLock<ExecutionStats>,
}

impl<M: Module> VistleModule<M> {
    pub fn new(module: M) -> Self {
        let stats = ExecutionStats::new(module.info().id);
        Self {
            inner: module,
            inputs: RwLock::new(HashMap::new()),
            status: RwLock::new(ModuleStatus::Initializing),
            stats: RwLock::new(stats),
        }
    }

    pub async fn set_input(&self, port_name: &str, objects: InputPort) -> Result<(), crate::Error> {
        // Validate port exists
        if self.inner.ports().get(port_name).is_none() {
            return Err(crate::Error::Module(format!("Port {} not found", port_name)));
        }

        let mut inputs = self.inputs.write().await;
        inputs.insert(port_name.to_string(), objects);
        Ok(())
    }

    pub async fn execute(&self, ctx: &ComputeContext, router: &MessageRouter) -> Result<(), crate::Error> {
        // Update status
        *self.status.write().await = ModuleStatus::Executing;

        // Send execution started message
        let start_msg = Message::new(
            self.inner.info().id,
            0, // broadcast
            MessageType::Execute {
                module_id: self.inner.info().id,
                timestep: ctx.timestep,
            },
        );
        router.route_message(MessageEnvelope {
            message: start_msg,
            payload: MessagePayload::None,
        }).await?;

        // Perform computation
        let inputs = self.inputs.read().await.clone();
        let result = self.inner.compute(ctx).await;

        // Update statistics
        let mut stats = self.stats.write().await;
        match &result {
            Ok(outputs) => {
                stats.increment_processed();
                for objects in outputs.values() {
                    stats.objects_created += objects.len();
                }
                *self.status.write().await = ModuleStatus::Completed;
            }
            Err(e) => {
                stats.add_error(e.to_string());
                *self.status.write().await = ModuleStatus::Error;
            }
        }

        // Send completion message
        let complete_msg = Message::new(
            self.inner.info().id,
            0,
            MessageType::ComputationComplete {
                module_id: self.inner.info().id,
                objects_created: stats.objects_created as Vec<ObjectId>,
            },
        );
        router.route_message(MessageEnvelope {
            message: complete_msg,
            payload: MessagePayload::None,
        }).await?;

        result.map(|_| ())
    }

    pub async fn status(&self) -> ModuleStatus {
        *self.status.read().await
    }

    pub async fn statistics(&self) -> ExecutionStats {
        self.stats.read().await.clone()
    }
}

/// Module registry for dynamic loading
pub struct ModuleRegistry {
    modules: RwLock<HashMap<String, Box<dyn Fn() -> Box<dyn Module> + Send + Sync>>>,
    instances: RwLock<HashMap<u32, Arc<dyn std::any::Any + Send + Sync>>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: RwLock::new(HashMap::new()),
            instances: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register<M: Module + 'static, F>(&self, name: &str, constructor: F)
    where
        F: Fn() -> M + Send + Sync + 'static,
    {
        let constructor = Box::new(move || Box::new(constructor()) as Box<dyn Module>);
        self.modules.write().await.insert(name.to_string(), constructor);
    }

    pub async fn create_instance(&self, name: &str, id: u32) -> Result<Arc<VistleModule<Box<dyn Module>>>, crate::Error> {
        let modules = self.modules.read().await;
        let constructor = modules.get(name)
            .ok_or_else(|| crate::Error::Module(format!("Module {} not found", name)))?;

        let module = constructor();
        let vistle_module = Arc::new(VistleModule::new(module));

        self.instances.write().await.insert(id, vistle_module.clone());

        Ok(vistle_module)
    }

    pub async fn get_instance(&self, id: u32) -> Option<Arc<VistleModule<Box<dyn Module>>>> {
        self.instances.read().await.get(&id)
            .and_then(|instance| instance.downcast_ref::<VistleModule<Box<dyn Module>>>()
                .map(|m| Arc::new(m.clone())))
    }

    pub async fn list_available(&self) -> Vec<String> {
        self.modules.read().await.keys().cloned().collect()
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Module factory for creating common module types
pub struct ModuleFactory;

impl ModuleFactory {
    /// Create a data source module that generates test data
    pub fn create_data_source(id: u32, name: &str) -> Result<Box<dyn Module>, crate::Error> {
        // This would be implemented with actual module logic
        // For now, return a placeholder
        Err(crate::Error::Module("Data source module not implemented".to_string()))
    }

    /// Create a filter module that processes data
    pub fn create_filter(id: u32, name: &str) -> Result<Box<dyn Module>, crate::Error> {
        Err(crate::Error::Module("Filter module not implemented".to_string()))
    }

    /// Create a renderer module for visualization
    pub fn create_renderer(id: u32, name: &str) -> Result<Box<dyn Module>, crate::Error> {
        Err(crate::Error::Module("Renderer module not implemented".to_string()))
    }
}

/// Helper macro for implementing the Module trait
#[macro_export]
macro_rules! vistle_module {
    ($name:ident, $desc:expr) => {
        impl $name {
            pub fn new(id: u32) -> Self {
                let info = ModuleInfo::new(id, stringify!($name), 0, 1);
                let mut params = ParameterSet::new();
                let mut ports = PortSet::new();

                Self::setup_parameters(&mut params);
                Self::setup_ports(&mut ports);

                Self {
                    info,
                    parameters: params,
                    ports,
                    inputs: HashMap::new(),
                }
            }

            fn setup_parameters(params: &mut ParameterSet) {
                // Override in implementation
            }

            fn setup_ports(ports: &mut PortSet) {
                // Override in implementation
            }
        }

        #[async_trait::async_trait]
        impl Module for $name {
            fn info(&self) -> &ModuleInfo {
                &self.info
            }

            fn parameters(&self) -> &ParameterSet {
                &self.parameters
            }

            fn ports(&self) -> &PortSet {
                &self.ports
            }

            async fn set_input(&mut self, port_name: &str, objects: InputPort) -> Result<(), crate::Error> {
                self.inputs.insert(port_name.to_string(), objects);
                Ok(())
            }

            async fn compute(&mut self, _ctx: &ComputeContext) -> Result<OutputPorts, crate::Error> {
                Err(crate::Error::Module("Compute not implemented".to_string()))
            }

            fn stats(&self) -> &ExecutionStats {
                &self.stats
            }
        }
    };
}
