use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;

mod admin;
mod migration;

mod category {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "categories")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub name: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

mod post {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, EnumIter, DeriveActiveEnum)]
    #[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
    pub enum Status {
        #[sea_orm(string_value = "draft")]
        Draft,
        #[sea_orm(string_value = "published")]
        Published,
    }

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "posts")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub title: String,
        pub body: String,
        pub status: Status,
        pub category_id: Option<i32>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

async fn connect_db() -> DatabaseConnection {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://blog:blog@localhost:5432/blog".to_string());
    Database::connect(&url)
        .await
        .expect("failed to connect to database")
}

#[tokio::main]
async fn main() {
    let db = connect_db().await;

    migration::Migrator::up(&db, None)
        .await
        .expect("blog migrations failed");

    let router = admin::build(db).await;

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind port 3000");

    println!("Blog admin running at http://localhost:3000/admin");
    println!("Login: admin / admin");

    axum::serve(listener, router)
        .await
        .expect("server error");
}
