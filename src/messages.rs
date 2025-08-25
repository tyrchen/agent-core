//! Message types for agent input and output communication.

use serde::{Deserialize, Serialize};

use crate::error::OutputError;

/// Input message from user to agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputMessage {
    /// The text message content
    pub message: String,

    /// Optional images attached to the message
    pub images: Vec<ImageInput>,
}

impl InputMessage {
    /// Create a new input message with text only.
    pub fn new<S: Into<String>>(message: S) -> Self {
        Self {
            message: message.into(),
            images: Vec::new(),
        }
    }

    /// Create a new input message with text and images.
    pub fn with_images<S: Into<String>>(message: S, images: Vec<ImageInput>) -> Self {
        Self {
            message: message.into(),
            images,
        }
    }

    /// Add an image to the message.
    pub fn add_image(mut self, image: ImageInput) -> Self {
        self.images.push(image);
        self
    }
}

impl<S: Into<String>> From<S> for InputMessage {
    fn from(message: S) -> Self {
        Self::new(message)
    }
}

/// Image input data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInput {
    /// Base64 encoded image data
    pub data: String,

    /// MIME type (e.g., "image/jpeg", "image/png")
    pub mime_type: String,

    /// Optional description or alt text for the image
    pub description: Option<String>,
}

impl ImageInput {
    /// Create a new image input.
    pub fn new<S1, S2>(data: S1, mime_type: S2) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        Self {
            data: data.into(),
            mime_type: mime_type.into(),
            description: None,
        }
    }

    /// Create a new image input with description.
    pub fn with_description<S1, S2, S3>(data: S1, mime_type: S2, description: S3) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
        S3: Into<String>,
    {
        Self {
            data: data.into(),
            mime_type: mime_type.into(),
            description: Some(description.into()),
        }
    }

    /// Set the description.
    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Output message from agent to user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputMessage {
    /// Unique identifier for the turn
    pub turn_id: u64,

    /// The output data payload
    pub data: OutputData,

    /// Timestamp when the message was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl OutputMessage {
    /// Create a new output message.
    pub fn new(turn_id: u64, data: OutputData) -> Self {
        Self {
            turn_id,
            data,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Output data types from the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputData {
    /// Turn/conversation start
    Start,

    /// Primary response content
    Primary { content: String },

    /// Streaming response fragment
    PrimaryDelta { content: String },

    /// Tool execution started
    ToolStart {
        tool_name: String,
        arguments: serde_json::Value,
    },

    /// Tool execution completed
    ToolComplete {
        tool_name: String,
        result: serde_json::Value,
    },

    /// Tool output stream
    ToolOutput { tool_name: String, output: String },

    /// Agent reasoning process
    Reasoning { content: String },

    /// Reasoning content delta
    ReasoningDelta { content: String },

    /// Todo list/plan update
    TodoUpdate { todos: Vec<crate::plan::TodoItem> },

    /// Turn completed successfully
    Completed,

    /// Error occurred
    Error { error: OutputError },
}

impl OutputData {
    /// Create a primary content message.
    pub fn primary<S: Into<String>>(content: S) -> Self {
        Self::Primary {
            content: content.into(),
        }
    }

    /// Create a primary delta message.
    pub fn primary_delta<S: Into<String>>(content: S) -> Self {
        Self::PrimaryDelta {
            content: content.into(),
        }
    }

    /// Create a tool start message.
    pub fn tool_start<S: Into<String>>(tool_name: S, arguments: serde_json::Value) -> Self {
        Self::ToolStart {
            tool_name: tool_name.into(),
            arguments,
        }
    }

    /// Create a tool complete message.
    pub fn tool_complete<S: Into<String>>(tool_name: S, result: serde_json::Value) -> Self {
        Self::ToolComplete {
            tool_name: tool_name.into(),
            result,
        }
    }

    /// Create a tool output message.
    pub fn tool_output<S1, S2>(tool_name: S1, output: S2) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        Self::ToolOutput {
            tool_name: tool_name.into(),
            output: output.into(),
        }
    }

    /// Create a reasoning message.
    pub fn reasoning<S: Into<String>>(content: S) -> Self {
        Self::Reasoning {
            content: content.into(),
        }
    }

    /// Create a reasoning delta message.
    pub fn reasoning_delta<S: Into<String>>(content: S) -> Self {
        Self::ReasoningDelta {
            content: content.into(),
        }
    }

    /// Create a todo update message.
    pub fn todo_update(todos: Vec<crate::plan::TodoItem>) -> Self {
        Self::TodoUpdate { todos }
    }

    /// Create an error message.
    pub fn error(error: OutputError) -> Self {
        Self::Error { error }
    }
}

impl std::fmt::Display for OutputMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.data {
            OutputData::Start => write!(f, "[Turn {}] Started", self.turn_id),
            OutputData::Primary { content } => write!(f, "{}", content),
            OutputData::PrimaryDelta { content } => write!(f, "{}", content),
            OutputData::ToolStart { tool_name, .. } => {
                write!(f, "[Tool] Starting {}", tool_name)
            }
            OutputData::ToolComplete { tool_name, .. } => {
                write!(f, "[Tool] Completed {}", tool_name)
            }
            OutputData::ToolOutput { tool_name, output } => {
                write!(f, "[{}] {}", tool_name, output)
            }
            OutputData::Reasoning { content } => write!(f, "[Reasoning] {}", content),
            OutputData::ReasoningDelta { content } => write!(f, "{}", content),
            OutputData::TodoUpdate { todos } => {
                write!(f, "[Plan] {} todos", todos.len())
            }
            OutputData::Completed => write!(f, "[Turn {}] Completed", self.turn_id),
            OutputData::Error { error } => write!(f, "[Error] {:?}", error),
        }
    }
}
