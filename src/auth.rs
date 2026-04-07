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
