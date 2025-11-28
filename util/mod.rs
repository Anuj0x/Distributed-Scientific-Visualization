//! Utility functions and macros

use std::collections::HashMap;

/// Collection of utility macros for Vistle development
pub mod macros {
    /// Helper macro for implementing common module patterns
    #[macro_export]
    macro_rules! vistle_module_base {
        ($name:ident) => {
            impl $name {
                pub fn base_setup(&mut self) {
                    // Common setup logic
                }
            }
        };
    }

    /// Macro for creating parameter builders
    #[macro_export]
    macro_rules! param_builder {
        ($($param:ident: $type:ty),*) => {
            pub struct ParamBuilder {
                $(pub $param: Option<$type>),*
            }

            impl ParamBuilder {
                pub fn new() -> Self {
                    Self {
                        $($param: None),*
                    }
                }

                $(
                    pub fn $param(mut self, value: $type) -> Self {
                        self.$param = Some(value);
                        self
                    }
                )*

                pub fn build(self) -> Result<($($type),*), String> {
                    Ok((
                        $(self.$param.ok_or_else(|| format!("Missing parameter: {}", stringify!($param)))?),*
                    ))
                }
            }
        };
    }

    /// Macro for timing code execution
    #[macro_export]
    macro_rules! time_execution {
        ($name:expr, $code:block) => {{
            let start = std::time::Instant::now();
            let result = $code;
            let duration = start.elapsed();
            tracing::info!("{} completed in {:?}", $name, duration);
            result
        }};
    }
}

/// Performance monitoring utilities
pub struct PerformanceMonitor {
    timings: HashMap<String, Vec<std::time::Duration>>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            timings: HashMap::new(),
        }
    }

    pub fn start_timer(&self, name: &str) -> Timer {
        Timer::new(name.to_string())
    }

    pub fn record_timing(&mut self, name: String, duration: std::time::Duration) {
        self.timings.entry(name).or_insert_with(Vec::new).push(duration);
    }

    pub fn get_average(&self, name: &str) -> Option<std::time::Duration> {
        self.timings.get(name).and_then(|durations| {
            if durations.is_empty() {
                None
            } else {
                let total: std::time::Duration = durations.iter().sum();
                Some(total / durations.len() as u32)
            }
        })
    }

    pub fn get_stats(&self, name: &str) -> Option<TimingStats> {
        self.timings.get(name).map(|durations| {
            if durations.is_empty() {
                return TimingStats {
                    count: 0,
                    average: std::time::Duration::ZERO,
                    min: std::time::Duration::ZERO,
                    max: std::time::Duration::ZERO,
                };
            }

            let count = durations.len();
            let total: std::time::Duration = durations.iter().sum();
            let average = total / count as u32;
            let min = durations.iter().min().unwrap().clone();
            let max = durations.iter().max().unwrap().clone();

            TimingStats { count, average, min, max }
        })
    }

    pub fn clear(&mut self) {
        self.timings.clear();
    }
}

pub struct Timer {
    name: String,
    start: std::time::Instant,
}

impl Timer {
    pub fn new(name: String) -> Self {
        Self {
            name,
            start: std::time::Instant::now(),
        }
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let duration = self.elapsed();
        tracing::debug!("Timer '{}' finished in {:?}", self.name, duration);
    }
}

#[derive(Debug, Clone)]
pub struct TimingStats {
    pub count: usize,
    pub average: std::time::Duration,
    pub min: std::time::Duration,
    pub max: std::time::Duration,
}

/// Memory usage tracking
pub struct MemoryTracker {
    initial_memory: usize,
    peak_memory: usize,
}

impl MemoryTracker {
    pub fn new() -> Self {
        let initial = get_current_memory_usage();
        Self {
            initial_memory: initial,
            peak_memory: initial,
        }
    }

    pub fn update(&mut self) {
        let current = get_current_memory_usage();
        if current > self.peak_memory {
            self.peak_memory = current;
        }
    }

    pub fn current_usage(&self) -> usize {
        get_current_memory_usage()
    }

    pub fn peak_usage(&self) -> usize {
        self.peak_memory
    }

    pub fn initial_usage(&self) -> usize {
        self.initial_memory
    }

    pub fn reset_peak(&mut self) {
        self.peak_memory = get_current_memory_usage();
    }
}

fn get_current_memory_usage() -> usize {
    // Platform-specific memory usage detection
    // This is a simplified implementation
    0 // Placeholder
}

/// Configuration utilities
pub mod config {
    use std::collections::HashMap;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SystemConfig {
        pub max_threads: usize,
        pub shared_memory_size: usize,
        pub enable_gpu: bool,
        pub log_level: String,
    }

    impl Default for SystemConfig {
        fn default() -> Self {
            Self {
                max_threads: num_cpus::get(),
                shared_memory_size: 1024 * 1024 * 1024, // 1GB
                enable_gpu: true,
                log_level: "info".to_string(),
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ModuleConfig {
        pub enabled_modules: Vec<String>,
        pub module_paths: Vec<String>,
        pub default_parameters: HashMap<String, HashMap<String, String>>,
    }

    impl Default for ModuleConfig {
        fn default() -> Self {
            Self {
                enabled_modules: vec![
                    "ReadData".to_string(),
                    "Filter".to_string(),
                    "Renderer".to_string(),
                ],
                module_paths: vec!["modules".to_string()],
                default_parameters: HashMap::new(),
            }
        }
    }
}

/// Math utilities for scientific computing
pub mod math {
    use ndarray::{Array1, Array2};

    /// Compute basic statistics for an array
    pub fn compute_stats(data: &Array1<f32>) -> ArrayStats {
        if data.is_empty() {
            return ArrayStats {
                min: 0.0,
                max: 0.0,
                mean: 0.0,
                std_dev: 0.0,
            };
        }

        let min = data.fold(f32::INFINITY, |a, &b| a.min(b));
        let max = data.fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let sum: f32 = data.sum();
        let mean = sum / data.len() as f32;

        let variance: f32 = data.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / data.len() as f32;
        let std_dev = variance.sqrt();

        ArrayStats { min, max, mean, std_dev }
    }

    #[derive(Debug, Clone)]
    pub struct ArrayStats {
        pub min: f32,
        pub max: f32,
        pub mean: f32,
        pub std_dev: f32,
    }

    /// Normalize array to [0, 1] range
    pub fn normalize(data: &mut Array1<f32>) {
        let stats = compute_stats(data);
        let range = stats.max - stats.min;

        if range > 0.0 {
            data.mapv_inplace(|x| (x - stats.min) / range);
        }
    }

    /// Clamp array values to range
    pub fn clamp(data: &mut Array1<f32>, min: f32, max: f32) {
        data.mapv_inplace(|x| x.clamp(min, max));
    }
}

/// File I/O utilities
pub mod io {
    use std::path::Path;
    use tokio::fs;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Read binary data from file
    pub async fn read_binary<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, crate::Error> {
        let mut file = fs::File::open(path).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;
        Ok(buffer)
    }

    /// Write binary data to file
    pub async fn write_binary<P: AsRef<Path>>(path: P, data: &[u8]) -> Result<(), crate::Error> {
        let mut file = fs::File::create(path).await?;
        file.write_all(data).await?;
        Ok(())
    }

    /// Read text from file
    pub async fn read_text<P: AsRef<Path>>(path: P) -> Result<String, crate::Error> {
        let content = fs::read_to_string(path).await?;
        Ok(content)
    }

    /// Write text to file
    pub async fn write_text<P: AsRef<Path>>(path: P, text: &str) -> Result<(), crate::Error> {
        fs::write(path, text).await?;
        Ok(())
    }
}
