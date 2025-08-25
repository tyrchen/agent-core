//! MCP (Model Context Protocol) server integration support.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for MCP servers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpServerConfig {
    /// Command-based MCP server (spawned as subprocess)
    Command {
        /// Server name/identifier
        name: String,

        /// Command to execute
        command: String,

        /// Command line arguments
        #[serde(default)]
        args: Vec<String>,

        /// Environment variables for the server process
        #[serde(default)]
        env: HashMap<String, String>,

        /// Working directory for the server process
        #[serde(default)]
        working_directory: Option<String>,

        /// Timeout for server startup in seconds
        #[serde(default = "default_timeout")]
        startup_timeout: u64,

        /// Whether to automatically restart the server if it crashes
        #[serde(default)]
        auto_restart: bool,
    },

    /// HTTP-based MCP server
    Http {
        /// Server name/identifier
        name: String,

        /// Base URL for the HTTP server
        url: String,

        /// Authentication headers
        #[serde(default)]
        headers: HashMap<String, String>,

        /// Connection timeout in seconds
        #[serde(default = "default_timeout")]
        timeout: u64,

        /// Whether to verify SSL certificates
        #[serde(default = "default_true")]
        verify_ssl: bool,

        /// Optional API key for authentication
        #[serde(default)]
        api_key: Option<String>,
    },
}

impl McpServerConfig {
    /// Create a new command-based MCP server configuration.
    pub fn command<S1, S2>(name: S1, command: S2) -> McpServerConfigBuilder<Command>
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        McpServerConfigBuilder::new_command(name.into(), command.into())
    }

    /// Create a new HTTP-based MCP server configuration.
    pub fn http<S1, S2>(name: S1, url: S2) -> McpServerConfigBuilder<Http>
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        McpServerConfigBuilder::new_http(name.into(), url.into())
    }

    /// Get the server name.
    pub fn name(&self) -> &str {
        match self {
            McpServerConfig::Command { name, .. } => name,
            McpServerConfig::Http { name, .. } => name,
        }
    }

    /// Check if this is a command-based server.
    pub fn is_command(&self) -> bool {
        matches!(self, McpServerConfig::Command { .. })
    }

    /// Check if this is an HTTP-based server.
    pub fn is_http(&self) -> bool {
        matches!(self, McpServerConfig::Http { .. })
    }
}

/// Builder for MCP server configurations with type safety.
pub struct McpServerConfigBuilder<T> {
    _marker: std::marker::PhantomData<T>,
    config: McpServerConfig,
}

/// Type marker for command-based servers
pub struct Command;

/// Type marker for HTTP-based servers
pub struct Http;

impl McpServerConfigBuilder<Command> {
    fn new_command(name: String, command: String) -> Self {
        Self {
            _marker: std::marker::PhantomData,
            config: McpServerConfig::Command {
                name,
                command,
                args: Vec::new(),
                env: HashMap::new(),
                working_directory: None,
                startup_timeout: default_timeout(),
                auto_restart: false,
            },
        }
    }

    /// Set command line arguments.
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        if let McpServerConfig::Command {
            args: server_args, ..
        } = &mut self.config
        {
            *server_args = args.into_iter().map(|s| s.into()).collect();
        }
        self
    }

    /// Add a single command line argument.
    pub fn arg<S: Into<String>>(mut self, arg: S) -> Self {
        if let McpServerConfig::Command { args, .. } = &mut self.config {
            args.push(arg.into());
        }
        self
    }

    /// Set environment variables.
    pub fn env<I, K, V>(mut self, env: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        if let McpServerConfig::Command {
            env: server_env, ..
        } = &mut self.config
        {
            *server_env = env.into_iter().map(|(k, v)| (k.into(), v.into())).collect();
        }
        self
    }

    /// Add a single environment variable.
    pub fn env_var<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        if let McpServerConfig::Command { env, .. } = &mut self.config {
            env.insert(key.into(), value.into());
        }
        self
    }

    /// Set the working directory.
    pub fn working_directory<S: Into<String>>(mut self, dir: S) -> Self {
        if let McpServerConfig::Command {
            working_directory, ..
        } = &mut self.config
        {
            *working_directory = Some(dir.into());
        }
        self
    }

    /// Set the startup timeout.
    pub fn startup_timeout(mut self, timeout: u64) -> Self {
        if let McpServerConfig::Command {
            startup_timeout, ..
        } = &mut self.config
        {
            *startup_timeout = timeout;
        }
        self
    }

    /// Enable automatic restart on crash.
    pub fn auto_restart(mut self, enable: bool) -> Self {
        if let McpServerConfig::Command { auto_restart, .. } = &mut self.config {
            *auto_restart = enable;
        }
        self
    }

    /// Build the configuration.
    pub fn build(self) -> McpServerConfig {
        self.config
    }
}

impl McpServerConfigBuilder<Http> {
    fn new_http(name: String, url: String) -> Self {
        Self {
            _marker: std::marker::PhantomData,
            config: McpServerConfig::Http {
                name,
                url,
                headers: HashMap::new(),
                timeout: default_timeout(),
                verify_ssl: true,
                api_key: None,
            },
        }
    }

    /// Set HTTP headers.
    pub fn headers<I, K, V>(mut self, headers: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        if let McpServerConfig::Http {
            headers: server_headers,
            ..
        } = &mut self.config
        {
            *server_headers = headers
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect();
        }
        self
    }

    /// Add a single HTTP header.
    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        if let McpServerConfig::Http { headers, .. } = &mut self.config {
            headers.insert(key.into(), value.into());
        }
        self
    }

    /// Set the connection timeout.
    pub fn timeout(mut self, timeout: u64) -> Self {
        if let McpServerConfig::Http {
            timeout: server_timeout,
            ..
        } = &mut self.config
        {
            *server_timeout = timeout;
        }
        self
    }

    /// Set whether to verify SSL certificates.
    pub fn verify_ssl(mut self, verify: bool) -> Self {
        if let McpServerConfig::Http { verify_ssl, .. } = &mut self.config {
            *verify_ssl = verify;
        }
        self
    }

    /// Set the API key for authentication.
    pub fn api_key<S: Into<String>>(mut self, key: S) -> Self {
        if let McpServerConfig::Http { api_key, .. } = &mut self.config {
            *api_key = Some(key.into());
        }
        self
    }

    /// Set Authorization Bearer token header.
    pub fn bearer_token<S: Into<String>>(self, token: S) -> Self {
        self.header("Authorization", format!("Bearer {}", token.into()))
    }

    /// Build the configuration.
    pub fn build(self) -> McpServerConfig {
        self.config
    }
}

/// MCP server connection status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum McpServerStatus {
    /// Server is not started
    NotStarted,

    /// Server is starting up
    Starting,

    /// Server is connected and ready
    Connected,

    /// Server connection is lost but may recover
    Disconnected,

    /// Server has failed and cannot recover
    Failed,

    /// Server is being shut down
    ShuttingDown,

    /// Server has been shut down
    Stopped,
}

impl std::fmt::Display for McpServerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            McpServerStatus::NotStarted => write!(f, "Not Started"),
            McpServerStatus::Starting => write!(f, "Starting"),
            McpServerStatus::Connected => write!(f, "Connected"),
            McpServerStatus::Disconnected => write!(f, "Disconnected"),
            McpServerStatus::Failed => write!(f, "Failed"),
            McpServerStatus::ShuttingDown => write!(f, "Shutting Down"),
            McpServerStatus::Stopped => write!(f, "Stopped"),
        }
    }
}

/// Information about an MCP server's runtime state.
#[derive(Debug, Clone)]
pub struct McpServerInfo {
    /// Server configuration
    pub config: McpServerConfig,

    /// Current connection status
    pub status: McpServerStatus,

    /// Last error message if any
    pub last_error: Option<String>,

    /// Number of connection attempts
    pub connection_attempts: u32,

    /// Server uptime duration
    pub uptime: Option<std::time::Duration>,

    /// When the server was last connected
    pub last_connected: Option<std::time::SystemTime>,

    /// Available tools provided by this server
    pub available_tools: Vec<String>,

    /// Available resources provided by this server
    pub available_resources: Vec<String>,

    /// Server capabilities
    pub capabilities: HashMap<String, serde_json::Value>,
}

impl McpServerInfo {
    /// Create new server info from configuration.
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            status: McpServerStatus::NotStarted,
            last_error: None,
            connection_attempts: 0,
            uptime: None,
            last_connected: None,
            available_tools: Vec::new(),
            available_resources: Vec::new(),
            capabilities: HashMap::new(),
        }
    }

    /// Check if the server is currently operational.
    pub fn is_operational(&self) -> bool {
        self.status == McpServerStatus::Connected
    }

    /// Check if the server has failed permanently.
    pub fn is_failed(&self) -> bool {
        self.status == McpServerStatus::Failed
    }
}

// Default value functions
fn default_timeout() -> u64 {
    30
}

fn default_true() -> bool {
    true
}
