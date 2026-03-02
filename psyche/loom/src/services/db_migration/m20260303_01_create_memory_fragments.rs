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
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(MemoryFragments::Content).text().not_null())
                    .col(
                        ColumnDef::new(MemoryFragments::Timestamp)
                            .date_time()
                            .not_null(),
                    )
                    .col(ColumnDef::new(MemoryFragments::Kind).text().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_memory_fragments_timestamp")
                    .table(MemoryFragments::Table)
                    .col(MemoryFragments::Timestamp)
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
    Timestamp,
    Kind,
}
