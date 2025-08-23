use super::http_server::AppState;
use crate::schema::types::{
    operations::{Mutation, Query},
    Operation,
};
use crate::security::types::SignedMessage;
use crate::security::VerificationResult;
use crate::datafold_node::DataFoldNode;
use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::{json, Value};
use base64::{engine::general_purpose, Engine as _};
use crate::log_feature;
use crate::logging::features::LogFeature;

/// Execute an operation (query or mutation).
#[derive(Deserialize)]
pub struct OperationRequest {
    operation: String,
}

pub async fn execute_operation(
    request: web::Json<OperationRequest>,
    state: web::Data<AppState>,
) -> impl Responder {
    let operation_str = &request.operation;

    let operation: Operation = match serde_json::from_str(operation_str) {
        Ok(op) => op,
        Err(e) => {
            return HttpResponse::BadRequest()
                .json(json!({"error": format!("Failed to parse operation: {}", e)}));
        }
    };

    let mut node_guard = state.node.lock().await;

    match node_guard.execute_operation(operation) {
        Ok(result) => HttpResponse::Ok().json(json!({"data": result})),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({"error": format!("Failed to execute operation: {}", e)})),
    }
}

/// Execute a query.
pub async fn execute_query(query: web::Json<Value>, state: web::Data<AppState>) -> impl Responder {
    let query_value = query.into_inner();
    log_feature!(
        LogFeature::Query,
        info,
        "Received query request: {}",
        serde_json::to_string(&query_value).unwrap_or_else(|_| "Invalid JSON".to_string())
    );

    // Parse the simple web UI operation
    let web_operation = match serde_json::from_value::<Operation>(query_value) {
        Ok(op) => match op {
            Operation::Query { .. } => op,
            _ => {
                return HttpResponse::BadRequest()
                    .json(json!({"error": "Expected a query operation"}))
            }
        },
        Err(e) => {
            return HttpResponse::BadRequest()
                .json(json!({"error": format!("Failed to parse query: {}", e)}))
        }
    };

    // Convert to full internal query with default trust_distance=0 and pub_key="web-ui"
    let internal_query = match web_operation {
        Operation::Query {
            schema,
            fields,
            filter,
        } => Query {
            schema_name: schema,
            fields,
            pub_key: "web-ui".to_string(),
            trust_distance: 0,
            filter,
        },
        _ => {
            return HttpResponse::BadRequest().json(json!({"error": "Expected a query operation"}))
        }
    };

    let mut node_guard = state.node.lock().await;

    match node_guard.query(internal_query) {
        Ok(results) => {
            log_feature!(LogFeature::Query, info, "Query executed successfully");
            // Convert Vec<Result<Value, SchemaError>> to Vec<Value> with errors as JSON
            let unwrapped: Vec<Value> = results
                .into_iter()
                .map(|r| r.unwrap_or_else(|e| serde_json::json!({"error": e.to_string()})))
                .collect();
            HttpResponse::Ok().json(json!({"data": unwrapped}))
        }
        Err(e) => {
            log_feature!(LogFeature::Query, error, "Query execution failed: {}", e);
            HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to execute query: {}", e)}))
        }
    }
}

/// Execute a mutation.
pub async fn execute_mutation(
    signed_message: web::Json<SignedMessage>,
    state: web::Data<AppState>,
) -> impl Responder {
    let mut node_guard = state.node.lock().await;

    let verification_data = match verify_signature(&mut node_guard, &signed_message).await {
        Ok(data) => data,
        Err(resp) => return resp,
    };

    let mutation_value = match decode_payload(&signed_message).await {
        Ok(value) => value,
        Err(resp) => return resp,
    };

    log_feature!(
        LogFeature::Mutation,
        info,
        "Received mutation request: {}",
        serde_json::to_string(&mutation_value).unwrap_or_else(|_| "Invalid JSON".to_string())
    );

    let internal_mutation = match build_internal_mutation(mutation_value, &verification_data).await {
        Ok(m) => m,
        Err(resp) => return resp,
    };

    match node_guard.mutate(internal_mutation) {
        Ok(_) => {
            log_feature!(LogFeature::Mutation, info, "Mutation executed successfully");
            HttpResponse::Ok().json(json!({"success": true}))
        }
        Err(e) => {
            log_feature!(LogFeature::Mutation, error, "Mutation execution failed: {}", e);
            HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to execute mutation: {}", e)}))
        }
    }
}

async fn verify_signature(
    node: &mut DataFoldNode,
    signed_message: &SignedMessage,
) -> Result<VerificationResult, HttpResponse> {
    let verification_result = node.security_manager.verify_message(signed_message);

    if let Err(e) = verification_result {
        log_feature!(LogFeature::Mutation, warn, "Signature verification failed: {}", e);
        return Err(
            HttpResponse::Unauthorized()
                .json(json!({"error": format!("Signature verification failed: {}", e)})),
        );
    }

    let verification_data = verification_result.unwrap();
    if !verification_data.is_valid {
        log_feature!(
            LogFeature::Mutation,
            warn,
            "Invalid signature for public key: {}",
            signed_message.public_key_id
        );
        return Err(HttpResponse::Forbidden().json(json!({"error": "Invalid signature"})));
    }

    Ok(verification_data)
}

async fn decode_payload(signed_message: &SignedMessage) -> Result<Value, HttpResponse> {
    let decoded_payload = match general_purpose::STANDARD.decode(&signed_message.payload) {
        Ok(bytes) => bytes,
        Err(e) => {
            log_feature!(LogFeature::Mutation, warn, "Base64 decoding failed: {}", e);
            return Err(HttpResponse::BadRequest().json(json!({"error": "Invalid base64 payload"})));
        }
    };

    let mutation_value: Value = match serde_json::from_slice(&decoded_payload) {
        Ok(val) => val,
        Err(e) => {
            log_feature!(LogFeature::Mutation, warn, "Payload JSON parsing failed: {}", e);
            return Err(HttpResponse::BadRequest().json(json!({"error": "Invalid JSON payload format"})));
        }
    };

    Ok(mutation_value)
}

async fn build_internal_mutation(
    mutation_value: Value,
    verification_data: &VerificationResult,
) -> Result<Mutation, HttpResponse> {
    let web_operation = match serde_json::from_value::<Operation>(mutation_value) {
        Ok(op) => match op {
            Operation::Mutation { .. } => op,
            _ => {
                return Err(HttpResponse::BadRequest().json(json!({"error": "Expected a mutation operation"})));}
        },
        Err(e) => {
            return Err(
                HttpResponse::BadRequest().json(json!({"error": format!("Failed to parse mutation: {}", e)})),
            );
        }
    };

    match web_operation {
        Operation::Mutation {
            schema,
            data,
            mutation_type,
        } => {
            let fields_and_values = match data {
                Value::Object(map) => map.into_iter().collect(),
                _ => {
                    return Err(
                        HttpResponse::BadRequest().json(json!({"error": "Mutation data must be an object"})),
                    );
                }
            };

            Ok(Mutation {
                schema_name: schema,
                fields_and_values,
                pub_key: verification_data
                    .public_key_info
                    .as_ref()
                    .map(|info| info.owner_id.clone())
                    .unwrap_or_else(|| "unknown".to_string()),
                trust_distance: 0,
                mutation_type,
                synchronous: None,
            })
        }
        _ => Err(HttpResponse::BadRequest().json(json!({"error": "Expected a mutation operation"}))),
    }
}

pub async fn list_transforms(state: web::Data<AppState>) -> impl Responder {
    let node = state.node.lock().await;
    match node.list_transforms() {
        Ok(map) => HttpResponse::Ok().json(json!({ "data": map })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "error": format!("Failed to list transforms: {}", e) })),
    }
}

pub async fn run_transform(path: web::Path<String>, state: web::Data<AppState>) -> impl Responder {
    let id = path.into_inner();
    let mut node = state.node.lock().await;
    match node.run_transform(&id) {
        Ok(val) => HttpResponse::Ok().json(json!({ "data": val })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "error": format!("Failed to run transform: {}", e) })),
    }
}

pub async fn add_to_transform_queue(
    path: web::Path<String>,
    state: web::Data<AppState>,
) -> impl Responder {
    let transform_id = path.into_inner();
    let node = state.node.lock().await;

    match node.list_transforms() {
        Ok(transforms) => {
            if !transforms.contains_key(&transform_id) {
                return HttpResponse::NotFound().json(json!({"error": format!("Transform '{}' not found. Available transforms: {:?}", transform_id, transforms.keys().collect::<Vec<_>>())}));
            }
        }
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(json!({"error": format!("Failed to verify transform: {}", e)}))
        }
    }

    match node.add_transform_to_queue(&transform_id) {
        Ok(_) => HttpResponse::Ok().json(json!({"success": true, "message": format!("Transform '{}' added to queue", transform_id)})),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("Failed to add transform to queue: {}", e)})),
    }
}

pub async fn get_transform_queue(state: web::Data<AppState>) -> impl Responder {
    let node = state.node.lock().await;
    match node.get_transform_queue_info() {
        Ok(info) => HttpResponse::Ok().json(info),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "error": format!("Failed to get transform queue info: {}", e) })),
    }
}

#[cfg(test)]
mod tests {
    
    
    
    

}
