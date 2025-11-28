#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use loco_rs::prelude::*;
use axum::extract::Multipart;
use serde::{Deserialize, Serialize};
use crate::models::_entities::documents::{self, Entity as Documents};
use crate::ingestion::{IngestionTarget, GraphDbType};
use sea_orm::{EntityTrait, Set, ActiveModelTrait};

#[derive(Debug, Deserialize, Serialize)]
pub struct IngestParams {
    pub target: IngestionTarget,
    pub graph_db: Option<GraphDbType>,
}

#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub document_id: i32,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub document_id: i32,
    pub filename: Option<String>,
    pub status: Option<String>,
    pub progress: Option<i32>,
    pub error_message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TextIngestRequest {
    pub text: String,
    pub target: IngestionTarget,
    pub graph_db: Option<GraphDbType>,
    pub metadata: Option<serde_json::Value>,
}

/// Upload and ingest a file
#[debug_handler]
pub async fn upload_file(
    State(ctx): State<AppContext>,
    mut multipart: Multipart,
) -> Result<Response> {
    let mut file_path: Option<String> = None;
    let mut filename: Option<String> = None;
    let mut target = IngestionTarget::Both;
    let mut graph_db: Option<GraphDbType> = None;

    // Process multipart form data
    while let Some(field) = multipart.next_field().await.map_err(|e| Error::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        
        match name.as_str() {
            "file" => {
                let field_filename = field.file_name().unwrap_or("unknown").to_string();
                filename = Some(field_filename.clone());
                
                // Save file to temp directory
                let data = field.bytes().await.map_err(|e| Error::BadRequest(e.to_string()))?;
                let temp_dir = std::env::temp_dir();
                let temp_file_path = temp_dir.join(&field_filename);
                
                tokio::fs::write(&temp_file_path, data)
                    .await
                    .map_err(|e| Error::BadRequest(e.to_string()))?;
                
                file_path = Some(temp_file_path.to_string_lossy().to_string());
            }
            "target" => {
                let text = field.text().await.map_err(|e| Error::BadRequest(e.to_string()))?;
                target = serde_json::from_str(&format!("\"{}\"", text))
                    .unwrap_or(IngestionTarget::Both);
            }
            "graph_db" => {
                let text = field.text().await.map_err(|e| Error::BadRequest(e.to_string()))?;
                graph_db = serde_json::from_str(&format!("\"{}\"", text)).ok();
            }
            _ => {}
        }
    }

    let file_path = file_path.ok_or_else(|| Error::BadRequest("No file uploaded".to_string()))?;
    let filename = filename.unwrap_or_else(|| "unknown".to_string());

    // Create document record
    let doc = documents::ActiveModel {
        filename: Set(Some(filename)),
        status: Set(Some("queued".to_string())),
        ingestion_type: Set(Some(format!("{:?}", target))),
        graph_db: Set(graph_db.as_ref().map(|g| format!("{:?}", g))),
        progress: Set(Some(0)),
        ..Default::default()
    };

    let doc = doc.insert(&ctx.db).await?;

    // Queue the ingestion job
    use crate::workers::ingest::{IngestWorker, IngestWorkerArgs};
    
    IngestWorker::perform_later(&ctx, IngestWorkerArgs {
        document_id: doc.id,
        file_path: Some(file_path.clone()),
        text: None,
        target: format!("{:?}", target),
        graph_db: graph_db.map(|g| format!("{:?}", g)),
    })
    .await?;
    
    format::json(IngestResponse {
        document_id: doc.id,
        status: "queued".to_string(),
        message: format!("File {} queued for ingestion", file_path),
    })
}

/// Ingest raw text
#[debug_handler]
pub async fn ingest_text(
    State(ctx): State<AppContext>,
    Json(req): Json<TextIngestRequest>,
) -> Result<Response> {
    // Create document record
    let doc = documents::ActiveModel {
        filename: Set(Some("text_input".to_string())),
        status: Set(Some("queued".to_string())),
        ingestion_type: Set(Some(format!("{:?}", req.target))),
        graph_db: Set(req.graph_db.as_ref().map(|g| format!("{:?}", g))),
        progress: Set(Some(0)),
        metadata: Set(req.metadata.map(|m| m.to_string())),
        ..Default::default()
    };

    let doc = doc.insert(&ctx.db).await?;

    // Queue the text ingestion job
    use crate::workers::ingest::{IngestWorker, IngestWorkerArgs};
    
    IngestWorker::perform_later(&ctx, IngestWorkerArgs {
        document_id: doc.id,
        file_path: None,
        text: Some(req.text),
        target: format!("{:?}", req.target),
        graph_db: req.graph_db.map(|g| format!("{:?}", g)),
    })
    .await?;
    
    format::json(IngestResponse {
        document_id: doc.id,
        status: "queued".to_string(),
        message: "Text queued for ingestion".to_string(),
    })
}

/// Get document status
#[debug_handler]
pub async fn status(
    State(ctx): State<AppContext>,
    Path(id): Path<i32>,
) -> Result<Response> {
    let doc = Documents::find_by_id(id)
        .one(&ctx.db)
        .await?
        .ok_or_else(|| Error::NotFound)?;

    format::json(StatusResponse {
        document_id: doc.id,
        filename: doc.filename,
        status: doc.status,
        progress: doc.progress,
        error_message: doc.error_message,
    })
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/ingest")
        .add("/file", post(upload_file))
        .add("/text", post(ingest_text))
        .add("/status/{id}", get(status))
}
