# Agent Core

[English](./README.md) | 中文文档

一个用于将 Codex AI 代理能力嵌入到应用程序中的 Rust 库。该库提供了高级 API，用于创建和管理基于 LLM 驱动的 AI 代理，具有工具执行能力，构建在 Codex 平台之上。

## 功能特性

- **高级代理 API**：用于创建和管理 AI 代理的简单接口
- **配置系统**：灵活的建造者模式用于代理配置
- **消息类型**：结构化的输入/输出消息处理，支持图像
- **计划管理**：使用 MPSC 通道进行实时更新的任务跟踪
- **代理控制**：暂停、恢复和停止功能
- **工具支持**：内置工具（Bash、WebSearch、FileRead、FileWrite、ApplyPatch）+ 自定义工具
- **MCP 服务器集成**：支持基于命令和基于 HTTP 的 MCP 服务器
- **可选功能**：会话管理和实用函数

## 快速开始

添加到您的 `Cargo.toml`：

```toml
[dependencies]
agent-core = "0.1.0"

# 可选功能
agent-core = { version = "0.1.0", features = ["session", "utils"] }
```

### 基本用法

```rust
use agent_core::{Agent, AgentConfig};

#[tokio::main]
async fn main() -> agent_core::Result<()> {
    // 创建代理配置
    let config = AgentConfig::builder()
        .model("gpt-4")
        .system_prompt("你是一个有用的编程助手")
        .sandbox_workspace_write()
        .approval_never()
        .build()?;

    // 创建并使用代理
    let mut agent = Agent::new(config)?;

    // 简单查询（注意：需要正确设置 Codex）
    match agent.query("解释量子计算").await {
        Ok(response) => println!("{}", response),
        Err(e) => println!("错误: {}", e),
    }

    Ok(())
}
```

### 使用通道的高级用法

```rust
use agent_core::{Agent, AgentConfig, InputMessage, ToolConfig};
use async_channel;

#[tokio::main]
async fn main() -> agent_core::Result<()> {
    // 配置工具
    let config = AgentConfig::builder()
        .model("gpt-4")
        .tool(ToolConfig::bash())
        .tool(ToolConfig::web_search())
        .tool(ToolConfig::file_read())
        .tool(ToolConfig::file_write())
        .build()?;

    let mut agent = Agent::new(config)?;

    // 创建通道
    let (input_tx, input_rx) = async_channel::bounded(10);
    let (plan_tx, mut plan_rx) = async_channel::bounded(100);
    let (output_tx, mut output_rx) = async_channel::bounded(100);

    // 启动代理执行
    let handle = agent.execute(input_rx, plan_tx, output_tx).await?;

    // 监控计划更新
    tokio::spawn(async move {
        while let Ok(plan) = plan_rx.recv().await {
            println!("计划更新，包含 {} 个待办事项", plan.todos.len());
            for todo in &plan.todos {
                println!("- [{}] {}", todo.status, todo.content);
            }
        }
    });

    // 监控输出
    tokio::spawn(async move {
        while let Ok(output) = output_rx.recv().await {
            println!("输出: {}", output);
        }
    });

    // 发送输入
    let input = InputMessage::new("用 Python 创建一个简单的 Web 服务器");
    input_tx.send(input).await?;

    // 控制代理
    let controller = handle.controller();

    // 等待一段时间后暂停
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    controller.pause().await?;
    println!("代理已暂停");

    // 片刻后恢复
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    controller.resume().await?;
    println!("代理已恢复");

    // 等待完成
    handle.await?;

    Ok(())
}
```

## 配置

### 代理配置

```rust
use agent_core::{AgentConfig, ToolConfig, McpServerConfig};

let config = AgentConfig::builder()
    // 模型设置
    .model("gpt-4")
    .api_key_env("OPENAI_API_KEY")?
    .system_prompt("自定义系统提示")
    .max_turns(50)

    // 策略
    .sandbox_workspace_write()
    .approval_never()

    // 工具
    .tool(ToolConfig::bash_with_network())
    .tool(ToolConfig::web_search())
    .tools(vec![
        ToolConfig::file_read(),
        ToolConfig::file_write(),
        ToolConfig::apply_patch(),
    ])

    // MCP 服务器
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

    // 环境和工作目录
    .working_directory("/path/to/project")
    .env("NODE_ENV", "development")

    .build()?;
```

### 自定义工具

```rust
use agent_core::{ToolConfig, CustomToolHandler, ToolExecutionContext, ToolExecutionResult};

struct MyCustomTool;

impl CustomToolHandler for MyCustomTool {
    fn execute(
        &self,
        parameters: serde_json::Value,
        context: &ToolExecutionContext,
    ) -> agent_core::Result<ToolExecutionResult> {
        // 工具实现
        Ok(ToolExecutionResult::success("工具执行成功"))
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
        "我的自定义工具".to_string()
    }
}

let tool = ToolConfig::custom(
    "my_tool",
    "我的工具描述",
    serde_json::json!({"type": "object"}),
    Box::new(MyCustomTool),
);
```

## 架构

该库围绕几个关键组件构建：

- **Agent**：管理对话的主要代理结构
- **AgentConfig**：使用建造者模式的配置
- **AgentController**：状态管理（暂停/恢复/停止）
- **Messages**：输入/输出消息类型
- **Plan**：带有待办事项跟踪的任务管理
- **Tools**：内置和自定义工具支持
- **MCP**：模型上下文协议服务器集成

## 示例

该库包含两个全面的示例，演示了 Python 助手功能：

### 控制台 Python 助手

用于与 AI 代理交互以解决 Python 编程任务的命令行界面。

```bash
# 运行控制台示例
cargo run --example python_assistant
```

功能特性：

- 使用 `uv` 自动设置 Python 环境
- 实时计划跟踪与进度指示器
- 流式输出显示
- 工具执行监控

### TUI Python 助手

具有分栏视图的终端用户界面，提供增强的交互体验。

```bash
# 运行 TUI 示例（需要 tui 功能）
cargo run --features tui --example python_assistant_tui
```

功能特性：

- 分栏界面，包含对话和计划视图
- 实时消息流与自动滚动
- 长消息的文本换行
- 彩色编码的消息角色
- 键盘导航（↑/↓ 箭头，PageUp/PageDown）
- 带状态指示器的计划跟踪（⏳ 待处理，🔄 进行中，✅ 已完成）

## 当前状态

⚠️ **注意**：该库与 Codex 平台集成以提供 AI 代理功能。完整功能需要正确的 Codex 设置和配置。

### 已实现

- ✅ 完整的 API 结构和类型
- ✅ 使用建造者模式的配置系统
- ✅ 消息类型和通道通信
- ✅ 带实时更新的计划管理系统
- ✅ 状态管理的代理控制器
- ✅ 工具配置和执行系统
- ✅ MCP 服务器配置
- ✅ Python 助手示例（控制台和 TUI）
- ✅ 流式输出支持
- ✅ 消息中的图像支持
- ✅ 会话管理（可选功能）
- ✅ 实用函数（可选功能）

### 先决条件

对于 Python 助手示例：

- 安装 `uv`（Python 包管理器）：`curl -LsSf https://astral.sh/uv/install.sh | sh`
- 设置 `OPENAI_API_KEY` 环境变量
- 确保 Codex 后端服务正在运行

### 待办事项

- 🔄 增强的 MCP 服务器通信
- 🔄 更多工具实现
- 🔄 更多示例应用
- 🔄 性能优化

## 许可证

MIT 许可证 - 详见 LICENSE 文件。
