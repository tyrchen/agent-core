//! Utility functions for text processing and output formatting (optional feature).

/// Text processing utilities for agent outputs.
pub mod processing {
    /// Clean and normalize agent output text.
    pub fn clean_output(raw_output: &str) -> String {
        // TODO: Implement output cleaning
        // This could include:
        // - Removing extra whitespace
        // - Normalizing line endings
        // - Removing control characters
        // - Fixing encoding issues

        raw_output.trim().to_string()
    }

    /// Format code with syntax highlighting and proper indentation.
    pub fn format_code(code: &str) -> String {
        // TODO: Implement code formatting
        // This could include:
        // - Language detection
        // - Syntax highlighting
        // - Proper indentation
        // - Code beautification

        code.to_string()
    }

    /// Extract structured data from agent responses.
    pub fn extract_structured_data(text: &str) -> Option<serde_json::Value> {
        // TODO: Implement structured data extraction
        // This could parse JSON, YAML, or other structured formats from text

        serde_json::from_str(text).ok()
    }

    /// Convert markdown to HTML.
    pub fn markdown_to_html(markdown: &str) -> String {
        // TODO: Implement markdown conversion
        // This would convert markdown text to HTML for display

        markdown.to_string()
    }

    /// Truncate text to a maximum length while preserving word boundaries.
    pub fn truncate_text(text: &str, max_length: usize) -> String {
        if text.len() <= max_length {
            return text.to_string();
        }

        let truncated = &text[..max_length];
        if let Some(last_space) = truncated.rfind(' ') {
            format!("{}...", &truncated[..last_space])
        } else {
            format!("{}...", &text[..max_length.saturating_sub(3)])
        }
    }

    /// Count tokens in text (approximate).
    pub fn count_tokens(text: &str) -> usize {
        // Very rough approximation: 1 token ≈ 4 characters
        // Real implementation would use a proper tokenizer
        (text.len() + 3) / 4
    }
}

/// Performance monitoring utilities.
pub mod performance {
    use std::time::{Duration, Instant};

    /// Simple performance timer.
    pub struct Timer {
        start: Instant,
        name: String,
    }

    impl Timer {
        /// Start a new timer with a name.
        pub fn new<S: Into<String>>(name: S) -> Self {
            Self {
                start: Instant::now(),
                name: name.into(),
            }
        }

        /// Get elapsed time since timer was started.
        pub fn elapsed(&self) -> Duration {
            self.start.elapsed()
        }

        /// Stop the timer and return elapsed time.
        pub fn stop(self) -> Duration {
            let elapsed = self.elapsed();
            tracing::debug!("Timer '{}' completed in {:?}", self.name, elapsed);
            elapsed
        }
    }

    /// Memory usage statistics.
    #[derive(Debug, Clone)]
    pub struct MemoryStats {
        /// Memory usage in bytes
        pub bytes: usize,

        /// Human-readable memory usage
        pub formatted: String,
    }

    impl MemoryStats {
        /// Get current memory usage (placeholder implementation).
        pub fn current() -> Self {
            // TODO: Implement actual memory usage measurement
            Self {
                bytes: 0,
                formatted: "Unknown".to_string(),
            }
        }
    }

    /// Format bytes as human-readable size.
    pub fn format_bytes(bytes: usize) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }
}

/// Logging and debug utilities.
pub mod debug {
    use crate::messages::{InputMessage, OutputMessage};

    /// Log an input message for debugging.
    pub fn log_input(input: &InputMessage) {
        tracing::debug!(
            "Input: {} (with {} images)",
            input.message,
            input.images.len()
        );
    }

    /// Log an output message for debugging.
    pub fn log_output(output: &OutputMessage) {
        tracing::debug!("Output [Turn {}]: {:?}", output.turn_id, output.data);
    }

    /// Create a debug dump of agent state.
    pub fn dump_agent_state(_agent: &crate::agent::Agent) -> String {
        // TODO: Implement agent state dumping for debugging
        "Agent state dump not yet implemented".to_string()
    }
}
