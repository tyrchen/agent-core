//! TUI application that uses agent-core to execute Python scripts based on user intentions.
//!
//! This example demonstrates:
//! 1. Setting up a Python environment with uv
//! 2. Converting user intentions into Python scripts
//! 3. Executing scripts and displaying results

use agent_core::{
    Agent, AgentConfig, AgentHandle, InputMessage, OutputData, OutputMessage, PlanMessage,
    TodoItem, ToolConfig,
};
use anyhow::Result;
use async_channel::{Receiver, Sender, bounded};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::{
    fs,
    io::{self, Stdout},
    path::PathBuf,
    time::Duration,
};
use tokio::time::sleep;

/// Application state
struct App {
    /// User input buffer
    input: String,
    /// Messages history (user inputs and AI responses)
    messages: Vec<Message>,
    /// Current agent status
    status: String,
    /// Whether the app should quit
    should_quit: bool,
    /// Agent handle for controlling execution
    agent_handle: Option<AgentHandle>,
    /// Channel for sending input to agent
    input_tx: Option<Sender<InputMessage>>,
    /// Channel for receiving output from agent
    output_rx: Option<Receiver<OutputMessage>>,
    /// Channel for receiving plan updates
    plan_rx: Option<Receiver<PlanMessage>>,
    /// Current plan items
    current_plan: Vec<TodoItem>,
    /// Scroll offset for messages
    messages_scroll: usize,
    /// Python environment path
    _python_env_path: PathBuf,
    /// Whether Python environment is ready
    python_env_ready: bool,
    /// Track if we're currently streaming output
    is_streaming: bool,
}

#[derive(Clone)]
struct Message {
    role: MessageRole,
    content: String,
    _timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, PartialEq)]
enum MessageRole {
    User,
    Assistant,
    System,
    Error,
}

impl App {
    fn new() -> Self {
        Self {
            input: String::new(),
            messages: vec![Message {
                role: MessageRole::System,
                content: "Welcome to Python Assistant! I'll help you solve problems using Python scripts.".to_string(),
                _timestamp: chrono::Utc::now(),
            }],
            status: "Initializing...".to_string(),
            should_quit: false,
            agent_handle: None,
            input_tx: None,
            output_rx: None,
            plan_rx: None,
            current_plan: Vec::new(),
            messages_scroll: 0,
            _python_env_path: PathBuf::from("/tmp/python_assistant_env"),
            python_env_ready: false,
            is_streaming: false,
        }
    }

    async fn initialize_agent(&mut self) -> Result<()> {
        self.status = "Setting up Python environment...".to_string();

        // First, ensure uv is installed and setup Python environment
        self.setup_python_environment().await?;

        self.status = "Initializing AI agent...".to_string();

        // Configure the agent with necessary tools
        // Load system prompt from file
        let system_prompt = fs::read_to_string("examples/system_prompt.md").unwrap_or_else(|_| {
            eprintln!("⚠️  Warning: Could not load system_prompt.md, using default prompt");
            include_str!("system_prompt.md").to_string()
        });

        let config = AgentConfig::builder()
            .model("gpt-5-mini")
            .system_prompt(&system_prompt)
            .max_turns(10)
            .tool(ToolConfig::Bash {
                allow_network: true,
                environment: std::collections::HashMap::new(),
                working_directory: None,
                timeout: Some(60),
            })
            .tool(ToolConfig::FileWrite {
                max_file_size: 10_000_000, // 10MB
                allowed_extensions: vec![],
                allow_overwrite: true,
                create_directories: true,
            })
            .tool(ToolConfig::FileRead {
                max_file_size: 10_000_000, // 10MB
                allowed_extensions: vec![],
                allow_binary: false,
            })
            .working_directory(PathBuf::from("/tmp"))
            .build()?;

        let mut agent = Agent::new(config)?;

        // Create channels for communication
        let (input_tx, input_rx) = bounded(100);
        let (output_tx, output_rx) = bounded(100);
        let (plan_tx, plan_rx) = bounded(100);

        // Start the agent
        let handle = agent.execute(input_rx, plan_tx, output_tx).await?;

        self.agent_handle = Some(handle);
        self.input_tx = Some(input_tx);
        self.output_rx = Some(output_rx);
        self.plan_rx = Some(plan_rx);

        self.status = "Ready! Type your request and press Enter.".to_string();
        self.messages.push(Message {
            role: MessageRole::System,
            content: "Python environment ready! I can now help you with Python programming tasks."
                .to_string(),
            _timestamp: chrono::Utc::now(),
        });

        Ok(())
    }

    async fn setup_python_environment(&mut self) -> Result<()> {
        use std::process::Command;

        // Check if uv is installed
        let uv_check = Command::new("bash").arg("-c").arg("which uv").output()?;

        if !uv_check.status.success() {
            self.messages.push(Message {
                role: MessageRole::Error,
                content: "Error: 'uv' is not installed. Please install it first: curl -LsSf https://astral.sh/uv/install.sh | sh".to_string(),
                _timestamp: chrono::Utc::now(),
            });
            return Err(anyhow::anyhow!("uv not found"));
        }

        // Check uv version
        let uv_version = Command::new("uv").arg("--version").output()?;

        if uv_version.status.success() {
            let version = String::from_utf8_lossy(&uv_version.stdout);
            self.messages.push(Message {
                role: MessageRole::System,
                content: format!(
                    "✅ uv {} ready - scripts will run with: uv run script.py",
                    version.trim()
                ),
                _timestamp: chrono::Utc::now(),
            });
        }

        self.python_env_ready = true;
        Ok(())
    }

    async fn _send_message(&mut self, message: String) -> Result<()> {
        // Add user message to history
        self.messages.push(Message {
            role: MessageRole::User,
            content: message.clone(),
            _timestamp: chrono::Utc::now(),
        });

        // Enhance the message with Python execution context
        let enhanced_message = format!(
            "User request: {}\n\n\
             Please solve this by writing a Python script and executing it.\n\
             CRITICAL: Use FileWrite to save the script as /tmp/script.py\n\
             CRITICAL: Execute using: bash -c 'cd /tmp && uv run script.py'\n\
             Show the results to the user.",
            message
        );

        // Send to agent
        if let Some(tx) = &self.input_tx {
            tx.send(InputMessage::new(enhanced_message)).await?;
            self.status = "Processing...".to_string();
        }

        Ok(())
    }

    async fn process_agent_output(&mut self) {
        // Process output messages
        if let Some(rx) = &mut self.output_rx {
            while let Ok(output) = rx.try_recv() {
                match output.data {
                    OutputData::Start => {
                        self.status = "🔄 Processing...".to_string();
                        self.is_streaming = false; // Reset streaming state for new response
                    }
                    OutputData::Primary { content } => {
                        // Only create new message if we're not in streaming mode
                        if !self.is_streaming {
                            self.messages.push(Message {
                                role: MessageRole::Assistant,
                                content,
                                _timestamp: chrono::Utc::now(),
                            });
                        }
                    }
                    OutputData::PrimaryDelta { content } => {
                        self.is_streaming = true; // Mark that we're streaming
                        // Append to last assistant message if it exists
                        if let Some(last) = self.messages.last_mut() {
                            if last.role == MessageRole::Assistant {
                                last.content.push_str(&content);
                            } else {
                                self.messages.push(Message {
                                    role: MessageRole::Assistant,
                                    content,
                                    _timestamp: chrono::Utc::now(),
                                });
                            }
                        } else {
                            self.messages.push(Message {
                                role: MessageRole::Assistant,
                                content,
                                _timestamp: chrono::Utc::now(),
                            });
                        }
                    }
                    OutputData::ToolStart {
                        tool_name,
                        arguments,
                    } => {
                        self.status = format!("🔧 Executing: {}", tool_name);
                        self.messages.push(Message {
                            role: MessageRole::System,
                            content: format!(
                                "🔧 Running tool: {} with args: {}",
                                tool_name, arguments
                            ),
                            _timestamp: chrono::Utc::now(),
                        });
                    }
                    OutputData::ToolComplete { tool_name, result } => {
                        // Only show ToolComplete output if we haven't already shown it via ToolOutput
                        // Check if the last few messages already contain output from this tool
                        let recent_has_tool_output =
                            self.messages.iter().rev().take(5).any(|msg| {
                                msg.role == MessageRole::System
                                    && msg.content.starts_with(&format!("📋 {}", tool_name))
                            });

                        if !recent_has_tool_output
                            && let Some(output_str) = result.as_str()
                            && !output_str.trim().is_empty()
                        {
                            self.messages.push(Message {
                                role: MessageRole::System,
                                content: format!("📋 {} complete:\n{}", tool_name, output_str),
                                _timestamp: chrono::Utc::now(),
                            });
                        }
                    }
                    OutputData::ToolOutput { tool_name, output } => {
                        if !output.trim().is_empty() {
                            // Show streaming tool output
                            let lines: Vec<&str> = output.lines().collect();
                            let display_output = if lines.len() > 10 {
                                // Truncate very long output
                                format!(
                                    "📋 {} output (truncated):\n{}\n...\n{}",
                                    tool_name,
                                    lines[..5].join("\n"),
                                    lines[lines.len() - 5..].join("\n")
                                )
                            } else {
                                format!("📋 {} output:\n{}", tool_name, output)
                            };

                            self.messages.push(Message {
                                role: MessageRole::System,
                                content: display_output,
                                _timestamp: chrono::Utc::now(),
                            });
                        }
                    }
                    OutputData::Reasoning { content } => {
                        // Only create new message if we're not in streaming mode
                        if !self.is_streaming {
                            self.messages.push(Message {
                                role: MessageRole::System,
                                content: format!("🤔 {}", content),
                                _timestamp: chrono::Utc::now(),
                            });
                        }
                    }
                    OutputData::ReasoningDelta { content } => {
                        // Append reasoning to last system message if it exists
                        if let Some(last) = self.messages.last_mut() {
                            if last.role == MessageRole::System && last.content.starts_with("🤔")
                            {
                                last.content.push_str(&content);
                            } else {
                                self.messages.push(Message {
                                    role: MessageRole::System,
                                    content: format!("🤔 {}", content),
                                    _timestamp: chrono::Utc::now(),
                                });
                            }
                        } else {
                            self.messages.push(Message {
                                role: MessageRole::System,
                                content: format!("🤔 {}", content),
                                _timestamp: chrono::Utc::now(),
                            });
                        }
                    }
                    OutputData::TodoUpdate { todos } => {
                        // Update the current plan directly
                        self.current_plan = todos;
                    }
                    OutputData::Completed => {
                        self.status = "✅ Ready".to_string();
                        self.is_streaming = false; // Reset streaming state when completed
                    }
                    OutputData::Error { error } => {
                        // Make error more visible and persistent
                        let error_msg = format!("❌ ERROR: {:?}", error);
                        self.messages.push(Message {
                            role: MessageRole::Error,
                            content: error_msg.clone(),
                            _timestamp: chrono::Utc::now(),
                        });
                        self.status = format!("❌ Error: {:?}", error);
                        // Don't change streaming state on error
                    }
                }
            }
        }

        // Process plan updates from plan channel
        if let Some(rx) = &mut self.plan_rx {
            while let Ok(plan) = rx.try_recv() {
                self.current_plan = plan.todos;
            }
        }
    }

    fn on_key_event(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Up => {
                if self.messages_scroll > 0 {
                    self.messages_scroll -= 1;
                }
            }
            KeyCode::Down => {
                if self.messages_scroll < self.messages.len().saturating_sub(10) {
                    self.messages_scroll += 1;
                }
            }
            KeyCode::PageUp => {
                self.messages_scroll = self.messages_scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.messages_scroll =
                    (self.messages_scroll + 10).min(self.messages.len().saturating_sub(10));
            }
            KeyCode::Enter => {
                if !self.input.is_empty() {
                    let message = self.input.clone();
                    self.input.clear();

                    // Add to messages immediately for UI feedback
                    self.messages.push(Message {
                        role: MessageRole::User,
                        content: message.clone(),
                        _timestamp: chrono::Utc::now(),
                    });

                    // Send message with fallback to try_send if blocking
                    if let Some(tx) = &self.input_tx {
                        match tx.try_send(InputMessage::new(message.clone())) {
                            Ok(_) => {
                                self.status = "🔄 Processing...".to_string();
                                self.is_streaming = false; // Reset streaming state for new message
                            }
                            Err(async_channel::TrySendError::Full(_)) => {
                                // Channel is full, spawn a task to send asynchronously
                                let tx_clone = tx.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = tx_clone.send(InputMessage::new(message)).await
                                    {
                                        eprintln!("Failed to send message to agent: {}", e);
                                    }
                                });
                                self.status = "🔄 Processing...".to_string();
                                self.is_streaming = false;
                            }
                            Err(async_channel::TrySendError::Closed(_)) => {
                                self.messages.push(Message {
                                    role: MessageRole::Error,
                                    content: "Agent channel closed - agent may have stopped"
                                        .to_string(),
                                    _timestamp: chrono::Utc::now(),
                                });
                                self.status = "❌ Agent offline".to_string();
                            }
                        }
                    } else {
                        self.messages.push(Message {
                            role: MessageRole::Error,
                            content: "Agent not initialized".to_string(),
                            _timestamp: chrono::Utc::now(),
                        });
                    }
                }
            }
            KeyCode::Char(c) => {
                self.input.push(c);
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            _ => {}
        }
        Ok(())
    }
}

fn draw_ui(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),   // Messages area
            Constraint::Length(3), // Input area
            Constraint::Length(3), // Status bar
        ])
        .split(frame.area());

    // Draw messages
    draw_messages(frame, app, chunks[0]);

    // Draw input
    draw_input(frame, app, chunks[1]);

    // Draw status
    draw_status(frame, app, chunks[2]);

    // Draw plan sidebar if there are plan items
    if !app.current_plan.is_empty() {
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(chunks[0]);

        draw_messages(frame, app, main_chunks[0]);
        draw_plan(frame, app, main_chunks[1]);
    }
}

fn draw_messages(frame: &mut Frame, app: &App, area: Rect) {
    // Build all messages with proper wrapping
    let mut all_lines: Vec<Line> = Vec::new();
    let width = area.width.saturating_sub(4) as usize; // Account for borders and padding

    for msg in &app.messages {
        let style = match msg.role {
            MessageRole::User => Style::default().fg(Color::Cyan),
            MessageRole::Assistant => Style::default().fg(Color::Green),
            MessageRole::System => Style::default().fg(Color::Yellow),
            MessageRole::Error => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        };

        let prefix = match msg.role {
            MessageRole::User => "👤 You: ",
            MessageRole::Assistant => "🤖 Assistant: ",
            MessageRole::System => "⚙️ System: ",
            MessageRole::Error => "❌ Error: ",
        };

        // Wrap message text properly
        let full_text = format!("{}{}", prefix, msg.content);
        let wrapped_lines = textwrap::wrap(&full_text, width);

        for (i, line) in wrapped_lines.iter().enumerate() {
            if i == 0 {
                // First line with prefix
                all_lines.push(Line::from(line.to_string()).style(style));
            } else {
                // Continuation lines with indent
                all_lines.push(Line::from(format!("        {}", line)).style(style));
            }
        }

        // Add empty line between messages for readability
        all_lines.push(Line::from(""));
    }

    // Calculate scroll position for auto-scroll to latest
    let visible_height = area.height.saturating_sub(2) as usize;
    let total_lines = all_lines.len();
    let scroll = if total_lines > visible_height {
        // Auto-scroll to show latest messages unless user has manually scrolled
        if app.messages_scroll == 0 {
            total_lines.saturating_sub(visible_height)
        } else {
            app.messages_scroll
        }
    } else {
        0
    };

    // Get visible lines
    let visible_lines: Vec<Line> = all_lines
        .into_iter()
        .skip(scroll)
        .take(visible_height)
        .collect();

    let title = format!("Conversation ({} messages)", app.messages.len());
    let messages_widget = Paragraph::new(visible_lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false });

    frame.render_widget(messages_widget, area);
}

fn draw_input(frame: &mut Frame, app: &App, area: Rect) {
    let hint = if app.input.is_empty() {
        "Type your Python request here..."
    } else {
        ""
    };

    let display_text = if app.input.is_empty() {
        hint.to_string()
    } else {
        app.input.clone()
    };

    let style = if app.input.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let input = Paragraph::new(display_text.as_str()).style(style).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Input (Enter to send | ↑↓ scroll | Ctrl+C quit)"),
    );

    frame.render_widget(input, area);

    // Set cursor position
    if !app.input.is_empty() {
        frame.set_cursor_position((area.x + app.input.len() as u16 + 1, area.y + 1));
    }
}

fn draw_status(frame: &mut Frame, app: &App, area: Rect) {
    let status_text = format!(
        "Status: {} | Python: {}",
        app.status,
        if app.python_env_ready {
            "Ready"
        } else {
            "Not Ready"
        }
    );

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Magenta))
        .block(Block::default().borders(Borders::ALL).title("Status"));

    frame.render_widget(status, area);
}

fn draw_plan(frame: &mut Frame, app: &App, area: Rect) {
    use agent_core::TodoStatus;

    let mut lines: Vec<Line> = Vec::new();
    let width = area.width.saturating_sub(4) as usize; // Account for borders

    for todo in &app.current_plan {
        let (emoji, color) = match todo.status {
            TodoStatus::Completed => ("✅", Color::Green),
            TodoStatus::InProgress => ("🔄", Color::Yellow),
            TodoStatus::Pending => ("⏳", Color::Gray),
        };

        let text = format!("{} {}", emoji, todo.content);
        // Wrap long plan items
        let wrapped = textwrap::wrap(&text, width);

        for (i, line) in wrapped.iter().enumerate() {
            if i == 0 {
                lines.push(Line::from(line.to_string()).style(Style::default().fg(color)));
            } else {
                // Indent continuation lines
                lines.push(Line::from(format!("   {}", line)).style(Style::default().fg(color)));
            }
        }

        // Add spacing between items
        lines.push(Line::from(""));
    }

    let title = format!("Current Plan ({} items)", app.current_plan.len());
    let plan_widget = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false });

    frame.render_widget(plan_widget, area);
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, mut app: App) -> Result<()> {
    // Initialize the agent with better error handling
    if let Err(e) = app.initialize_agent().await {
        app.messages.push(Message {
            role: MessageRole::Error,
            content: format!("Failed to initialize agent: {}", e),
            _timestamp: chrono::Utc::now(),
        });
        app.status = format!("❌ Initialization failed: {}", e);
        // Continue to show the UI so user can see the error
    }

    loop {
        // Draw UI with error handling
        if let Err(e) = terminal.draw(|f| draw_ui(f, &app)) {
            eprintln!("Failed to draw UI: {}", e);
            break;
        }

        // Process agent output
        app.process_agent_output().await;

        // Handle events with better error handling
        match event::poll(Duration::from_millis(100)) {
            Ok(true) => {
                match event::read() {
                    Ok(Event::Key(key)) => {
                        if let Err(e) = app.on_key_event(key) {
                            app.messages.push(Message {
                                role: MessageRole::Error,
                                content: format!("Input error: {}", e),
                                _timestamp: chrono::Utc::now(),
                            });
                        }
                    }
                    Err(e) => {
                        eprintln!("Event read error: {}", e);
                        // Continue running despite input errors
                    }
                    _ => {}
                }
            }
            Ok(false) => {
                // No events, continue
            }
            Err(e) => {
                eprintln!("Event polling error: {}", e);
                // Continue running despite polling errors
            }
        }

        // Check if we should quit
        if app.should_quit {
            break;
        }

        // Small sleep to prevent busy loop
        sleep(Duration::from_millis(10)).await;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create and run app
    let app = App::new();
    let res = run_app(&mut terminal, app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Print any error
    if let Err(err) = res {
        eprintln!("Application error: {:?}", err);
    }

    Ok(())
}
