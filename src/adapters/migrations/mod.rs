use sea_orm_migration::prelude::*;

mod m20240001_auth_users;
mod m20240002_casbin_rule;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240001_auth_users::Migration),
            Box::new(m20240002_casbin_rule::Migration),
        ]
    }
}
