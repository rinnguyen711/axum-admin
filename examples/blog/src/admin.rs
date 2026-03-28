use axum::Router;
use axum_admin::{adapters::seaorm::SeaOrmAdapter, AdminApp, DefaultAdminAuth, EntityAdmin, Field};
use sea_orm::DatabaseConnection;

use crate::{category, post};

pub fn build(db: DatabaseConnection) -> Router {
    AdminApp::new()
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
        .into_router()
}
