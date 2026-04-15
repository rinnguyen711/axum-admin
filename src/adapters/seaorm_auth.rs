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
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, Statement};
use sea_orm_adapter::SeaOrmAdapter;
use std::sync::Arc;
use tokio::sync::RwLock as TokioRwLock;
use uuid::Uuid;

pub struct SeaOrmAdminAuth {
    pub(crate) db: DatabaseConnection,
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
    ///
    /// Internally roles are stored with a `role:` prefix to avoid collisions with
    /// usernames that share the same string (e.g. a user named "admin" and the role
    /// "admin").
    pub async fn seed_roles(&self, entity_names: &[String]) -> Result<(), AdminError> {
        use casbin::MgmtApi;
        let actions = ["view", "create", "edit", "delete"];
        let mut enforcer = self.enforcer.write().await;
        for entity in entity_names {
            for action in &actions {
                let rule = vec!["role:admin".to_string(), entity.clone(), ToString::to_string(action)];
                if !enforcer.has_policy(rule.clone()) {
                    enforcer
                        .add_policy(rule)
                        .await
                        .map_err(|e| AdminError::Internal(e.to_string()))?;
                }
            }
            let viewer_rule = vec!["role:viewer".to_string(), entity.clone(), "view".to_string()];
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
    /// The role is stored internally with a `role:` prefix so that a user named "admin"
    /// and the "admin" role do not alias each other in Casbin's policy store.
    pub async fn assign_role(&self, username: &str, role: &str) -> Result<(), AdminError> {
        use casbin::RbacApi;
        let prefixed_role = format!("role:{role}");
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
            .add_role_for_user(username, &prefixed_role, None)
            .await
            .map_err(|e| AdminError::Internal(e.to_string()))?;
        Ok(())
    }

    /// Returns all role names (without the `role:` prefix) currently in the Casbin policy store.
    pub fn list_roles(&self) -> Vec<String> {
        use casbin::MgmtApi;
        let enforcer = match self.enforcer.try_read() {
            Ok(e) => e,
            Err(_) => return vec![],
        };
        let mut roles: Vec<String> = enforcer
            .get_all_subjects()
            .into_iter()
            .filter(|s| s.starts_with("role:"))
            .map(|s| s.strip_prefix("role:").unwrap_or(&s).to_string())
            .collect();
        roles.sort();
        roles.dedup();
        roles
    }

    /// Get the assigned role for a user ("admin" or "viewer"), or None if superuser/unassigned.
    /// Strips the internal `role:` prefix before returning.
    pub fn get_user_role(&self, username: &str) -> Option<String> {
        use casbin::RbacApi;
        let enforcer = self.enforcer.try_read().ok()?;
        enforcer
            .get_roles_for_user(username, None)
            .into_iter()
            .next()
            .map(|r| r.strip_prefix("role:").unwrap_or(&r).to_string())
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
        let expires_at = chrono::Utc::now().naive_utc() + chrono::Duration::hours(24);

        let backend = self.db.get_database_backend();
        let sql = crate::adapters::seaorm::rebind(
            "INSERT INTO auth_sessions (id, username, is_superuser, expires_at) VALUES (?, ?, ?, ?)",
            backend,
        );
        let stmt = Statement::from_sql_and_values(
            backend,
            &sql,
            [
                sea_orm::Value::String(Some(Box::new(session_id.clone()))),
                sea_orm::Value::String(Some(Box::new(user.username.clone()))),
                sea_orm::Value::Bool(Some(user.is_superuser)),
                sea_orm::Value::ChronoDateTime(Some(Box::new(expires_at))),
            ],
        );
        self.db
            .execute(stmt)
            .await
            .map_err(|e| AdminError::Internal(e.to_string()))?;

        Ok(AdminUser {
            username: user.username,
            session_id,
            is_superuser: user.is_superuser,
        })
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<AdminUser>, AdminError> {
        use sea_orm::TryGetable;

        let now = chrono::Utc::now().naive_utc();
        let backend = self.db.get_database_backend();

        let sql = crate::adapters::seaorm::rebind(
            "SELECT username, is_superuser, expires_at FROM auth_sessions WHERE id = ?",
            backend,
        );
        let stmt = Statement::from_sql_and_values(
            backend,
            &sql,
            [sea_orm::Value::String(Some(Box::new(session_id.to_string())))],
        );
        let row = self
            .db
            .query_one(stmt)
            .await
            .map_err(|e| AdminError::Internal(e.to_string()))?;

        let row = match row {
            None => return Ok(None),
            Some(r) => r,
        };

        let expires_at: chrono::NaiveDateTime = chrono::NaiveDateTime::try_get(&row, "", "expires_at")
            .map_err(|_| AdminError::Internal("failed to read expires_at".to_string()))?;

        if expires_at <= now {
            let del_sql = crate::adapters::seaorm::rebind(
                "DELETE FROM auth_sessions WHERE id = ?",
                backend,
            );
            let del_stmt = Statement::from_sql_and_values(
                backend,
                &del_sql,
                [sea_orm::Value::String(Some(Box::new(session_id.to_string())))],
            );
            let _ = self.db.execute(del_stmt).await;
            return Ok(None);
        }

        let username: String = String::try_get(&row, "", "username")
            .map_err(|_| AdminError::Internal("failed to read username".to_string()))?;
        let is_superuser: bool = bool::try_get(&row, "", "is_superuser")
            .map_err(|_| AdminError::Internal("failed to read is_superuser".to_string()))?;

        Ok(Some(AdminUser {
            username,
            session_id: session_id.to_string(),
            is_superuser,
        }))
    }
}
