//! Qdrant vector store adapter.
//!
//! Handles creation of the collection (if absent), upsert of vectors, and
//! approximate-nearest-neighbour search.  Each point payload carries the raw
//! `serde_json::Value` metadata so callers can reconstruct domain objects.

use qdrant_client::{
    qdrant::{
        CreateCollectionBuilder, Distance, PointStruct, SearchPointsBuilder,
        UpsertPointsBuilder, VectorParamsBuilder,
    },
    Qdrant,
};
use serde_json::Value;
use tracing::{debug, info, instrument};
use uuid::Uuid;

use crate::{config::StorageConfig, Result, StorageError};

pub struct QdrantStore {
    client: Qdrant,
    collection: String,
    vector_size: u64,
}

impl QdrantStore {
    /// Connect to Qdrant (remote or in-process) and ensure the collection exists.
    #[instrument(skip(config))]
    pub async fn new(config: &StorageConfig) -> Result<Self> {
        let client = if let Some(url) = &config.qdrant_url {
            let mut builder = Qdrant::from_url(url);
            if let Some(key) = &config.qdrant_api_key {
                builder = builder.api_key(key.clone());
            }
            builder.build()?
        } else {
            // In-memory Qdrant (useful for tests / Qdrant Edge)
            Qdrant::from_url("http://localhost:6334").build()?
        };

        let store = Self {
            client,
            collection: config.qdrant_collection.clone(),
            vector_size: config.vector_size,
        };

        store.ensure_collection().await?;
        Ok(store)
    }

    async fn ensure_collection(&self) -> Result<()> {
        let exists = self
            .client
            .collection_exists(&self.collection)
            .await?;

        if !exists {
            info!(collection = %self.collection, "Creating Qdrant collection");
            self.client
                .create_collection(
                    CreateCollectionBuilder::new(&self.collection).vectors_config(
                        VectorParamsBuilder::new(self.vector_size, Distance::Cosine),
                    ),
                )
                .await?;
        }
        Ok(())
    }

    /// Upsert a single embedding point with associated metadata payload.
    #[instrument(skip(self, vector, payload))]
    pub async fn upsert(
        &self,
        id: Uuid,
        vector: Vec<f32>,
        payload: Value,
    ) -> Result<()> {
        debug!(id = %id, "Upserting vector point");
        let point = PointStruct::new(
            id.to_string(),
            vector,
            serde_json::from_value::<std::collections::HashMap<String, qdrant_client::qdrant::Value>>(
                payload,
            )
            .map_err(|e| StorageError::Other(e.to_string()))?,
        );

        self.client
            .upsert_points(UpsertPointsBuilder::new(&self.collection, vec![point]).wait(true))
            .await?;
        Ok(())
    }

    /// Search for the `top_k` nearest neighbours to the given query vector.
    ///
    /// Returns a list of `(id, score, payload)` tuples ordered by descending similarity.
    #[instrument(skip(self, query_vector))]
    pub async fn search(
        &self,
        query_vector: Vec<f32>,
        top_k: u64,
        score_threshold: Option<f32>,
    ) -> Result<Vec<(Uuid, f32, Value)>> {
        let mut builder =
            SearchPointsBuilder::new(&self.collection, query_vector, top_k).with_payload(true);

        if let Some(threshold) = score_threshold {
            builder = builder.score_threshold(threshold);
        }

        let result = self.client.search_points(builder).await?;

        let hits = result
            .result
            .into_iter()
            .filter_map(|point| {
                let id = point.id?.point_id_options?;
                let uuid = match id {
                    qdrant_client::qdrant::point_id::PointIdOptions::Uuid(u) => {
                        Uuid::parse_str(&u).ok()?
                    }
                    _ => return None,
                };
                let payload =
                    serde_json::to_value(&point.payload).unwrap_or(Value::Null);
                Some((uuid, point.score, payload))
            })
            .collect();

        Ok(hits)
    }

    /// Delete a point by ID.
    pub async fn delete(&self, id: Uuid) -> Result<()> {
        use qdrant_client::qdrant::{DeletePointsBuilder, PointsIdsList};
        self.client
            .delete_points(
                DeletePointsBuilder::new(&self.collection).points(PointsIdsList {
                    ids: vec![qdrant_client::qdrant::PointId {
                        point_id_options: Some(
                            qdrant_client::qdrant::point_id::PointIdOptions::Uuid(
                                id.to_string(),
                            ),
                        ),
                    }],
                }),
            )
            .await?;
        Ok(())
    }
}
