use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Modify the created_at column from BIGINT to DATETIME(6)
        manager
            .alter_table(
                Table::alter()
                    .table(MemoryFragments::Table)
                    .modify_column(
                        ColumnDef::new(MemoryFragments::CreatedAt)
                            .date_time()
                            .extra("DATETIME(6)")
                            .not_null()
                    )
                    .to_owned()
            )
            .await?;

        // Add updated_at column
        manager
            .alter_table(
                Table::alter()
                    .table(MemoryFragments::Table)
                    .add_column(
                        ColumnDef::new(MemoryFragments::UpdatedAt)
                            .date_time()
                            .extra("DATETIME(6)")
                            .not_null()
                            .default("CURRENT_TIMESTAMP(6)")
                    )
                    .to_owned()
            )
            .await?;

        // Update the timestamp index to work with DATETIME
        manager
            .drop_index(
                Index::drop()
                    .name("idx_memory_fragments_timestamp")
                    .to_owned()
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_memory_fragments_timestamp")
                    .table(MemoryFragments::Table)
                    .col(MemoryFragments::CreatedAt)
                    .to_owned()
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the updated_at column
        manager
            .alter_table(
                Table::alter()
                    .table(MemoryFragments::Table)
                    .drop_column(MemoryFragments::UpdatedAt)
                    .to_owned()
            )
            .await?;

        // Convert back from DATETIME(6) to BIGINT
        manager
            .alter_table(
                Table::alter()
                    .table(MemoryFragments::Table)
                    .modify_column(
                        ColumnDef::new(MemoryFragments::CreatedAt)
                            .big_integer()
                            .not_null()
                    )
                    .to_owned()
            )
            .await?;

        // Recreate index for BIGINT timestamp
        manager
            .drop_index(
                Index::drop()
                    .name("idx_memory_fragments_timestamp")
                    .to_owned()
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_memory_fragments_timestamp")
                    .table(MemoryFragments::Table)
                    .col(MemoryFragments::CreatedAt)
                    .to_owned()
            )
            .await
    }
}

#[derive(DeriveIden)]
#[allow(unused)]
enum MemoryFragments {
    Table,
    Id,
    Content,
    CreatedAt,
    UpdatedAt,
    Source,
    Importance,
    Confidence,
    Tags,
    Notes,
    Associations,
}