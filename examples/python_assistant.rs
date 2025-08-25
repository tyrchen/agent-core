//! Console application that demonstrates agent-core functionality with Python script execution.
//!
//! This example shows:
//! 1. Setting up a Python environment with uv
//! 2. Processing user input through the agent
//! 3. Displaying real-time progress and results
//!
//! Run with: cargo run --example python_assistant

use agent_core::{Agent, AgentConfig, InputMessage, OutputData, OutputMessage, PlanMessage};
use anyhow::Result;
use async_channel::{Receiver, bounded};
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
    process::Command,
};
use tokio::time::{Duration, sleep};

/// Setup Python environment using uv
fn setup_python_environment() -> Result<()> {
    println!("🔧 Checking Python environment setup...");

    // Check if uv is installed
    let uv_check = Command::new("bash").arg("-c").arg("which uv").output()?;

    if !uv_check.status.success() {
        eprintln!("❌ Error: 'uv' is not installed.");
        eprintln!("   Please install it first: curl -LsSf https://astral.sh/uv/install.sh | sh");
        return Err(anyhow::anyhow!("uv not found"));
    }

    // Check uv version
    let uv_version = Command::new("uv").arg("--version").output()?;

    if uv_version.status.success() {
        let version = String::from_utf8_lossy(&uv_version.stdout);
        println!("✅ uv is installed: {}", version.trim());

        // With modern uv, we don't need to pre-create environments
        // uv run handles everything automatically
        println!("📦 uv will handle Python environments automatically");
        println!("   Scripts can be run directly with: uv run script.py");
        println!("   Dependencies can be specified inline in scripts");
    }

    Ok(())
}

/// Process output from the agent with real-time display
async fn process_agent_messages(
    output_rx: &mut Receiver<OutputMessage>,
    plan_rx: &mut Receiver<PlanMessage>,
) -> Result<()> {
    let mut assistant_message = String::new();
    let mut completed = false;
    let mut is_streaming = false;

    println!("\n🤖 Assistant:");

    // Process messages with a timeout
    let timeout = sleep(Duration::from_secs(60)); // Increased timeout for complex tasks
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            // Process output messages
            output = output_rx.recv() => {
                match output {
                    Ok(msg) => {
                        match msg.data {
                            OutputData::Start => {
                                println!("🔄 Processing your request...");
                                is_streaming = false;  // Reset streaming state for new message
                                assistant_message.clear();  // Clear previous message content
                            }
                            OutputData::Primary { content } => {
                                // Only print if we're not in streaming mode
                                if !is_streaming {
                                    println!("{}", content);
                                }
                                assistant_message.push_str(&content);
                            }
                            OutputData::PrimaryDelta { content } => {
                                // Mark that we're streaming
                                is_streaming = true;
                                print!("{}", content);
                                io::stdout().flush()?;
                                assistant_message.push_str(&content);
                            }
                            OutputData::ToolStart { tool_name, arguments } => {
                                println!("\n🔧 Running {}: {}", tool_name, arguments);
                            }
                            OutputData::ToolComplete { tool_name, result } => {
                                if let Some(output_str) = result.as_str()
                                    && !output_str.trim().is_empty() {
                                        println!("📋 {} output:", tool_name);
                                        // Limit output to prevent flooding
                                        let lines: Vec<&str> = output_str.lines().collect();
                                        if lines.len() > 20 {
                                            for line in lines.iter().take(10) {
                                                println!("  {}", line);
                                            }
                                            println!("  ... ({} lines omitted) ...", lines.len() - 20);
                                            for line in lines.iter().skip(lines.len() - 10) {
                                                println!("  {}", line);
                                            }
                                        } else {
                                            for line in lines {
                                                println!("  {}", line);
                                            }
                                        }
                                    }
                            }
                            OutputData::ToolOutput { tool_name: _, output } => {
                                if !output.trim().is_empty() {
                                    println!("  {}", output);
                                }
                            }
                            OutputData::Reasoning { content } => {
                                // Only print non-streaming reasoning
                                if !is_streaming {
                                    println!("🤔 {}", content);
                                }
                            }
                            OutputData::ReasoningDelta { content } => {
                                // We could show reasoning if needed, but usually it's too verbose
                                // Uncomment below to see reasoning in real-time
                                // print!("{}", content);
                                // io::stdout().flush()?;
                                _ = content; // Suppress unused warning
                            }
                            OutputData::TodoUpdate { todos } => {
                                println!("\n📋 Current Plan:");
                                for todo in &todos {
                                    use agent_core::TodoStatus;
                                    let status_icon = match todo.status {
                                        TodoStatus::Completed => "✅",
                                        TodoStatus::InProgress => "🔄",
                                        TodoStatus::Pending => "⏳",
                                    };
                                    println!("  {} {}", status_icon, todo.content);
                                }
                            }
                            OutputData::Error { error } => {
                                eprintln!("\n❌ Error: {:?}", error);
                            }
                            OutputData::Completed => {
                                completed = true;
                                println!("\n✅ Task completed: {}", completed);

                                // Reset streaming state on completion
                                let _ = is_streaming;
                                return Ok(());
                            }
                        }
                    }
                    Err(e) => {
                        if !completed {
                            eprintln!("\n⚠️ Output channel error: {}. This may indicate the agent has stopped.", e);
                            eprintln!("💡 Try restarting the application if the problem persists.");
                        }
                        break;
                    }
                }
            }

            // Process plan messages
            plan = plan_rx.recv() => {
                match plan {
                    Ok(plan_msg) => {
                        println!("\n📝 Plan Update:");
                        for todo in &plan_msg.todos {
                            use agent_core::TodoStatus;
                            let status_icon = match todo.status {
                                TodoStatus::Completed => "✅",
                                TodoStatus::InProgress => "🔄",
                                TodoStatus::Pending => "⏳",
                            };
                            println!("  {} {}", status_icon, todo.content);
                        }
                    }
                    Err(_) => {
                        // Plan channel closed is OK
                    }
                }
            }

            // Timeout protection
            _ = &mut timeout => {
                println!("\n⏱️ Request timed out after 60 seconds");
                println!("💡 The agent may still be processing. You can:");
                println!("   1. Wait a bit longer for complex tasks");
                println!("   2. Try a simpler request");
                println!("   3. Restart the application");
                break;
            }
        }

        // Check if both channels are closed and we're done
        if output_rx.is_closed() && plan_rx.is_closed() {
            break;
        }
    }

    if !assistant_message.is_empty() && !completed {
        println!("\n📝 Response: {}", assistant_message);
    }

    Ok(())
}

/// Main application loop
async fn run_assistant() -> Result<()> {
    println!("🐍 Python Assistant - Powered by AI");
    println!("====================================");
    println!();

    // Setup Python environment first
    setup_python_environment()?;

    println!("\n🤖 Initializing AI agent...");

    // Load system prompt from file
    let system_prompt = fs::read_to_string("examples/system_prompt.md").unwrap_or_else(|_| {
        eprintln!("⚠️  Warning: Could not load system_prompt.md, using default prompt");
        include_str!("system_prompt.md").to_string()
    });

    // Configure the agent
    let config = AgentConfig::builder()
        .model("gpt-5-mini")
        .system_prompt(&system_prompt)
        .max_turns(10)
        .working_directory(PathBuf::from("/tmp"))
        .build()?;

    // Try to create the agent
    let mut agent = match Agent::new(config) {
        Ok(agent) => {
            println!("✅ Agent created successfully!");
            agent
        }
        Err(e) => {
            eprintln!("❌ Failed to create agent: {}", e);
            eprintln!("\n💡 Troubleshooting tips:");
            eprintln!("   1. Check if OPENAI_API_KEY environment variable is set:");
            eprintln!("      export OPENAI_API_KEY='your-api-key'");
            eprintln!("   2. Ensure you have proper API access");
            eprintln!("   3. Check network connectivity");
            eprintln!("\n📖 For more information, see the README.md");
            return Err(e.into());
        }
    };

    // Create channels for communication
    let (input_tx, input_rx) = bounded(100);
    let (output_tx, mut output_rx) = bounded(100);
    let (plan_tx, mut plan_rx) = bounded(100);

    println!("🚀 Starting agent execution...");

    // Start the agent execution
    let _handle = match agent.execute(input_rx, plan_tx, output_tx).await {
        Ok(handle) => {
            println!("✅ Agent is running and ready to process requests!");
            handle
        }
        Err(e) => {
            eprintln!("❌ Failed to start agent execution: {}", e);
            eprintln!("\n💡 This error typically occurs when:");
            eprintln!("   1. The Codex backend services are not running");
            eprintln!("   2. There's a configuration mismatch");
            eprintln!("   3. API credentials are invalid");
            eprintln!("\n📖 Please check the documentation for setup instructions.");
            return Err(e.into());
        }
    };

    println!("✅ Agent ready! Type your Python programming requests.\n");
    println!("Examples:");
    println!("  - 'Calculate fibonacci numbers up to 100'");
    println!("  - 'Download and analyze a CSV file from a URL'");
    println!("  - 'Create a bar chart of random data'");
    println!("  - 'Solve a quadratic equation'");
    println!("\nType 'quit' or 'exit' to stop.\n");

    // Main interaction loop
    loop {
        print!("\n👤 You: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.eq_ignore_ascii_case("quit") || input.eq_ignore_ascii_case("exit") {
            println!("👋 Goodbye!");
            break;
        }

        if input.eq_ignore_ascii_case("help") {
            println!("\n📚 Available commands:");
            println!("  - Type any Python programming request");
            println!("  - 'help' - Show this help message");
            println!("  - 'quit' or 'exit' - Exit the program");
            println!("\nExample requests:");
            println!("  - Calculate the 100th fibonacci number");
            println!("  - Find all prime numbers up to 1000");
            println!("  - Create a function to sort a list");
            continue;
        }

        if input.is_empty() {
            continue;
        }

        // Send the user's request to the agent
        match input_tx.send(InputMessage::new(input.to_string())).await {
            Ok(_) => {
                // Process agent messages with real-time display
                if let Err(e) = process_agent_messages(&mut output_rx, &mut plan_rx).await {
                    eprintln!("❌ Error processing response: {}", e);
                }
            }
            Err(e) => {
                eprintln!("❌ Failed to send message to agent: {}", e);
                eprintln!("   The agent may have stopped. Please restart the program.");
                break;
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run_assistant().await {
        eprintln!("❌ Application error: {}", e);
        std::process::exit(1);
    }
}
