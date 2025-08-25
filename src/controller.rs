//! Agent controller for managing agent execution state.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::{Mutex, oneshot};

use crate::error::{AgentError, Result};

/// Controller for managing agent execution state.
#[derive(Debug, Clone)]
pub struct AgentController {
    /// Shared state for the agent
    state: Arc<AgentState>,
}

/// Internal agent state.
#[derive(Debug)]
struct AgentState {
    /// Current execution state
    execution_state: Mutex<ExecutionState>,

    /// Current turn count
    turn_count: AtomicU64,

    /// Whether the agent is currently paused
    is_paused: AtomicBool,

    /// Whether the agent should stop execution
    should_stop: AtomicBool,

    /// Channel for sending control commands
    control_sender: Mutex<Option<tokio::sync::mpsc::UnboundedSender<ControlCommand>>>,
}

/// Internal execution state of the agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExecutionState {
    /// Agent is not running
    Idle,

    /// Agent is currently executing
    Running,

    /// Agent is paused but can be resumed
    Paused,

    /// Agent has been stopped
    Stopped,

    /// Agent has encountered an error
    Error(String),
}

/// Control commands that can be sent to the agent.
#[derive(Debug)]
pub(crate) enum ControlCommand {
    /// Pause the agent
    Pause(oneshot::Sender<Result<()>>),

    /// Resume the agent from pause
    Resume(oneshot::Sender<Result<()>>),

    /// Stop the agent permanently
    Stop(oneshot::Sender<Result<()>>),
}

impl AgentController {
    /// Create a new agent controller.
    pub(crate) fn new() -> (Self, tokio::sync::mpsc::UnboundedReceiver<ControlCommand>) {
        let (control_tx, control_rx) = tokio::sync::mpsc::unbounded_channel();

        let state = Arc::new(AgentState {
            execution_state: Mutex::new(ExecutionState::Idle),
            turn_count: AtomicU64::new(0),
            is_paused: AtomicBool::new(false),
            should_stop: AtomicBool::new(false),
            control_sender: Mutex::new(Some(control_tx)),
        });

        let controller = AgentController { state };

        (controller, control_rx)
    }

    /// Get the current execution state.
    pub async fn state(&self) -> AgentExecutionState {
        let execution_state = self.state.execution_state.lock().await;
        let turn_count = self.state.turn_count.load(Ordering::Relaxed);
        let is_paused = self.state.is_paused.load(Ordering::Relaxed);
        let should_stop = self.state.should_stop.load(Ordering::Relaxed);

        AgentExecutionState {
            execution_state: execution_state.clone().into(),
            turn_count,
            is_paused,
            should_stop,
        }
    }

    /// Get the current turn count.
    pub fn turn_count(&self) -> u64 {
        self.state.turn_count.load(Ordering::Relaxed)
    }

    /// Check if the agent is currently paused.
    pub fn is_paused(&self) -> bool {
        self.state.is_paused.load(Ordering::Relaxed)
    }

    /// Check if the agent should stop execution.
    pub fn should_stop(&self) -> bool {
        self.state.should_stop.load(Ordering::Relaxed)
    }

    /// Pause the agent execution.
    pub async fn pause(&self) -> Result<()> {
        let (response_tx, response_rx) = oneshot::channel();

        let control_sender = self.state.control_sender.lock().await;
        if let Some(sender) = control_sender.as_ref() {
            sender
                .send(ControlCommand::Pause(response_tx))
                .map_err(|_| AgentError::ChannelSend {
                    message: "Failed to send pause command".to_string(),
                })?;

            response_rx.await.map_err(|_| AgentError::ChannelReceive {
                message: "Failed to receive pause response".to_string(),
            })?
        } else {
            Err(AgentError::Execution {
                message: "Agent controller is not active".to_string(),
            })
        }
    }

    /// Resume the agent from pause.
    pub async fn resume(&self) -> Result<()> {
        let (response_tx, response_rx) = oneshot::channel();

        let control_sender = self.state.control_sender.lock().await;
        if let Some(sender) = control_sender.as_ref() {
            sender
                .send(ControlCommand::Resume(response_tx))
                .map_err(|_| AgentError::ChannelSend {
                    message: "Failed to send resume command".to_string(),
                })?;

            response_rx.await.map_err(|_| AgentError::ChannelReceive {
                message: "Failed to receive resume response".to_string(),
            })?
        } else {
            Err(AgentError::Execution {
                message: "Agent controller is not active".to_string(),
            })
        }
    }

    /// Stop the agent execution permanently.
    pub async fn stop(&self) -> Result<()> {
        let (response_tx, response_rx) = oneshot::channel();

        let control_sender = self.state.control_sender.lock().await;
        if let Some(sender) = control_sender.as_ref() {
            sender
                .send(ControlCommand::Stop(response_tx))
                .map_err(|_| AgentError::ChannelSend {
                    message: "Failed to send stop command".to_string(),
                })?;

            response_rx.await.map_err(|_| AgentError::ChannelReceive {
                message: "Failed to receive stop response".to_string(),
            })?
        } else {
            Err(AgentError::Execution {
                message: "Agent controller is not active".to_string(),
            })
        }
    }

    /// Internal method to update the turn count.
    pub(crate) fn increment_turn_count(&self) {
        self.state.turn_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Internal method to set execution state.
    pub(crate) async fn set_execution_state(&self, state: ExecutionState) {
        let mut execution_state = self.state.execution_state.lock().await;
        *execution_state = state;
    }

    /// Internal method to handle control commands.
    pub(crate) async fn handle_control_command(&self, command: ControlCommand) {
        match command {
            ControlCommand::Pause(response_tx) => {
                self.state.is_paused.store(true, Ordering::Relaxed);
                self.set_execution_state(ExecutionState::Paused).await;
                let _ = response_tx.send(Ok(()));
            }
            ControlCommand::Resume(response_tx) => {
                self.state.is_paused.store(false, Ordering::Relaxed);
                self.set_execution_state(ExecutionState::Running).await;
                let _ = response_tx.send(Ok(()));
            }
            ControlCommand::Stop(response_tx) => {
                self.state.should_stop.store(true, Ordering::Relaxed);
                self.state.is_paused.store(false, Ordering::Relaxed);
                self.set_execution_state(ExecutionState::Stopped).await;
                let _ = response_tx.send(Ok(()));
            }
        }
    }

    /// Check if the agent can continue execution (not paused and not stopped).
    #[allow(dead_code)]
    pub(crate) fn can_continue(&self) -> bool {
        !self.is_paused() && !self.should_stop()
    }

    /// Wait for the agent to be resumed if it's currently paused.
    pub(crate) async fn wait_if_paused(&self) {
        while self.is_paused() && !self.should_stop() {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    /// Mark the agent as having encountered an error.
    pub(crate) async fn set_error<S: Into<String>>(&self, error: S) {
        self.set_execution_state(ExecutionState::Error(error.into()))
            .await;
    }
}

/// Public representation of agent execution state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentExecutionState {
    /// Current execution state
    pub execution_state: PublicExecutionState,

    /// Current turn count
    pub turn_count: u64,

    /// Whether the agent is paused
    pub is_paused: bool,

    /// Whether the agent should stop
    pub should_stop: bool,
}

/// Public execution state (without internal error details).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicExecutionState {
    /// Agent is not running
    Idle,

    /// Agent is currently executing
    Running,

    /// Agent is paused but can be resumed
    Paused,

    /// Agent has been stopped
    Stopped,

    /// Agent has encountered an error
    Error,
}

impl std::fmt::Display for PublicExecutionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublicExecutionState::Idle => write!(f, "Idle"),
            PublicExecutionState::Running => write!(f, "Running"),
            PublicExecutionState::Paused => write!(f, "Paused"),
            PublicExecutionState::Stopped => write!(f, "Stopped"),
            PublicExecutionState::Error => write!(f, "Error"),
        }
    }
}

impl From<ExecutionState> for PublicExecutionState {
    fn from(state: ExecutionState) -> Self {
        match state {
            ExecutionState::Idle => PublicExecutionState::Idle,
            ExecutionState::Running => PublicExecutionState::Running,
            ExecutionState::Paused => PublicExecutionState::Paused,
            ExecutionState::Stopped => PublicExecutionState::Stopped,
            ExecutionState::Error(_) => PublicExecutionState::Error,
        }
    }
}

impl AgentExecutionState {
    /// Check if the agent is currently running or can run.
    pub fn is_active(&self) -> bool {
        matches!(
            self.execution_state,
            PublicExecutionState::Running | PublicExecutionState::Paused
        )
    }

    /// Check if the agent has finished execution (stopped or error).
    pub fn is_finished(&self) -> bool {
        matches!(
            self.execution_state,
            PublicExecutionState::Stopped | PublicExecutionState::Error
        )
    }
}
