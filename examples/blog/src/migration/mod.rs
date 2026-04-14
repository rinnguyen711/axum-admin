use sea_orm_migration::prelude::*;

mod m20240001_create_blog_tables;
mod m20240002_seed_blog_data;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240001_create_blog_tables::Migration),
            Box::new(m20240002_seed_blog_data::Migration),
        ]
    }

    fn migration_table_name() -> sea_orm::DynIden {
        sea_orm_migration::prelude::Alias::new("blog_migrations").into_iden()
    }
}
