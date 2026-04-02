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

    #[derive(Clone, Debug, PartialEq, EnumIter, DeriveActiveEnum)]
    #[sea_orm(rs_type = "String", db_type = "Enum")]
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
            ('Hello World',          'My first post.',                    'published', 1),
            ('Why Rust?',            'Rust is fast and safe.',            'published', 2),
            ('Building Admins',      'axum-admin makes it easy.',         'draft',     3),
            ('Async in Rust',        'Tokio makes async simple.',         'published', 2),
            ('Error Handling',       'Thiserror and anyhow.',             'published', 2),
            ('Axum Basics',          'Routing and extractors.',           'published', 1),
            ('SeaORM Guide',         'Querying with SeaORM.',             'draft',     2),
            ('Web Performance',      'Optimizing web apps.',              'published', 3),
            ('Cargo Workspaces',     'Managing multi-crate projects.',    'published', 2),
            ('Type Safety',          'Leveraging the type system.',       'published', 2),
            ('Deploy with Docker',   'Containerizing Rust apps.',         'draft',     3),
            ('Testing Strategies',   'Unit and integration tests.',       'published', 1),
            ('Lifetimes Explained',  'Understanding borrow checker.',     'published', 2),
            ('REST API Design',      'Building clean REST APIs.',         'published', 3),
            ('Middleware in Axum',   'Writing custom middleware.',        'draft',     1),
            ('Database Migrations',  'Schema evolution strategies.',      'published', 2),
            ('Serde Deep Dive',      'Serialization and deserialization.','published', 1),
            ('CI/CD for Rust',       'GitHub Actions for Rust projects.', 'draft',     3),
            ('Benchmarking Rust',    'Using criterion.rs.',               'published', 2),
            ('Security Best Practices','Input validation and secrets.',   'published', 1),
            ('GraphQL with Rust',    'Building GraphQL APIs.',            'draft',     3),
            ('WASM and Rust',        'Compiling Rust to WebAssembly.',    'published', 1),
            ('Traits vs Generics',   'When to use each.',                 'published', 2),
            ('Macros in Rust',       'Procedural and declarative macros.','draft',     2)",
    ))
    .await
    .expect("failed to seed posts");

    // Tags and post_tags junction for ManyToMany example
    db.execute(Statement::from_string(
        DbBackend::Sqlite,
        "CREATE TABLE tags (
            id   INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL
        )",
    ))
    .await
    .expect("failed to create tags table");

    db.execute(Statement::from_string(
        DbBackend::Sqlite,
        "CREATE TABLE post_tags (
            post_id INTEGER NOT NULL,
            tag_id  INTEGER NOT NULL,
            PRIMARY KEY (post_id, tag_id)
        )",
    ))
    .await
    .expect("failed to create post_tags table");

    db.execute(Statement::from_string(
        DbBackend::Sqlite,
        "INSERT INTO tags (name) VALUES ('tutorial'), ('performance'), ('async'), ('tooling'), ('beginner')",
    ))
    .await
    .expect("failed to seed tags");

    db.execute(Statement::from_string(
        DbBackend::Sqlite,
        "INSERT INTO post_tags (post_id, tag_id) VALUES (1, 1), (1, 5), (2, 1), (3, 1), (4, 3), (5, 1)",
    ))
    .await
    .expect("failed to seed post_tags");

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
