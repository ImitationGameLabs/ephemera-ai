use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
                    .col(ColumnDef::new(MemoryFragments::CreatedAt).date_time().extra("(6)").not_null())
                    .col(ColumnDef::new(MemoryFragments::UpdatedAt).date_time().extra("(6)").not_null())
                    .col(ColumnDef::new(MemoryFragments::Source).text().not_null())
                    .col(ColumnDef::new(MemoryFragments::Importance).integer().not_null())
                    .col(ColumnDef::new(MemoryFragments::Confidence).integer().not_null())
                    .col(ColumnDef::new(MemoryFragments::Tags).text().not_null())
                    .col(ColumnDef::new(MemoryFragments::Notes).text().not_null())
                    .col(ColumnDef::new(MemoryFragments::Associations).text().not_null())
                    .col(ColumnDef::new(MemoryFragments::ClaimedIdentity).text().not_null())
                    .col(ColumnDef::new(MemoryFragments::AssessedIdentity).text().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_memory_fragments_timestamp")
                    .table(MemoryFragments::Table)
                    .col(MemoryFragments::CreatedAt)
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
