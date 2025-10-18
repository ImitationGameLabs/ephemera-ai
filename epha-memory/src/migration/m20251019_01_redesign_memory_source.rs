use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop old table and create new clean structure
        manager
            .drop_table(Table::drop().table(MemoryFragmentsV1::Table).to_owned())
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MemoryFragments::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MemoryFragments::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(MemoryFragments::Content).text().not_null())
                    .col(ColumnDef::new(MemoryFragments::CreatedAt).big_integer().not_null())
                    // New source structure: JSON with channel, identifier, metadata
                    .col(ColumnDef::new(MemoryFragments::Source).text().not_null())
                    .col(ColumnDef::new(MemoryFragments::Importance).tiny_integer().not_null())
                    .col(ColumnDef::new(MemoryFragments::Confidence).tiny_integer().not_null())
                    // JSON arrays for tags and associations
                    .col(ColumnDef::new(MemoryFragments::Tags).text().not_null())
                    .col(ColumnDef::new(MemoryFragments::Notes).text().not_null())
                    .col(ColumnDef::new(MemoryFragments::Associations).text().not_null())
                    .to_owned(),
            )
            .await?;

        // Create indexes for better query performance
        manager
            .create_index(
                Index::create()
                    .name("idx_memory_fragments_timestamp")
                    .table(MemoryFragments::Table)
                    .col(MemoryFragments::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_memory_fragments_importance")
                    .table(MemoryFragments::Table)
                    .col(MemoryFragments::Importance)
                    .to_owned(),
            )
            .await?;

        // Simple index on source for basic filtering
        manager
            .create_index(
                Index::create()
                    .name("idx_memory_fragments_source")
                    .table(MemoryFragments::Table)
                    .col(MemoryFragments::Source)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MemoryFragments::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum MemoryFragments {
    Table,
    Id,
    Content,
    CreatedAt,
    Source,
    Importance,
    Confidence,
    Tags,
    Notes,
    Associations,
}

// Old table name for reference
#[derive(DeriveIden)]
enum MemoryFragmentsV1 {
    Table,
}