use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::datafold_node::schema_routes::list_schemas,
        crate::datafold_node::schema_routes::load_schemas,
        crate::datafold_node::schema_routes::get_schema,
        crate::datafold_node::schema_routes::approve_schema,
        crate::datafold_node::schema_routes::block_schema,
        crate::datafold_node::query_routes::execute_query,
        crate::datafold_node::query_routes::execute_mutation,
        crate::datafold_node::query_routes::list_transforms,
        crate::datafold_node::query_routes::get_transform_queue,
        crate::datafold_node::query_routes::add_to_transform_queue,
        crate::datafold_node::security_routes::get_system_public_key,
        crate::datafold_node::system_routes::get_system_status,
        crate::datafold_node::system_routes::get_node_private_key,
        crate::datafold_node::system_routes::get_node_public_key,
        crate::datafold_node::system_routes::reset_database,
        crate::datafold_node::log_routes::list_logs,
        crate::datafold_node::log_routes::stream_logs,
        crate::datafold_node::log_routes::get_config,
        crate::datafold_node::log_routes::update_feature_level,
        crate::datafold_node::log_routes::reload_config,
        crate::datafold_node::log_routes::get_features,
        crate::ingestion::routes::process_json,
        crate::ingestion::routes::get_status,
        crate::ingestion::routes::health_check,
        crate::ingestion::routes::validate_json,
        crate::ingestion::routes::get_ingestion_config,
        crate::ingestion::routes::save_ingestion_config
    ),
    components(
        schemas(
            crate::schema::types::schema::Schema,
            crate::schema::types::schema::SchemaType,
            crate::schema::types::key_config::KeyConfig
        )
    ),
    tags(
        (name = "schemas", description = "Schema management endpoints"),
        (name = "query", description = "Query and mutation endpoints"),
        (name = "security", description = "Security and key management endpoints"),
        (name = "system", description = "System management endpoints"),
        (name = "logs", description = "Logging endpoints"),
        (name = "ingestion", description = "Ingestion endpoints")
    )
)]
struct ApiDoc;

pub fn build_openapi() -> String {
    serde_json::to_string(&ApiDoc::openapi()).unwrap_or_else(|_| "{}".to_string())
}


