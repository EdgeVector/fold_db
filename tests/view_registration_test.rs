use fold_db::schema::types::field_value_type::FieldValueType;
use fold_db::schema::types::operations::Query;
use fold_db::schema::types::schema::DeclarativeSchemaType as SchemaType;
use fold_db::schema::SchemaCore;
use fold_db::view::registry::ViewState;
use fold_db::view::types::TransformView;
use std::collections::HashMap;

fn blogpost_schema_json() -> String {
    r#"{
        "name": "BlogPost",
        "key": { "range_field": "publish_date" },
        "fields": {
            "title": {},
            "content": {},
            "publish_date": {}
        }
    }"#
    .to_string()
}

fn weather_schema_json() -> String {
    r#"{
        "name": "Weather",
        "key": { "range_field": "date" },
        "fields": {
            "temp_celsius": {},
            "date": {}
        }
    }"#
    .to_string()
}

fn identity_view(name: &str, source_schema: &str, source_field: &str) -> TransformView {
    TransformView::new(
        name,
        SchemaType::Single,
        None,
        vec![Query::new(
            source_schema.to_string(),
            vec![source_field.to_string()],
        )],
        None,
        HashMap::from([(source_field.to_string(), FieldValueType::Any)]),
    )
}

#[tokio::test]
async fn register_view_with_valid_source() {
    let core = SchemaCore::new_for_testing().await.unwrap();
    core.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();

    let view = identity_view("ContentView", "BlogPost", "content");
    core.register_view(view).await.unwrap();

    let retrieved = core.get_view("ContentView").unwrap().unwrap();
    assert_eq!(retrieved.name, "ContentView");
    assert!(retrieved.is_identity());
}

#[tokio::test]
async fn register_view_fails_with_missing_source() {
    let core = SchemaCore::new_for_testing().await.unwrap();
    let view = identity_view("BadView", "NonExistent", "field");
    let result = core.register_view(view).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test]
async fn register_view_fails_when_name_collides_with_schema() {
    let core = SchemaCore::new_for_testing().await.unwrap();
    core.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();

    let view = identity_view("BlogPost", "BlogPost", "content");
    let result = core.register_view(view).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("already used by a schema"));
}

#[tokio::test]
async fn list_views_returns_all_registered() {
    let core = SchemaCore::new_for_testing().await.unwrap();
    core.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    core.load_schema_from_json(&weather_schema_json())
        .await
        .unwrap();

    core.register_view(identity_view("V1", "BlogPost", "content"))
        .await
        .unwrap();
    core.register_view(identity_view("V2", "Weather", "temp_celsius"))
        .await
        .unwrap();

    let views = core.get_views_with_states().unwrap();
    assert_eq!(views.len(), 2);
    assert!(views.iter().all(|(_, s)| *s == ViewState::Available));
}

#[tokio::test]
async fn approve_and_block_view() {
    let core = SchemaCore::new_for_testing().await.unwrap();
    core.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    core.register_view(identity_view("MyView", "BlogPost", "title"))
        .await
        .unwrap();

    core.approve_view("MyView").await.unwrap();
    let views = core.get_views_with_states().unwrap();
    let (_, state) = views.iter().find(|(v, _)| v.name == "MyView").unwrap();
    assert_eq!(*state, ViewState::Approved);

    core.block_view("MyView").await.unwrap();
    let views = core.get_views_with_states().unwrap();
    let (_, state) = views.iter().find(|(v, _)| v.name == "MyView").unwrap();
    assert_eq!(*state, ViewState::Blocked);
}

#[tokio::test]
async fn remove_view_cleans_up() {
    let core = SchemaCore::new_for_testing().await.unwrap();
    core.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    core.register_view(identity_view("TempView", "BlogPost", "title"))
        .await
        .unwrap();

    assert!(core.get_view("TempView").unwrap().is_some());

    core.remove_view("TempView").await.unwrap();
    assert!(core.get_view("TempView").unwrap().is_none());
}

#[tokio::test]
async fn view_persists_across_schema_core_instances() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let db_ops = std::sync::Arc::new(
        fold_db::db_operations::DbOperations::from_sled(db.clone())
            .await
            .unwrap(),
    );
    let bus = std::sync::Arc::new(
        fold_db::fold_db_core::infrastructure::message_bus::AsyncMessageBus::new(),
    );

    // First instance: load schema and register view
    {
        let core = SchemaCore::new(db_ops.clone(), bus.clone()).await.unwrap();
        core.load_schema_from_json(&blogpost_schema_json())
            .await
            .unwrap();
        core.register_view(identity_view("PersistView", "BlogPost", "content"))
            .await
            .unwrap();
    }

    // Second instance: view should be loaded from storage
    {
        let core2 = SchemaCore::new(db_ops, bus).await.unwrap();
        let view = core2.get_view("PersistView").unwrap();
        assert!(view.is_some(), "View should persist across instances");
        assert_eq!(view.unwrap().name, "PersistView");
    }
}

#[tokio::test]
async fn multi_source_view() {
    let core = SchemaCore::new_for_testing().await.unwrap();
    core.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    core.load_schema_from_json(&weather_schema_json())
        .await
        .unwrap();

    // View with input queries from two different schemas + WASM transform
    let view = TransformView::new(
        "Dashboard",
        SchemaType::Single,
        None,
        vec![
            Query::new("BlogPost".to_string(), vec!["content".to_string()]),
            Query::new("Weather".to_string(), vec!["temp_celsius".to_string()]),
        ],
        Some(vec![0, 1, 2]), // Placeholder WASM (won't be executed in registration)
        HashMap::from([
            ("summary".to_string(), FieldValueType::String),
            ("temp".to_string(), FieldValueType::Number),
        ]),
    );

    core.register_view(view).await.unwrap();
    let retrieved = core.get_view("Dashboard").unwrap().unwrap();
    assert_eq!(retrieved.input_queries.len(), 2);
    assert_eq!(retrieved.output_fields.len(), 2);
    assert!(retrieved.source_schemas().contains(&"BlogPost".to_string()));
    assert!(retrieved.source_schemas().contains(&"Weather".to_string()));
}

#[tokio::test]
async fn view_can_reference_another_view_as_source() {
    let core = SchemaCore::new_for_testing().await.unwrap();
    core.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();

    // ViewA reads from BlogPost
    core.register_view(identity_view("ViewA", "BlogPost", "content"))
        .await
        .unwrap();

    // ViewB reads from ViewA (a view, not a schema)
    let view_b = TransformView::new(
        "ViewB",
        SchemaType::Single,
        None,
        vec![Query::new("ViewA".to_string(), vec!["content".to_string()])],
        None,
        HashMap::from([("content".to_string(), FieldValueType::Any)]),
    );
    core.register_view(view_b).await.unwrap();

    assert!(core.get_view("ViewB").unwrap().is_some());
}

#[tokio::test]
async fn name_exists_checks_both_schemas_and_views() {
    let core = SchemaCore::new_for_testing().await.unwrap();
    core.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();
    core.register_view(identity_view("MyView", "BlogPost", "title"))
        .await
        .unwrap();

    assert!(core.name_exists("BlogPost").unwrap());
    assert!(core.name_exists("MyView").unwrap());
    assert!(!core.name_exists("Unknown").unwrap());
}

#[tokio::test]
async fn register_view_with_typed_output_fields() {
    let core = SchemaCore::new_for_testing().await.unwrap();
    core.load_schema_from_json(&blogpost_schema_json())
        .await
        .unwrap();

    let view = TransformView::new(
        "TypedView",
        SchemaType::Single,
        None,
        vec![Query::new(
            "BlogPost".to_string(),
            vec!["title".to_string(), "content".to_string()],
        )],
        Some(vec![0, 1, 2]), // WASM placeholder
        HashMap::from([
            ("word_count".to_string(), FieldValueType::Integer),
            ("enriched_title".to_string(), FieldValueType::String),
        ]),
    );

    core.register_view(view).await.unwrap();
    let retrieved = core.get_view("TypedView").unwrap().unwrap();
    assert_eq!(
        *retrieved.output_fields.get("word_count").unwrap(),
        FieldValueType::Integer
    );
    assert_eq!(
        *retrieved.output_fields.get("enriched_title").unwrap(),
        FieldValueType::String
    );
}
