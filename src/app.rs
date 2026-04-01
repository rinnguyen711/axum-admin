use crate::{auth::AdminAuth, entity::EntityAdmin, render::AdminRenderer};
use std::{path::PathBuf, sync::Arc};

pub struct AdminAppState {
    pub title: String,
    pub icon: String,
    pub entities: Vec<EntityAdmin>,
    pub renderer: AdminRenderer,
}

pub struct AdminApp {
    pub title: String,
    pub icon: String,
    pub prefix: String,
    pub entities: Vec<EntityAdmin>,
    pub auth: Option<Arc<dyn AdminAuth>>,
    pub(crate) templates: Vec<(String, String)>,
    pub(crate) template_dirs: Vec<PathBuf>,
}

impl AdminApp {
    pub fn new() -> Self {
        Self {
            title: "Admin".to_string(),
            icon: "fa-solid fa-bolt".to_string(),
            prefix: "/admin".to_string(),
            entities: Vec::new(),
            auth: None,
            templates: Vec::new(),
            template_dirs: Vec::new(),
        }
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    /// Set the Font Awesome icon class for the app logo in the sidebar.
    /// Defaults to `"fa-solid fa-bolt"`.
    pub fn icon(mut self, icon: &str) -> Self {
        self.icon = icon.to_string();
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

    /// Load templates from a directory on disk. Any `.html` file whose name
    /// matches a built-in template will override it; unknown names are added
    /// as new templates. Files are loaded at startup — no runtime reloading.
    ///
    /// Multiple directories can be registered; later calls take precedence.
    pub fn template_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.template_dirs.push(path.into());
        self
    }

    pub(crate) fn into_state(self) -> (Arc<dyn AdminAuth>, Arc<AdminAppState>) {
        let auth = self
            .auth
            .expect("AdminApp requires .auth() to be configured before calling into_router()");

        // Collect directory templates before inline overrides so that
        // .template() always wins over .template_dir().
        let mut all_templates: Vec<(String, String)> = Vec::new();
        for dir in &self.template_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("html") {
                        if let (Some(name), Ok(content)) = (
                            path.file_name().and_then(|n| n.to_str()).map(str::to_string),
                            std::fs::read_to_string(&path),
                        ) {
                            all_templates.push((name, content));
                        }
                    }
                }
            }
        }
        all_templates.extend(self.templates);

        let state = Arc::new(AdminAppState {
            title: self.title,
            icon: self.icon,
            entities: self.entities,
            renderer: AdminRenderer::with_overrides(all_templates),
        });
        (auth, state)
    }
}

impl Default for AdminApp {
    fn default() -> Self {
        Self::new()
    }
}
