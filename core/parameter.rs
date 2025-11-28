//! Parameter system for module configuration

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Parameter value container
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

/// Parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub description: String,
    pub value: ParameterValue,
    pub param_type: ParameterType,
    pub min_value: Option<ParameterValue>,
    pub max_value: Option<ParameterValue>,
}

impl Parameter {
    pub fn new(name: &str, description: &str, value: ParameterValue) -> Self {
        let param_type = match &value {
            ParameterValue::Int(_) => ParameterType::Int { min: None, max: None },
            ParameterValue::Float(_) => ParameterType::Float { min: None, max: None },
            ParameterValue::String(_) => ParameterType::String,
            ParameterValue::Bool(_) => ParameterType::Bool,
            ParameterValue::VecInt(_) => ParameterType::VectorInt { min: None, max: None },
            ParameterValue::VecFloat(_) => ParameterType::VectorFloat { min: None, max: None },
            ParameterValue::VecString(_) => ParameterType::VectorString,
        };

        Self {
            name: name.to_string(),
            description: description.to_string(),
            value,
            param_type,
            min_value: None,
            max_value: None,
        }
    }
}

/// Parameter type information
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

/// Collection of parameters for a module
#[derive(Debug, Clone, Default)]
pub struct ParameterSet {
    parameters: HashMap<String, Parameter>,
}

impl ParameterSet {
    pub fn new() -> Self {
        Self {
            parameters: HashMap::new(),
        }
    }

    pub fn add(&mut self, param: Parameter) {
        self.parameters.insert(param.name.clone(), param);
    }

    pub fn get(&self, name: &str) -> Option<&Parameter> {
        self.parameters.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Parameter> {
        self.parameters.get_mut(name)
    }

    pub fn set_value(&mut self, name: &str, value: ParameterValue) -> Result<(), String> {
        if let Some(param) = self.parameters.get_mut(name) {
            // Basic type validation
            match (&param.param_type, &value) {
                (ParameterType::Int { .. }, ParameterValue::Int(_)) => {}
                (ParameterType::Float { .. }, ParameterValue::Float(_)) => {}
                (ParameterType::String, ParameterValue::String(_)) => {}
                (ParameterType::Bool, ParameterValue::Bool(_)) => {}
                (ParameterType::VectorInt { .. }, ParameterValue::VecInt(_)) => {}
                (ParameterType::VectorFloat { .. }, ParameterValue::VecFloat(_)) => {}
                (ParameterType::VectorString, ParameterValue::VecString(_)) => {}
                _ => return Err(format!("Type mismatch for parameter {}", name)),
            }
            param.value = value;
            Ok(())
        } else {
            Err(format!("Parameter {} not found", name))
        }
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<String, Parameter> {
        self.parameters.iter()
    }

    pub fn names(&self) -> Vec<String> {
        self.parameters.keys().cloned().collect()
    }
}

/// Port definition for module connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Port {
    pub name: String,
    pub description: String,
    pub port_type: PortType,
    pub optional: bool,
}

impl Port {
    pub fn new_input(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            port_type: PortType::Input,
            optional: false,
        }
    }

    pub fn new_output(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            port_type: PortType::Output,
            optional: false,
        }
    }

    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortType {
    Input,
    Output,
}

/// Collection of ports for a module
#[derive(Debug, Clone, Default)]
pub struct PortSet {
    ports: HashMap<String, Port>,
}

impl PortSet {
    pub fn new() -> Self {
        Self {
            ports: HashMap::new(),
        }
    }

    pub fn add(&mut self, port: Port) {
        self.ports.insert(port.name.clone(), port);
    }

    pub fn get(&self, name: &str) -> Option<&Port> {
        self.ports.get(name)
    }

    pub fn inputs(&self) -> Vec<&Port> {
        self.ports.values().filter(|p| matches!(p.port_type, PortType::Input)).collect()
    }

    pub fn outputs(&self) -> Vec<&Port> {
        self.ports.values().filter(|p| matches!(p.port_type, PortType::Output)).collect()
    }

    pub fn names(&self) -> Vec<String> {
        self.ports.keys().cloned().collect()
    }
}
