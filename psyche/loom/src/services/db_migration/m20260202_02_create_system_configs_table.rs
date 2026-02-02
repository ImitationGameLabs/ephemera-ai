use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create system configs table with auto_increment primary key
        manager
            .create_table(
                Table::create()
                    .table(SystemConfigs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SystemConfigs::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(SystemConfigs::Content).text().not_null())
                    .col(ColumnDef::new(SystemConfigs::ContentHash).string().not_null().unique_key())
                    .col(ColumnDef::new(SystemConfigs::MemoryFragmentId).big_integer())
                    .col(ColumnDef::new(SystemConfigs::CreatedAt).date_time().extra("(6)").not_null())
                    .to_owned(),
            )
            .await?;

        // Create index on created_at for time-based queries
        manager
            .create_index(
                Index::create()
                    .name("idx_system_configs_created_at")
                    .table(SystemConfigs::Table)
                    .col(SystemConfigs::CreatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SystemConfigs::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum SystemConfigs {
    Table,
    Id,
    Content,
    ContentHash,
    MemoryFragmentId,
    CreatedAt,
}
