use crate::commands::CommandOutput;
use crate::error::CliError;
use fold_db::datafold_node::OperationProcessor;
use fold_db::schema::types::field::HashRangeFilter;
use fold_db::schema::types::operations::Query;

pub async fn run(
    schema: &str,
    fields: &str,
    hash: Option<&str>,
    range: Option<&str>,
    processor: &OperationProcessor,
) -> Result<CommandOutput, CliError> {
    let field_list: Vec<String> = fields.split(',').map(|s| s.trim().to_string()).collect();
    let filter = build_filter(hash, range);
    let query = Query::new_with_filter(schema.to_string(), field_list, filter);
    let results = processor.execute_query_json(query).await?;
    Ok(CommandOutput::QueryResults(results))
}

fn build_filter(hash: Option<&str>, range: Option<&str>) -> Option<HashRangeFilter> {
    match (hash, range) {
        (Some(h), Some(r)) => Some(HashRangeFilter::HashRangeKey {
            hash: h.to_string(),
            range: r.to_string(),
        }),
        (Some(h), None) => Some(HashRangeFilter::HashKey(h.to_string())),
        (None, Some(r)) => Some(HashRangeFilter::RangePrefix(r.to_string())),
        (None, None) => None,
    }
}
