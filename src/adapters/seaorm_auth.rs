use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "auth_users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(unique)]
    pub username: String,
    pub password_hash: String,
    pub is_active: bool,
    pub is_superuser: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub type AuthUserEntity = Entity;
pub type AuthUserModel = Model;
pub type AuthUserActiveModel = ActiveModel;

use crate::adapters::migrations::Migrator;
use crate::auth::{AdminAuth, AdminUser};
use crate::error::AdminError;
use async_trait::async_trait;
use sea_orm_migration::MigratorTrait;
use casbin::{CoreApi, DefaultModel, Enforcer};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use sea_orm_adapter::SeaOrmAdapter;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use tokio::sync::RwLock as TokioRwLock;
use uuid::Uuid;

pub struct SeaOrmAdminAuth {
    pub(crate) db: DatabaseConnection,
    sessions: Arc<RwLock<HashMap<String, AdminUser>>>,
    enforcer: Arc<TokioRwLock<Enforcer>>,
}

impl SeaOrmAdminAuth {
    pub async fn new(db: DatabaseConnection) -> Result<Self, AdminError> {
        // Run auth schema migrations on every startup (idempotent via IF NOT EXISTS)
        Migrator::up(&db, None)
            .await
            .map_err(|e| AdminError::Internal(e.to_string()))?;

        let model_text = "[request_definition]\nr = sub, obj, act\n\n[policy_definition]\np = sub, obj, act\n\n[role_definition]\ng = _, _\n\n[policy_effect]\ne = some(where (p.eft == allow))\n\n[matchers]\nm = g(r.sub, p.sub) && r.obj == p.obj && r.act == p.act\n";
        let model = DefaultModel::from_str(model_text)
            .await
            .map_err(|e| AdminError::Internal(e.to_string()))?;

        let adapter = SeaOrmAdapter::new(db.clone())
            .await
            .map_err(|e| AdminError::Internal(e.to_string()))?;

        let enforcer = Enforcer::new(model, adapter)
            .await
            .map_err(|e| AdminError::Internal(e.to_string()))?;

        Ok(Self {
            db,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            enforcer: Arc::new(TokioRwLock::new(enforcer)),
        })
    }

    /// Create a user only if no users exist yet. Safe to call on every startup.
    pub async fn ensure_user(&self, username: &str, password: &str) -> Result<(), AdminError> {
        use sea_orm::PaginatorTrait;
        let count = AuthUserEntity::find()
            .count(&self.db)
            .await
            .map_err(|e| AdminError::Internal(e.to_string()))?;
        if count > 0 {
            return Ok(());
        }
        self.create_user(username, password, true).await?;
        self.assign_role(username, "admin").await
    }

    /// Seed default Casbin policies for the two predefined roles.
    /// Idempotent — skips rules that already exist.
    /// Call this after all entities are registered (i.e. from AdminApp::into_router).
    pub async fn seed_roles(&self, entity_names: &[String]) -> Result<(), AdminError> {
        use casbin::MgmtApi;
        let actions = ["view", "create", "edit", "delete"];
        let mut enforcer = self.enforcer.write().await;
        for entity in entity_names {
            for action in &actions {
                let rule = vec!["admin".to_string(), entity.clone(), ToString::to_string(action)];
                if !enforcer.has_policy(rule.clone()) {
                    enforcer
                        .add_policy(rule)
                        .await
                        .map_err(|e| AdminError::Internal(e.to_string()))?;
                }
            }
            let viewer_rule = vec!["viewer".to_string(), entity.clone(), "view".to_string()];
            if !enforcer.has_policy(viewer_rule.clone()) {
                enforcer
                    .add_policy(viewer_rule)
                    .await
                    .map_err(|e| AdminError::Internal(e.to_string()))?;
            }
        }
        Ok(())
    }

    /// Assign a role ("admin" or "viewer") to a user.
    /// Removes any previously assigned role first (a user has exactly one role).
    pub async fn assign_role(&self, username: &str, role: &str) -> Result<(), AdminError> {
        use casbin::MgmtApi;
        use casbin::RbacApi;
        let mut enforcer = self.enforcer.write().await;
        // Remove existing role assignments for this user
        let current_roles = enforcer.get_roles_for_user(username, None);
        for r in current_roles {
            enforcer
                .delete_role_for_user(username, &r, None)
                .await
                .map_err(|e| AdminError::Internal(e.to_string()))?;
        }
        enforcer
            .add_role_for_user(username, role, None)
            .await
            .map_err(|e| AdminError::Internal(e.to_string()))?;
        Ok(())
    }

    /// Get the assigned role for a user ("admin" or "viewer"), or None if superuser/unassigned.
    pub fn get_user_role(&self, username: &str) -> Option<String> {
        use casbin::RbacApi;
        let enforcer = self.enforcer.blocking_read();
        enforcer.get_roles_for_user(username, None).into_iter().next()
    }

    /// Create a new user with a hashed password.
    pub async fn create_user(
        &self,
        username: &str,
        password: &str,
        is_superuser: bool,
    ) -> Result<(), AdminError> {
        use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
        use rand_core::OsRng;

        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AdminError::Internal(e.to_string()))?
            .to_string();

        let now = chrono::Utc::now().naive_utc();
        let model = AuthUserActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            username: Set(username.to_string()),
            password_hash: Set(hash),
            is_active: Set(true),
            is_superuser: Set(is_superuser),
            created_at: Set(now),
            updated_at: Set(now),
        };
        model
            .insert(&self.db)
            .await
            .map_err(|e| AdminError::Internal(e.to_string()))?;
        Ok(())
    }

    /// Change password: verify old password, hash and store new one.
    pub async fn change_password(
        &self,
        username: &str,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), AdminError> {
        use argon2::{
            password_hash::{PasswordHash, PasswordVerifier, SaltString},
            Argon2, PasswordHasher,
        };
        use rand_core::OsRng;
        use sea_orm::IntoActiveModel;

        let user = AuthUserEntity::find()
            .filter(Column::Username.eq(username))
            .one(&self.db)
            .await
            .map_err(|e| AdminError::Internal(e.to_string()))?
            .ok_or(AdminError::Unauthorized)?;

        let parsed = PasswordHash::new(&user.password_hash)
            .map_err(|e| AdminError::Internal(e.to_string()))?;
        Argon2::default()
            .verify_password(old_password.as_bytes(), &parsed)
            .map_err(|_| AdminError::Unauthorized)?;

        let salt = SaltString::generate(&mut OsRng);
        let new_hash = Argon2::default()
            .hash_password(new_password.as_bytes(), &salt)
            .map_err(|e| AdminError::Internal(e.to_string()))?
            .to_string();

        let mut active = user.into_active_model();
        active.password_hash = Set(new_hash);
        active.updated_at = Set(chrono::Utc::now().naive_utc());
        active
            .update(&self.db)
            .await
            .map_err(|e| AdminError::Internal(e.to_string()))?;
        Ok(())
    }

    pub fn db(&self) -> &DatabaseConnection {
        &self.db
    }

    pub fn enforcer(&self) -> Arc<TokioRwLock<Enforcer>> {
        Arc::clone(&self.enforcer)
    }
}

#[async_trait]
impl AdminAuth for SeaOrmAdminAuth {
    async fn authenticate(&self, username: &str, password: &str) -> Result<AdminUser, AdminError> {
        use argon2::{
            password_hash::{PasswordHash, PasswordVerifier},
            Argon2,
        };

        let user = AuthUserEntity::find()
            .filter(Column::Username.eq(username))
            .filter(Column::IsActive.eq(true))
            .one(&self.db)
            .await
            .map_err(|e| AdminError::Internal(e.to_string()))?
            .ok_or(AdminError::Unauthorized)?;

        let parsed = PasswordHash::new(&user.password_hash)
            .map_err(|e| AdminError::Internal(e.to_string()))?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .map_err(|_| AdminError::Unauthorized)?;

        let session_id = Uuid::new_v4().to_string();
        let admin_user = AdminUser {
            username: user.username.clone(),
            session_id: session_id.clone(),
            is_superuser: user.is_superuser,
        };
        self.sessions
            .write()
            .unwrap()
            .insert(session_id, admin_user.clone());
        Ok(admin_user)
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<AdminUser>, AdminError> {
        let sessions = self.sessions.read().unwrap();
        Ok(sessions.get(session_id).cloned())
    }
}
