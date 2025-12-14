use crate::datafold_node::static_assets::Asset;
use serde::Serialize;

#[derive(Serialize)]
pub struct UiAssetResponse {
    pub content: Vec<u8>,
    pub mime_type: String,
}

/// Helper to get a UI asset for Lambda serving
pub fn get_ui_asset(path: &str) -> Option<UiAssetResponse> {
    // Should match the logic in http_server.rs:serve_ui
    let path = if path.is_empty() || path == "/" { "index.html" } else { path };
    let path = path.trim_start_matches('/');

    match Asset::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Some(UiAssetResponse {
                content: content.data.into_owned(),
                mime_type: mime.as_ref().to_string(),
            })
        }
        None => {
            // SPA Fallback
            if let Some(content) = Asset::get("index.html") {
                Some(UiAssetResponse {
                    content: content.data.into_owned(),
                    mime_type: "text/html".to_string(),
                })
            } else {
                None
            }
        }
    }
}
