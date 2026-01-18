use datafold::lambda::ui::get_ui_asset;
use lambda_http::{run, service_fn, Body, Error, Request, RequestExt, Response};

/// Example Lambda handler that serves both the API and the React UI
///
/// This demonstrates how to use the `datafold::lambda::ui` module to serve
/// the bundled React application from a Lambda function behind API Gateway.
async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    let path = event.uri().path();

    // 1. API Routing
    // Requests starting with /api are handled by your backend logic
    if path.starts_with("/api") {
        let resp = Response::builder()
            .status(200)
            .header("content-type", "application/json")
            .body(Body::from(r#"{"message": "Hello from DataFold API!"}"#))
            .map_err(Box::new)?;
        return Ok(resp);
    }

    // 2. UI Routing
    // All other requests are treated as UI asset requests.
    // get_ui_asset handles the "SPA fallback" logic automatically
    // (serving index.html for unknown paths like /dashboard).
    if let Some(asset) = get_ui_asset(path) {
        let resp = Response::builder()
            .status(200)
            .header("content-type", asset.mime_type)
            // Optional: Add caching for static assets
            .header("cache-control", "public, max-age=3600")
            .body(Body::from(asset.content))
            .map_err(Box::new)?;
        return Ok(resp);
    }

    // 3. Fallback (should be covered by get_ui_asset, but just in case)
    Ok(Response::builder()
        .status(404)
        .body(Body::from("Not Found"))
        .unwrap())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    run(service_fn(function_handler)).await
}
