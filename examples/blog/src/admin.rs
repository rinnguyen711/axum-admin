use axum::Router;
use axum_admin::{
    adapters::seaorm::{SeaOrmAdapter, SeaOrmManyToManyAdapter},
    adapters::seaorm_auth::SeaOrmAdminAuth,
    AdminApp, EntityAdmin, EntityGroupAdmin, Field,
};
use sea_orm::DatabaseConnection;

use crate::{category, post, tag};

pub async fn build(db: DatabaseConnection) -> Router {
    let auth = SeaOrmAdminAuth::new(db.clone())
        .await
        .expect("failed to initialize auth");
    auth.ensure_user("admin", "admin")
        .await
        .expect("failed to ensure admin user");

    AdminApp::new()
        .title("Blog Admin")
        .icon("fa-solid fa-newspaper")
        .prefix("/admin")
        .template_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/templates"))
        .seaorm_auth(auth)
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
                                .max_length(255),
                        )
                        .field(Field::textarea("body").max_length(10000))
                        .field(Field::foreign_key(
                            "category_id",
                            "Category",
                            Box::new(SeaOrmAdapter::<category::Entity>::new(db.clone())),
                            "id",
                            "name",
                        ))
                        .field(
                            Field::many_to_many(
                                "tags",
                                Box::new(SeaOrmManyToManyAdapter::new(
                                    db.clone(),
                                    "post_tags",
                                    "post_id",
                                    "tag_id",
                                    "tags",
                                    "id",
                                    "name",
                                )),
                            )
                            .label("Tags"),
                        )
                        .search_fields(vec!["title".to_string(), "body".to_string()])
                        .filter_fields(vec!["status".to_string(), "category_id".to_string()])
                        .adapter(Box::new(SeaOrmAdapter::<post::Entity>::new(db.clone()))),
                )
                .register(
                    EntityAdmin::from_entity::<tag::Entity>("tags")
                        .label("Tags")
                        .icon("fa-solid fa-tag")
                        .list_display(vec!["id".to_string(), "name".to_string()])
                        .search_fields(vec!["name".to_string()])
                        .adapter(Box::new(SeaOrmAdapter::<tag::Entity>::new(db.clone()))),
                ),
        )
        .into_router()
        .await
}
