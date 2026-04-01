use crate::{auth::AdminAuth, entity::EntityAdmin, render::AdminRenderer};
use std::sync::Arc;

pub struct AdminAppState {
    pub title: String,
    pub entities: Vec<EntityAdmin>,
    pub renderer: AdminRenderer,
}

pub struct AdminApp {
    pub title: String,
    pub prefix: String,
    pub entities: Vec<EntityAdmin>,
    pub auth: Option<Arc<dyn AdminAuth>>,
    pub(crate) templates: Vec<(String, String)>,
}

impl AdminApp {
    pub fn new() -> Self {
        Self {
            title: "Admin".to_string(),
            prefix: "/admin".to_string(),
            entities: Vec::new(),
            auth: None,
            templates: Vec::new(),
        }
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn prefix(mut self, prefix: &str) -> Self {
        self.prefix = prefix.to_string();
        self
    }

    pub fn register(mut self, entity: EntityAdmin) -> Self {
        self.entities.push(entity);
        self
    }

    pub fn auth(mut self, auth: Box<dyn AdminAuth>) -> Self {
        self.auth = Some(Arc::from(auth));
        self
    }

    /// Override or add a template by name. The name must match what the router
    /// uses (e.g. `"home.html"`, `"layout.html"`, `"form.html"`).
    /// Custom templates can also be added and rendered via custom routes.
    pub fn template(mut self, name: &str, content: &str) -> Self {
        self.templates.push((name.to_string(), content.to_string()));
        self
    }

    pub(crate) fn into_state(self) -> (Arc<dyn AdminAuth>, Arc<AdminAppState>) {
        let auth = self
            .auth
            .expect("AdminApp requires .auth() to be configured before calling into_router()");
        let state = Arc::new(AdminAppState {
            title: self.title,
            entities: self.entities,
            renderer: AdminRenderer::with_overrides(self.templates),
        });
        (auth, state)
    }
}

impl Default for AdminApp {
    fn default() -> Self {
        Self::new()
    }
}
