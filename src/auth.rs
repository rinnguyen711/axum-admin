use crate::error::AdminError;
use async_trait::async_trait;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AdminUser {
    pub username: String,
    pub session_id: String,
    /// true = bypasses all permission checks (superuser access)
    pub is_superuser: bool,
}

impl AdminUser {
    pub fn superuser(username: &str, session_id: &str) -> Self {
        Self {
            username: username.to_string(),
            session_id: session_id.to_string(),
            is_superuser: true,
        }
    }
}

#[async_trait]
pub trait AdminAuth: Send + Sync {
    async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<AdminUser, AdminError>;

    async fn get_session(&self, session_id: &str) -> Result<Option<AdminUser>, AdminError>;
}

/// In-memory admin auth. Credentials configured at startup, sessions stored in memory.
pub struct DefaultAdminAuth {
    credentials: Arc<RwLock<HashMap<String, String>>>,
    sessions: Arc<RwLock<HashMap<String, AdminUser>>>,
}

impl DefaultAdminAuth {
    pub fn new() -> Self {
        Self {
            credentials: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn add_user(self, username: &str, password: &str) -> Self {
        let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)
            .expect("bcrypt hash failed");
        self.credentials
            .write()
            .unwrap()
            .insert(username.to_string(), hash);
        self
    }
}

impl Default for DefaultAdminAuth {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AdminAuth for DefaultAdminAuth {
    async fn authenticate(&self, username: &str, password: &str) -> Result<AdminUser, AdminError> {
        let hash = {
            let creds = self.credentials.read().unwrap();
            creds.get(username).cloned()
        };

        let hash = hash.ok_or(AdminError::Unauthorized)?;

        let valid = bcrypt::verify(password, &hash).unwrap_or(false);
        if !valid {
            return Err(AdminError::Unauthorized);
        }

        let session_id = Uuid::new_v4().to_string();
        let user = AdminUser {
            username: username.to_string(),
            session_id: session_id.clone(),
            is_superuser: true,
        };

        self.sessions
            .write()
            .unwrap()
            .insert(session_id, user.clone());

        Ok(user)
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<AdminUser>, AdminError> {
        let sessions = self.sessions.read().unwrap();
        Ok(sessions.get(session_id).cloned())
    }
}

/// Returns true if the user can perform `required` action.
/// - `None` required → always allowed.
/// - `is_superuser` → always allowed.
/// - `enforcer` present → ask Casbin. Permission format: "entity.action" (e.g. "posts.view").
/// - No enforcer → deny non-superusers (safe default).
#[cfg(feature = "seaorm")]
pub async fn check_permission(
    user: &AdminUser,
    required: &Option<String>,
    enforcer: Option<&std::sync::Arc<tokio::sync::RwLock<casbin::Enforcer>>>,
) -> bool {
    use casbin::CoreApi;
    let perm = match required {
        None => return true,
        Some(p) => p,
    };
    if user.is_superuser {
        return true;
    }
    let enforcer = match enforcer {
        Some(e) => e,
        None => return false,
    };
    let parts: Vec<&str> = perm.splitn(2, '.').collect();
    let (obj, act) = if parts.len() == 2 {
        (parts[0], parts[1])
    } else {
        (perm.as_str(), "")
    };
    let guard = enforcer.read().await;
    guard.enforce((user.username.as_str(), obj, act)).unwrap_or(false)
}

#[cfg(not(feature = "seaorm"))]
pub fn check_permission(
    user: &AdminUser,
    required: &Option<String>,
    _enforcer: Option<&()>,
) -> bool {
    match required {
        None => true,
        Some(_) => user.is_superuser,
    }
}
