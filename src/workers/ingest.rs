use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use crate::ingestion::{IngestionTarget, GraphDbType, pipeline::IngestionPipeline};
use crate::models::_entities::documents::Entity as Documents;
use sea_orm::EntityTrait;

#[derive(Deserialize, Debug, Serialize)]
pub struct IngestWorkerArgs {
    pub document_id: i32,
    pub file_path: Option<String>,
    pub text: Option<String>,
    pub target: String,
    pub graph_db: Option<String>,
}

pub struct IngestWorker {
    pub ctx: AppContext,
}

impl IngestWorker {
    pub fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }
}

#[async_trait]
impl BackgroundWorker<IngestWorkerArgs> for IngestWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    async fn perform(&self, args: IngestWorkerArgs) -> Result<()> {
        tracing::info!("Processing ingestion for document {}", args.document_id);

        // Parse target and graph_db
        let target: IngestionTarget = serde_json::from_str(&format!("\"{}\"", args.target))
            .map_err(|e| Error::BadRequest(e.to_string()))?;
        
        let graph_db: Option<GraphDbType> = args.graph_db
            .and_then(|g| serde_json::from_str(&format!("\"{}\"", g)).ok());

        // Get configuration from environment or config
        let chroma_url = std::env::var("CHROMA_URL").ok();
        let graph_config = if let Some(ref db_type) = graph_db {
            Some(match db_type {
                GraphDbType::Neo4j => {
                    serde_json::json!({
                        "uri": std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string()),
                        "user": std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string()),
                        "password": std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "password".to_string()),
                    })
                }
                GraphDbType::Falkordb => {
                    serde_json::json!({
                        "uri": std::env::var("FALKORDB_URI").unwrap_or_else(|_| "redis://localhost:6379".to_string()),
                        "graph_name": std::env::var("FALKORDB_GRAPH").unwrap_or_else(|_| "akashic".to_string()),
                    })
                }
                GraphDbType::Graphiti => {
                    serde_json::json!({
                        "script_path": std::env::var("GRAPHITI_SCRIPT").unwrap_or_else(|_| "graphiti_ingest.py".to_string()),
                    })
                }
            })
        } else {
            None
        };

        // Create pipeline
        let pipeline = IngestionPipeline::new(
            self.ctx.db.clone(),
            chroma_url.as_deref(),
            graph_db,
            graph_config,
        )
        .await
        .map_err(|e| Error::BadRequest(e.to_string()))?;

        // Process based on whether we have a file or text
        let result = if let Some(file_path) = args.file_path {
            pipeline.process_file(args.document_id, &file_path, target).await
        } else if let Some(text) = args.text {
            pipeline.process_text(args.document_id, &text, target).await
        } else {
            Err(anyhow::anyhow!("Neither file_path nor text provided"))
        };

        // Handle errors
        if let Err(e) = result {
            tracing::error!("Ingestion failed for document {}: {}", args.document_id, e);
            pipeline.handle_error(args.document_id, &e.to_string()).await
                .map_err(|e| Error::BadRequest(e.to_string()))?;
            return Err(Error::BadRequest(e.to_string()));
        }

        tracing::info!("Successfully processed document {}", args.document_id);
        Ok(())
    }
}
