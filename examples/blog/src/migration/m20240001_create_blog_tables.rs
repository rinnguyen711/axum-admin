use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20240001_000001_create_blog_tables"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Categories::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Categories::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Categories::Name).string().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Posts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Posts::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Posts::Title).string().not_null())
                    .col(
                        ColumnDef::new(Posts::Body)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .col(
                        ColumnDef::new(Posts::Status)
                            .string()
                            .not_null()
                            .default("draft"),
                    )
                    .col(ColumnDef::new(Posts::CategoryId).integer())
                    .foreign_key(
                        ForeignKey::create()
                            .from(Posts::Table, Posts::CategoryId)
                            .to(Categories::Table, Categories::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Tags::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Tags::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Tags::Name).string().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PostTags::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(PostTags::PostId).integer().not_null())
                    .col(ColumnDef::new(PostTags::TagId).integer().not_null())
                    .primary_key(
                        Index::create()
                            .col(PostTags::PostId)
                            .col(PostTags::TagId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(PostTags::Table, PostTags::PostId)
                            .to(Posts::Table, Posts::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(PostTags::Table, PostTags::TagId)
                            .to(Tags::Table, Tags::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PostTags::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Posts::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Tags::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Categories::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Categories {
    Table,
    Id,
    Name,
}

#[derive(Iden)]
enum Posts {
    Table,
    Id,
    Title,
    Body,
    Status,
    CategoryId,
}

#[derive(Iden)]
enum Tags {
    Table,
    Id,
    Name,
}

#[derive(Iden)]
enum PostTags {
    Table,
    PostId,
    TagId,
}
