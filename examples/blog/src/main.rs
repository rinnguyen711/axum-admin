use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};

mod admin;

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

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "posts")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub title: String,
        pub body: String,
        pub status: String,
        pub category_id: Option<i32>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

async fn setup_db() -> DatabaseConnection {
    let db = Database::connect("sqlite::memory:")
        .await
        .expect("failed to connect to sqlite");

    db.execute(Statement::from_string(
        DbBackend::Sqlite,
        "CREATE TABLE categories (
            id   INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL
        )",
    ))
    .await
    .expect("failed to create categories table");

    db.execute(Statement::from_string(
        DbBackend::Sqlite,
        "CREATE TABLE posts (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            title       TEXT NOT NULL,
            body        TEXT NOT NULL DEFAULT '',
            status      TEXT NOT NULL DEFAULT 'draft',
            category_id INTEGER
        )",
    ))
    .await
    .expect("failed to create posts table");

    db.execute(Statement::from_string(
        DbBackend::Sqlite,
        "INSERT INTO categories (name) VALUES ('Tech'), ('Rust'), ('Web')",
    ))
    .await
    .expect("failed to seed categories");

    db.execute(Statement::from_string(
        DbBackend::Sqlite,
        "INSERT INTO posts (title, body, status, category_id) VALUES
            ('Hello World',      'My first post.',           'published', 1),
            ('Why Rust?',        'Rust is fast and safe.',   'published', 2),
            ('Building Admins',  'axum-admin makes it easy.','draft',     3)",
    ))
    .await
    .expect("failed to seed posts");

    db
}

#[tokio::main]
async fn main() {
    let db = setup_db().await;
    let router = admin::build(db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind port 3000");

    println!("Blog admin running at http://localhost:3000/admin");
    println!("Login: admin / admin");

    axum::serve(listener, router)
        .await
        .expect("server error");
}
