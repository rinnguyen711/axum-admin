use crate::{auth::AdminAuth, entity::{EntityAdmin, EntityGroupAdmin}, render::AdminRenderer};
use std::{path::PathBuf, sync::Arc};

pub enum AdminEntry {
    Entity(EntityAdmin),
    Group(EntityGroupAdmin),
}

impl From<EntityAdmin> for AdminEntry {
    fn from(e: EntityAdmin) -> Self { AdminEntry::Entity(e) }
}

impl From<EntityGroupAdmin> for AdminEntry {
    fn from(g: EntityGroupAdmin) -> Self { AdminEntry::Group(g) }
}

pub struct AdminAppState {
    pub title: String,
    pub icon: String,
    pub entities: Vec<EntityAdmin>,
    pub renderer: AdminRenderer,
    #[cfg(feature = "seaorm")]
    pub enforcer: Option<std::sync::Arc<tokio::sync::RwLock<casbin::Enforcer>>>,
    #[cfg(not(feature = "seaorm"))]
    pub enforcer: Option<()>,
    #[cfg(feature = "seaorm")]
    pub seaorm_auth: Option<std::sync::Arc<crate::adapters::seaorm_auth::SeaOrmAdminAuth>>,
    pub show_auth_nav: bool,
}

pub struct AdminApp {
    pub title: String,
    pub icon: String,
    pub prefix: String,
    pub entities: Vec<EntityAdmin>,
    pub auth: Option<Arc<dyn AdminAuth>>,
    pub(crate) templates: Vec<(String, String)>,
    pub(crate) template_dirs: Vec<PathBuf>,
    /// Maximum multipart body size in bytes. Defaults to 10 MiB.
    pub upload_limit: usize,
    #[cfg(feature = "seaorm")]
    pub(crate) enforcer: Option<std::sync::Arc<tokio::sync::RwLock<casbin::Enforcer>>>,
    #[cfg(not(feature = "seaorm"))]
    pub(crate) enforcer: Option<()>,
    #[cfg(feature = "seaorm")]
    pub(crate) seaorm_auth: Option<std::sync::Arc<crate::adapters::seaorm_auth::SeaOrmAdminAuth>>,
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
            upload_limit: 10 * 1024 * 1024, // 10 MiB
            enforcer: None,
            #[cfg(feature = "seaorm")]
            seaorm_auth: None,
        }
    }

    /// Set the maximum allowed multipart upload size in bytes.
    /// Defaults to 10 MiB (10 * 1024 * 1024).
    pub fn upload_limit(mut self, bytes: usize) -> Self {
        self.upload_limit = bytes;
        self
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

    pub fn register(mut self, entry: impl Into<AdminEntry>) -> Self {
        match entry.into() {
            AdminEntry::Entity(e) => self.entities.push(e),
            AdminEntry::Group(g) => self.entities.extend(g.into_entities()),
        }
        self
    }

    pub fn auth(mut self, auth: Box<dyn AdminAuth>) -> Self {
        self.auth = Some(Arc::from(auth));
        self
    }

    #[cfg(feature = "seaorm")]
    pub fn seaorm_auth(mut self, auth: crate::adapters::seaorm_auth::SeaOrmAdminAuth) -> Self {
        let arc = std::sync::Arc::new(auth);
        self.enforcer = Some(arc.enforcer());
        self.auth = Some(arc.clone() as std::sync::Arc<dyn crate::auth::AdminAuth>);
        self.seaorm_auth = Some(arc);
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

    pub(crate) fn into_state(self) -> (Arc<dyn AdminAuth>, Arc<AdminAppState>, usize) {
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

        let upload_limit = self.upload_limit;
        let show_auth_nav = {
            #[cfg(feature = "seaorm")]
            { self.seaorm_auth.is_some() }
            #[cfg(not(feature = "seaorm"))]
            { false }
        };
        let state = Arc::new(AdminAppState {
            title: self.title,
            icon: self.icon,
            entities: self.entities,
            renderer: AdminRenderer::with_overrides(all_templates),
            enforcer: self.enforcer,
            #[cfg(feature = "seaorm")]
            seaorm_auth: self.seaorm_auth,
            show_auth_nav,
        });
        (auth, state, upload_limit)
    }
}

impl Default for AdminApp {
    fn default() -> Self {
        Self::new()
    }
}
