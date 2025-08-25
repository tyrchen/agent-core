//! Plan management system with MPSC channels for task tracking.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Plan message sent through MPSC channels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanMessage {
    /// List of todo items in the current plan
    pub todos: Vec<TodoItem>,

    /// Optional metadata about the plan
    pub metadata: Option<PlanMetadata>,

    /// Timestamp when the plan was updated
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl PlanMessage {
    /// Create a new plan message.
    pub fn new(todos: Vec<TodoItem>) -> Self {
        Self {
            todos,
            metadata: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create a new plan message with metadata.
    pub fn with_metadata(todos: Vec<TodoItem>, metadata: PlanMetadata) -> Self {
        Self {
            todos,
            metadata: Some(metadata),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Get completed todos.
    pub fn completed_todos(&self) -> Vec<&TodoItem> {
        self.todos
            .iter()
            .filter(|todo| todo.status == TodoStatus::Completed)
            .collect()
    }

    /// Get pending todos.
    pub fn pending_todos(&self) -> Vec<&TodoItem> {
        self.todos
            .iter()
            .filter(|todo| todo.status == TodoStatus::Pending)
            .collect()
    }

    /// Get in-progress todos.
    pub fn in_progress_todos(&self) -> Vec<&TodoItem> {
        self.todos
            .iter()
            .filter(|todo| todo.status == TodoStatus::InProgress)
            .collect()
    }

    /// Get blocked todos.
    pub fn blocked_todos(&self) -> Vec<&TodoItem> {
        self.todos
            .iter()
            .filter(|todo| todo.status == TodoStatus::Blocked)
            .collect()
    }

    /// Get completion percentage (0.0 to 1.0).
    pub fn completion_percentage(&self) -> f32 {
        if self.todos.is_empty() {
            return 1.0;
        }

        let completed_count = self.completed_todos().len() as f32;
        completed_count / self.todos.len() as f32
    }
}

/// Individual todo item in a plan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TodoItem {
    /// Unique identifier for the todo item
    pub id: uuid::Uuid,

    /// Content/description of the task
    pub content: String,

    /// Current status of the task
    pub status: TodoStatus,

    /// Optional priority level (1-5, where 5 is highest priority)
    pub priority: Option<u8>,

    /// Optional tags for categorizing tasks
    pub tags: Vec<String>,

    /// When the task was created
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// When the task was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,

    /// Optional due date
    pub due_date: Option<chrono::DateTime<chrono::Utc>>,

    /// Optional estimated effort (in hours)
    pub estimated_hours: Option<f32>,

    /// Optional additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl TodoItem {
    /// Create a new todo item with content.
    pub fn new<S: Into<String>>(content: S) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4(),
            content: content.into(),
            status: TodoStatus::Pending,
            priority: None,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            due_date: None,
            estimated_hours: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the priority level (1-5).
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = Some(priority.clamp(1, 5));
        self.updated_at = chrono::Utc::now();
        self
    }

    /// Add tags to the todo item.
    pub fn with_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags.extend(tags.into_iter().map(|s| s.into()));
        self.updated_at = chrono::Utc::now();
        self
    }

    /// Set the due date.
    pub fn with_due_date(mut self, due_date: chrono::DateTime<chrono::Utc>) -> Self {
        self.due_date = Some(due_date);
        self.updated_at = chrono::Utc::now();
        self
    }

    /// Set estimated hours for completion.
    pub fn with_estimated_hours(mut self, hours: f32) -> Self {
        self.estimated_hours = Some(hours);
        self.updated_at = chrono::Utc::now();
        self
    }

    /// Update the status of the todo item.
    pub fn update_status(&mut self, status: TodoStatus) {
        self.status = status;
        self.updated_at = chrono::Utc::now();
    }

    /// Mark the todo as completed.
    pub fn complete(&mut self) {
        self.update_status(TodoStatus::Completed);
    }

    /// Mark the todo as in progress.
    pub fn start(&mut self) {
        self.update_status(TodoStatus::InProgress);
    }

    /// Mark the todo as blocked.
    pub fn block(&mut self) {
        self.update_status(TodoStatus::Blocked);
    }

    /// Reset the todo to pending.
    pub fn reset(&mut self) {
        self.update_status(TodoStatus::Pending);
    }

    /// Check if the todo is overdue.
    pub fn is_overdue(&self) -> bool {
        if let Some(due_date) = self.due_date {
            chrono::Utc::now() > due_date && self.status != TodoStatus::Completed
        } else {
            false
        }
    }

    /// Add metadata to the todo item.
    pub fn add_metadata<K, V>(&mut self, key: K, value: V) -> Result<(), serde_json::Error>
    where
        K: Into<String>,
        V: Serialize,
    {
        let json_value = serde_json::to_value(value)?;
        self.metadata.insert(key.into(), json_value);
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    /// Get metadata value.
    pub fn get_metadata<T>(&self, key: &str) -> Option<Result<T, serde_json::Error>>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.metadata
            .get(key)
            .map(|value| serde_json::from_value(value.clone()))
    }
}

/// Status of a todo item.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    /// Task is pending and hasn't been started
    Pending,

    /// Task is currently being worked on
    InProgress,

    /// Task has been completed successfully
    Completed,

    /// Task is blocked by dependencies or external factors
    Blocked,
}

impl std::fmt::Display for TodoStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TodoStatus::Pending => write!(f, "Pending"),
            TodoStatus::InProgress => write!(f, "In Progress"),
            TodoStatus::Completed => write!(f, "Completed"),
            TodoStatus::Blocked => write!(f, "Blocked"),
        }
    }
}

/// Optional metadata for plan messages.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanMetadata {
    /// Name or title of the plan
    pub name: Option<String>,

    /// Description of the plan's purpose
    pub description: Option<String>,

    /// Plan version or iteration number
    pub version: Option<u32>,

    /// Who or what created this plan
    pub created_by: Option<String>,

    /// Tags for categorizing plans
    pub tags: Vec<String>,

    /// Additional custom metadata
    pub custom: HashMap<String, serde_json::Value>,
}

impl PlanMetadata {
    /// Create a new plan metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the plan name.
    pub fn with_name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the plan description.
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the plan version.
    pub fn with_version(mut self, version: u32) -> Self {
        self.version = Some(version);
        self
    }

    /// Set who created the plan.
    pub fn with_created_by<S: Into<String>>(mut self, created_by: S) -> Self {
        self.created_by = Some(created_by.into());
        self
    }

    /// Add tags to the plan.
    pub fn with_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags.extend(tags.into_iter().map(|s| s.into()));
        self
    }

    /// Add custom metadata.
    pub fn with_custom<K, V>(mut self, key: K, value: V) -> Result<Self, serde_json::Error>
    where
        K: Into<String>,
        V: Serialize,
    {
        let json_value = serde_json::to_value(value)?;
        self.custom.insert(key.into(), json_value);
        Ok(self)
    }
}
