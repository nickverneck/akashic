use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(m, "documents",
            &[
            
            ("id", ColType::PkAuto),
            
            ("filename", ColType::StringNull),
            ("status", ColType::StringNull),
            ("ingestion_type", ColType::StringNull),
            ("graph_db", ColType::StringNull),
            ("progress", ColType::IntegerNull),
            ("metadata", ColType::TextNull),
            ("error_message", ColType::TextNull),
            ],
            &[
            ]
        ).await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "documents").await
    }
}
