#[cfg(feature = "seaorm")]
mod tests {
    use sea_orm::{Database, DbErr};

    async fn setup_db() -> Result<sea_orm::DatabaseConnection, DbErr> {
        let db = Database::connect("sqlite::memory:").await?;
        use sea_orm::ConnectionTrait;
        db.execute_unprepared(
            "CREATE TABLE auth_users (
                id TEXT PRIMARY KEY,
                username TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                is_superuser INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )"
        ).await?;
        Ok(db)
    }

    #[tokio::test]
    async fn create_and_find_user() {
        let db = setup_db().await.unwrap();
        use axum_admin::adapters::seaorm_auth::{AuthUserActiveModel, AuthUserEntity};
        use sea_orm::{ActiveModelTrait, EntityTrait, Set};

        let user = AuthUserActiveModel {
            id: Set("id-1".to_string()),
            username: Set("alice".to_string()),
            password_hash: Set("hash".to_string()),
            is_active: Set(true),
            is_superuser: Set(false),
            created_at: Set(chrono::Utc::now().naive_utc()),
            updated_at: Set(chrono::Utc::now().naive_utc()),
        };
        user.insert(&db).await.unwrap();

        let found = AuthUserEntity::find_by_id("id-1".to_string())
            .one(&db)
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().username, "alice");
    }
}
