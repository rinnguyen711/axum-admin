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
            EntityAdmin::from_entity::<category::Entity>("categories")
                .label("Categories")
                .list_display(vec!["id".to_string(), "name".to_string()])
                .search_fields(vec!["name".to_string()])
                .filter_fields(vec!["name".to_string()])
                .adapter(Box::new(SeaOrmAdapter::<category::Entity>::new(db.clone()))),
        )
        .register(
            EntityAdmin::from_entity::<post::Entity>("posts")
                .label("Posts")
                .field(
                    Field::foreign_key(
                        "category_id",
                        "Category",
                        Box::new(SeaOrmAdapter::<category::Entity>::new(db.clone())),
                        "id",
                        "name",
                    )
                )
                .search_fields(vec!["title".to_string(), "body".to_string()])
                .filter_fields(vec!["status".to_string(), "category_id".to_string()])
                .adapter(Box::new(SeaOrmAdapter::<post::Entity>::new(db.clone()))),
        )
        .into_router()
}
