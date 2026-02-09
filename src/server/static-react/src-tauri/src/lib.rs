use std::sync::Arc;
use tokio::sync::Mutex;
use fold_db::datafold_node::DataFoldNode;
use fold_db::load_node_config;
use fold_db::server::{start_embedded_server, EmbeddedServerHandle};
use tauri::{Manager, State};
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
        .join(".datafold")
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
  tauri::Builder::default()
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_dialog::init())
    .invoke_handler(tauri::generate_handler![
      get_server_status,
      open_data_directory,
      get_app_version
    ])
    .setup(|app| {
      // Set up logging
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      // Initialize DataFold server
      let server_port = 9001;
      
      // Start the server in a background task
      let app_handle = app.handle().clone();
      tauri::async_runtime::spawn(async move {
        match start_datafold_server(server_port).await {
          Ok(handle) => {
            log::info!("DataFold server started successfully on port {}", server_port);
            
            // Store the server handle in app state
            if let Some(state) = app_handle.try_state::<AppState>() {
              let mut server = state.server_handle.lock().await;
              *server = Some(handle);
            }
          }
          Err(e) => {
            log::error!("Failed to start DataFold server: {}", e);
            // Continue running the app even if server fails to start
            // This allows the user to see error messages
          }
        }
      });

      // Initialize app state
      app.manage(AppState {
        server_handle: Arc::new(Mutex::new(None)),
        server_port,
      });

      log::info!("DataFold desktop app initialized. Server will be available at http://localhost:{}", server_port);

      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

/// Start the DataFold embedded server
async fn start_datafold_server(port: u16) -> Result<EmbeddedServerHandle, String> {
    // Determine the data directory for the app
    // Use a dedicated directory in the user's home folder
    let data_dir = dirs::home_dir()
        .ok_or_else(|| "Could not determine home directory".to_string())?
        .join(".datafold")
        .join("data");

    // Create the directory if it doesn't exist
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create data directory: {}", e))?;

    log::info!("Using data directory: {:?}", data_dir);

    // Load node configuration
    let mut config = load_node_config(None, None)
        .map_err(|e| format!("Failed to load config: {}", e))?;

    // Set the database path via DatabaseConfig
    config.database = fold_db::DatabaseConfig::Local { path: data_dir };

    // Set schema service URL - use a default or environment variable
    // For the native app, we'll use a local schema service if available
    // or allow it to run without one
    if let Ok(schema_url) = std::env::var("DATAFOLD_SCHEMA_SERVICE_URL") {
        config.schema_service_url = Some(schema_url);
    } else {
        // Use mock for now - schemas can be added manually
        config.schema_service_url = Some("mock://local".to_string());
    }

    // Create the node (async)
    let node = DataFoldNode::new(config).await
        .map_err(|e| format!("Failed to create node: {}", e))?;

    // Start the embedded server
    let handle = start_embedded_server(node, port).await
        .map_err(|e| format!("Failed to start server: {}", e))?;

    Ok(handle)
}

// Add dirs dependency for home directory detection
// This will be added to Cargo.toml
