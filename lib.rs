//! # Vistle - Modern Distributed Scientific Visualization
//!
//! A high-performance, memory-safe distributed visualization system built in Rust.
//! This crate provides the core functionality for parallel scientific data processing
//! and visualization across multiple compute nodes.

pub mod core;
pub mod compute;
pub mod mpi;
pub mod render;
pub mod ui;
pub mod util;

pub use core::*;
pub use compute::*;
pub use mpi::*;
pub use render::*;
pub use ui::*;

/// Initialize the Vistle system
pub async fn init() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("Initializing Vistle v{}", env!("CARGO_PKG_VERSION"));
    Ok(())
}

/// Main error type for Vistle operations
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("MPI error: {0}")]
    Mpi(#[from] mpi::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("Shared memory error: {0}")]
    SharedMemory(String),

    #[error("Compute error: {0}")]
    Compute(String),

    #[error("Render error: {0}")]
    Render(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Module error: {0}")]
    Module(String),
}

pub type Result<T> = std::result::Result<T, Error>;
