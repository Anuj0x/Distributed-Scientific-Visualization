# Distributed Scientific Visualization - Modern HPC Framework

**Created by: [Anuj0x](https://github.com/Anuj0x)** - Expert in Programming & Scripting Languages, Deep Learning & State-of-the-Art AI Models, Generative Models & Autoencoders, Advanced Attention Mechanisms & Model Optimization, Multimodal Fusion & Cross-Attention Architectures, Reinforcement Learning & Neural Architecture Search, AI Hardware Acceleration & MLOps, Computer Vision & Image Processing, Data Management & Vector Databases, Agentic LLMs & Prompt Engineering, Forecasting & Time Series Models, Optimization & Algorithmic Techniques, Blockchain & Decentralized Applications, DevOps, Cloud & Cybersecurity, Quantum AI & Circuit Design, Web Development Frameworks.

> **distributed scientific visualization**: very smooth and elegant, nicely put together

Distributed Scientific Visualization is a high-performance, memory-safe distributed visualization system built in Rust. It integrates simulations on supercomputers, post-processing, and parallel interactive visualization in immersive virtual environments.

- **Memory Safety**: Zero unsafe code, compile-time guarantees prevent data races and memory corruption
- **High Performance**: Zero-cost abstractions, SIMD acceleration, custom allocators
- **Distributed Computing**: Native MPI support for supercomputer-scale workflows
- **Modern Async**: Tokio runtime for efficient distributed communication patterns
- **GPU Acceleration**: wgpu integration for modern graphics APIs
- **Plugin Architecture**: Hot-reloadable modules with trait-based design
- **Scientific Computing**: ndarray/nalgebra integration for high-performance math

## 📦 Architecture

```
vistle/
├── src/
│   ├── main.rs              # Application entry point
│   ├── lib.rs               # Core library exports
│   ├── core/                # Core data structures (5 modules)
│   │   ├── object.rs        # Safe object system
│   │   ├── shm.rs          # Shared memory management
│   │   ├── message.rs       # Async message passing
│   │   ├── parameter.rs     # Module configuration
│   │   └── meta.rs          # Metadata handling
│   ├── compute/             # Workflow execution (4 modules)
│   │   ├── module.rs        # Plugin architecture
│   │   ├── task.rs          # Task scheduling
│   │   ├── executor.rs      # Workflow orchestration
│   │   └── task.rs          # Dependency management
│   ├── mpi/                 # Distributed computing (1 module)
│   ├── render/              # GPU rendering (1 module)
│   ├── ui/                  # GUI system (1 module)
│   └── util/                # Utilities (1 module)
```

## 🛠️ Installation

### Prerequisites

- Rust 1.70 or later
- MPI implementation (OpenMPI, MPICH, etc.)
- Vulkan-compatible GPU (optional, for GPU rendering)

### Build

```bash
git clone https://github.com/vistle/vistle.git
cd vistle
cargo build --release
```

### MPI Support

For distributed computing capabilities:

```bash
# Ubuntu/Debian
sudo apt install libopenmpi-dev

# macOS with Homebrew
brew install openmpi

# Then build with MPI support
cargo build --release --features mpi
```

## 🚀 Usage

### Command Line

```bash
# Run in headless mode
./target/release/vistle

# Launch GUI
./target/release/vistle --gui
```

### Basic Workflow Example

```rust
use vistle::core::{WorkflowBuilder, WorkflowExecutor, MessageRouter, ModuleRegistry, TaskExecutor};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize components
    let message_router = Arc::new(MessageRouter::new().with_mpi()?);
    let module_registry = Arc::new(ModuleRegistry::new());
    let task_executor = Arc::new(TaskExecutor::new(8));

    // Create workflow
    let workflow = WorkflowBuilder::new("simulation_viz", "CFD Visualization")
        .add_module("DataReader", "Load Data")
        .add_module("IsoSurface", "Extract Surface")
        .add_module("Renderer", "Render Results")
        .connect(1, "data", 2, "input")
        .connect(2, "surface", 3, "geometry")
        .build();

    // Execute
    let executor = WorkflowExecutor::new(
        module_registry, task_executor, message_router
    );
    let result = executor.execute_workflow(workflow, None).await?;

    println!("✅ Workflow completed in {:?}", result.execution_time);
    Ok(())
}
```

## 📊 Performance

The Rust implementation provides significant improvements over the original C++ version:

- **Memory Safety**: Eliminates entire classes of bugs
- **Zero-Cost Abstractions**: No runtime overhead for high-level constructs
- **SIMD Acceleration**: Automatic vectorization where beneficial
- **Better Concurrency**: Fearless parallelism without data race bugs
- **Smaller Binary**: Reduced memory footprint and faster startup

## 🔧 Architecture Details

### Core Systems

1. **Object System**: Safe, trait-based object hierarchy with automatic serialization
2. **Shared Memory**: Lock-free shared memory management for distributed data
3. **Message Passing**: Async channels with priority queues and routing
4. **Module System**: Plugin architecture with hot reloading capabilities
5. **Task Execution**: DAG-based workflow execution with automatic dependency resolution

### Distributed Computing

- **MPI Integration**: Native Rust bindings for high-performance communication
- **Data Partitioning**: Automatic load balancing across compute nodes
- **Fault Tolerance**: Graceful handling of node failures
- **Scalability**: Designed for exascale computing systems

### Rendering Pipeline

- **Modern Graphics**: wgpu abstraction over Vulkan/Metal/D3D12
- **Compute Shaders**: GPU-accelerated visualization algorithms
- **Multi-threading**: Parallel rendering pipelines
- **VR Support**: Native integration with VR headsets

## 🤝 Contributing

We welcome contributions! Please see our [contributing guide](CONTRIBUTING.md) for details.

### Development Setup

```bash
# Clone repository
git clone https://github.com/vistle/vistle.git
cd vistle

# Install development dependencies
cargo install cargo-watch cargo-expand cargo-flamegraph

# Run tests
cargo test

# Run benchmarks
cargo bench
```
