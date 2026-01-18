#![cfg(feature = "aws-backend")]
use aws_sdk_dynamodb::config::Region;
use aws_smithy_runtime_api::client::http::HttpClient;
use aws_smithy_runtime_api::client::orchestrator::HttpRequest;
use aws_smithy_runtime_api::client::runtime_components::RuntimeComponents;
use datafold::storage::dynamodb_backend::DynamoDbKvStore;
use datafold::storage::traits::KvStore;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct CapturedRequest {
    pub uri: String,
    pub body: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct MockHttpClient {
    pub requests: Arc<Mutex<Vec<CapturedRequest>>>,
    pub response_status: u16,
    pub response_body: String,
}

impl MockHttpClient {
    pub fn new() -> Self {
        Self {
            requests: Arc::new(Mutex::new(Vec::new())),
            response_status: 200,
            response_body: "{}".to_string(),
        }
    }

    pub fn with_response(mut self, status: u16, body: &str) -> Self {
        self.response_status = status;
        self.response_body = body.to_string();
        self
    }

    pub fn get_last_request(&self) -> Option<CapturedRequest> {
        self.requests.lock().unwrap().last().cloned()
    }
}

impl HttpClient for MockHttpClient {
    fn http_connector(
        &self,
        _settings: &aws_smithy_runtime_api::client::http::HttpConnectorSettings,
        _components: &RuntimeComponents,
    ) -> aws_smithy_runtime_api::client::http::SharedHttpConnector {
        aws_smithy_runtime_api::client::http::SharedHttpConnector::new(self.clone())
    }
}

impl aws_smithy_runtime_api::client::http::HttpConnector for MockHttpClient {
    fn call(
        &self,
        request: HttpRequest,
    ) -> aws_smithy_runtime_api::client::http::HttpConnectorFuture {
        let requests = self.requests.clone();
        let status = self.response_status;
        let body = self.response_body.clone();

        aws_smithy_runtime_api::client::http::HttpConnectorFuture::new(async move {
            // Capture request details
            let uri = request.uri().to_string();
            let body_bytes = if let Some(bytes) = request.body().bytes() {
                bytes.to_vec()
            } else {
                Vec::new() // Handle streaming body if needed, but for now assume in-memory
            };

            requests.lock().unwrap().push(CapturedRequest {
                uri,
                body: body_bytes,
            });

            let sdk_body = aws_smithy_types::body::SdkBody::from(body);
            let response = http::Response::builder()
                .status(status)
                .header("Content-Type", "application/x-amz-json-1.0")
                .body(sdk_body)
                .unwrap();

            Ok(
                aws_smithy_runtime_api::client::orchestrator::HttpResponse::try_from(response)
                    .unwrap(),
            )
        })
    }
}

fn create_mock_client(mock: MockHttpClient) -> aws_sdk_dynamodb::Client {
    let config = aws_sdk_dynamodb::Config::builder()
        .behavior_version(aws_sdk_dynamodb::config::BehaviorVersion::latest())
        .region(Region::new("us-east-1"))
        .endpoint_url("http://localhost")
        .credentials_provider(aws_sdk_dynamodb::config::Credentials::new(
            "test", "test", None, None, "test",
        ))
        .http_client(mock)
        .build();
    aws_sdk_dynamodb::Client::from_conf(config)
}

#[tokio::test]
async fn test_dynamodb_put_mock() {
    let mock = MockHttpClient::new();
    let client = create_mock_client(mock.clone());
    let store = DynamoDbKvStore::new(
        Arc::new(client),
        "TestTable".to_string(),
        "user1".to_string(),
    );

    let key = b"test_key";
    let value = b"test_value";

    store.put(key, value.to_vec()).await.expect("put failed");

    // Verify request
    let request = mock.get_last_request().expect("no request sent");
    let body_str = std::str::from_utf8(&request.body).expect("invalid utf8");

    // DynamoDB uses JSON
    // We expect a PutItem request
    // {"TableName":"TestTable","Item":{"PK":{"S":"user1:test_key"},"SK":{"S":"test_key"},"Value":{"S":"test_value"}}}

    println!("Request body: {}", body_str);
    // assert!(body_str.contains("PutItem")); // The action is in headers
    assert!(body_str.contains("TestTable"));
    // PK should be just "user1"
    assert!(body_str.contains("\"PK\":{\"S\":\"user1\"}"));
    // SK should be "test_key"
    assert!(body_str.contains("\"SK\":{\"S\":\"test_key\"}"));
    assert!(body_str.contains("test_value"));
}

#[tokio::test]
async fn test_dynamodb_get_mock() {
    // Construct a mock response for GetItem
    // Response format: {"Item":{"PK":{"S":"..."},"Value":{"S":"test_value"}}}
    let response_body = r#"
    {
        "Item": {
            "PK": {"S": "user1"},
            "SK": {"S": "test_key"},
            "Value": {"S": "test_value"}
        }
    }
    "#;

    let mock = MockHttpClient::new().with_response(200, response_body);
    let client = create_mock_client(mock.clone());
    let store = DynamoDbKvStore::new(
        Arc::new(client),
        "TestTable".to_string(),
        "user1".to_string(),
    );

    let key = b"test_key";
    let result = store.get(key).await.expect("get failed");

    assert_eq!(result, Some(b"test_value".to_vec()));

    // Verify request
    let request = mock.get_last_request().expect("no request sent");
    let body_str = std::str::from_utf8(&request.body).expect("invalid utf8");

    println!("Request body: {}", body_str);
    // Note: GetItem body might be empty if using query params, or JSON if using POST
    // AWS SDK usually uses POST for DynamoDB
    assert!(body_str.contains("TestTable"));
    // PK should be just "user1"
    assert!(body_str.contains("\"PK\":{\"S\":\"user1\"}"));
}

#[tokio::test]
async fn test_dynamodb_namespace_isolation_mock() {
    let mock = MockHttpClient::new();
    let client = create_mock_client(mock.clone());

    // Create two stores with different table names (simulating namespaces)
    let store1 = DynamoDbKvStore::new(
        Arc::new(client.clone()),
        "Namespace1".to_string(),
        "user1".to_string(),
    );
    let store2 = DynamoDbKvStore::new(
        Arc::new(client),
        "Namespace2".to_string(),
        "user1".to_string(),
    );

    let key = b"key";
    let value = b"value";

    // Put to store1
    store1.put(key, value.to_vec()).await.expect("put failed");
    let req1 = mock.get_last_request().expect("req1 missing");
    let body1 = std::str::from_utf8(&req1.body).expect("utf8");
    assert!(body1.contains("Namespace1"));
    assert!(!body1.contains("Namespace2"));

    // Put to store2
    store2.put(key, value.to_vec()).await.expect("put failed");
    let req2 = mock.get_last_request().expect("req2 missing");
    let body2 = std::str::from_utf8(&req2.body).expect("utf8");
    assert!(body2.contains("Namespace2"));
    assert!(!body2.contains("Namespace1"));
}

#[tokio::test]
async fn test_dynamodb_special_chars_mock() {
    let mock = MockHttpClient::new();
    let client = create_mock_client(mock.clone());
    let store = DynamoDbKvStore::new(
        Arc::new(client),
        "TestTable".to_string(),
        "user1".to_string(),
    );

    // Test key with special chars: colon, slash, space, unicode
    let key = "key:with/special chars_and_🚀".as_bytes();
    let value = b"value";

    store.put(key, value.to_vec()).await.expect("put failed");

    let request = mock.get_last_request().expect("request missing");
    let body_str = std::str::from_utf8(&request.body).expect("utf8");

    // Verify SK contains the special chars (JSON escaped)
    // "key:with/special chars_and_🚀"
    assert!(body_str.contains("key:with/special chars_and_"));
    // Unicode might be escaped or raw depending on serde, but usually raw in UTF-8
    // We'll check for the prefix to be safe
    assert!(body_str.contains("key:with/special"));
}
