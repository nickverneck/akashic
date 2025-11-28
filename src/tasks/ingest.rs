use loco_rs::prelude::*;
use crate::ingestion::{IngestionTarget, GraphDbType, pipeline::IngestionPipeline};
use crate::models::_entities::documents;
use sea_orm::{Set, ActiveModelTrait};
use std::io::{self, Read};

pub struct IngestTask;

#[async_trait]
impl Task for IngestTask {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "ingest".to_string(),
            detail: "Ingest files or text into vector/graph databases".to_string(),
        }
    }

    async fn run(&self, app_context: &AppContext, vars: &task::Vars) -> Result<()> {
        // Parse arguments - Vars is a HashMap-like structure
        let cli_args: Vec<String> = std::env::args().collect();
        
        // Simple argument parsing
        let mut file_path: Option<String> = None;
        let mut stdin = false;
        let mut target = IngestionTarget::Both;
        let mut graph_db: Option<GraphDbType> = None;
        
        let mut i = 0;
        while i < cli_args.len() {
            match cli_args[i].as_str() {
                "--file" | "-f" => {
                    if i + 1 < cli_args.len() {
                        file_path = Some(cli_args[i + 1].clone());
                        i += 1;
                    }
                }
                "--stdin" => {
                    stdin = true;
                }
                "--target" | "-t" => {
                    if i + 1 < cli_args.len() {
                        target = serde_json::from_str(&format!("\"{}\"", cli_args[i + 1]))
                            .unwrap_or(IngestionTarget::Both);
                        i += 1;
                    }
                }
                "--graph-db" | "-g" => {
                    if i + 1 < cli_args.len() {
                        graph_db = serde_json::from_str(&format!("\"{}\"", cli_args[i + 1])).ok();
                        i += 1;
                    }
                }
                _ => {}
            }
            i += 1;
        }

        // Get text from stdin or file
        let (text, filename) = if stdin {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)
                .map_err(|e| Error::string(&format!("Failed to read from stdin: {}", e)))?;
            (Some(buffer), "stdin".to_string())
        } else if let Some(ref path) = file_path {
            (None, path.clone())
        } else {
            return Err(Error::string("Either --file or --stdin must be provided"));
        };

        // Create document record
        let doc = documents::ActiveModel {
            filename: Set(Some(filename.clone())),
            status: Set(Some("processing".to_string())),
            ingestion_type: Set(Some(format!("{:?}", target))),
            graph_db: Set(graph_db.as_ref().map(|g| format!("{:?}", g))),
            progress: Set(Some(0)),
            ..Default::default()
        };

        let doc = doc.insert(&app_context.db).await?;
        
        println!("Created document record with ID: {}", doc.id);

        // Get configuration from environment
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
            app_context.db.clone(),
            chroma_url.as_deref(),
            graph_db,
            graph_config,
        )
        .await
        .map_err(|e| Error::string(&format!("Failed to create pipeline: {}", e)))?;

        // Process
        println!("Starting ingestion...");
        let result = if let Some(text_content) = text {
            pipeline.process_text(doc.id, &text_content, target).await
        } else {
            pipeline.process_file(doc.id, &filename, target).await
        };

        match result {
            Ok(_) => {
                println!("✓ Ingestion completed successfully for document {}", doc.id);
                Ok(())
            }
            Err(e) => {
                eprintln!("✗ Ingestion failed: {}", e);
                pipeline.handle_error(doc.id, &e.to_string()).await
                    .map_err(|e| Error::string(&format!("Failed to update error: {}", e)))?;
                Err(Error::string(&format!("Ingestion failed: {}", e)))
            }
        }
    }
}
