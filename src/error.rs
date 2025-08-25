//! Error types for the agent-core library.

use thiserror::Error;

/// Result type alias for agent-core operations.
pub type Result<T> = std::result::Result<T, AgentError>;

/// Main error type for agent-core operations.
#[derive(Error, Debug)]
pub enum AgentError {
    /// Configuration error
    #[error("Configuration error: {message}")]
    Config { message: String },

    /// Codex core error
    #[error("Codex error: {0}")]
    Codex(#[from] codex_core::error::CodexErr),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Channel send error
    #[error("Channel send error: {message}")]
    ChannelSend { message: String },

    /// Channel receive error
    #[error("Channel receive error: {message}")]
    ChannelReceive { message: String },

    /// Agent execution error
    #[error("Agent execution error: {message}")]
    Execution { message: String },

    /// Tool execution error
    #[error("Tool execution error: {message}")]
    Tool { message: String },

    /// MCP server error
    #[error("MCP server error: {message}")]
    Mcp { message: String },

    /// Generic error
    #[error("Agent error: {message}")]
    Generic { message: String },
}

/// Output error types that can be sent via OutputData::Error
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum OutputError {
    /// Tool execution failed
    ToolExecutionFailed { tool_name: String, error: String },

    /// Model request failed
    ModelRequestFailed { error: String },

    /// Configuration error
    ConfigurationError { error: String },

    /// Sandbox violation
    SandboxViolation { command: String, reason: String },

    /// Permission denied
    PermissionDenied { operation: String, reason: String },

    /// Resource limit exceeded
    ResourceLimitExceeded { resource: String, limit: String },

    /// General error
    General { message: String },
}

impl From<&str> for AgentError {
    fn from(message: &str) -> Self {
        AgentError::Generic {
            message: message.to_string(),
        }
    }
}

impl From<String> for AgentError {
    fn from(message: String) -> Self {
        AgentError::Generic { message }
    }
}

impl<T> From<async_channel::SendError<T>> for AgentError {
    fn from(err: async_channel::SendError<T>) -> Self {
        AgentError::ChannelSend {
            message: format!("Failed to send message: {}", err),
        }
    }
}

impl From<async_channel::RecvError> for AgentError {
    fn from(err: async_channel::RecvError) -> Self {
        AgentError::ChannelReceive {
            message: format!("Failed to receive message: {}", err),
        }
    }
}
