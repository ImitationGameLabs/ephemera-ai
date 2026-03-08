use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Pinned::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Pinned::MemoryId)
                            .big_integer()
                            .primary_key()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Pinned::Reason).text())
                    .col(
                        ColumnDef::new(Pinned::PinnedAt)
                            .date_time()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_pinned_memory_id")
                            .from(Pinned::Table, Pinned::MemoryId)
                            .to(MemoryFragments::Table, MemoryFragments::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Pinned::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Pinned {
    Table,
    MemoryId,
    Reason,
    PinnedAt,
}

#[derive(DeriveIden)]
enum MemoryFragments {
    Table,
    Id,
}
