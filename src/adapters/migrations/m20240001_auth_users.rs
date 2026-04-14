use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20240001_000001_create_auth_users"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AuthUsers::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(AuthUsers::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(AuthUsers::Username).string().not_null().unique_key())
                    .col(ColumnDef::new(AuthUsers::PasswordHash).string().not_null())
                    .col(ColumnDef::new(AuthUsers::IsActive).boolean().not_null().default(true))
                    .col(ColumnDef::new(AuthUsers::IsSuperuser).boolean().not_null().default(false))
                    .col(ColumnDef::new(AuthUsers::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(AuthUsers::UpdatedAt).timestamp().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AuthUsers::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum AuthUsers {
    Table,
    Id,
    Username,
    PasswordHash,
    IsActive,
    IsSuperuser,
    CreatedAt,
    UpdatedAt,
}
