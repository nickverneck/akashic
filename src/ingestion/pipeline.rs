use super::{VectorStore, GraphStore, IngestionTarget, GraphDbType};
use super::extractors::get_extractor;
use super::stores::{ChromaDbStore, create_graph_store};
use anyhow::{Context, Result};
use sea_orm::DatabaseConnection;
use crate::models::_entities::documents::{self, Entity as Documents};
use sea_orm::{EntityTrait, Set, ActiveModelTrait};

pub struct IngestionPipeline {
    db: DatabaseConnection,
    vector_store: Option<Box<dyn VectorStore>>,
    graph_store: Option<Box<dyn GraphStore>>,
}

impl IngestionPipeline {
    pub async fn new(
        db: DatabaseConnection,
        chroma_url: Option<&str>,
        graph_db_type: Option<GraphDbType>,
        graph_config: Option<serde_json::Value>,
    ) -> Result<Self> {
        let vector_store = if let Some(url) = chroma_url {
            Some(Box::new(ChromaDbStore::new(url, "akashic").await?) as Box<dyn VectorStore>)
        } else {
            None
        };

        let graph_store = if let (Some(db_type), Some(config)) = (graph_db_type, graph_config) {
            Some(create_graph_store(db_type, &config).await?)
        } else {
            None
        };

        Ok(Self {
            db,
            vector_store,
            graph_store,
        })
    }

    pub async fn process_file(
        &self,
        document_id: i32,
        file_path: &str,
        target: IngestionTarget,
    ) -> Result<()> {
        // Update status to processing
        self.update_document_status(document_id, "processing", 0).await?;

        // Extract text
        let extractor = get_extractor(file_path)
            .context("No extractor found for this file type")?;
        
        let text = extractor.extract(file_path).await
            .context("Failed to extract text from file")?;

        self.update_document_status(document_id, "processing", 30).await?;

        // Ingest based on target
        match target {
            IngestionTarget::Vector => {
                if let Some(ref store) = self.vector_store {
                    store.ingest(&document_id.to_string(), &text, None).await?;
                }
                self.update_document_status(document_id, "processing", 80).await?;
            }
            IngestionTarget::Graph => {
                if let Some(ref store) = self.graph_store {
                    store.ingest(&document_id.to_string(), &text, None).await?;
                }
                self.update_document_status(document_id, "processing", 80).await?;
            }
            IngestionTarget::Both => {
                if let Some(ref store) = self.vector_store {
                    store.ingest(&document_id.to_string(), &text, None).await?;
                }
                self.update_document_status(document_id, "processing", 60).await?;

                if let Some(ref store) = self.graph_store {
                    store.ingest(&document_id.to_string(), &text, None).await?;
                }
                self.update_document_status(document_id, "processing", 80).await?;
            }
        }

        // Mark as completed
        self.update_document_status(document_id, "completed", 100).await?;

        Ok(())
    }

    pub async fn process_text(
        &self,
        document_id: i32,
        text: &str,
        target: IngestionTarget,
    ) -> Result<()> {
        self.update_document_status(document_id, "processing", 10).await?;

        match target {
            IngestionTarget::Vector => {
                if let Some(ref store) = self.vector_store {
                    store.ingest(&document_id.to_string(), text, None).await?;
                }
            }
            IngestionTarget::Graph => {
                if let Some(ref store) = self.graph_store {
                    store.ingest(&document_id.to_string(), text, None).await?;
                }
            }
            IngestionTarget::Both => {
                if let Some(ref store) = self.vector_store {
                    store.ingest(&document_id.to_string(), text, None).await?;
                }
                if let Some(ref store) = self.graph_store {
                    store.ingest(&document_id.to_string(), text, None).await?;
                }
            }
        }

        self.update_document_status(document_id, "completed", 100).await?;

        Ok(())
    }

    async fn update_document_status(
        &self,
        document_id: i32,
        status: &str,
        progress: i32,
    ) -> Result<()> {
        let doc = Documents::find_by_id(document_id)
            .one(&self.db)
            .await?
            .context("Document not found")?;

        let mut active: documents::ActiveModel = doc.into();
        active.status = Set(Some(status.to_string()));
        active.progress = Set(Some(progress));
        active.update(&self.db).await?;

        Ok(())
    }

    pub async fn handle_error(&self, document_id: i32, error: &str) -> Result<()> {
        let doc = Documents::find_by_id(document_id)
            .one(&self.db)
            .await?
            .context("Document not found")?;

        let mut active: documents::ActiveModel = doc.into();
        active.status = Set(Some("failed".to_string()));
        active.error_message = Set(Some(error.to_string()));
        active.update(&self.db).await?;

        Ok(())
    }
}
