/// Query bridge visualization types
use serde::{Deserialize, Serialize};
use std::fmt;

/// Status of a query execution step
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StepStatus {
    /// Step is pending execution
    Pending,
    /// Step is currently running
    Running,
    /// Step completed successfully
    Completed,
    /// Step encountered an error
    Error(String),
}

impl fmt::Display for StepStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StepStatus::Pending => write!(f, "Pending"),
            StepStatus::Running => write!(f, "Running"),
            StepStatus::Completed => write!(f, "Completed"),
            StepStatus::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}

/// Single step in a query execution timeline
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryStep {
    /// Name of the step
    pub name: String,
    /// Optional description of what this step does
    pub description: Option<String>,
    /// Log messages from step execution
    pub logs: Vec<String>,
    /// Optional input data to the step
    pub inputs: Option<serde_json::Value>,
    /// Optional output data from the step
    pub outputs: Option<serde_json::Value>,
}

impl QueryStep {
    /// Create a new query step
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            logs: Vec::new(),
            inputs: None,
            outputs: None,
        }
    }

    /// Set the description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add a log message
    pub fn with_log(mut self, log: impl Into<String>) -> Self {
        self.logs.push(log.into());
        self
    }

    /// Set the inputs
    pub fn with_inputs(mut self, inputs: serde_json::Value) -> Self {
        self.inputs = Some(inputs);
        self
    }

    /// Set the outputs
    pub fn with_outputs(mut self, outputs: serde_json::Value) -> Self {
        self.outputs = Some(outputs);
        self
    }
}

/// A single tool invocation during query execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolInvocation {
    /// Name of the tool being called
    pub tool_name: String,
    /// Arguments passed to the tool
    pub arguments: serde_json::Value,
    /// Result returned from the tool
    pub result: String,
    /// Time taken to execute in milliseconds
    pub execution_ms: u32,
}

impl ToolInvocation {
    /// Create a new tool invocation
    pub fn new(
        tool_name: impl Into<String>,
        arguments: serde_json::Value,
        result: impl Into<String>,
        execution_ms: u32,
    ) -> Self {
        Self {
            tool_name: tool_name.into(),
            arguments,
            result: result.into(),
            execution_ms,
        }
    }
}

/// A state transition in the query execution state machine
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateTransition {
    /// Name of the state we're coming from
    pub from_state: String,
    /// Name of the state we're transitioning to
    pub to_state: String,
    /// Timestamp of the transition (ISO 8601 format)
    pub timestamp: String,
    /// Event that triggered this transition
    pub event: String,
}

impl StateTransition {
    /// Create a new state transition
    pub fn new(
        from_state: impl Into<String>,
        to_state: impl Into<String>,
        timestamp: impl Into<String>,
        event: impl Into<String>,
    ) -> Self {
        Self {
            from_state: from_state.into(),
            to_state: to_state.into(),
            timestamp: timestamp.into(),
            event: event.into(),
        }
    }
}
