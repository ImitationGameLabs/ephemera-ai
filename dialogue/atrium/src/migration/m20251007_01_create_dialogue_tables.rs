use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create users table
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Users::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Users::Name).string().not_null())
                    .col(ColumnDef::new(Users::Bio).text().not_null())
                    .col(ColumnDef::new(Users::Password).text().not_null())
                    .col(ColumnDef::new(Users::MessageHeight).integer().not_null().default(0))
                    .col(ColumnDef::new(Users::LastSeen).date_time().null())
                    .col(ColumnDef::new(Users::CreatedAt).date_time().not_null())
                    .to_owned(),
            )
            .await?;

        // Create indexes for users table
        manager
            .create_index(
                Index::create()
                    .name("idx_users_name_unique")
                    .table(Users::Table)
                    .col(Users::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Create messages table
        manager
            .create_table(
                Table::create()
                    .table(Messages::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Messages::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Messages::Content).text().not_null())
                    .col(ColumnDef::new(Messages::Sender).string().not_null())
                    .col(ColumnDef::new(Messages::CreatedAt).date_time().not_null())
                    .to_owned(),
            )
            .await?;

        // Create indexes for messages table
        manager
            .create_index(
                Index::create()
                    .name("idx_messages_created_at")
                    .table(Messages::Table)
                    .col(Messages::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_messages_sender")
                    .table(Messages::Table)
                    .col(Messages::Sender)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop messages table first
        manager
            .drop_table(Table::drop().table(Messages::Table).to_owned())
            .await?;

        // Drop users table
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Name,
    Bio,
    Password,
    MessageHeight,
    LastSeen,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Messages {
    Table,
    Id,
    Content,
    Sender,
    CreatedAt,
}