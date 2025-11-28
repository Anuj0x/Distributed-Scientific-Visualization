//! Workflow execution engine

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};

use crate::core::{
    MessageRouter, Message, MessageType, MessageEnvelope, MessagePayload,
    ComputeContext, ObjectRegistry, ShmManager,
};
use crate::compute::{ModuleRegistry, TaskExecutor, Task, TaskId, TaskBuilder, TaskPriority};

/// Workflow execution engine
pub struct WorkflowExecutor {
    module_registry: Arc<ModuleRegistry>,
    task_executor: Arc<TaskExecutor>,
    message_router: Arc<MessageRouter>,
    object_registry: Arc<ObjectRegistry>,
    shm_manager: Arc<ShmManager>,
    active_workflows: RwLock<HashMap<String, WorkflowState>>,
}

impl WorkflowExecutor {
    pub fn new(
        module_registry: Arc<ModuleRegistry>,
        task_executor: Arc<TaskExecutor>,
        message_router: Arc<MessageRouter>,
    ) -> Self {
        Self {
            module_registry,
            task_executor,
            message_router,
            object_registry: Arc::new(ObjectRegistry::new()),
            shm_manager: Arc::new(ShmManager::new()),
            active_workflows: RwLock::new(HashMap::new()),
        }
    }

    /// Execute a workflow with the given specification
    pub async fn execute_workflow(
        &self,
        workflow: WorkflowSpec,
        timeout_duration: Option<Duration>,
    ) -> Result<WorkflowResult, crate::Error> {
        let workflow_id = workflow.id.clone();

        // Initialize workflow state
        let state = WorkflowState {
            id: workflow_id.clone(),
            spec: workflow,
            status: WorkflowStatus::Running,
            start_time: std::time::Instant::now(),
            tasks_completed: 0,
            tasks_total: 0,
        };

        self.active_workflows.write().await.insert(workflow_id.clone(), state);

        // Build and submit tasks
        self.build_workflow_tasks(&workflow_id).await?;

        // Execute tasks with timeout if specified
        let execution_result = if let Some(duration) = timeout_duration {
            match timeout(duration, self.task_executor.execute_all()).await {
                Ok(result) => result,
                Err(_) => return Err(crate::Error::Module("Workflow execution timeout".to_string())),
            }
        } else {
            self.task_executor.execute_all().await
        };

        // Process results
        let results = execution_result?;
        let success = results.iter().all(|r| r.success);

        // Update workflow state
        let mut workflows = self.active_workflows.write().await;
        if let Some(state) = workflows.get_mut(&workflow_id) {
            state.status = if success { WorkflowStatus::Completed } else { WorkflowStatus::Failed };
            state.tasks_completed = results.len();
        }

        Ok(WorkflowResult {
            workflow_id,
            success,
            task_results: results,
            execution_time: std::time::Instant::now().elapsed(),
        })
    }

    /// Build tasks from workflow specification
    async fn build_workflow_tasks(&self, workflow_id: &str) -> Result<(), crate::Error> {
        let workflows = self.active_workflows.read().await;
        let workflow = workflows.get(workflow_id)
            .ok_or_else(|| crate::Error::Module("Workflow not found".to_string()))?;

        let mut task_map = HashMap::new();
        let mut task_dependencies = HashMap::new();

        // Create tasks for each module in the workflow
        for module_spec in &workflow.spec.modules {
            let module = self.module_registry.create_instance(
                &module_spec.module_type,
                module_spec.id,
            ).await?;

            let context = ComputeContext::new(module_spec.id, 0, 1); // Single rank for now

            let task = TaskBuilder::new()
                .module(module)
                .context(context)
                .priority(module_spec.priority)
                .build()
                .map_err(|e| crate::Error::Module(format!("Failed to build task: {}", e)))?;

            let task_id = task.id;
            task_map.insert(module_spec.id, task_id);
            task_dependencies.insert(task_id, module_spec.dependencies.clone());

            self.task_executor.add_task(task).await;
        }

        // Set up task dependencies
        for (task_id, deps) in task_dependencies {
            let dep_task_ids = deps.iter()
                .filter_map(|dep_id| task_map.get(dep_id))
                .copied()
                .collect::<Vec<_>>();

            // Update task dependencies (would need access to task graph)
            // This is a simplified version - in practice, the task graph
            // would handle dependency resolution
        }

        Ok(())
    }

    /// Get workflow status
    pub async fn workflow_status(&self, workflow_id: &str) -> Option<WorkflowStatus> {
        self.active_workflows.read().await
            .get(workflow_id)
            .map(|state| state.status)
    }

    /// Cancel a running workflow
    pub async fn cancel_workflow(&self, workflow_id: &str) -> Result<(), crate::Error> {
        let mut workflows = self.active_workflows.write().await;
        if let Some(state) = workflows.get_mut(workflow_id) {
            state.status = WorkflowStatus::Cancelled;
            // Send cancellation messages to modules
            // Implementation would cancel running tasks
        }
        Ok(())
    }

    /// Get active workflows
    pub async fn active_workflows(&self) -> Vec<String> {
        self.active_workflows.read().await
            .keys()
            .cloned()
            .collect()
    }

    /// Process incoming messages and update workflow state
    pub async fn process_messages(&self) -> Result<(), crate::Error> {
        // Process messages from the router
        self.message_router.process_messages().await?;

        // Update workflow states based on messages
        // This would handle module completion notifications,
        // error reports, etc.

        Ok(())
    }
}

/// Workflow specification
#[derive(Debug, Clone)]
pub struct WorkflowSpec {
    pub id: String,
    pub name: String,
    pub description: String,
    pub modules: Vec<ModuleSpec>,
    pub connections: Vec<ConnectionSpec>,
}

impl WorkflowSpec {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
            modules: Vec::new(),
            connections: Vec::new(),
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    pub fn add_module(mut self, module: ModuleSpec) -> Self {
        self.modules.push(module);
        self
    }

    pub fn add_connection(mut self, connection: ConnectionSpec) -> Self {
        self.connections.push(connection);
        self
    }
}

/// Module specification in a workflow
#[derive(Debug, Clone)]
pub struct ModuleSpec {
    pub id: u32,
    pub module_type: String,
    pub name: String,
    pub parameters: HashMap<String, String>, // Parameter name -> value as string
    pub dependencies: Vec<u32>, // Module IDs this depends on
    pub priority: TaskPriority,
}

impl ModuleSpec {
    pub fn new(id: u32, module_type: &str, name: &str) -> Self {
        Self {
            id,
            module_type: module_type.to_string(),
            name: name.to_string(),
            parameters: HashMap::new(),
            dependencies: Vec::new(),
            priority: TaskPriority::Normal,
        }
    }

    pub fn with_parameter(mut self, name: &str, value: &str) -> Self {
        self.parameters.insert(name.to_string(), value.to_string());
        self
    }

    pub fn depends_on(mut self, module_id: u32) -> Self {
        self.dependencies.push(module_id);
        self
    }

    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }
}

/// Connection specification between modules
#[derive(Debug, Clone)]
pub struct ConnectionSpec {
    pub from_module: u32,
    pub from_port: String,
    pub to_module: u32,
    pub to_port: String,
}

/// Workflow execution state
#[derive(Debug, Clone)]
struct WorkflowState {
    id: String,
    spec: WorkflowSpec,
    status: WorkflowStatus,
    start_time: std::time::Instant,
    tasks_completed: usize,
    tasks_total: usize,
}

/// Workflow execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Workflow execution result
#[derive(Debug)]
pub struct WorkflowResult {
    pub workflow_id: String,
    pub success: bool,
    pub task_results: Vec<crate::compute::TaskResult>,
    pub execution_time: std::time::Duration,
}

/// Workflow builder for fluent construction
pub struct WorkflowBuilder {
    spec: WorkflowSpec,
    next_module_id: u32,
}

impl WorkflowBuilder {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            spec: WorkflowSpec::new(id, name),
            next_module_id: 1,
        }
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.spec.description = desc.to_string();
        self
    }

    pub fn add_module(mut self, module_type: &str, name: &str) -> ModuleBuilder {
        let module_id = self.next_module_id;
        self.next_module_id += 1;

        let module_spec = ModuleSpec::new(module_id, module_type, name);
        self.spec.modules.push(module_spec);

        ModuleBuilder {
            workflow_builder: self,
            module_id,
        }
    }

    pub fn connect(mut self, from: u32, from_port: &str, to: u32, to_port: &str) -> Self {
        let connection = ConnectionSpec {
            from_module: from,
            from_port: from_port.to_string(),
            to_module: to,
            to_port: to_port.to_string(),
        };
        self.spec.connections.push(connection);
        self
    }

    pub fn build(self) -> WorkflowSpec {
        self.spec
    }
}

/// Module builder for fluent module configuration
pub struct ModuleBuilder {
    workflow_builder: WorkflowBuilder,
    module_id: u32,
}

impl ModuleBuilder {
    pub fn parameter(mut self, name: &str, value: &str) -> Self {
        if let Some(module) = self.workflow_builder.spec.modules.last_mut() {
            if module.id == self.module_id {
                module.parameters.insert(name.to_string(), value.to_string());
            }
        }
        self
    }

    pub fn priority(mut self, priority: TaskPriority) -> Self {
        if let Some(module) = self.workflow_builder.spec.modules.last_mut() {
            if module.id == self.module_id {
                module.priority = priority;
            }
        }
        self
    }

    pub fn depends_on(mut self, dependency_id: u32) -> Self {
        if let Some(module) = self.workflow_builder.spec.modules.last_mut() {
            if module.id == self.module_id {
                module.dependencies.push(dependency_id);
            }
        }
        self
    }

    pub fn add_module(mut self, module_type: &str, name: &str) -> ModuleBuilder {
        self.workflow_builder = self.workflow_builder.add_module(module_type, name);
        self
    }

    pub fn connect(mut self, from: u32, from_port: &str, to: u32, to_port: &str) -> WorkflowBuilder {
        self.workflow_builder.connect(from, from_port, to, to_port)
    }

    pub fn build(self) -> WorkflowSpec {
        self.workflow_builder.build()
    }
}
