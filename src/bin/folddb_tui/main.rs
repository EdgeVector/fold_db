mod app;
mod event;
mod tabs;
mod ui;

use app::App;
use clap::Parser;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use event::{Event, EventHandler};
use fold_db::{
    fold_node::{load_node_config, FoldNode, OperationProcessor},
    logging::{config::LogConfig, LoggingSystem},
    DatabaseConfig,
};
use ratatui::prelude::*;
use std::io;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "folddb_tui", about = "FoldDB Interactive TUI Dashboard")]
struct Cli {
    /// Path to data directory
    #[arg(long)]
    data_path: Option<String>,

    /// Path to config file
    #[arg(long)]
    config: Option<String>,

    /// Schema service URL
    #[arg(long)]
    schema_service_url: Option<String>,
}

fn user_hash_from_pubkey(pubkey: &str) -> String {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(pubkey.as_bytes());
    digest[..16]
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let mut config = load_node_config(cli.config.as_deref(), None)?;

    if let Some(path) = &cli.data_path {
        config.database = DatabaseConfig::Local {
            path: path.into(),
        };
    }
    if let Some(url) = &cli.schema_service_url {
        config.schema_service_url = Some(url.clone());
    }

    // Initialize logging — disable console (would corrupt TUI), enable web buffer for log panel
    let mut log_config = LogConfig::default();
    log_config.outputs.console.enabled = false;
    log_config.outputs.web.enabled = true;
    if let Err(e) = LoggingSystem::init_with_config(log_config).await {
        eprintln!("Warning: logging init failed: {}", e);
    }

    let node = FoldNode::new(config).await.map_err(|e| {
        format!("Failed to create node: {}", e)
    })?;

    let user_hash = std::env::var("FOLD_USER_HASH")
        .unwrap_or_else(|_| user_hash_from_pubkey(node.get_node_public_key()));

    let processor = Arc::new(OperationProcessor::new(node));

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and event handler
    let log_rx = fold_db::logging::subscribe();
    let mut app = App::new(processor, user_hash, log_rx);
    let event_handler = EventHandler::new(250);

    // Trigger initial data load
    app.load_dashboard_data();

    // Main loop
    let result = run_app(&mut terminal, &mut app, &event_handler).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    event_handler: &EventHandler,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        match event_handler.next()? {
            Event::Key(key_event) => app.handle_key(key_event),
            Event::Tick => app.on_tick(),
        }

        // Process any completed async results
        app.process_async_results();

        if app.should_quit {
            return Ok(());
        }
    }
}
