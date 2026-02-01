//! Summary struct - the core structured output from the LLM agent.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Structured summary output from the LLM.
///
/// This schema is enforced by rstructor, ensuring the LLM returns valid data.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Summary {
    /// Title or headline for the summarized content
    pub title: String,
    /// Main conclusion or takeaway from the content
    pub conclusion: String,
    /// Main takeaways from the content
    pub key_points: Vec<String>,
    /// Named entities mentioned (people, organizations, technologies, etc.)
    pub entities: Vec<String>,
    /// Actionable items or next steps identified in the content
    pub action_items: Vec<String>,
}

impl Summary {
    /// Create a new summary
    pub fn new(
        title: String,
        conclusion: String,
        key_points: Vec<String>,
        entities: Vec<String>,
        action_items: Vec<String>,
    ) -> Self {
        Self {
            title,
            conclusion,
            key_points,
            entities,
            action_items,
        }
    }

    /// Check if the summary has any content
    pub fn is_empty(&self) -> bool {
        self.conclusion.is_empty()
            && self.key_points.is_empty()
            && self.entities.is_empty()
            && self.action_items.is_empty()
    }
}
