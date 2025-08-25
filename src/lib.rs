//! # Agent Core
//!
//! A Rust library for embedding Codex AI agent capabilities into applications.
//!
//! This library provides high-level APIs for creating and managing LLM-driven AI agents
//! with tool execution capabilities, built on top of the Codex platform.
//!
//! ## Example
//!
//! ```no_run
//! use agent_core::{Agent, AgentConfig};
//!
//! #[tokio::main]
//! async fn main() -> agent_core::Result<()> {
//!     let config = AgentConfig::builder()
//!         .model("gpt-4")
//!         .build()?;
//!
//!     let mut agent = Agent::new(config)?;
//!
//!     // Note: Full integration requires proper Codex setup
//!     // This will return an error in the current implementation
//!     match agent.query("Explain quantum computing").await {
//!         Ok(response) => println!("{}", response),
//!         Err(e) => println!("Error: {}", e),
//!     }
//!
//!     Ok(())
//! }
//! ```

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

pub mod agent;
pub mod config;
pub mod controller;
pub mod error;
pub mod mcp;
pub mod messages;
pub mod plan;
pub mod tools;

// Optional features
#[cfg(feature = "session")]
pub mod session;

#[cfg(feature = "utils")]
pub mod utils;

// Re-exports for convenience
pub use agent::{Agent, AgentHandle};
pub use config::{AgentConfig, AgentConfigBuilder};
pub use controller::AgentController;
pub use error::{AgentError, OutputError, Result};
pub use mcp::McpServerConfig;
pub use messages::{ImageInput, InputMessage, OutputData, OutputMessage};
pub use plan::{PlanMessage, PlanMetadata, TodoItem, TodoStatus};
pub use tools::{CustomToolHandler, ToolConfig};

// Re-export codex types for convenience
pub use codex_protocol::protocol::{AskForApproval, SandboxPolicy};

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = AgentConfig::builder().model("gpt-4").build().unwrap();

        assert_eq!(config.model(), "gpt-4");
    }
}
