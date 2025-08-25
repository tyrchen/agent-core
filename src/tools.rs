//! Tool support for AI agents including built-in and custom tools.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::Result;

/// Configuration for different types of tools available to the agent.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolConfig {
    /// Shell command execution with configurable network access
    Bash {
        /// Whether to allow network access during command execution
        allow_network: bool,

        /// Additional environment variables to set
        #[serde(default)]
        environment: HashMap<String, String>,

        /// Working directory for command execution (relative to agent's working directory)
        #[serde(default)]
        working_directory: Option<String>,

        /// Timeout for command execution in seconds
        #[serde(default)]
        timeout: Option<u64>,
    },

    /// Web search capability
    WebSearch {
        /// Maximum number of search results to return
        #[serde(default = "default_search_results")]
        max_results: usize,

        /// Search engine to use (if configurable)
        #[serde(default)]
        search_engine: Option<String>,

        /// Additional search parameters
        #[serde(default)]
        parameters: HashMap<String, serde_json::Value>,
    },

    /// File reading capability
    FileRead {
        /// Maximum file size to read in bytes
        #[serde(default = "default_max_file_size")]
        max_file_size: usize,

        /// Allowed file extensions (empty means all allowed)
        #[serde(default)]
        allowed_extensions: Vec<String>,

        /// Whether to read binary files
        #[serde(default)]
        allow_binary: bool,
    },

    /// File writing capability
    FileWrite {
        /// Maximum file size to write in bytes
        #[serde(default = "default_max_file_size")]
        max_file_size: usize,

        /// Allowed file extensions (empty means all allowed)
        #[serde(default)]
        allowed_extensions: Vec<String>,

        /// Whether to allow overwriting existing files
        #[serde(default = "default_true")]
        allow_overwrite: bool,

        /// Whether to create directories if they don't exist
        #[serde(default = "default_true")]
        create_directories: bool,
    },

    /// Patch application tool for code modifications
    ApplyPatch {
        /// Maximum patch size in bytes
        #[serde(default = "default_max_patch_size")]
        max_patch_size: usize,

        /// Whether to create backup files before applying patches
        #[serde(default = "default_true")]
        create_backup: bool,

        /// Whether to validate patch syntax before applying
        #[serde(default = "default_true")]
        validate_syntax: bool,
    },

    /// Custom tool with user-defined behavior
    Custom {
        /// Tool name identifier
        name: String,

        /// Human-readable description of what the tool does
        description: String,

        /// JSON Schema for tool parameters
        parameters: serde_json::Value,

        /// The actual tool handler
        #[serde(skip)]
        handler: Option<Box<dyn CustomToolHandler>>,
    },
}

impl ToolConfig {
    /// Create a bash tool configuration with default settings.
    pub fn bash() -> Self {
        Self::Bash {
            allow_network: false,
            environment: HashMap::new(),
            working_directory: None,
            timeout: None,
        }
    }

    /// Create a bash tool with network access enabled.
    pub fn bash_with_network() -> Self {
        Self::Bash {
            allow_network: true,
            environment: HashMap::new(),
            working_directory: None,
            timeout: None,
        }
    }

    /// Create a web search tool with default settings.
    pub fn web_search() -> Self {
        Self::WebSearch {
            max_results: default_search_results(),
            search_engine: None,
            parameters: HashMap::new(),
        }
    }

    /// Create a file read tool with default settings.
    pub fn file_read() -> Self {
        Self::FileRead {
            max_file_size: default_max_file_size(),
            allowed_extensions: Vec::new(),
            allow_binary: false,
        }
    }

    /// Create a file write tool with default settings.
    pub fn file_write() -> Self {
        Self::FileWrite {
            max_file_size: default_max_file_size(),
            allowed_extensions: Vec::new(),
            allow_overwrite: true,
            create_directories: true,
        }
    }

    /// Create an apply patch tool with default settings.
    pub fn apply_patch() -> Self {
        Self::ApplyPatch {
            max_patch_size: default_max_patch_size(),
            create_backup: true,
            validate_syntax: true,
        }
    }

    /// Create a custom tool configuration.
    pub fn custom<S1, S2>(
        name: S1,
        description: S2,
        parameters: serde_json::Value,
        handler: Box<dyn CustomToolHandler>,
    ) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        Self::Custom {
            name: name.into(),
            description: description.into(),
            parameters,
            handler: Some(handler),
        }
    }

    /// Get the tool name/identifier.
    pub fn name(&self) -> &str {
        match self {
            ToolConfig::Bash { .. } => "bash",
            ToolConfig::WebSearch { .. } => "web_search",
            ToolConfig::FileRead { .. } => "file_read",
            ToolConfig::FileWrite { .. } => "file_write",
            ToolConfig::ApplyPatch { .. } => "apply_patch",
            ToolConfig::Custom { name, .. } => name,
        }
    }

    /// Get a human-readable description of the tool.
    pub fn description(&self) -> String {
        match self {
            ToolConfig::Bash { allow_network, .. } => {
                if *allow_network {
                    "Execute shell commands with network access".to_string()
                } else {
                    "Execute shell commands without network access".to_string()
                }
            }
            ToolConfig::WebSearch { .. } => "Search the web for information".to_string(),
            ToolConfig::FileRead { .. } => "Read files from the filesystem".to_string(),
            ToolConfig::FileWrite { .. } => "Write files to the filesystem".to_string(),
            ToolConfig::ApplyPatch { .. } => "Apply code patches to files".to_string(),
            ToolConfig::Custom { description, .. } => description.clone(),
        }
    }
}

/// Trait for implementing custom tools.
pub trait CustomToolHandler: Send + Sync {
    /// Execute the custom tool with the given parameters.
    fn execute(
        &self,
        parameters: serde_json::Value,
        context: &ToolExecutionContext,
    ) -> Result<ToolExecutionResult>;

    /// Get the tool's JSON Schema for parameter validation.
    fn parameter_schema(&self) -> serde_json::Value;

    /// Get a human-readable description of what this tool does.
    fn description(&self) -> String;
}

/// Context provided to tools during execution.
#[derive(Debug)]
pub struct ToolExecutionContext {
    /// Current working directory
    pub working_directory: std::path::PathBuf,

    /// Environment variables available to the tool
    pub environment: HashMap<String, String>,

    /// Agent configuration
    pub agent_config: crate::config::AgentConfig,

    /// Current turn ID
    pub turn_id: u64,

    /// Tool execution timeout
    pub timeout: Option<std::time::Duration>,
}

/// Result of tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    /// Whether the tool execution was successful
    pub success: bool,

    /// Output text from the tool
    pub output: String,

    /// Optional structured data returned by the tool
    pub data: Option<serde_json::Value>,

    /// Exit code or error code (0 for success)
    pub exit_code: Option<i32>,

    /// Additional metadata about the execution
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ToolExecutionResult {
    /// Create a successful tool result.
    pub fn success<S: Into<String>>(output: S) -> Self {
        Self {
            success: true,
            output: output.into(),
            data: None,
            exit_code: Some(0),
            metadata: HashMap::new(),
        }
    }

    /// Create a successful tool result with data.
    pub fn success_with_data<S: Into<String>>(output: S, data: serde_json::Value) -> Self {
        Self {
            success: true,
            output: output.into(),
            data: Some(data),
            exit_code: Some(0),
            metadata: HashMap::new(),
        }
    }

    /// Create a failed tool result.
    pub fn failure<S: Into<String>>(output: S, exit_code: i32) -> Self {
        Self {
            success: false,
            output: output.into(),
            data: None,
            exit_code: Some(exit_code),
            metadata: HashMap::new(),
        }
    }

    /// Create an error tool result.
    pub fn error<S: Into<String>>(error_message: S) -> Self {
        Self {
            success: false,
            output: error_message.into(),
            data: None,
            exit_code: Some(-1),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the result.
    pub fn with_metadata<K, V>(mut self, key: K, value: V) -> Result<Self>
    where
        K: Into<String>,
        V: Serialize,
    {
        let json_value = serde_json::to_value(value)?;
        self.metadata.insert(key.into(), json_value);
        Ok(self)
    }
}

// Default value functions for serde defaults
fn default_search_results() -> usize {
    10
}

fn default_max_file_size() -> usize {
    10 * 1024 * 1024 // 10 MB
}

fn default_max_patch_size() -> usize {
    1024 * 1024 // 1 MB
}

fn default_true() -> bool {
    true
}

impl Clone for ToolConfig {
    fn clone(&self) -> Self {
        match self {
            Self::Bash {
                allow_network,
                environment,
                working_directory,
                timeout,
            } => Self::Bash {
                allow_network: *allow_network,
                environment: environment.clone(),
                working_directory: working_directory.clone(),
                timeout: *timeout,
            },
            Self::WebSearch {
                max_results,
                search_engine,
                parameters,
            } => Self::WebSearch {
                max_results: *max_results,
                search_engine: search_engine.clone(),
                parameters: parameters.clone(),
            },
            Self::FileRead {
                max_file_size,
                allowed_extensions,
                allow_binary,
            } => Self::FileRead {
                max_file_size: *max_file_size,
                allowed_extensions: allowed_extensions.clone(),
                allow_binary: *allow_binary,
            },
            Self::FileWrite {
                max_file_size,
                allowed_extensions,
                allow_overwrite,
                create_directories,
            } => Self::FileWrite {
                max_file_size: *max_file_size,
                allowed_extensions: allowed_extensions.clone(),
                allow_overwrite: *allow_overwrite,
                create_directories: *create_directories,
            },
            Self::ApplyPatch {
                max_patch_size,
                create_backup,
                validate_syntax,
            } => Self::ApplyPatch {
                max_patch_size: *max_patch_size,
                create_backup: *create_backup,
                validate_syntax: *validate_syntax,
            },
            Self::Custom {
                name,
                description,
                parameters,
                ..
            } => {
                // Note: handler is not cloned, as trait objects can't be cloned in general
                Self::Custom {
                    name: name.clone(),
                    description: description.clone(),
                    parameters: parameters.clone(),
                    handler: None,
                }
            }
        }
    }
}

impl std::fmt::Debug for dyn CustomToolHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CustomToolHandler {{ description: {} }}",
            self.description()
        )
    }
}
