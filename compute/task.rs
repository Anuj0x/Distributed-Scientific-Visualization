//! Task execution and dependency management

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use futures::future::join_all;

use crate::core::{ComputeContext, ObjectId};
use crate::compute::{Module, OutputPorts};

/// Execution task representing a module computation
pub struct Task {
    pub id: TaskId,
    pub module: Arc<dyn Module>,
    pub context: ComputeContext,
    pub dependencies: Vec<TaskId>,
    pub dependents: Vec<TaskId>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
}

impl Task {
    pub fn new(
        id: TaskId,
        module: Arc<dyn Module>,
        context: ComputeContext,
    ) -> Self {
        Self {
            id,
            module,
            context,
            dependencies: Vec::new(),
            dependents: Vec::new(),
            status: TaskStatus::Pending,
            priority: TaskPriority::Normal,
        }
    }

    pub fn with_dependencies(mut self, deps: Vec<TaskId>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Check if all dependencies are satisfied
    pub fn dependencies_satisfied(&self, completed_tasks: &HashSet<TaskId>) -> bool {
        self.dependencies.iter().all(|dep| completed_tasks.contains(dep))
    }

    /// Get the module ID for this task
    pub fn module_id(&self) -> u32 {
        self.context.module_id
    }
}

/// Unique task identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

impl TaskId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Default for TaskId {
    fn default() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// Task execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Task execution priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Task graph for managing dependencies and execution order
pub struct TaskGraph {
    tasks: HashMap<TaskId, Task>,
    completed: HashSet<TaskId>,
    ready_queue: VecDeque<TaskId>,
    semaphore: Arc<Semaphore>, // Limit concurrent executions
}

impl TaskGraph {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            tasks: HashMap::new(),
            completed: HashSet::new(),
            ready_queue: VecDeque::new(),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    pub fn add_task(&mut self, task: Task) {
        let task_id = task.id;

        // Check if dependencies are satisfied
        if task.dependencies_satisfied(&self.completed) {
            self.ready_queue.push_back(task_id);
        }

        self.tasks.insert(task_id, task);
    }

    pub fn mark_completed(&mut self, task_id: TaskId) {
        if self.completed.insert(task_id) {
            // Check dependents that might now be ready
            let dependents = self.tasks.get(&task_id)
                .map(|t| t.dependents.clone())
                .unwrap_or_default();

            for dependent_id in dependents {
                if let Some(dependent) = self.tasks.get(&dependent_id) {
                    if dependent.dependencies_satisfied(&self.completed)
                        && !self.ready_queue.contains(&dependent_id) {
                        self.ready_queue.push_back(dependent_id);
                    }
                }
            }
        }
    }

    pub fn get_ready_task(&mut self) -> Option<TaskId> {
        self.ready_queue.pop_front()
    }

    pub fn get_task(&self, id: TaskId) -> Option<&Task> {
        self.tasks.get(&id)
    }

    pub fn get_task_mut(&mut self, id: TaskId) -> Option<&mut Task> {
        self.tasks.get_mut(&id)
    }

    pub fn is_complete(&self) -> bool {
        self.tasks.len() == self.completed.len()
    }

    pub fn pending_count(&self) -> usize {
        self.tasks.len() - self.completed.len()
    }

    pub fn semaphore(&self) -> Arc<Semaphore> {
        self.semaphore.clone()
    }
}

/// Task execution result
#[derive(Debug)]
pub struct TaskResult {
    pub task_id: TaskId,
    pub success: bool,
    pub outputs: Option<OutputPorts>,
    pub error: Option<String>,
    pub execution_time: std::time::Duration,
}

/// Task executor for running tasks concurrently
pub struct TaskExecutor {
    graph: RwLock<TaskGraph>,
    results: RwLock<HashMap<TaskId, TaskResult>>,
}

impl TaskExecutor {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            graph: RwLock::new(TaskGraph::new(max_concurrent)),
            results: RwLock::new(HashMap::new()),
        }
    }

    /// Add a task to the execution graph
    pub async fn add_task(&self, task: Task) {
        let mut graph = self.graph.write().await;
        graph.add_task(task);
    }

    /// Execute all tasks in the graph
    pub async fn execute_all(&self) -> Result<Vec<TaskResult>, crate::Error> {
        let semaphore = {
            let graph = self.graph.read().await;
            graph.semaphore()
        };

        let mut handles = Vec::new();

        loop {
            let permit = semaphore.acquire().await
                .map_err(|_| crate::Error::Module("Failed to acquire execution permit".to_string()))?;

            let task_id = {
                let mut graph = self.graph.write().await;
                match graph.get_ready_task() {
                    Some(id) => id,
                    None => break, // No more ready tasks
                }
            };

            let graph_clone = self.graph.clone();
            let results_clone = self.results.clone();

            let handle = tokio::spawn(async move {
                let start_time = std::time::Instant::now();

                // Get task
                let task = {
                    let graph = graph_clone.read().await;
                    graph.get_task(task_id).cloned()
                };

                let result = if let Some(mut task) = task {
                    // Update status to running
                    task.status = TaskStatus::Running;

                    // Execute task (placeholder - would call actual module)
                    let success = true; // Placeholder
                    let outputs = None; // Placeholder
                    let error = None; // Placeholder

                    TaskResult {
                        task_id,
                        success,
                        outputs,
                        error,
                        execution_time: start_time.elapsed(),
                    }
                } else {
                    TaskResult {
                        task_id,
                        success: false,
                        outputs: None,
                        error: Some("Task not found".to_string()),
                        execution_time: start_time.elapsed(),
                    }
                };

                // Store result
                {
                    let mut results = results_clone.write().await;
                    results.insert(task_id, result.clone());
                }

                // Mark task as completed
                {
                    let mut graph = graph_clone.write().await;
                    graph.mark_completed(task_id);
                }

                // Release permit
                drop(permit);

                result
            });

            handles.push(handle);
        }

        // Wait for all tasks to complete
        let results = join_all(handles).await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| crate::Error::Module(format!("Task execution failed: {}", e)))?;

        Ok(results)
    }

    /// Get execution results
    pub async fn results(&self) -> HashMap<TaskId, TaskResult> {
        self.results.read().await.clone()
    }

    /// Check if execution is complete
    pub async fn is_complete(&self) -> bool {
        self.graph.read().await.is_complete()
    }

    /// Get pending task count
    pub async fn pending_count(&self) -> usize {
        self.graph.read().await.pending_count()
    }
}

/// Task builder for fluent task construction
pub struct TaskBuilder {
    module: Option<Arc<dyn Module>>,
    context: Option<ComputeContext>,
    dependencies: Vec<TaskId>,
    priority: TaskPriority,
}

impl TaskBuilder {
    pub fn new() -> Self {
        Self {
            module: None,
            context: None,
            dependencies: Vec::new(),
            priority: TaskPriority::Normal,
        }
    }

    pub fn module(mut self, module: Arc<dyn Module>) -> Self {
        self.module = Some(module);
        self
    }

    pub fn context(mut self, context: ComputeContext) -> Self {
        self.context = Some(context);
        self
    }

    pub fn depends_on(mut self, task_id: TaskId) -> Self {
        self.dependencies.push(task_id);
        self
    }

    pub fn priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn build(self) -> Result<Task, String> {
        let module = self.module.ok_or("Module not specified")?;
        let context = self.context.ok_or("Context not specified")?;

        let task = Task::new(TaskId::default(), module, context)
            .with_dependencies(self.dependencies)
            .with_priority(self.priority);

        Ok(task)
    }
}

impl Default for TaskBuilder {
    fn default() -> Self {
        Self::new()
    }
}
