#[cfg(feature = "seaorm")]
mod tests {
    use axum_admin::adapters::seaorm::sea_value_to_json;
    use sea_orm::Value as SeaValue;
    use serde_json::{json, Value};

    #[test]
    fn sea_value_string_to_json() {
        let v = SeaValue::String(Some(Box::new("hello".to_string())));
        assert_eq!(sea_value_to_json(v), json!("hello"));
    }

    #[test]
    fn sea_value_null_string_to_json() {
        let v = SeaValue::String(None);
        assert_eq!(sea_value_to_json(v), Value::Null);
    }

    #[test]
    fn sea_value_int_to_json() {
        let v = SeaValue::Int(Some(42));
        assert_eq!(sea_value_to_json(v), json!(42));
    }

    #[test]
    fn sea_value_bool_to_json() {
        let v = SeaValue::Bool(Some(true));
        assert_eq!(sea_value_to_json(v), json!(true));
    }
}

#[cfg(feature = "seaorm")]
mod integration {
    use axum_admin::adapters::seaorm::SeaOrmAdapter;
    use axum_admin::{AdminError, DataAdapter, ListParams};
    use sea_orm::{entity::prelude::*, Database, DatabaseConnection, DbBackend, Statement};
    use std::collections::HashMap;

    // Minimal test entity
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "items")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub name: String,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}

    async fn setup_db() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.execute(Statement::from_string(
            DbBackend::Sqlite,
            "CREATE TABLE items (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL)"
                .to_string(),
        ))
        .await
        .unwrap();
        db.execute(Statement::from_string(
            DbBackend::Sqlite,
            "INSERT INTO items (name) VALUES ('Alpha'), ('Beta'), ('Gamma')".to_string(),
        ))
        .await
        .unwrap();
        db
    }

    #[tokio::test]
    async fn list_returns_all_rows() {
        let db = setup_db().await;
        let adapter = SeaOrmAdapter::<Entity>::new(db.clone());
        let rows = adapter.list(ListParams::default()).await.unwrap();
        assert_eq!(rows.len(), 3);
    }

    #[tokio::test]
    async fn count_returns_total() {
        let db = setup_db().await;
        let adapter = SeaOrmAdapter::<Entity>::new(db.clone());
        let count = adapter.count(&ListParams::default()).await.unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn list_search_filters_rows() {
        let db = setup_db().await;
        let adapter = SeaOrmAdapter::<Entity>::new(db.clone())
            .search_columns(vec!["name".to_string()]);

        let params = ListParams {
            search: Some("Alpha".to_string()),
            ..ListParams::default()
        };
        let rows = adapter.list(params).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], serde_json::json!("Alpha"));
    }

    #[tokio::test]
    async fn list_pagination() {
        let db = setup_db().await;
        let adapter = SeaOrmAdapter::<Entity>::new(db.clone());

        let params = ListParams {
            page: 1,
            per_page: 2,
            ..ListParams::default()
        };
        let rows = adapter.list(params).await.unwrap();
        assert_eq!(rows.len(), 2);

        let params2 = ListParams {
            page: 2,
            per_page: 2,
            ..ListParams::default()
        };
        let rows2 = adapter.list(params2).await.unwrap();
        assert_eq!(rows2.len(), 1);
    }

    #[tokio::test]
    async fn get_returns_record() {
        let db = setup_db().await;
        let adapter = SeaOrmAdapter::<Entity>::new(db.clone());
        let record = adapter.get(&serde_json::json!(1)).await.unwrap();
        assert_eq!(record["name"], serde_json::json!("Alpha"));
    }

    #[tokio::test]
    async fn get_not_found() {
        let db = setup_db().await;
        let adapter = SeaOrmAdapter::<Entity>::new(db.clone());
        let result = adapter.get(&serde_json::json!(9999)).await;
        assert!(matches!(result, Err(AdminError::NotFound)));
    }

    #[tokio::test]
    async fn create_inserts_record() {
        let db = setup_db().await;
        let adapter = SeaOrmAdapter::<Entity>::new(db.clone());
        let data = HashMap::from([("name".to_string(), serde_json::json!("Delta"))]);
        let new_id = adapter.create(data).await.unwrap();
        assert!(new_id.as_i64().unwrap() > 0);

        let count = adapter.count(&ListParams::default()).await.unwrap();
        assert_eq!(count, 4);
    }

    #[tokio::test]
    async fn update_modifies_record() {
        let db = setup_db().await;
        let adapter = SeaOrmAdapter::<Entity>::new(db.clone());
        let data =
            HashMap::from([("name".to_string(), serde_json::json!("AlphaUpdated"))]);
        adapter.update(&serde_json::json!(1), data).await.unwrap();

        let record = adapter.get(&serde_json::json!(1)).await.unwrap();
        assert_eq!(record["name"], serde_json::json!("AlphaUpdated"));
    }

    #[tokio::test]
    async fn delete_removes_record() {
        let db = setup_db().await;
        let adapter = SeaOrmAdapter::<Entity>::new(db.clone());
        adapter.delete(&serde_json::json!(1)).await.unwrap();

        let count = adapter.count(&ListParams::default()).await.unwrap();
        assert_eq!(count, 2);

        let result = adapter.get(&serde_json::json!(1)).await;
        assert!(matches!(result, Err(AdminError::NotFound)));
    }

    #[tokio::test]
    async fn search_via_params_search_columns() {
        // search_columns populated via ListParams (as the router does via entity.search_fields)
        let db = setup_db().await;
        let adapter = SeaOrmAdapter::<Entity>::new(db.clone()); // no .search_columns() on adapter

        let params = ListParams {
            search: Some("Beta".to_string()),
            search_columns: vec!["name".to_string()],
            ..ListParams::default()
        };
        let rows = adapter.list(params.clone()).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], serde_json::json!("Beta"));

        let count = adapter.count(&params).await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn order_by_asc() {
        let db = setup_db().await;
        let adapter = SeaOrmAdapter::<Entity>::new(db.clone());

        let params = ListParams {
            order_by: Some(("name".to_string(), axum_admin::SortOrder::Asc)),
            ..ListParams::default()
        };
        let rows = adapter.list(params).await.unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0]["name"], serde_json::json!("Alpha"));
        assert_eq!(rows[1]["name"], serde_json::json!("Beta"));
        assert_eq!(rows[2]["name"], serde_json::json!("Gamma"));
    }

    #[tokio::test]
    async fn order_by_desc() {
        let db = setup_db().await;
        let adapter = SeaOrmAdapter::<Entity>::new(db.clone());

        let params = ListParams {
            order_by: Some(("name".to_string(), axum_admin::SortOrder::Desc)),
            ..ListParams::default()
        };
        let rows = adapter.list(params).await.unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0]["name"], serde_json::json!("Gamma"));
        assert_eq!(rows[1]["name"], serde_json::json!("Beta"));
        assert_eq!(rows[2]["name"], serde_json::json!("Alpha"));
    }
}
