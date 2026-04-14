use crate::app::AdminAppState;
use axum::{extract::Extension, response::{Html, IntoResponse, Response}};
use serde::Serialize;
use std::sync::Arc;

use super::helpers::build_nav;

#[derive(Serialize)]
struct EntityPermRow {
    label: String,
    view: bool,
    create: bool,
    edit: bool,
    delete: bool,
}

#[derive(Serialize)]
struct RoleSection {
    role_label: String,
    entities: Vec<EntityPermRow>,
}

#[derive(Serialize)]
struct RolesContext {
    admin_title: String,
    admin_icon: String,
    nav: Vec<crate::render::context::NavItem>,
    current_entity: String,
    show_auth_nav: bool,
    roles: Vec<RoleSection>,
}

pub(super) async fn role_list(
    Extension(state): Extension<Arc<AdminAppState>>,
) -> Response {
    let entity_labels: Vec<(String, String)> = state
        .entities
        .iter()
        .map(|e| (e.entity_name.clone(), e.label.clone()))
        .collect();

    let admin_entities: Vec<EntityPermRow> = entity_labels
        .iter()
        .map(|(_, label)| EntityPermRow {
            label: label.clone(),
            view: true,
            create: true,
            edit: true,
            delete: true,
        })
        .collect();

    let viewer_entities: Vec<EntityPermRow> = entity_labels
        .iter()
        .map(|(_, label)| EntityPermRow {
            label: label.clone(),
            view: true,
            create: false,
            edit: false,
            delete: false,
        })
        .collect();

    let ctx = RolesContext {
        admin_title: state.title.clone(),
        admin_icon: state.icon.clone(),
        nav: build_nav(&state, ""),
        current_entity: "__roles".to_string(),
        show_auth_nav: state.show_auth_nav,
        roles: vec![
            RoleSection { role_label: "Admin".to_string(), entities: admin_entities },
            RoleSection { role_label: "Viewer".to_string(), entities: viewer_entities },
        ],
    };
    Html(state.renderer.render("roles.html", ctx)).into_response()
}
