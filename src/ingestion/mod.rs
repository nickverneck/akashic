pub mod extractors;
pub mod stores;
pub mod pipeline;

use async_trait::async_trait;
use anyhow::Result;

/// Trait for extracting text from different file formats
#[async_trait]
pub trait Extractor: Send + Sync {
    async fn extract(&self, file_path: &str) -> Result<String>;
    fn supports(&self, file_path: &str) -> bool;
}

/// Trait for vector database operations
#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn ingest(&self, document_id: &str, text: &str, metadata: Option<serde_json::Value>) -> Result<()>;
}

/// Trait for graph database operations
#[async_trait]
pub trait GraphStore: Send + Sync {
    async fn ingest(&self, document_id: &str, text: &str, metadata: Option<serde_json::Value>) -> Result<()>;
}

/// Ingestion target type
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IngestionTarget {
    Vector,
    Graph,
    Both,
}

/// Graph database type
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GraphDbType {
    Neo4j,
    Falkordb,
    Graphiti,
}
