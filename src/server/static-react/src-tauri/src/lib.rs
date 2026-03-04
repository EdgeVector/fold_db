use std::sync::Arc;
use tokio::sync::Mutex;
use fold_db::load_node_config;
use fold_db::security::Ed25519KeyPair;
use fold_db::server::{start_embedded_server_lazy, EmbeddedServerHandle, NodeManagerConfig};
use fold_db::DatabaseConfig;
use tauri::{Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use serde::{Serialize, Deserialize};

/// Shared state for the Tauri application
pub struct AppState {
    pub server_handle: Arc<Mutex<Option<EmbeddedServerHandle>>>,
    pub server_port: u16,
}

/// Server status response
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerStatus {
    pub running: bool,
    pub port: u16,
    pub url: String,
}

/// Get the current server status
#[tauri::command]
async fn get_server_status(state: State<'_, AppState>) -> Result<ServerStatus, String> {
    let handle = state.server_handle.lock().await;
    let running = handle.as_ref().map(|h| h.is_running()).unwrap_or(false);

    Ok(ServerStatus {
        running,
        port: state.server_port,
        url: format!("http://localhost:{}", state.server_port),
    })
}

/// Open the data directory in Finder/Explorer
#[tauri::command]
async fn open_data_directory() -> Result<(), String> {
    let data_dir = dirs::home_dir()
        .ok_or("Could not determine home directory")?
        .join(".folddb")
        .join("data");

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&data_dir)
            .spawn()
            .map_err(|e| format!("Failed to open directory: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&data_dir)
            .spawn()
            .map_err(|e| format!("Failed to open directory: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&data_dir)
            .spawn()
            .map_err(|e| format!("Failed to open directory: {}", e))?;
    }

    Ok(())
}

/// Get the app version
#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  // Start the server in a separate thread with its own tokio runtime
  // so we can block on it without deadlocking Tauri's runtime
  let server_port = 9001u16;
  let (tx, rx) = std::sync::mpsc::channel::<Result<EmbeddedServerHandle, String>>();

  std::thread::spawn(move || {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let result = rt.block_on(start_fold_server(server_port));
    match result {
      Ok(handle) => {
        eprintln!("[FoldDB] Server started on port {}", server_port);
        let _ = tx.send(Ok(handle));
        // Keep runtime alive so the server keeps running
        rt.block_on(std::future::pending::<()>());
      }
      Err(e) => {
        eprintln!("[FoldDB] Failed to start server: {}", e);
        let _ = tx.send(Err(e));
      }
    }
  });

  // Wait for the server to start (with timeout)
  let server_result = rx.recv_timeout(std::time::Duration::from_secs(30));

  let (server_handle, startup_error) = match server_result {
    Ok(Ok(handle)) => (Some(handle), None),
    Ok(Err(e)) => (None, Some(e)),
    Err(_) => (None, Some("Server failed to start within 30 seconds.".to_string())),
  };

  tauri::Builder::default()
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_dialog::init())
    .invoke_handler(tauri::generate_handler![
      get_server_status,
      open_data_directory,
      get_app_version
    ])
    .setup(move |app| {
      // Try to set up logging — may fail if the embedded server already initialized a logger
      let _ = app.handle().plugin(
        tauri_plugin_log::Builder::default()
          .level(log::LevelFilter::Info)
          .build(),
      );

      // Initialize app state
      app.manage(AppState {
        server_handle: Arc::new(Mutex::new(server_handle)),
        server_port,
      });

      // If the server failed to start, show an error dialog and exit.
      // Note: sled lock conflicts no longer happen at startup (lazy init),
      // but other failures (config load, identity generation) can still occur.
      if let Some(error) = startup_error {
        let message = format!("FoldDB server failed to start:\n\n{}", error);

        app.dialog()
          .message(message)
          .kind(MessageDialogKind::Error)
          .title("FoldDB - Startup Error")
          .blocking_show();

        std::process::exit(1);
      }

      // Create the main window — server is already listening
      let url = format!("http://localhost:{}", server_port);
      let _window = WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url.parse().unwrap()))
        .title("FoldDB - Personal Database")
        .inner_size(1400.0, 900.0)
        .min_inner_size(1000.0, 700.0)
        .center()
        .build()
        .map_err(|e| format!("Failed to create window: {}", e))?;

      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

/// Start the Fold embedded server with lazy database initialization.
///
/// No database is opened at startup — the node is created lazily on the first
/// API request. This means the UI window appears immediately without waiting
/// for sled locks or other DB initialization.
async fn start_fold_server(port: u16) -> Result<EmbeddedServerHandle, String> {
    let data_dir = dirs::home_dir()
        .ok_or_else(|| "Could not determine home directory".to_string())?
        .join(".folddb")
        .join("data");

    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create data directory: {}", e))?;

    eprintln!("[FoldDB] Using data directory: {:?}", data_dir);

    // Load or generate node identity (persisted across launches)
    let identity_path = dirs::home_dir()
        .ok_or_else(|| "Could not determine home directory".to_string())?
        .join(".folddb")
        .join("node_identity.json");

    let (pub_key, priv_key) = if identity_path.exists() {
        let content = std::fs::read_to_string(&identity_path)
            .map_err(|e| format!("Failed to read identity: {}", e))?;
        let identity: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse identity: {}", e))?;
        let priv_k = identity["private_key"].as_str()
            .ok_or("Missing private_key in identity file")?.to_string();
        let pub_k = identity["public_key"].as_str()
            .ok_or("Missing public_key in identity file")?.to_string();
        eprintln!("[FoldDB] Loaded existing node identity");
        (pub_k, priv_k)
    } else {
        let keypair = Ed25519KeyPair::generate()
            .map_err(|e| format!("Failed to generate keypair: {}", e))?;
        let pub_k = keypair.public_key_base64();
        let priv_k = keypair.secret_key_base64();
        let identity = serde_json::json!({
            "private_key": priv_k,
            "public_key": pub_k,
        });
        std::fs::write(&identity_path, serde_json::to_string_pretty(&identity).unwrap())
            .map_err(|e| format!("Failed to save identity: {}", e))?;
        eprintln!("[FoldDB] Generated and saved new node identity");
        (pub_k, priv_k)
    };

    // Set NODE_CONFIG to a writable path so persist_node_config() can save.
    // The bundled .app runs inside a read-only code-signed directory,
    // so the default relative "config/node_config.json" would fail.
    let config_path = dirs::home_dir()
        .ok_or_else(|| "Could not determine home directory".to_string())?
        .join(".folddb")
        .join("node_config.json");
    std::env::set_var("NODE_CONFIG", &config_path);
    eprintln!("[FoldDB] Config path: {:?}", config_path);

    // Set FOLD_UPLOAD_PATH so upload storage uses an absolute writable path.
    // Without this, UploadStorageConfig defaults to the relative "data/uploads"
    // which resolves inside the read-only .app bundle on macOS.
    let upload_path = dirs::home_dir()
        .ok_or_else(|| "Could not determine home directory".to_string())?
        .join(".folddb")
        .join("uploads");
    std::env::set_var("FOLD_UPLOAD_PATH", &upload_path);
    eprintln!("[FoldDB] Upload path: {:?}", upload_path);

    // Set FOLD_CONFIG_DIR so ingestion_config.json is saved/loaded from ~/.folddb/
    // rather than ./config/ which resolves into the read-only .app bundle.
    let config_dir = dirs::home_dir()
        .ok_or_else(|| "Could not determine home directory".to_string())?
        .join(".folddb");
    std::env::set_var("FOLD_CONFIG_DIR", &config_dir);

    // Load node configuration (no DB access — just reads config file)
    let mut config = load_node_config(None, None)
        .map_err(|e| format!("Failed to load config: {}", e))?;

    // Set identity, database path, and schema service
    config = config.with_identity(&pub_key, &priv_key);
    config.database = DatabaseConfig::Local { path: data_dir };

    if let Ok(schema_url) = std::env::var("FOLD_SCHEMA_SERVICE_URL") {
        config.schema_service_url = Some(schema_url);
    } else {
        config.schema_service_url = Some("https://axo709qs11.execute-api.us-east-1.amazonaws.com".to_string());
    }

    // Build NodeManagerConfig — no FoldNode created yet
    let node_manager_config = NodeManagerConfig {
        base_config: config,
    };

    eprintln!("[FoldDB] Starting server with lazy database initialization...");

    let handle = start_embedded_server_lazy(node_manager_config, port).await
        .map_err(|e| format!("Failed to start server: {}", e))?;

    // Wait for the HTTP server to actually be listening before returning.
    // With lazy init the handle returns before actix binds the port.
    for i in 0..50 {
        if std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
            eprintln!("[FoldDB] Server is listening on port {} (took {}ms)", port, i * 50);
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    Ok(handle)
}
