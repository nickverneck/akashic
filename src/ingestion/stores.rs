use super::{VectorStore, GraphStore, GraphDbType};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;

/// ChromaDB Vector Store (using HTTP API)
pub struct ChromaDbStore {
    base_url: String,
    collection_name: String,
    client: reqwest::Client,
}

impl ChromaDbStore {
    pub async fn new(url: &str, collection_name: &str) -> Result<Self> {
        let client = reqwest::Client::new();
        let base_url = url.trim_end_matches('/').to_string();
        
        // Create collection if it doesn't exist
        let _response = client
            .post(format!("{}/api/v1/collections", base_url))
            .json(&json!({
                "name": collection_name,
                "metadata": {}
            }))
            .send()
            .await;
        // Ignore error if collection already exists
        
        Ok(Self {
            base_url,
            collection_name: collection_name.to_string(),
            client,
        })
    }
}

#[async_trait]
impl VectorStore for ChromaDbStore {
    async fn ingest(&self, document_id: &str, text: &str, metadata: Option<serde_json::Value>) -> Result<()> {
        // Chunk the text (simple implementation - split by paragraphs)
        let chunks: Vec<&str> = text.split("\n\n").filter(|s| !s.trim().is_empty()).collect();
        
        let mut ids = Vec::new();
        let mut documents = Vec::new();
        let mut metadatas = Vec::new();
        
        for (idx, chunk) in chunks.iter().enumerate() {
            let chunk_id = format!("{}_{}", document_id, idx);
            let mut chunk_metadata = metadata.clone().unwrap_or(json!({}));
            
            if let Some(obj) = chunk_metadata.as_object_mut() {
                obj.insert("chunk_index".to_string(), json!(idx));
                obj.insert("document_id".to_string(), json!(document_id));
            }

            ids.push(chunk_id);
            documents.push(chunk.to_string());
            metadatas.push(chunk_metadata);
        }

        // Add documents to collection
        let response = self.client
            .post(format!("{}/api/v1/collections/{}/add", self.base_url, self.collection_name))
            .json(&json!({
                "ids": ids,
                "documents": documents,
                "metadatas": metadatas
            }))
            .send()
            .await
            .context("Failed to send request to ChromaDB")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("ChromaDB request failed: {}", error_text);
        }

        Ok(())
    }
}

/// Neo4j Graph Store
pub struct Neo4jStore {
    graph: neo4rs::Graph,
}

impl Neo4jStore {
    pub async fn new(uri: &str, user: &str, password: &str) -> Result<Self> {
        let graph = neo4rs::Graph::new(uri, user, password)
            .await
            .context("Failed to connect to Neo4j")?;
        
        Ok(Self { graph })
    }
}

#[async_trait]
impl GraphStore for Neo4jStore {
    async fn ingest(&self, document_id: &str, text: &str, metadata: Option<serde_json::Value>) -> Result<()> {
        // Create a Document node
        let query = neo4rs::query(
            "CREATE (d:Document {id: $id, text: $text, metadata: $metadata, created_at: datetime()})"
        )
        .param("id", document_id)
        .param("text", text)
        .param("metadata", metadata.unwrap_or(json!({})).to_string());

        self.graph.run(query).await.context("Failed to insert into Neo4j")?;

        // TODO: Add more sophisticated graph extraction (entities, relationships)
        // For now, we just create a simple document node

        Ok(())
    }
}

/// FalkorDB Graph Store (using Redis protocol)
pub struct FalkorDbStore {
    client: redis::Client,
    graph_name: String,
}

impl FalkorDbStore {
    pub async fn new(url: &str, graph_name: &str) -> Result<Self> {
        let client = redis::Client::open(url)
            .context("Failed to create FalkorDB/Redis client")?;
        
        Ok(Self {
            client,
            graph_name: graph_name.to_string(),
        })
    }
}

#[async_trait]
impl GraphStore for FalkorDbStore {
    async fn ingest(&self, document_id: &str, text: &str, metadata: Option<serde_json::Value>) -> Result<()> {
        use redis::AsyncCommands;
        
        let mut con = self.client.get_multiplexed_async_connection()
            .await
            .context("Failed to get Redis connection")?;
        
        // Create a Document node using Cypher query
        let query = format!(
            "CREATE (d:Document {{id: '{}', text: '{}', metadata: '{}', created_at: timestamp()}})",
            document_id.replace("'", "\\'"),
            text.replace("'", "\\'").chars().take(1000).collect::<String>(), // Limit text size
            metadata.unwrap_or(json!({})).to_string().replace("'", "\\'")
        );

        let _: String = redis::cmd("GRAPH.QUERY")
            .arg(&self.graph_name)
            .arg(&query)
            .query_async(&mut con)
            .await
            .context("Failed to execute FalkorDB query")?;

        Ok(())
    }
}

/// Graphiti Store (Python-based using PyO3)
#[cfg(feature = "graphiti")]
pub struct GraphitiStore {
    python_script_path: String,
}

#[cfg(feature = "graphiti")]
impl GraphitiStore {
    pub fn new(python_script_path: &str) -> Self {
        Self {
            python_script_path: python_script_path.to_string(),
        }
    }

    /// Use PyO3 to call Python directly
    pub async fn ingest_with_pyo3(&self, document_id: &str, text: &str, metadata: Option<serde_json::Value>) -> Result<()> {
        use pyo3::prelude::*;
        use pyo3::types::PyDict;

        Python::attach(|py| {
            // Import the graphiti module
            let graphiti = py.import("graphiti")?;
            
            // Create a dictionary for metadata
            let meta_dict = PyDict::new(py);
            if let Some(meta) = metadata {
                for (key, value) in meta.as_object().unwrap_or(&serde_json::Map::new()) {
                    meta_dict.set_item(key, value.to_string())?;
                }
            }

            // Call the ingestion function
            // This is a placeholder - actual Graphiti API may differ
            let _result = graphiti.call_method1(
                "ingest_document",
                (document_id, text, meta_dict)
            )?;

            Ok::<(), PyErr>(())
        }).context("Failed to call Graphiti via PyO3")?;

        Ok(())
    }
}

#[cfg(feature = "graphiti")]
#[async_trait]
impl GraphStore for GraphitiStore {
    async fn ingest(&self, document_id: &str, text: &str, metadata: Option<serde_json::Value>) -> Result<()> {
        // For now, use PyO3 approach
        self.ingest_with_pyo3(document_id, text, metadata).await
    }
}

/// Factory to create the appropriate graph store
pub async fn create_graph_store(
    db_type: GraphDbType,
    config: &serde_json::Value,
) -> Result<Box<dyn GraphStore>> {
    match db_type {
        GraphDbType::Neo4j => {
            let uri = config["uri"].as_str().context("Missing neo4j uri")?;
            let user = config["user"].as_str().context("Missing neo4j user")?;
            let password = config["password"].as_str().context("Missing neo4j password")?;
            
            Ok(Box::new(Neo4jStore::new(uri, user, password).await?))
        }
        GraphDbType::Falkordb => {
            let uri = config["uri"].as_str().context("Missing falkordb uri")?;
            let graph_name = config["graph_name"].as_str().unwrap_or("akashic");
            
            Ok(Box::new(FalkorDbStore::new(uri, graph_name).await?))
        }
        GraphDbType::Graphiti => {
            #[cfg(feature = "graphiti")]
            {
                let script_path = config["script_path"].as_str().unwrap_or("graphiti_ingest.py");
                Ok(Box::new(GraphitiStore::new(script_path)))
            }
            #[cfg(not(feature = "graphiti"))]
            {
                anyhow::bail!("Graphiti support not enabled. Rebuild with --features graphiti")
            }
        }
    }
}
