//! Shared domain types for the memory system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Message ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub session_id: Uuid,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl Message {
    pub fn user(session_id: Uuid, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            role: MessageRole::User,
            content: content.into(),
            timestamp: Utc::now(),
            metadata: serde_json::Value::Null,
        }
    }

    pub fn assistant(session_id: Uuid, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            role: MessageRole::Assistant,
            content: content.into(),
            timestamp: Utc::now(),
            metadata: serde_json::Value::Null,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

// ── MemoryEntry ───────────────────────────────────────────────────────────────

/// A distilled / compressed memory that lives in the semantic layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: Uuid,
    /// Human-readable compressed representation of one or more messages.
    pub content: String,
    /// IDs of source messages this entry was distilled from.
    pub source_message_ids: Vec<Uuid>,
    /// 0.0 – 1.0 importance score (higher = more likely to be retrieved).
    pub importance_score: f32,
    pub created_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
    /// How many times this entry has been retrieved (boosts importance over time).
    pub access_count: u32,
    pub tags: Vec<String>,
    /// Embedding vector — `None` when stored in the graph layer only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

impl MemoryEntry {
    pub fn new(content: impl Into<String>, source_message_ids: Vec<Uuid>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            content: content.into(),
            source_message_ids,
            importance_score: 0.5,
            created_at: now,
            last_accessed_at: now,
            access_count: 0,
            tags: Vec::new(),
            embedding: None,
        }
    }

    pub fn record_access(&mut self) {
        self.access_count += 1;
        self.last_accessed_at = Utc::now();
        // Gradually raise importance the more often a memory is retrieved
        self.importance_score = (self.importance_score + 0.05).min(1.0);
    }
}

// ── Entity ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: Uuid,
    pub name: String,
    pub entity_type: EntityType,
    pub description: Option<String>,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EntityType {
    Person,
    Place,
    Topic,
    Object,
    Event,
    #[serde(other)]
    Other,
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Person => "person",
            Self::Place => "place",
            Self::Topic => "topic",
            Self::Object => "object",
            Self::Event => "event",
            Self::Other => "other",
        };
        write!(f, "{s}")
    }
}

// ── EntityRelation ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRelation {
    pub from_id: Uuid,
    pub to_id: Uuid,
    pub relation_type: String,
    /// 0.0 – 1.0 strength (how confident / frequently observed).
    pub strength: f32,
    pub created_at: DateTime<Utc>,
}

// ── RetrievedContext ──────────────────────────────────────────────────────────

/// The context package assembled by [`crate::retriever::AssociativeRetriever`]
/// and injected into an LLM prompt.
#[derive(Debug, Clone, Default)]
pub struct RetrievedContext {
    /// Distilled semantic memories, ranked by relevance.
    pub memories: Vec<MemoryEntry>,
    /// Entities related to the current query.
    pub relevant_entities: Vec<Entity>,
    /// Recent verbatim conversation turns.
    pub recency_context: Vec<Message>,
    /// Rough estimate of how many LLM tokens this context will consume.
    pub estimated_tokens: usize,
}

impl RetrievedContext {
    /// Render the context as a flat list of text snippets for prompt injection.
    pub fn to_snippets(&self) -> Vec<String> {
        let mut snippets = Vec::new();

        // Entities first — compact facts
        if !self.relevant_entities.is_empty() {
            let entity_lines: Vec<String> = self
                .relevant_entities
                .iter()
                .map(|e| {
                    format!(
                        "[Entity: {}] type={} {}",
                        e.name,
                        e.entity_type,
                        e.description.as_deref().unwrap_or("")
                    )
                })
                .collect();
            snippets.push(entity_lines.join("\n"));
        }

        // Semantic memories
        for mem in &self.memories {
            snippets.push(format!(
                "[Memory, importance={:.2}] {}",
                mem.importance_score, mem.content
            ));
        }

        // Recent turns
        for msg in &self.recency_context {
            snippets.push(format!("[{}] {}", msg.role.to_string().to_uppercase(), msg.content));
        }

        snippets
    }
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Assistant => write!(f, "assistant"),
            Self::System => write!(f, "system"),
        }
    }
}
