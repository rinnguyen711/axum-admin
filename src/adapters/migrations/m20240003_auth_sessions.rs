use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20240003_000001_create_auth_sessions"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AuthSessions::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(AuthSessions::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(AuthSessions::Username).string().not_null())
                    .col(ColumnDef::new(AuthSessions::IsSuperuser).boolean().not_null().default(false))
                    .col(ColumnDef::new(AuthSessions::ExpiresAt).timestamp().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AuthSessions::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum AuthSessions {
    Table,
    Id,
    Username,
    IsSuperuser,
    ExpiresAt,
}
