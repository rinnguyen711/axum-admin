use crate::{auth::AdminAuth, entity::EntityAdmin};
use std::sync::Arc;

pub struct AdminApp {
    pub title: String,
    pub prefix: String,
    pub entities: Vec<EntityAdmin>,
    pub auth: Option<Arc<dyn AdminAuth>>,
}

impl AdminApp {
    pub fn new() -> Self {
        Self {
            title: "Admin".to_string(),
            prefix: "/admin".to_string(),
            entities: Vec::new(),
            auth: None,
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
}

impl Default for AdminApp {
    fn default() -> Self {
        Self::new()
    }
}
