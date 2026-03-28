use axum_admin::{
    adapters::seaorm::SeaOrmAdapter,
    AdminApp, DefaultAdminAuth, EntityAdmin, Field,
};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};

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

    let router = AdminApp::new()
        .title("Blog Admin")
        .prefix("/admin")
        .auth(Box::new(
            DefaultAdminAuth::new().add_user("admin", "admin"),
        ))
        .register(
            EntityAdmin::new::<category::Entity>("categories")
                .label("Categories")
                .field(Field::text("name").required())
                .list_display(vec!["id".to_string(), "name".to_string()])
                .search_fields(vec!["name".to_string()])
                .adapter(Box::new(SeaOrmAdapter::<category::Entity>::new(db.clone()))),
        )
        .register(
            EntityAdmin::new::<post::Entity>("posts")
                .label("Posts")
                .field(Field::text("title").required())
                .field(Field::text("body"))
                .field(Field::select(
                    "status",
                    vec![
                        ("draft".to_string(), "Draft".to_string()),
                        ("published".to_string(), "Published".to_string()),
                    ],
                ))
                .field(Field::number("category_id").label("Category ID"))
                .list_display(vec![
                    "id".to_string(),
                    "title".to_string(),
                    "status".to_string(),
                    "category_id".to_string(),
                ])
                .search_fields(vec!["title".to_string(), "body".to_string()])
                .adapter(Box::new(SeaOrmAdapter::<post::Entity>::new(db.clone()))),
        )
        .into_router();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind port 3000");

    println!("Blog admin running at http://localhost:3000/admin");
    println!("Login: admin / admin");

    axum::serve(listener, router)
        .await
        .expect("server error");
}
