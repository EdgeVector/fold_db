#![cfg(feature = "aws-backend")]
use aws_smithy_runtime_api::client::http::HttpClient;
use aws_smithy_runtime_api::client::orchestrator::HttpRequest;
use aws_smithy_runtime_api::client::runtime_components::RuntimeComponents;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct MockHttpClient {
    pub requests: Arc<Mutex<Vec<HttpRequest>>>,
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
            requests.lock().unwrap().push(request);

            let sdk_body = aws_smithy_types::body::SdkBody::from(body);
            let response = http::Response::builder()
                .status(status)
                .body(sdk_body)
                .unwrap();

            Ok(
                aws_smithy_runtime_api::client::orchestrator::HttpResponse::try_from(response)
                    .unwrap(),
            )
        })
    }
}

#[test]
fn check_mock_compiles() {
    let mock = MockHttpClient::new();
    let _ = mock;
}
