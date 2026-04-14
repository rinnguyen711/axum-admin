use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20240002_000001_create_casbin_rule"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CasbinRule::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CasbinRule::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CasbinRule::Ptype).string().not_null().default(""))
                    .col(ColumnDef::new(CasbinRule::V0).string().not_null().default(""))
                    .col(ColumnDef::new(CasbinRule::V1).string().not_null().default(""))
                    .col(ColumnDef::new(CasbinRule::V2).string().not_null().default(""))
                    .col(ColumnDef::new(CasbinRule::V3).string().not_null().default(""))
                    .col(ColumnDef::new(CasbinRule::V4).string().not_null().default(""))
                    .col(ColumnDef::new(CasbinRule::V5).string().not_null().default(""))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CasbinRule::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum CasbinRule {
    Table,
    Id,
    Ptype,
    V0,
    V1,
    V2,
    V3,
    V4,
    V5,
}
