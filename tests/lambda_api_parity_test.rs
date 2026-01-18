#[cfg(feature = "lambda")]
#[cfg(test)]
mod tests {
    use datafold::lambda::LambdaContext;
    // This test primarily checks that the API methods are present and compile.
    // Execution might fail without a proper environment (Lambda context initialized).
    #[tokio::test]
    async fn test_lambda_api_signatures_exist() {
        // Schema API
        let _ = LambdaContext::get_backfill_status;

        // Query API
        let _ = LambdaContext::native_index_search;
        let _ = LambdaContext::execute_mutation;
        let _ = LambdaContext::execute_mutations_batch;
        let _ = LambdaContext::list_transforms;
        let _ = LambdaContext::get_transform_queue;
        let _ = LambdaContext::add_to_transform_queue;
        let _ = LambdaContext::get_all_backfills;
        let _ = LambdaContext::get_active_backfills;
        let _ = LambdaContext::get_backfill;
        let _ = LambdaContext::get_backfill_statistics;
        let _ = LambdaContext::get_transform_statistics;
        let _ = LambdaContext::get_indexing_status;

        // Ingestion API
        let _ = LambdaContext::health_check;
        let _ = LambdaContext::get_ingestion_config;
        let _ = LambdaContext::save_ingestion_config;

        // System API
        let _ = LambdaContext::get_database_config;
        let _ = LambdaContext::update_database_config;

        // Logging API
        let _ = LambdaContext::list_logs;
        let _ = LambdaContext::get_log_config;
        let _ = LambdaContext::reload_log_config;
        let _ = LambdaContext::get_log_features;
        let _ = LambdaContext::update_log_feature_level;
    }
}
