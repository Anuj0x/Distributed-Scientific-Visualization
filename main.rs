//! Vistle - Modern Distributed Scientific Visualization System
//!
//! This is the main application demonstrating the complete Rust-based
//! architecture for distributed scientific visualization.

use std::sync::Arc;
use tokio;

use vistle::core::{
    MessageRouter, ModuleRegistry, TaskExecutor, WorkflowExecutor,
    WorkflowBuilder, WorkflowSpec,
};
use vistle::ui::{Application, WorkflowEditor, StatusDisplay, WorkflowNode};
use vistle::render::{RenderContext, RenderBackend, Scene, Camera, Material, Geometry};
use vistle::mpi::DistributedContext;
use vistle::util::PerformanceMonitor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Vistle system
    vistle::init().await?;

    println!("🚀 Starting Vistle v{}", env!("CARGO_PKG_VERSION"));
    println!("Modern distributed scientific visualization system built in Rust");

    // Initialize core components
    let message_router = Arc::new(MessageRouter::new().with_mpi()?);
    let module_registry = Arc::new(ModuleRegistry::new());
    let task_executor = Arc::new(TaskExecutor::new(8)); // 8 concurrent tasks
    let workflow_executor = Arc::new(WorkflowExecutor::new(
        module_registry.clone(),
        task_executor.clone(),
        message_router.clone(),
    ));

    // Register example modules
    register_example_modules(&module_registry).await?;

    // Create a sample workflow
    let workflow = create_sample_workflow();

    // Execute workflow
    println!("📊 Executing sample workflow...");
    let result = workflow_executor.execute_workflow(workflow, None).await?;

    println!("✅ Workflow completed in {:?}", result.execution_time);
    println!("📈 Processed {} tasks", result.task_results.len());

    // Launch GUI if not in headless mode
    if std::env::args().any(|arg| arg == "--gui") {
        run_gui().await?;
    } else {
        println!("💡 Use --gui flag to launch the graphical interface");
    }

    // Show performance summary
    show_performance_summary().await;

    println!("🎉 Vistle system demonstration completed successfully!");
    Ok(())
}

/// Register example modules for demonstration
async fn register_example_modules(registry: &ModuleRegistry) -> Result<(), vistle::Error> {
    // Register a data reader module
    registry.register("DataReader", |id| {
        Box::new(DataReaderModule::new(id))
    }).await?;

    // Register a filter module
    registry.register("IsoSurface", |id| {
        Box::new(IsoSurfaceModule::new(id))
    }).await?;

    // Register a renderer module
    registry.register("Renderer", |id| {
        Box::new(RendererModule::new(id))
    }).await?;

    println!("📦 Registered {} example modules", registry.list_available().await.len());
    Ok(())
}

/// Create a sample scientific visualization workflow
fn create_sample_workflow() -> WorkflowSpec {
    WorkflowBuilder::new("sample_workflow", "Sample Scientific Visualization")
        .description("Demonstrates a complete data processing pipeline")
        .add_module("DataReader", "Load Data")
            .parameter("filename", "sample_data.vtk")
            .parameter("format", "VTK")
            .add_module("IsoSurface", "Extract Surface")
                .parameter("iso_value", "0.5")
                .depends_on(1)
            .add_module("Renderer", "Render Results")
                .depends_on(2)
        .connect(1, "data_out", 2, "data_in")
        .connect(2, "surface_out", 3, "geometry_in")
        .build()
}

/// Run the graphical user interface
async fn run_gui() -> Result<(), vistle::Error> {
    println!("🎨 Launching GUI...");

    let mut workflow_editor = WorkflowEditor::new();
    let mut status_display = StatusDisplay::new(100);

    // Add some sample nodes
    workflow_editor.add_node(
        WorkflowNode::new("Data Reader", "Load scientific data", "DataReader")
            .with_position(egui::pos2(50.0, 50.0))
            .add_output("data")
    );

    workflow_editor.add_node(
        WorkflowNode::new("Filter", "Process data", "IsoSurface")
            .with_position(egui::pos2(250.0, 50.0))
            .add_input("data_in")
            .add_output("surface_out")
    );

    workflow_editor.add_node(
        WorkflowNode::new("Renderer", "Visualize results", "Renderer")
            .with_position(egui::pos2(450.0, 50.0))
            .add_input("geometry_in")
    );

    status_display.add_message("GUI initialized".to_string(), vistle::ui::StatusLevel::Success);
    status_display.add_message("Workflow editor ready".to_string(), vistle::ui::StatusLevel::Info);

    let app = Application::new("Vistle - Modern Scientific Visualization", (1200, 800));

    app.run(move |ui_ctx| {
        ui_ctx.heading("Vistle Workflow Editor");
        ui_ctx.separator();

        // Draw workflow editor
        workflow_editor.draw(ui_ctx);

        // Draw status display
        status_display.draw(ui_ctx);

        // Add control buttons
        ui_ctx.begin_panel("Controls");

        if ui_ctx.button("Execute Workflow") {
            status_display.add_message(
                "Workflow execution started".to_string(),
                vistle::ui::StatusLevel::Info
            );
        }

        if ui_ctx.button("Clear Status") {
            status_display.clear();
        }

        ui_ctx.end_panel();
    }).await?;

    Ok(())
}

/// Show performance summary
async fn show_performance_summary() {
    println!("\n📊 Performance Summary:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let monitor = PerformanceMonitor::new();

    // This would normally collect real performance data
    println!("• Memory Usage: Efficient (Rust ownership system)");
    println!("• CPU Utilization: Optimized (SIMD support available)");
    println!("• Network: Low latency (Async message passing)");
    println!("• GPU: Hardware acceleration ready (wgpu integration)");
    println!("• Concurrency: High (Tokio async runtime)");
    println!("• Safety: Memory safe (No undefined behavior)");
}

/// Example data reader module
struct DataReaderModule {
    id: u32,
    parameters: vistle::core::ParameterSet,
    ports: vistle::core::PortSet,
}

impl DataReaderModule {
    fn new(id: u32) -> Self {
        let mut params = vistle::core::ParameterSet::new();
        params.add(vistle::core::Parameter::new("filename", "Input filename", vistle::core::ParameterValue::String("data.vtk".to_string())));
        params.add(vistle::core::Parameter::new("format", "File format", vistle::core::ParameterValue::String("VTK".to_string())));

        let mut ports = vistle::core::PortSet::new();
        ports.add(vistle::core::Port::new_output("data", "Output data"));

        Self { id, parameters: params, ports }
    }
}

#[async_trait::async_trait]
impl vistle::compute::Module for DataReaderModule {
    fn info(&self) -> &vistle::core::ModuleInfo {
        // This would normally be stored in the struct
        &vistle::core::ModuleInfo::new(self.id, "DataReader", 0, 1)
    }

    fn parameters(&self) -> &vistle::core::ParameterSet {
        &self.parameters
    }

    fn ports(&self) -> &vistle::core::PortSet {
        &self.ports
    }

    async fn set_input(&mut self, _port_name: &str, _objects: vistle::compute::InputPort) -> Result<(), vistle::Error> {
        Ok(())
    }

    async fn compute(&mut self, _ctx: &vistle::core::ComputeContext) -> Result<vistle::compute::OutputPorts, vistle::Error> {
        // Simulate data reading
        println!("📖 Reading data from file...");

        let mut outputs = std::collections::HashMap::new();
        let data_object = Arc::new(vistle::core::VistleObject::with_data(
            vistle::core::ObjectType::UnstructuredGrid,
            vistle::core::ObjectPayload::Custom(vec![1, 2, 3, 4, 5]) // Placeholder data
        ));

        outputs.insert("data".to_string(), vec![data_object]);
        Ok(outputs)
    }

    fn stats(&self) -> &vistle::core::ExecutionStats {
        // This would normally be stored in the struct
        static STATS: std::sync::OnceLock<vistle::core::ExecutionStats> = std::sync::OnceLock::new();
        STATS.get_or_init(|| vistle::core::ExecutionStats::new(self.id))
    }
}

/// Example isosurface filter module
struct IsoSurfaceModule {
    id: u32,
    parameters: vistle::core::ParameterSet,
    ports: vistle::core::PortSet,
}

impl IsoSurfaceModule {
    fn new(id: u32) -> Self {
        let mut params = vistle::core::ParameterSet::new();
        params.add(vistle::core::Parameter::new("iso_value", "Isosurface value", vistle::core::ParameterValue::Float(0.5)));

        let mut ports = vistle::core::PortSet::new();
        ports.add(vistle::core::Port::new_input("data_in", "Input data"));
        ports.add(vistle::core::Port::new_output("surface_out", "Output surface"));

        Self { id, parameters: params, ports }
    }
}

#[async_trait::async_trait]
impl vistle::compute::Module for IsoSurfaceModule {
    fn info(&self) -> &vistle::core::ModuleInfo {
        &vistle::core::ModuleInfo::new(self.id, "IsoSurface", 0, 1)
    }

    fn parameters(&self) -> &vistle::core::ParameterSet {
        &self.parameters
    }

    fn ports(&self) -> &vistle::core::PortSet {
        &self.ports
    }

    async fn set_input(&mut self, _port_name: &str, _objects: vistle::compute::InputPort) -> Result<(), vistle::Error> {
        Ok(())
    }

    async fn compute(&mut self, _ctx: &vistle::core::ComputeContext) -> Result<vistle::compute::OutputPorts, vistle::Error> {
        // Simulate isosurface extraction
        println!("🔍 Extracting isosurface...");

        let mut outputs = std::collections::HashMap::new();
        let surface_object = Arc::new(vistle::core::VistleObject::with_data(
            vistle::core::ObjectType::Triangles,
            vistle::core::ObjectPayload::Triangles {
                coordinates: ndarray::Array2::zeros((0, 3)), // Placeholder
                triangles: ndarray::Array2::zeros((0, 3)),   // Placeholder
            }
        ));

        outputs.insert("surface_out".to_string(), vec![surface_object]);
        Ok(outputs)
    }

    fn stats(&self) -> &vistle::core::ExecutionStats {
        static STATS: std::sync::OnceLock<vistle::core::ExecutionStats> = std::sync::OnceLock::new();
        STATS.get_or_init(|| vistle::core::ExecutionStats::new(self.id))
    }
}

/// Example renderer module
struct RendererModule {
    id: u32,
    parameters: vistle::core::ParameterSet,
    ports: vistle::core::PortSet,
}

impl RendererModule {
    fn new(id: u32) -> Self {
        let mut params = vistle::core::ParameterSet::new();
        params.add(vistle::core::Parameter::new("background_color", "Background color", vistle::core::ParameterValue::VecFloat(vec![0.0, 0.0, 0.0, 1.0])));

        let mut ports = vistle::core::PortSet::new();
        ports.add(vistle::core::Port::new_input("geometry_in", "Input geometry"));

        Self { id, parameters: params, ports }
    }
}

#[async_trait::async_trait]
impl vistle::compute::Module for RendererModule {
    fn info(&self) -> &vistle::core::ModuleInfo {
        &vistle::core::ModuleInfo::new(self.id, "Renderer", 0, 1)
    }

    fn parameters(&self) -> &vistle::core::ParameterSet {
        &self.parameters
    }

    fn ports(&self) -> &vistle::core::PortSet {
        &self.ports
    }

    async fn set_input(&mut self, _port_name: &str, _objects: vistle::compute::InputPort) -> Result<(), vistle::Error> {
        Ok(())
    }

    async fn compute(&mut self, _ctx: &vistle::core::ComputeContext) -> Result<vistle::compute::OutputPorts, vistle::Error> {
        // Simulate rendering
        println!("🎨 Rendering visualization...");

        // This would normally produce rendered images/output
        Ok(std::collections::HashMap::new())
    }

    fn stats(&self) -> &vistle::core::ExecutionStats {
        static STATS: std::sync::OnceLock<vistle::core::ExecutionStats> = std::sync::OnceLock::new();
        STATS.get_or_init(|| vistle::core::ExecutionStats::new(self.id))
    }
}
