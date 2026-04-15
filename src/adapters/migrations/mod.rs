use sea_orm_migration::prelude::*;

mod m20240001_auth_users;
mod m20240002_casbin_rule;
mod m20240003_auth_sessions;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240001_auth_users::Migration),
            Box::new(m20240002_casbin_rule::Migration),
            Box::new(m20240003_auth_sessions::Migration),
        ]
    }

    fn migration_table_name() -> sea_orm::DynIden {
        sea_orm_migration::prelude::Alias::new("axum_admin_migrations").into_iden()
    }
}
