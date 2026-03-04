use crate::log_feature;
use crate::logging::features::LogFeature;
use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::json;
use std::path::{Path, PathBuf};

/// Expand a leading `~` or `~/` to the current user's home directory.
///
/// Returns `Err` when the path starts with `~` but the `HOME` environment
/// variable is not set.
fn expand_tilde(raw: &str) -> Result<PathBuf, String> {
    if raw == "~" || raw.starts_with("~/") {
        match std::env::var("HOME") {
            Ok(home) => {
                let rest = raw.strip_prefix("~/").unwrap_or("");
                Ok(PathBuf::from(home).join(rest))
            }
            Err(_) => Err(format!(
                "Cannot expand ~ in path \"{}\": HOME environment variable not set",
                raw
            )),
        }
    } else {
        Ok(PathBuf::from(raw))
    }
}

/// Returns true if the given path string points to an existing directory.
pub fn is_existing_directory(p: &str) -> bool {
    match expand_tilde(p) {
        Ok(path) => path.is_dir(),
        Err(_) => false,
    }
}

/// Request body for filesystem path completion
#[derive(Deserialize)]
pub struct PathCompleteRequest {
    pub partial_path: String,
}

/// Complete a partial filesystem path with matching directories
///
/// This endpoint provides directory-only path completion for the folder picker UI.
/// It lists directories matching a partial path prefix, hiding dotfiles.
pub async fn complete_path(body: web::Json<PathCompleteRequest>) -> impl Responder {
    let partial = match expand_tilde(&body.partial_path) {
        Ok(p) => p,
        Err(msg) => {
            log_feature!(LogFeature::HttpServer, warn, "{}", msg);
            return HttpResponse::BadRequest().json(json!({
                "error": msg,
                "completions": Vec::<String>::new()
            }));
        }
    };
    let partial_str = partial.to_string_lossy();

    let (parent, prefix) = if partial_str.ends_with('/') || partial_str.ends_with('\\') {
        (PathBuf::from(partial_str.as_ref()), String::new())
    } else {
        let parent = partial.parent().unwrap_or(Path::new("/")).to_path_buf();
        let prefix = partial
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        (parent, prefix)
    };

    let entries = match std::fs::read_dir(&parent) {
        Ok(entries) => entries,
        Err(_) => return HttpResponse::Ok().json(json!({ "completions": Vec::<String>::new() })),
    };

    let prefix_lower = prefix.to_lowercase();
    let mut completions: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter(|e| {
            if prefix.is_empty() {
                return true;
            }
            e.file_name()
                .to_string_lossy()
                .to_lowercase()
                .starts_with(&prefix_lower)
        })
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .map(|e| e.path().to_string_lossy().to_string())
        .collect();

    completions.sort();
    completions.truncate(20);

    HttpResponse::Ok().json(json!({ "completions": completions }))
}

/// Request body for listing directory contents
#[derive(Deserialize)]
pub struct ListDirectoryRequest {
    pub path: String,
}

/// List subdirectories inside a given directory path.
///
/// Returns directory **names** (not full paths), up to 200 entries, hiding dotfiles.
/// Used by the web-based directory browser modal.
pub async fn list_directory(body: web::Json<ListDirectoryRequest>) -> impl Responder {
    let dir_path = match expand_tilde(&body.path) {
        Ok(p) => p,
        Err(msg) => {
            log_feature!(LogFeature::HttpServer, warn, "{}", msg);
            return HttpResponse::BadRequest().json(json!({
                "error": msg,
                "path": body.path,
                "directories": Vec::<String>::new(),
            }));
        }
    };
    let resolved = dir_path.to_string_lossy().to_string();

    if !dir_path.is_dir() {
        return HttpResponse::Ok().json(json!({
            "error": format!("'{}' is not a directory or is unreadable", body.path),
            "path": resolved,
            "directories": Vec::<String>::new(),
        }));
    }

    let entries = match std::fs::read_dir(&dir_path) {
        Ok(entries) => entries,
        Err(e) => {
            return HttpResponse::Ok().json(json!({
                "error": format!("Cannot read directory: {}", e),
                "path": resolved,
                "directories": Vec::<String>::new(),
            }));
        }
    };

    let mut directories: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    directories.sort();
    directories.truncate(200);

    HttpResponse::Ok().json(json!({
        "path": resolved,
        "directories": directories,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;

    // --- expand_tilde tests ---

    #[tokio::test]
    async fn test_expand_tilde_home() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(expand_tilde("~").unwrap(), PathBuf::from(&home));
    }

    #[tokio::test]
    async fn test_expand_tilde_subpath() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(
            expand_tilde("~/Documents").unwrap(),
            PathBuf::from(&home).join("Documents")
        );
    }

    #[tokio::test]
    async fn test_expand_tilde_noop_absolute() {
        assert_eq!(expand_tilde("/tmp").unwrap(), PathBuf::from("/tmp"));
    }

    #[tokio::test]
    async fn test_expand_tilde_noop_relative() {
        assert_eq!(expand_tilde("foo/bar").unwrap(), PathBuf::from("foo/bar"));
    }

    // --- complete_path tests ---

    #[tokio::test]
    async fn test_complete_path_with_known_dir() {
        let req = test::TestRequest::post().to_http_request();
        let body = web::Json(PathCompleteRequest {
            partial_path: "/tmp".to_string(),
        });
        let resp = complete_path(body).await.respond_to(&req);
        assert_eq!(resp.status(), 200);

        let bytes = actix_web::body::to_bytes(resp.into_body())
            .await
            .ok()
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json["completions"].is_array());
    }

    #[tokio::test]
    async fn test_complete_path_tilde_expansion() {
        // Use a temp directory as HOME so the test works in minimal containers
        let td = tempfile::tempdir().unwrap();
        std::fs::create_dir(td.path().join("visible_dir")).unwrap();
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", td.path());

        let req = test::TestRequest::post().to_http_request();
        let body = web::Json(PathCompleteRequest {
            partial_path: "~/".to_string(),
        });
        let resp = complete_path(body).await.respond_to(&req);
        assert_eq!(resp.status(), 200);

        let bytes = actix_web::body::to_bytes(resp.into_body())
            .await
            .ok()
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let completions = json["completions"].as_array().unwrap();
        assert!(!completions.is_empty());
        // Completions should be absolute paths (tilde expanded)
        for c in completions {
            assert!(c.as_str().unwrap().starts_with('/'));
        }

        match original_home {
            Some(h) => std::env::set_var("HOME", h),
            None => std::env::remove_var("HOME"),
        }
    }

    #[tokio::test]
    async fn test_complete_path_nonexistent() {
        let req = test::TestRequest::post().to_http_request();
        let body = web::Json(PathCompleteRequest {
            partial_path: "/nonexistent_path_abc123/".to_string(),
        });
        let resp = complete_path(body).await.respond_to(&req);
        assert_eq!(resp.status(), 200);

        let bytes = actix_web::body::to_bytes(resp.into_body())
            .await
            .ok()
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json["completions"].as_array().unwrap().is_empty());
    }

    // --- list_directory tests ---

    #[tokio::test]
    async fn test_list_directory_root() {
        let req = test::TestRequest::post().to_http_request();
        let body = web::Json(ListDirectoryRequest {
            path: "/".to_string(),
        });
        let resp = list_directory(body).await.respond_to(&req);
        assert_eq!(resp.status(), 200);

        let bytes = actix_web::body::to_bytes(resp.into_body())
            .await
            .ok()
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let dirs = json["directories"].as_array().unwrap();
        assert!(!dirs.is_empty());
        assert!(json["error"].is_null());
        // Should contain well-known directories
        let names: Vec<&str> = dirs.iter().map(|d| d.as_str().unwrap()).collect();
        assert!(names.contains(&"usr"));
    }

    #[tokio::test]
    async fn test_list_directory_tilde_expansion() {
        // Use a temp directory as HOME so the test works in minimal containers
        let td = tempfile::tempdir().unwrap();
        std::fs::create_dir(td.path().join("visible_dir")).unwrap();
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", td.path());

        let req = test::TestRequest::post().to_http_request();
        let body = web::Json(ListDirectoryRequest {
            path: "~".to_string(),
        });
        let resp = list_directory(body).await.respond_to(&req);
        assert_eq!(resp.status(), 200);

        let bytes = actix_web::body::to_bytes(resp.into_body())
            .await
            .ok()
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let dirs = json["directories"].as_array().unwrap();
        assert!(!dirs.is_empty());
        // Path should be the expanded home directory
        assert!(json["path"].as_str().unwrap().starts_with('/'));

        match original_home {
            Some(h) => std::env::set_var("HOME", h),
            None => std::env::remove_var("HOME"),
        }
    }

    #[tokio::test]
    async fn test_list_directory_nonexistent() {
        let req = test::TestRequest::post().to_http_request();
        let body = web::Json(ListDirectoryRequest {
            path: "/nonexistent_path_abc123".to_string(),
        });
        let resp = list_directory(body).await.respond_to(&req);
        assert_eq!(resp.status(), 200);

        let bytes = actix_web::body::to_bytes(resp.into_body())
            .await
            .ok()
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json["directories"].as_array().unwrap().is_empty());
        assert!(json["error"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_list_directory_returns_names_not_paths() {
        let td = tempfile::tempdir().unwrap();
        std::fs::create_dir(td.path().join("subdir_a")).unwrap();
        std::fs::create_dir(td.path().join("subdir_b")).unwrap();
        // Create a dotfile dir that should be hidden
        std::fs::create_dir(td.path().join(".hidden")).unwrap();

        let req = test::TestRequest::post().to_http_request();
        let body = web::Json(ListDirectoryRequest {
            path: td.path().to_string_lossy().to_string(),
        });
        let resp = list_directory(body).await.respond_to(&req);
        assert_eq!(resp.status(), 200);

        let bytes = actix_web::body::to_bytes(resp.into_body())
            .await
            .ok()
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let dirs = json["directories"].as_array().unwrap();
        let names: Vec<&str> = dirs.iter().map(|d| d.as_str().unwrap()).collect();
        assert_eq!(names, vec!["subdir_a", "subdir_b"]);
        assert!(!names.contains(&".hidden"));
    }
}
