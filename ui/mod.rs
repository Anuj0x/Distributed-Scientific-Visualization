//! User interface system for workflow editing and visualization

use std::sync::Arc;

/// UI backend types
#[derive(Debug, Clone)]
pub enum UiBackend {
    Egui,
    Web,
    Native,
}

/// Main application window
pub struct Application {
    backend: UiBackend,
    title: String,
    size: (u32, u32),
}

impl Application {
    pub fn new(title: &str, size: (u32, u32)) -> Self {
        Self {
            backend: UiBackend::Egui,
            title: title.to_string(),
            size,
        }
    }

    pub fn with_backend(mut self, backend: UiBackend) -> Self {
        self.backend = backend;
        self
    }

    pub async fn run<F>(self, update_fn: F) -> Result<(), crate::Error>
    where
        F: FnMut(&mut UiContext) + 'static,
    {
        match self.backend {
            UiBackend::Egui => {
                self.run_egui(update_fn).await
            }
            _ => Err(crate::Error::Config("Unsupported UI backend".to_string())),
        }
    }

    async fn run_egui<F>(self, mut update_fn: F) -> Result<(), crate::Error>
    where
        F: FnMut(&mut UiContext) + 'static,
    {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size(self.size)
                .with_title(&self.title),
            ..Default::default()
        };

        eframe::run_simple_native(&self.title, options, move |ctx, _frame| {
            let mut ui_ctx = UiContext::new(ctx);
            update_fn(&mut ui_ctx);
        })
        .map_err(|e| crate::Error::Config(format!("UI error: {}", e)))?;

        Ok(())
    }
}

/// UI context for drawing operations
pub struct UiContext<'a> {
    ctx: &'a egui::Context,
    current_panel: Option<String>,
}

impl<'a> UiContext<'a> {
    pub fn new(ctx: &'a egui::Context) -> Self {
        Self {
            ctx,
            current_panel: None,
        }
    }

    pub fn begin_panel(&mut self, name: &str) {
        self.current_panel = Some(name.to_string());
    }

    pub fn end_panel(&mut self) {
        self.current_panel = None;
    }

    pub fn label(&mut self, text: &str) {
        if let Some(panel) = &self.current_panel {
            egui::Window::new(panel).show(self.ctx, |ui| {
                ui.label(text);
            });
        } else {
            egui::CentralPanel::default().show(self.ctx, |ui| {
                ui.label(text);
            });
        }
    }

    pub fn button(&mut self, text: &str) -> bool {
        let mut clicked = false;

        if let Some(panel) = &self.current_panel {
            egui::Window::new(panel).show(self.ctx, |ui| {
                clicked = ui.button(text).clicked();
            });
        } else {
            egui::CentralPanel::default().show(self.ctx, |ui| {
                clicked = ui.button(text).clicked();
            });
        }

        clicked
    }

    pub fn slider(&mut self, text: &str, value: &mut f32, range: std::ops::RangeInclusive<f32>) {
        if let Some(panel) = &self.current_panel {
            egui::Window::new(panel).show(self.ctx, |ui| {
                ui.add(egui::Slider::new(value, range).text(text));
            });
        } else {
            egui::CentralPanel::default().show(self.ctx, |ui| {
                ui.add(egui::Slider::new(value, range).text(text));
            });
        }
    }

    pub fn checkbox(&mut self, text: &str, checked: &mut bool) {
        if let Some(panel) = &self.current_panel {
            egui::Window::new(panel).show(self.ctx, |ui| {
                ui.checkbox(checked, text);
            });
        } else {
            egui::CentralPanel::default().show(self.ctx, |ui| {
                ui.checkbox(checked, text);
            });
        }
    }

    pub fn text_input(&mut self, label: &str, text: &mut String) {
        if let Some(panel) = &self.current_panel {
            egui::Window::new(panel).show(self.ctx, |ui| {
                ui.text_edit_singleline(text).labelled_by(ui.label(label).id);
            });
        } else {
            egui::CentralPanel::default().show(self.ctx, |ui| {
                ui.text_edit_singleline(text).labelled_by(ui.label(label).id);
            });
        }
    }

    pub fn separator(&mut self) {
        if let Some(panel) = &self.current_panel {
            egui::Window::new(panel).show(self.ctx, |ui| {
                ui.separator();
            });
        } else {
            egui::CentralPanel::default().show(self.ctx, |ui| {
                ui.separator();
            });
        }
    }

    pub fn heading(&mut self, text: &str) {
        if let Some(panel) = &self.current_panel {
            egui::Window::new(panel).show(self.ctx, |ui| {
                ui.heading(text);
            });
        } else {
            egui::CentralPanel::default().show(self.ctx, |ui| {
                ui.heading(text);
            });
        }
    }
}

/// Workflow editor for visual programming
pub struct WorkflowEditor {
    workflows: Vec<WorkflowNode>,
    connections: Vec<Connection>,
    selected_node: Option<usize>,
    drag_offset: Option<egui::Vec2>,
}

impl WorkflowEditor {
    pub fn new() -> Self {
        Self {
            workflows: Vec::new(),
            connections: Vec::new(),
            selected_node: None,
            drag_offset: None,
        }
    }

    pub fn add_node(&mut self, node: WorkflowNode) {
        self.workflows.push(node);
    }

    pub fn draw(&mut self, ui: &mut UiContext) {
        // Draw workflow nodes and connections
        for (i, node) in self.workflows.iter_mut().enumerate() {
            self.draw_node(ui, i, node);
        }

        for connection in &self.connections {
            self.draw_connection(ui, connection);
        }
    }

    fn draw_node(&mut self, ui: &mut UiContext, index: usize, node: &mut WorkflowNode) {
        let node_rect = egui::Rect::from_min_size(
            node.position,
            egui::vec2(120.0, 80.0),
        );

        // Node background
        ui.ctx.request_repaint();

        // Handle interaction
        let response = ui.ctx.input(|i| {
            let pointer_pos = i.pointer.hover_pos().unwrap_or_default();

            if node_rect.contains(pointer_pos) {
                if i.pointer.primary_down() && self.selected_node.is_none() {
                    self.selected_node = Some(index);
                    self.drag_offset = Some(pointer_pos - node.position);
                }
            }

            if i.pointer.primary_released() {
                self.selected_node = None;
                self.drag_offset = None;
            }

            if let (Some(selected), Some(offset)) = (self.selected_node, self.drag_offset) {
                if selected == index && i.pointer.is_moving() {
                    if let Some(current_pos) = i.pointer.hover_pos() {
                        node.position = current_pos - offset;
                    }
                }
            }
        });

        // Draw node content
        egui::Window::new(&node.title)
            .fixed_pos(node.position)
            .fixed_size(egui::vec2(120.0, 80.0))
            .show(ui.ctx, |ui_window| {
                ui_window.label(&node.description);
                for port in &node.inputs {
                    ui_window.label(format!("→ {}", port));
                }
                for port in &node.outputs {
                    ui_window.label(format!("← {}", port));
                }
            });
    }

    fn draw_connection(&self, ui: &mut UiContext, connection: &Connection) {
        // Draw connection lines between nodes
        // Implementation would draw bezier curves between ports
    }
}

/// Node in workflow editor
#[derive(Debug, Clone)]
pub struct WorkflowNode {
    pub title: String,
    pub description: String,
    pub position: egui::Pos2,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub module_type: String,
}

impl WorkflowNode {
    pub fn new(title: &str, description: &str, module_type: &str) -> Self {
        Self {
            title: title.to_string(),
            description: description.to_string(),
            position: egui::Pos2::ZERO,
            inputs: Vec::new(),
            outputs: Vec::new(),
            module_type: module_type.to_string(),
        }
    }

    pub fn with_position(mut self, pos: egui::Pos2) -> Self {
        self.position = pos;
        self
    }

    pub fn add_input(mut self, name: &str) -> Self {
        self.inputs.push(name.to_string());
        self
    }

    pub fn add_output(mut self, name: &str) -> Self {
        self.outputs.push(name.to_string());
        self
    }
}

/// Connection between workflow nodes
#[derive(Debug, Clone)]
pub struct Connection {
    pub from_node: usize,
    pub from_port: String,
    pub to_node: usize,
    pub to_port: String,
}

/// Status display for workflow execution
pub struct StatusDisplay {
    messages: Vec<(String, StatusLevel)>,
    max_messages: usize,
}

impl StatusDisplay {
    pub fn new(max_messages: usize) -> Self {
        Self {
            messages: Vec::new(),
            max_messages,
        }
    }

    pub fn add_message(&mut self, message: String, level: StatusLevel) {
        self.messages.push((message, level));
        if self.messages.len() > self.max_messages {
            self.messages.remove(0);
        }
    }

    pub fn draw(&self, ui: &mut UiContext) {
        ui.begin_panel("Status");

        for (message, level) in &self.messages {
            let color = match level {
                StatusLevel::Info => egui::Color32::BLUE,
                StatusLevel::Warning => egui::Color32::YELLOW,
                StatusLevel::Error => egui::Color32::RED,
                StatusLevel::Success => egui::Color32::GREEN,
            };

            ui.ctx.request_repaint();
            // In real implementation, would style the text with color
            ui.label(message);
        }

        ui.end_panel();
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    Warning,
    Error,
    Success,
}

/// Progress bar for long-running operations
pub struct ProgressBar {
    progress: f32,
    label: String,
}

impl ProgressBar {
    pub fn new(label: &str) -> Self {
        Self {
            progress: 0.0,
            label: label.to_string(),
        }
    }

    pub fn set_progress(&mut self, progress: f32) {
        self.progress = progress.clamp(0.0, 1.0);
    }

    pub fn draw(&self, ui: &mut UiContext) {
        ui.begin_panel(&self.label);

        // Draw progress bar
        ui.ctx.request_repaint();
        // In real implementation, would draw a proper progress bar

        ui.end_panel();
    }
}
