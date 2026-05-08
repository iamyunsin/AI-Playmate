//! Entity graph — named entities and their relationships stored in SurrealDB.
//!
//! Uses SurrealDB's native graph (`RELATE`) syntax so entity connections can be
//! traversed in multi-hop fashion during associative retrieval.

use chrono::Utc;
use tracing::{debug, instrument};
use uuid::Uuid;

use ai_playmate_storage::SurrealStore;

use crate::{
    types::{Entity, EntityRelation, EntityType},
    Result,
};

const ENTITY_TABLE: &str = "entity";

pub struct EntityGraph {
    store: std::sync::Arc<SurrealStore>,
}

impl EntityGraph {
    pub fn new(store: std::sync::Arc<SurrealStore>) -> Self {
        Self { store }
    }

    /// Upsert an entity.  If an entity with the same name already exists its
    /// `last_seen` timestamp is updated and the existing record is returned.
    #[instrument(skip(self))]
    pub async fn upsert_entity(
        &self,
        name: &str,
        entity_type: EntityType,
        description: Option<String>,
    ) -> Result<Entity> {
        // Look up by name first
        let sql = "SELECT * FROM entity WHERE name = $name LIMIT 1;";
        let existing: Vec<Entity> = self
            .store
            .query_with(sql, serde_json::json!({ "name": name }))
            .await?;

        if let Some(mut entity) = existing.into_iter().next() {
            entity.last_seen = Utc::now();
            if description.is_some() {
                entity.description = description;
            }
            self.store
                .upsert(ENTITY_TABLE, &entity.id.to_string(), entity.clone())
                .await?;
            return Ok(entity);
        }

        let now = Utc::now();
        let entity = Entity {
            id: Uuid::new_v4(),
            name: name.to_string(),
            entity_type,
            description,
            first_seen: now,
            last_seen: now,
        };
        self.store
            .upsert(ENTITY_TABLE, &entity.id.to_string(), entity.clone())
            .await?;
        debug!(name = %name, "Created entity");
        Ok(entity)
    }

    /// Create a directed relation between two entities.
    pub async fn relate_entities(&self, relation: &EntityRelation) -> Result<()> {
        self.store
            .relate(
                ENTITY_TABLE,
                &relation.from_id.to_string(),
                &relation.relation_type,
                ENTITY_TABLE,
                &relation.to_id.to_string(),
                serde_json::json!({
                    "strength": relation.strength,
                    "created_at": relation.created_at,
                }),
            )
            .await?;
        Ok(())
    }

    /// Find entities related to the given `entity_id` within `hops` traversal steps.
    pub async fn related_entities(&self, entity_id: Uuid, hops: u8) -> Result<Vec<Entity>> {
        // Build a graph traversal query
        // e.g. SELECT ->knows->entity.* FROM entity:<id>
        // For simplicity we use a 1-hop or 2-hop expansion
        let _direction = "->knows->";
        let sql = if hops == 1 {
            format!(
                "SELECT ->*->entity.* AS related FROM {ENTITY_TABLE}:$id FETCH related;"
            )
        } else {
            format!(
                "SELECT ->*->entity.*->*->entity.* AS related FROM {ENTITY_TABLE}:$id FETCH related;"
            )
        };

        let rows: Vec<serde_json::Value> = self
            .store
            .query_with(&sql, serde_json::json!({ "id": entity_id.to_string() }))
            .await?;

        let entities = rows
            .into_iter()
            .filter_map(|v| {
                v.get("related")
                    .and_then(|r| serde_json::from_value::<Vec<Entity>>(r.clone()).ok())
            })
            .flatten()
            .collect();

        Ok(entities)
    }

    /// Search for entities whose name contains the given query string.
    pub async fn search_by_name(&self, query: &str, limit: usize) -> Result<Vec<Entity>> {
        let sql = format!(
            "SELECT * FROM {ENTITY_TABLE} WHERE name CONTAINS $q LIMIT $lim;"
        );
        self.store
            .query_with(
                &sql,
                serde_json::json!({ "q": query, "lim": limit }),
            )
            .await
            .map_err(Into::into)
    }
}
