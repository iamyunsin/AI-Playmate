//! SurrealDB adapter for graph / document / session storage.
//!
//! Uses the embedded SurrealKV engine (pure Rust, no LLVM required) when no
//! remote URL is configured, making the app fully self-contained.
//!
//! ## Schema (managed in `init_schema`)
//!
//! * `message`       — raw conversation turns
//! * `memory_entry`  — compressed/distilled semantic memories
//! * `entity`        — named entities extracted from conversations
//! * `entity_relation` — directed edge: entity → entity
//! * `session`       — conversation session metadata

use serde::{de::DeserializeOwned, Serialize};
use surrealdb::{
    engine::local::{Db, SurrealKv},
    opt::auth::Root,
    Surreal,
};
use tracing::{info, instrument};

use crate::{config::StorageConfig, Result, StorageError};

pub struct SurrealStore {
    db: Surreal<Db>,
}

impl SurrealStore {
    /// Connect (or create) a local embedded SurrealDB database.
    #[instrument(skip(config))]
    pub async fn new(config: &StorageConfig) -> Result<Self> {
        let db = Surreal::new::<SurrealKv>(&config.surreal_data_dir).await?;

        db.signin(Root {
            username: &config.surreal_user,
            password: &config.surreal_pass,
        })
        .await?;

        db.use_ns(&config.surreal_namespace)
            .use_db(&config.surreal_database)
            .await?;

        let store = Self { db };
        store.init_schema().await?;
        info!("SurrealDB initialised at {}", config.surreal_data_dir);
        Ok(store)
    }

    /// Create tables and indices if they do not exist yet.
    async fn init_schema(&self) -> Result<()> {
        self.db
            .query(
                r#"
                DEFINE TABLE IF NOT EXISTS message SCHEMAFULL;
                DEFINE FIELD IF NOT EXISTS id          ON message TYPE string;
                DEFINE FIELD IF NOT EXISTS session_id  ON message TYPE string;
                DEFINE FIELD IF NOT EXISTS role        ON message TYPE string;
                DEFINE FIELD IF NOT EXISTS content     ON message TYPE string;
                DEFINE FIELD IF NOT EXISTS timestamp   ON message TYPE datetime;
                DEFINE FIELD IF NOT EXISTS metadata    ON message FLEXIBLE TYPE object;
                DEFINE INDEX IF NOT EXISTS idx_message_session ON message FIELDS session_id;

                DEFINE TABLE IF NOT EXISTS memory_entry SCHEMAFULL;
                DEFINE FIELD IF NOT EXISTS id               ON memory_entry TYPE string;
                DEFINE FIELD IF NOT EXISTS content          ON memory_entry TYPE string;
                DEFINE FIELD IF NOT EXISTS source_messages  ON memory_entry TYPE array;
                DEFINE FIELD IF NOT EXISTS importance_score ON memory_entry TYPE float;
                DEFINE FIELD IF NOT EXISTS created_at       ON memory_entry TYPE datetime;
                DEFINE FIELD IF NOT EXISTS last_accessed_at ON memory_entry TYPE datetime;
                DEFINE FIELD IF NOT EXISTS access_count     ON memory_entry TYPE int;
                DEFINE FIELD IF NOT EXISTS tags             ON memory_entry TYPE array;

                DEFINE TABLE IF NOT EXISTS entity SCHEMAFULL;
                DEFINE FIELD IF NOT EXISTS id          ON entity TYPE string;
                DEFINE FIELD IF NOT EXISTS name        ON entity TYPE string;
                DEFINE FIELD IF NOT EXISTS entity_type ON entity TYPE string;
                DEFINE FIELD IF NOT EXISTS description ON entity TYPE option<string>;
                DEFINE FIELD IF NOT EXISTS first_seen  ON entity TYPE datetime;
                DEFINE FIELD IF NOT EXISTS last_seen   ON entity TYPE datetime;
                DEFINE INDEX IF NOT EXISTS idx_entity_name ON entity FIELDS name;

                DEFINE TABLE IF NOT EXISTS entity_relation TYPE RELATION;

                DEFINE TABLE IF NOT EXISTS session SCHEMAFULL;
                DEFINE FIELD IF NOT EXISTS id         ON session TYPE string;
                DEFINE FIELD IF NOT EXISTS title      ON session TYPE option<string>;
                DEFINE FIELD IF NOT EXISTS created_at ON session TYPE datetime;
                DEFINE FIELD IF NOT EXISTS updated_at ON session TYPE datetime;
                "#,
            )
            .await?;
        Ok(())
    }

    // ── Generic CRUD ────────────────────────────────────────────────────────

    /// Insert or replace a record in `table` identified by `id`.
    pub async fn upsert<T>(&self, table: &str, id: &str, data: T) -> Result<()>
    where
        T: Serialize,
    {
        let value = serde_json::to_value(data)
            .map_err(|e| StorageError::Other(e.to_string()))?;
        let _: Option<serde_json::Value> = self
            .db
            .upsert((table, id))
            .content(value)
            .await
            .map_err(StorageError::Surreal)?;
        Ok(())
    }

    /// Fetch a single record by table + id.
    pub async fn get<T>(&self, table: &str, id: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let record: Option<T> = self
            .db
            .select((table, id))
            .await
            .map_err(StorageError::Surreal)?;
        Ok(record)
    }

    /// Execute an arbitrary SurrealQL query and deserialise the first result set.
    pub async fn query<T>(&self, sql: &str) -> Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        let mut response = self.db.query(sql).await?;
        let rows: Vec<T> = response.take(0)?;
        Ok(rows)
    }

    /// Execute a parameterised SurrealQL query.
    pub async fn query_with<T, V>(&self, sql: &str, bindings: V) -> Result<Vec<T>>
    where
        T: DeserializeOwned,
        V: Serialize,
    {
        let json_bindings: serde_json::Map<String, serde_json::Value> =
            match serde_json::to_value(bindings)
                .map_err(|e| StorageError::Other(e.to_string()))?
            {
                serde_json::Value::Object(map) => map,
                _ => {
                    return Err(StorageError::Other(
                        "query bindings must serialize to an object".to_string(),
                    ))
                }
            };
        let mut response = self.db.query(sql).bind(json_bindings).await?;
        let rows: Vec<T> = response.take(0)?;
        Ok(rows)
    }

    /// Create a `RELATE` edge between two entities.
    pub async fn relate(
        &self,
        from_table: &str,
        from_id: &str,
        relation: &str,
        to_table: &str,
        to_id: &str,
        data: serde_json::Value,
    ) -> Result<()> {
        let sql = format!(
            "RELATE {from_table}:{from_id} -> {relation} -> {to_table}:{to_id} CONTENT $data;"
        );
        self.db
            .query(sql)
            .bind(("data", data))
            .await?;
        Ok(())
    }

    /// Delete a record.
    pub async fn delete(&self, table: &str, id: &str) -> Result<()> {
        let _: Option<serde_json::Value> = self
            .db
            .delete((table, id))
            .await
            .map_err(StorageError::Surreal)?;
        Ok(())
    }
}
