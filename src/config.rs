//! Configuration system for AI agents with builder pattern support.

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

use codex_protocol::protocol::{AskForApproval, SandboxPolicy};
use serde::Serialize;

use crate::error::{AgentError, Result};
use crate::mcp::McpServerConfig;
use crate::tools::ToolConfig;

/// Main configuration for an AI agent.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Model identifier (e.g., "gpt-4", "gpt-5-mini")
    model: String,

    /// API key for the model provider
    api_key: Option<String>,

    /// System prompt/instructions for the agent
    system_prompt: Option<String>,

    /// Sandbox policy for tool execution
    sandbox_policy: SandboxPolicy,

    /// Approval policy for command execution
    approval_policy: AskForApproval,

    /// Maximum number of conversation turns
    max_turns: Option<u32>,

    /// Working directory for agent operations
    working_directory: PathBuf,

    /// Enabled tools
    tools: Vec<ToolConfig>,

    /// MCP server configurations
    mcp_servers: Vec<McpServerConfig>,

    /// Environment variables for the agent
    environment: HashMap<String, String>,

    /// Additional configuration options
    additional_config: HashMap<String, serde_json::Value>,
}

impl AgentConfig {
    /// Create a new configuration builder.
    pub fn builder() -> AgentConfigBuilder {
        AgentConfigBuilder::default()
    }

    /// Get the model identifier.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get the API key.
    pub fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    /// Get the system prompt.
    pub fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    /// Get the sandbox policy.
    pub fn sandbox_policy(&self) -> &SandboxPolicy {
        &self.sandbox_policy
    }

    /// Get the approval policy.
    pub fn approval_policy(&self) -> &AskForApproval {
        &self.approval_policy
    }

    /// Get the maximum number of turns.
    pub fn max_turns(&self) -> Option<u32> {
        self.max_turns
    }

    /// Get the working directory.
    pub fn working_directory(&self) -> &PathBuf {
        &self.working_directory
    }

    /// Get the enabled tools.
    pub fn tools(&self) -> &[ToolConfig] {
        &self.tools
    }

    /// Get the MCP server configurations.
    pub fn mcp_servers(&self) -> &[McpServerConfig] {
        &self.mcp_servers
    }

    /// Get environment variables.
    pub fn environment(&self) -> &HashMap<String, String> {
        &self.environment
    }

    /// Get additional configuration.
    pub fn additional_config(&self) -> &HashMap<String, serde_json::Value> {
        &self.additional_config
    }
}

/// Builder for AgentConfig with a fluent interface.
#[derive(Debug, Default)]
pub struct AgentConfigBuilder {
    model: Option<String>,
    api_key: Option<String>,
    system_prompt: Option<String>,
    sandbox_policy: Option<SandboxPolicy>,
    approval_policy: Option<AskForApproval>,
    max_turns: Option<u32>,
    working_directory: Option<PathBuf>,
    tools: Vec<ToolConfig>,
    mcp_servers: Vec<McpServerConfig>,
    environment: HashMap<String, String>,
    additional_config: HashMap<String, serde_json::Value>,
}

impl AgentConfigBuilder {
    /// Set the model identifier.
    pub fn model<S: Into<String>>(mut self, model: S) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the API key directly.
    pub fn api_key<S: Into<String>>(mut self, api_key: S) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set the API key from an environment variable.
    pub fn api_key_env<S: AsRef<str>>(mut self, env_var: S) -> Result<Self> {
        let key = env::var(env_var.as_ref()).map_err(|_| AgentError::Config {
            message: format!("Environment variable {} not found", env_var.as_ref()),
        })?;
        self.api_key = Some(key);
        Ok(self)
    }

    /// Set the system prompt.
    pub fn system_prompt<S: Into<String>>(mut self, prompt: S) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set the sandbox policy.
    pub fn sandbox_policy(mut self, policy: SandboxPolicy) -> Self {
        self.sandbox_policy = Some(policy);
        self
    }

    /// Set the approval policy.
    pub fn approval_policy(mut self, policy: AskForApproval) -> Self {
        self.approval_policy = Some(policy);
        self
    }

    /// Set the maximum number of conversation turns.
    pub fn max_turns(mut self, max_turns: u32) -> Self {
        self.max_turns = Some(max_turns);
        self
    }

    /// Set the working directory.
    pub fn working_directory<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.working_directory = Some(path.into());
        self
    }

    /// Add a tool to the configuration.
    pub fn tool(mut self, tool: ToolConfig) -> Self {
        self.tools.push(tool);
        self
    }

    /// Add multiple tools to the configuration.
    pub fn tools<I>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = ToolConfig>,
    {
        self.tools.extend(tools);
        self
    }

    /// Add an MCP server configuration.
    pub fn mcp_server(mut self, server: McpServerConfig) -> Self {
        self.mcp_servers.push(server);
        self
    }

    /// Add multiple MCP server configurations.
    pub fn mcp_servers<I>(mut self, servers: I) -> Self
    where
        I: IntoIterator<Item = McpServerConfig>,
    {
        self.mcp_servers.extend(servers);
        self
    }

    /// Set an environment variable.
    pub fn env<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.environment.insert(key.into(), value.into());
        self
    }

    /// Set multiple environment variables.
    pub fn envs<I, K, V>(mut self, envs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (key, value) in envs {
            self.environment.insert(key.into(), value.into());
        }
        self
    }

    /// Set additional configuration value.
    pub fn config<K, V>(mut self, key: K, value: V) -> Result<Self>
    where
        K: Into<String>,
        V: Serialize,
    {
        let json_value = serde_json::to_value(value)?;
        self.additional_config.insert(key.into(), json_value);
        Ok(self)
    }

    /// Build the configuration.
    pub fn build(self) -> Result<AgentConfig> {
        let model = self.model.unwrap_or_else(|| "gpt-4".to_string());
        let working_directory = self
            .working_directory
            .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        // Use provided policies or sensible defaults
        let sandbox_policy = self
            .sandbox_policy
            .unwrap_or(SandboxPolicy::WorkspaceWrite {
                writable_roots: Vec::new(),
                network_access: false,
                exclude_tmpdir_env_var: false,
                exclude_slash_tmp: false,
            });
        let approval_policy = self.approval_policy.unwrap_or(AskForApproval::Never);

        Ok(AgentConfig {
            model,
            api_key: self.api_key,
            system_prompt: self.system_prompt,
            sandbox_policy,
            approval_policy,
            max_turns: self.max_turns,
            working_directory,
            tools: self.tools,
            mcp_servers: self.mcp_servers,
            environment: self.environment,
            additional_config: self.additional_config,
        })
    }
}

/// Convenience methods for common sandbox policies
impl AgentConfigBuilder {
    /// Set sandbox policy to allow workspace write operations
    pub fn sandbox_workspace_write(self) -> Self {
        self.sandbox_policy(SandboxPolicy::WorkspaceWrite {
            writable_roots: Vec::new(),
            network_access: false,
            exclude_tmpdir_env_var: false,
            exclude_slash_tmp: false,
        })
    }

    /// Set sandbox policy to read-only mode
    pub fn sandbox_read_only(self) -> Self {
        self.sandbox_policy(SandboxPolicy::ReadOnly)
    }
}

/// Convenience methods for common approval policies
impl AgentConfigBuilder {
    /// Set approval policy to never ask for approval
    pub fn approval_never(self) -> Self {
        self.approval_policy(AskForApproval::Never)
    }

    /// Set approval policy to ask on request
    pub fn approval_on_request(self) -> Self {
        self.approval_policy(AskForApproval::OnRequest)
    }
}
