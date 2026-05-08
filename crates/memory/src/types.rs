//! Shared domain types for the memory system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Custom serde module for SurrealDB datetime compatibility ─────────────────
//
// chrono::DateTime<Utc> normally serializes as an RFC 3339 *string*, which
// SurrealDB's SCHEMAFULL `TYPE datetime` rejects.  By emitting
// `serialize_newtype_struct("Datetime", ...)` the SurrealDB serializer
// recognises the value as a native datetime instead of a Strand (string).
// For non-SurrealDB contexts (serde_json, etc.) newtype structs are
// transparent, so the outer string value is preserved.
mod surreal_dt {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    // Must match surrealdb-core's TOKEN constant:
    // `pub(crate) const TOKEN: &str = "$surrealdb::private::sql::Datetime";`
    // and `#[serde(rename = "$surrealdb::private::sql::Datetime")]` on Datetime struct.
    const TOKEN: &str = "$surrealdb::private::sql::Datetime";

    pub fn serialize<S: Serializer>(dt: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_newtype_struct(TOKEN, dt)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<DateTime<Utc>, D::Error> {
        DateTime::deserialize(d)
    }
}

// ── surreal_id ────────────────────────────────────────────────────────────────
//
// SurrealDB record-ID field (`id`).
// * Serialization is SKIPPED — the record ID is specified separately in the
//   `.upsert((table, id))` call; including it in the content causes the error
//   "a specific record has been specified".
// * Deserialization handles the `table:uuid-string` Thing format that SurrealDB
//   returns when reading records back.
mod surreal_id {
    use serde::{de, de::VariantAccess, Deserialize, Deserializer};
    use uuid::Uuid;

    // Not called — combined with #[serde(skip_serializing)].
    #[allow(dead_code)]
    pub fn serialize<S: serde::Serializer>(_: &Uuid, _: S) -> Result<S::Ok, S::Error> {
        unreachable!("surreal_id::serialize should never be called")
    }

    /// Reads a SurrealDB `Id` value (which serde_content represents as an enum)
    /// and extracts it as a String.  Handles `Id::String(s)` and `Id::Uuid(u)`.
    struct SurrealIdString(String);

    impl<'de> Deserialize<'de> for SurrealIdString {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            struct V;
            impl<'de> de::Visitor<'de> for V {
                type Value = String;
                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("a string or SurrealDB Id enum variant")
                }
                fn visit_str<E: de::Error>(self, v: &str) -> Result<String, E> {
                    Ok(v.to_string())
                }
                fn visit_string<E: de::Error>(self, v: String) -> Result<String, E> {
                    Ok(v)
                }
                // serde_content deserializes Id::String as Content::Enum { variant: "String", data: NewType(String) }
                fn visit_enum<A: de::EnumAccess<'de>>(self, data: A) -> Result<String, A::Error> {
                    let (variant, access): (String, _) = data.variant()?;
                    match variant.as_str() {
                        "String" => access.newtype_variant::<String>(),
                        "Uuid" => {
                            let u = access.newtype_variant::<Uuid>()?;
                            Ok(u.to_string())
                        }
                        _ => Err(de::Error::custom(format!(
                            "unsupported SurrealDB Id variant: {variant}"
                        ))),
                    }
                }
            }
            d.deserialize_any(V).map(SurrealIdString)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Uuid, D::Error> {
        struct V;
        impl<'de> de::Visitor<'de> for V {
            type Value = Uuid;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a UUID string or SurrealDB Thing")
            }
            // Plain string or "table:uuid" Thing-as-string
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Uuid, E> {
                let uuid_part = v.split_once(':').map_or(v, |(_, id)| id);
                Uuid::parse_str(uuid_part).map_err(de::Error::custom)
            }
            // SurrealDB Thing struct: { "tb": "message", "id": <Id enum> }
            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Uuid, A::Error> {
                let mut found: Option<String> = None;
                while let Some(key) = map.next_key::<String>()? {
                    if key == "id" {
                        // Id is a serde_content Enum — use SurrealIdString to decode it
                        let SurrealIdString(s) = map.next_value()?;
                        found = Some(s);
                    } else {
                        // Skip other fields (e.g. "tb") without deserializing into json::Value
                        map.next_value::<de::IgnoredAny>()?;
                    }
                }
                let s = found.ok_or_else(|| de::Error::custom("missing id in Thing"))?;
                Uuid::parse_str(&s).map_err(de::Error::custom)
            }
        }
        d.deserialize_any(V)
    }
}

// ── surreal_uuid ──────────────────────────────────────────────────────────────
//
// For UUID fields that are NOT the record ID (e.g. `session_id`).
// SurrealDB's serializer is non-human-readable so uuid::Uuid would encode as
// bytes; we force a string here to match `TYPE string` schema fields.
// On read-back, serde-content Deserializer has is_human_readable()=false, so
// Uuid::deserialize would call deserialize_bytes and fail on a string.
// We use deserialize_any + a visitor that handles both strings and bytes.
mod surreal_uuid {
    use serde::{de, Deserializer, Serializer};
    use uuid::Uuid;

    pub fn serialize<S: Serializer>(id: &Uuid, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&id.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Uuid, D::Error> {
        struct V;
        impl<'de> de::Visitor<'de> for V {
            type Value = Uuid;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a UUID string or bytes")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Uuid, E> {
                Uuid::parse_str(v).map_err(de::Error::custom)
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Uuid, E> {
                Uuid::parse_str(&v).map_err(de::Error::custom)
            }
            fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Uuid, E> {
                Uuid::from_slice(v).map_err(de::Error::custom)
            }
        }
        d.deserialize_any(V)
    }
}

// ── surreal_uuid_vec ──────────────────────────────────────────────────────────
mod surreal_uuid_vec {
    use serde::ser::SerializeSeq;
    use serde::{de, Deserializer, Serializer};
    use uuid::Uuid;

    pub fn serialize<S: Serializer>(ids: &[Uuid], s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(ids.len()))?;
        for id in ids {
            seq.serialize_element(&id.to_string())?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Uuid>, D::Error> {
        struct SeqV;
        impl<'de> de::Visitor<'de> for SeqV {
            type Value = Vec<Uuid>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a sequence of UUID strings")
            }
            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<Uuid>, A::Error> {
                let mut out = Vec::new();
                while let Some(s) = seq.next_element::<String>()? {
                    out.push(Uuid::parse_str(&s).map_err(de::Error::custom)?);
                }
                Ok(out)
            }
        }
        d.deserialize_seq(SeqV)
    }
}

// ── Message ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    #[serde(skip_serializing, deserialize_with = "surreal_id::deserialize")]
    pub id: Uuid,
    #[serde(with = "surreal_uuid")]
    pub session_id: Uuid,
    pub role: MessageRole,
    pub content: String,
    #[serde(with = "surreal_dt")]
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
            metadata: serde_json::json!({}),
        }
    }

    pub fn assistant(session_id: Uuid, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            role: MessageRole::Assistant,
            content: content.into(),
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

// SurrealDB serializer is non-human-readable; unit-variant enums would be
// encoded as integer indices.  Force string output so TYPE string fields work.
impl serde::Serialize for MessageRole {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::System => "system",
        })
    }
}

// ── MemoryEntry ───────────────────────────────────────────────────────────────

/// A distilled / compressed memory that lives in the semantic layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    #[serde(skip_serializing, deserialize_with = "surreal_id::deserialize")]
    pub id: Uuid,
    /// Human-readable compressed representation of one or more messages.
    pub content: String,
    /// IDs of source messages this entry was distilled from.
    #[serde(with = "surreal_uuid_vec")]
    pub source_message_ids: Vec<Uuid>,
    /// 0.0 – 1.0 importance score (higher = more likely to be retrieved).
    pub importance_score: f32,
    #[serde(with = "surreal_dt")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "surreal_dt")]
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
    #[serde(skip_serializing, deserialize_with = "surreal_id::deserialize")]
    pub id: Uuid,
    pub name: String,
    pub entity_type: EntityType,
    pub description: Option<String>,
    #[serde(with = "surreal_dt")]
    pub first_seen: DateTime<Utc>,
    #[serde(with = "surreal_dt")]
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
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

// Same reason as MessageRole — force string output for SurrealDB.
impl serde::Serialize for EntityType {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(match self {
            Self::Person => "person",
            Self::Place => "place",
            Self::Topic => "topic",
            Self::Object => "object",
            Self::Event => "event",
            Self::Other => "other",
        })
    }
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
    #[serde(with = "surreal_dt")]
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
