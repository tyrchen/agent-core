//! Main agent implementation with execution capabilities.

use std::time::Duration;

use async_channel::{Receiver, Sender};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use codex_core::CodexConversation;
use codex_core::config::Config as CodexConfig;
use codex_protocol::protocol::{Event, EventMsg, InputItem, Op, Submission};

use crate::config::AgentConfig;
use crate::controller::AgentController;
use crate::error::{AgentError, OutputError, Result};
use crate::messages::{InputMessage, OutputData, OutputMessage};
use crate::plan::PlanMessage;

/// Main agent structure for managing AI conversations.
pub struct Agent {
    /// Agent configuration
    config: AgentConfig,

    /// Internal Codex conversation handler
    codex_conversation: Option<CodexConversation>,

    /// Agent controller for state management
    controller: AgentController,

    /// Control command receiver
    control_rx: tokio::sync::mpsc::UnboundedReceiver<crate::controller::ControlCommand>,
}

impl Agent {
    /// Create a new agent with the given configuration.
    pub fn new(config: AgentConfig) -> Result<Self> {
        let (controller, control_rx) = AgentController::new();

        Ok(Agent {
            config,
            codex_conversation: None,
            controller,
            control_rx,
        })
    }

    /// Get a reference to the agent controller.
    pub fn controller(&self) -> &AgentController {
        &self.controller
    }

    /// Simple synchronous query method for basic use cases.
    pub async fn query<S: Into<String>>(&mut self, message: S) -> Result<String> {
        let input_message = InputMessage::new(message);

        // Create channels for this single query
        let (input_tx, input_rx) = async_channel::bounded(1);
        let (plan_tx, _plan_rx) = async_channel::bounded(100);
        let (output_tx, output_rx) = async_channel::bounded(100);

        // Send the input message
        input_tx.send(input_message).await?;
        input_tx.close();

        // Start execution in the background
        let handle = self.execute(input_rx, plan_tx, output_tx).await?;

        // Collect output messages until completion
        let mut result = String::new();

        while let Ok(output) = output_rx.recv().await {
            match output.data {
                OutputData::Primary { content } => {
                    result.push_str(&content);
                }
                OutputData::PrimaryDelta { content } => {
                    result.push_str(&content);
                }
                OutputData::Completed => {
                    break;
                }
                OutputData::Error { error } => {
                    return Err(AgentError::Execution {
                        message: format!("Query failed: {:?}", error),
                    });
                }
                _ => {
                    // Ignore other message types for simple query
                }
            }
        }

        // Wait for execution to complete
        handle.await?;

        Ok(result.trim().to_string())
    }

    /// Execute the agent with full channel-based interface.
    pub async fn execute(
        &mut self,
        input_rx: Receiver<InputMessage>,
        plan_tx: Sender<PlanMessage>,
        output_tx: Sender<OutputMessage>,
    ) -> Result<AgentHandle> {
        // Initialize Codex conversation if not already done
        if self.codex_conversation.is_none() {
            // For now, return an error as the actual implementation requires
            // more complex setup with the Codex system
            return Err(AgentError::Config {
                message: "Full Codex integration is not yet implemented. This is a wrapper library structure demonstration.".to_string(),
            });
        }

        // Set initial state
        self.controller
            .set_execution_state(crate::controller::ExecutionState::Running)
            .await;

        // Create the execution context
        let execution_context = ExecutionContext {
            config: self.config.clone(),
            controller: self.controller.clone(),
            codex_conversation: self.codex_conversation.take().ok_or_else(|| {
                AgentError::Generic {
                    message: "Failed to initialize Codex conversation".to_string(),
                }
            })?,
            input_rx,
            plan_tx,
            output_tx,
            control_rx: std::mem::replace(
                &mut self.control_rx,
                tokio::sync::mpsc::unbounded_channel().1,
            ),
        };

        // Spawn the execution task
        let join_handle = tokio::spawn(async move { execution_loop(execution_context).await });

        Ok(AgentHandle {
            controller: self.controller.clone(),
            join_handle,
        })
    }
}

/// Handle to a running agent execution.
pub struct AgentHandle {
    controller: AgentController,
    join_handle: JoinHandle<Result<()>>,
}

impl AgentHandle {
    /// Get the agent controller.
    pub fn controller(&self) -> &AgentController {
        &self.controller
    }

    /// Wait for the agent execution to complete.
    pub async fn await_completion(self) -> Result<()> {
        match self.join_handle.await {
            Ok(result) => result,
            Err(join_error) => Err(AgentError::Execution {
                message: format!("Agent execution task failed: {}", join_error),
            }),
        }
    }
}

impl std::future::Future for AgentHandle {
    type Output = Result<()>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        use std::pin::Pin;

        match Pin::new(&mut self.join_handle).poll(cx) {
            std::task::Poll::Ready(Ok(result)) => std::task::Poll::Ready(result),
            std::task::Poll::Ready(Err(join_error)) => {
                std::task::Poll::Ready(Err(AgentError::Execution {
                    message: format!("Agent execution task failed: {}", join_error),
                }))
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

/// Internal execution context.
struct ExecutionContext {
    config: AgentConfig,
    controller: AgentController,
    codex_conversation: CodexConversation,
    input_rx: Receiver<InputMessage>,
    plan_tx: Sender<PlanMessage>,
    output_tx: Sender<OutputMessage>,
    control_rx: tokio::sync::mpsc::UnboundedReceiver<crate::controller::ControlCommand>,
}

/// Main execution loop for the agent.
async fn execution_loop(mut context: ExecutionContext) -> Result<()> {
    info!("Starting agent execution loop");

    // Main execution loop
    loop {
        // Check for control commands
        tokio::select! {
            // Handle control commands
            control_command = context.control_rx.recv() => {
                if let Some(command) = control_command {
                    debug!("Received control command: {:?}", command);
                    context.controller.handle_control_command(command).await;

                    // If stopped, break the loop
                    if context.controller.should_stop() {
                        break;
                    }
                } else {
                    // Control channel closed, stop execution
                    break;
                }
            }

            // Handle input messages
            input_message = context.input_rx.recv() => {
                match input_message {
                    Ok(message) => {
                        // Wait if paused
                        context.controller.wait_if_paused().await;

                        // Check if we should stop
                        if context.controller.should_stop() {
                            break;
                        }

                        // Process the input message
                        if let Err(e) = process_input_message(
                            &mut context,
                            message,
                        ).await {
                            error!("Error processing input message: {}", e);

                            // Send error output
                            let error_output = OutputMessage::new(
                                context.controller.turn_count(),
                                OutputData::Error {
                                    error: OutputError::General {
                                        message: e.to_string(),
                                    },
                                },
                            );

                            if let Err(send_err) = context.output_tx.send(error_output).await {
                                error!("Failed to send error output: {}", send_err);
                            }

                            context.controller.set_error(e.to_string()).await;
                        }
                    }
                    Err(_) => {
                        // Input channel closed, finish current processing and exit
                        debug!("Input channel closed");
                        break;
                    }
                }
            }

            // Handle timeout or other conditions
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                // Periodic maintenance or heartbeat
                continue;
            }
        }
    }

    info!("Agent execution loop finished");

    // Send final completion message
    let completion_message =
        OutputMessage::new(context.controller.turn_count(), OutputData::Completed);

    if let Err(e) = context.output_tx.send(completion_message).await {
        warn!("Failed to send completion message: {}", e);
    }

    // Set final state
    if !context.controller.should_stop() {
        context
            .controller
            .set_execution_state(crate::controller::ExecutionState::Idle)
            .await;
    }

    Ok(())
}

/// Process a single input message.
async fn process_input_message(
    context: &mut ExecutionContext,
    input_message: InputMessage,
) -> Result<()> {
    debug!("Processing input message: {}", input_message.message);

    // Increment turn count
    context.controller.increment_turn_count();
    let turn_id = context.controller.turn_count();

    // Send start message
    let start_message = OutputMessage::new(turn_id, OutputData::Start);
    context.output_tx.send(start_message).await?;

    // Convert input message to Codex format
    let mut input_items = vec![InputItem::Text {
        text: input_message.message,
    }];

    // Add images if any
    for image in input_message.images {
        input_items.push(InputItem::Image {
            image_url: image.data, // Base64 data URL format expected
        });
    }

    // Create submission
    let submission = Submission {
        id: uuid::Uuid::new_v4().to_string(),
        op: Op::UserInput { items: input_items },
    };

    // Submit to Codex and process events
    context
        .codex_conversation
        .submit_with_id(submission)
        .await?;

    // Process events one by one
    loop {
        // Check if we should stop or pause
        if context.controller.should_stop() {
            break;
        }

        context.controller.wait_if_paused().await;

        // Get next event
        match context.codex_conversation.next_event().await {
            Ok(event) => {
                // Check for task completion
                let is_complete = matches!(event.msg, EventMsg::TaskComplete(_));

                // Convert Codex event to output message
                if let Some(output_data) = convert_event_to_output(&event) {
                    let output_message = OutputMessage::new(turn_id, output_data);
                    context.output_tx.send(output_message).await?;
                }

                // Handle plan updates
                if let Event {
                    msg: EventMsg::PlanUpdate { .. },
                    ..
                } = event
                {
                    // TODO: Extract plan data and send plan message
                    // This would require parsing the plan update event
                }

                // Break if task is complete
                if is_complete {
                    break;
                }
            }
            Err(e) => {
                error!("Error getting next event: {}", e);
                // Send error and break
                let error_output = OutputMessage::new(
                    turn_id,
                    OutputData::Error {
                        error: OutputError::General {
                            message: e.to_string(),
                        },
                    },
                );
                context.output_tx.send(error_output).await?;
                break;
            }
        }
    }

    Ok(())
}

/// Convert a Codex event to output data.
fn convert_event_to_output(event: &Event) -> Option<OutputData> {
    match &event.msg {
        EventMsg::AgentMessage(msg) => Some(OutputData::Primary {
            content: msg.message.clone(),
        }),
        EventMsg::AgentMessageDelta(delta) => Some(OutputData::PrimaryDelta {
            content: delta.delta.clone(),
        }),
        EventMsg::AgentReasoning(reasoning) => Some(OutputData::Reasoning {
            content: reasoning.text.clone(),
        }),
        EventMsg::AgentReasoningDelta(delta) => Some(OutputData::ReasoningDelta {
            content: delta.delta.clone(),
        }),
        EventMsg::TaskComplete(_) => Some(OutputData::Completed),
        EventMsg::Error(error) => Some(OutputData::Error {
            error: OutputError::General {
                message: error.message.clone(),
            },
        }),
        _ => None, // Ignore other event types for now
    }
}

impl Agent {
    /// Create Codex configuration from agent configuration.
    ///
    /// NOTE: This is a placeholder implementation. A real implementation would:
    /// 1. Convert AgentConfig fields to appropriate CodexConfig fields
    /// 2. Set up authentication properly
    /// 3. Configure model providers, tools, MCP servers, etc.
    fn _create_codex_config(&self) -> Result<CodexConfig> {
        // This is a placeholder - real implementation would need to:
        // - Map model settings from AgentConfig to CodexConfig
        // - Set up API keys and authentication
        // - Configure sandbox and approval policies
        // - Set up MCP servers and tools
        Err(AgentError::Config {
            message: "Codex configuration creation not yet implemented".to_string(),
        })
    }
}
