use axum::Router;
use axum_admin::{adapters::seaorm::{SeaOrmAdapter, SeaOrmManyToManyAdapter}, AdminApp, DefaultAdminAuth, EntityAdmin, EntityGroupAdmin, Field};
use sea_orm::DatabaseConnection;

use crate::{category, post};

pub fn build(db: DatabaseConnection) -> Router {
    AdminApp::new()
        .title("Blog Admin")
        .icon("fa-solid fa-newspaper")
        .prefix("/admin")
        // Load custom templates from this directory — any .html file here
        // overrides the built-in with the same name.
        // e.g. templates/home.html replaces the default dashboard.
        .template_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/templates"))
        .auth(Box::new(
            DefaultAdminAuth::new().add_user("admin", "admin"),
        ))
        .register(
            EntityGroupAdmin::new("Blog")
                .register(
                    EntityAdmin::from_entity::<category::Entity>("categories")
                        .label("Categories")
                        .icon("fa-solid fa-folder")
                        .list_display(vec!["id".to_string(), "name".to_string()])
                        .search_fields(vec!["name".to_string()])
                        .adapter(Box::new(SeaOrmAdapter::<category::Entity>::new(db.clone()))),
                )
                .register(
                    EntityAdmin::from_entity::<post::Entity>("posts")
                        .label("Posts")
                        .icon("fa-solid fa-file-lines")
                        .field(
                            Field::text("title")
                                .required()
                                .min_length(3)
                                .max_length(255)
                        )
                        .field(
                            Field::textarea("body")
                                .max_length(10000)
                        )
                        .field(
                            Field::foreign_key(
                                "category_id",
                                "Category",
                                Box::new(SeaOrmAdapter::<category::Entity>::new(db.clone())),
                                "id",
                                "name",
                            )
                        )
                        .field(
                            Field::many_to_many(
                                "tags",
                                Box::new(SeaOrmManyToManyAdapter::new(
                                    db.clone(),
                                    "post_tags", // junction table
                                    "post_id",   // FK to posts
                                    "tag_id",    // FK to tags
                                    "tags",      // options table
                                    "id",        // value column
                                    "name",      // label column
                                )),
                            ).label("Tags"),
                        )
                        .search_fields(vec!["title".to_string(), "body".to_string()])
                        .filter_fields(vec!["status".to_string(), "category_id".to_string()])
                        .adapter(Box::new(SeaOrmAdapter::<post::Entity>::new(db.clone()))),
                )
        )
        .into_router()
}
