# Agent Core

A Rust library for embedding Codex AI agent capabilities into applications. This library provides high-level APIs for creating and managing LLM-driven AI agents with tool execution capabilities, built on top of the Codex platform.

## Features

- **High-level Agent API**: Simple interface for creating and managing AI agents
- **Configuration System**: Flexible builder pattern for agent configuration
- **Message Types**: Structured input/output message handling with image support
- **Plan Management**: Task tracking with MPSC channels for real-time updates
- **Agent Control**: Pause, resume, and stop functionality
- **Tool Support**: Built-in tools (Bash, WebSearch, FileRead, FileWrite, ApplyPatch) + custom tools
- **MCP Server Integration**: Support for both command-based and HTTP-based MCP servers
- **Optional Features**: Session management and utility functions

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
agent-core = "0.1.0"

# Optional features
agent-core = { version = "0.1.0", features = ["session", "utils"] }
```

### Basic Usage

```rust
use agent_core::{Agent, AgentConfig};

#[tokio::main]
async fn main() -> agent_core::Result<()> {
    // Create agent configuration
    let config = AgentConfig::builder()
        .model("gpt-4")
        .system_prompt("You are a helpful coding assistant")
        .sandbox_workspace_write()
        .approval_never()
        .build()?;

    // Create and use agent
    let mut agent = Agent::new(config)?;

    // Simple query (Note: requires proper Codex setup)
    match agent.query("Explain quantum computing").await {
        Ok(response) => println!("{}", response),
        Err(e) => println!("Error: {}", e),
    }

    Ok(())
}
```

### Advanced Usage with Channels

```rust
use agent_core::{Agent, AgentConfig, InputMessage, ToolConfig};
use async_channel;

#[tokio::main]
async fn main() -> agent_core::Result<()> {
    // Configure with tools
    let config = AgentConfig::builder()
        .model("gpt-4")
        .tool(ToolConfig::bash())
        .tool(ToolConfig::web_search())
        .tool(ToolConfig::file_read())
        .tool(ToolConfig::file_write())
        .build()?;

    let mut agent = Agent::new(config)?;

    // Create channels
    let (input_tx, input_rx) = async_channel::bounded(10);
    let (plan_tx, mut plan_rx) = async_channel::bounded(100);
    let (output_tx, mut output_rx) = async_channel::bounded(100);

    // Start agent execution
    let handle = agent.execute(input_rx, plan_tx, output_tx).await?;

    // Monitor plan updates
    tokio::spawn(async move {
        while let Ok(plan) = plan_rx.recv().await {
            println!("Plan updated with {} todos", plan.todos.len());
            for todo in &plan.todos {
                println!("- [{}] {}", todo.status, todo.content);
            }
        }
    });

    // Monitor output
    tokio::spawn(async move {
        while let Ok(output) = output_rx.recv().await {
            println!("Output: {}", output);
        }
    });

    // Send input
    let input = InputMessage::new("Create a simple web server in Python");
    input_tx.send(input).await?;

    // Control agent
    let controller = handle.controller();

    // Wait a bit then pause
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    controller.pause().await?;
    println!("Agent paused");

    // Resume after a moment
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    controller.resume().await?;
    println!("Agent resumed");

    // Wait for completion
    handle.await?;

    Ok(())
}
```

## Configuration

### Agent Configuration

```rust
use agent_core::{AgentConfig, ToolConfig, McpServerConfig};

let config = AgentConfig::builder()
    // Model settings
    .model("gpt-4")
    .api_key_env("OPENAI_API_KEY")?
    .system_prompt("Custom system prompt")
    .max_turns(50)

    // Policies
    .sandbox_workspace_write()
    .approval_never()

    // Tools
    .tool(ToolConfig::bash_with_network())
    .tool(ToolConfig::web_search())
    .tools(vec![
        ToolConfig::file_read(),
        ToolConfig::file_write(),
        ToolConfig::apply_patch(),
    ])

    // MCP Servers
    .mcp_server(
        McpServerConfig::command("my-server", "my-mcp-server")
            .args(vec!["--config", "config.json"])
            .env_var("API_KEY", "secret")
            .build()
    )
    .mcp_server(
        McpServerConfig::http("web-server", "http://localhost:8080")
            .header("Authorization", "Bearer token")
            .build()
    )

    // Environment and working directory
    .working_directory("/path/to/project")
    .env("NODE_ENV", "development")

    .build()?;
```

### Custom Tools

```rust
use agent_core::{ToolConfig, CustomToolHandler, ToolExecutionContext, ToolExecutionResult};

struct MyCustomTool;

impl CustomToolHandler for MyCustomTool {
    fn execute(
        &self,
        parameters: serde_json::Value,
        context: &ToolExecutionContext,
    ) -> agent_core::Result<ToolExecutionResult> {
        // Tool implementation
        Ok(ToolExecutionResult::success("Tool executed successfully"))
    }

    fn parameter_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "input": {"type": "string"}
            }
        })
    }

    fn description(&self) -> String {
        "My custom tool".to_string()
    }
}

let tool = ToolConfig::custom(
    "my_tool",
    "Description of my tool",
    serde_json::json!({"type": "object"}),
    Box::new(MyCustomTool),
);
```

## Architecture

The library is structured around several key components:

- **Agent**: Main agent struct for managing conversations
- **AgentConfig**: Configuration with builder pattern
- **AgentController**: State management (pause/resume/stop)
- **Messages**: Input/output message types
- **Plan**: Task management with todo tracking
- **Tools**: Built-in and custom tool support
- **MCP**: Model Context Protocol server integration

## Current Status

⚠️ **Note**: This is currently a **wrapper library structure demonstration**. The full Codex integration requires additional implementation work to properly interface with the underlying Codex system.

### Implemented:
- ✅ Complete API structure and types
- ✅ Configuration system with builder pattern
- ✅ Message types and channel communication
- ✅ Plan management system
- ✅ Agent controller for state management
- ✅ Tool configuration system
- ✅ MCP server configuration
- ✅ Optional session and utils modules

### TODO:
- 🔄 Full Codex conversation integration
- 🔄 Actual tool execution implementation
- 🔄 MCP server communication
- 🔄 Authentication and API key management
- 🔄 Configuration mapping to Codex internals

## License

MIT License - see LICENSE file for details.
