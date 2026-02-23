use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop columns that are no longer needed
        // Note: SQLite doesn't support DROP COLUMN, but MySQL does

        // Drop subjective metadata columns
        manager
            .alter_table(
                Table::alter()
                    .table(MemoryFragments::Table)
                    .drop_column(MemoryFragments::Importance)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MemoryFragments::Table)
                    .drop_column(MemoryFragments::Confidence)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MemoryFragments::Table)
                    .drop_column(MemoryFragments::Tags)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MemoryFragments::Table)
                    .drop_column(MemoryFragments::Notes)
                    .to_owned(),
            )
            .await?;

        // Drop associations column
        manager
            .alter_table(
                Table::alter()
                    .table(MemoryFragments::Table)
                    .drop_column(MemoryFragments::Associations)
                    .to_owned(),
            )
            .await?;

        // Drop updated_at column (we only keep timestamp)
        manager
            .alter_table(
                Table::alter()
                    .table(MemoryFragments::Table)
                    .drop_column(MemoryFragments::UpdatedAt)
                    .to_owned(),
            )
            .await?;

        // Drop old identity columns if they exist
        // These may or may not exist depending on database state
        let _ = manager
            .alter_table(
                Table::alter()
                    .table(MemoryFragments::Table)
                    .drop_column(MemoryFragments::ClaimedIdentity)
                    .to_owned(),
            )
            .await;

        let _ = manager
            .alter_table(
                Table::alter()
                    .table(MemoryFragments::Table)
                    .drop_column(MemoryFragments::AssessedIdentity)
                    .to_owned(),
            )
            .await;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Re-add dropped columns with sensible defaults

        manager
            .alter_table(
                Table::alter()
                    .table(MemoryFragments::Table)
                    .add_column(
                        ColumnDef::new(MemoryFragments::UpdatedAt)
                            .date_time()
                            .extra("(6)")
                            .not_null()
                            .default("CURRENT_TIMESTAMP(6)"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MemoryFragments::Table)
                    .add_column(
                        ColumnDef::new(MemoryFragments::Associations)
                            .text()
                            .not_null()
                            .default("[]"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MemoryFragments::Table)
                    .add_column(
                        ColumnDef::new(MemoryFragments::Notes)
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MemoryFragments::Table)
                    .add_column(
                        ColumnDef::new(MemoryFragments::Tags)
                            .text()
                            .not_null()
                            .default("[]"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MemoryFragments::Table)
                    .add_column(
                        ColumnDef::new(MemoryFragments::Confidence)
                            .integer()
                            .not_null()
                            .default(128),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(MemoryFragments::Table)
                    .add_column(
                        ColumnDef::new(MemoryFragments::Importance)
                            .integer()
                            .not_null()
                            .default(128),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
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
    ClaimedIdentity,
    AssessedIdentity,
}
