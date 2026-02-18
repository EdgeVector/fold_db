use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use fold_db::{
    db_operations::IndexResult,
    fold_db_core::orchestration::IndexingStatus,
    fold_node::{llm_query::types::ToolCallRecord, OperationProcessor},
    ingestion::{
        smart_folder::SmartFolderScanResponse, IngestionProgress, IngestionResponse,
        ProgressTracker,
    },
    progress::InMemoryProgressStore,
    schema::{
        schema_types::SchemaWithState,
        types::{key_value::KeyValue, field::HashRangeFilter, Query},
    },
    DatabaseConfig,
};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

// --- Async result types ---

pub enum AsyncResult {
    Schemas(Result<Vec<SchemaWithState>, String>),
    IndexingStatus(Result<IndexingStatus, String>),
    FolderScan(Result<SmartFolderScanResponse, String>),
    FileIngestion(Result<IngestionResponse, String>),
    AiQuery(Result<(String, Vec<ToolCallRecord>), String>),
    SearchResults(Result<Vec<IndexResult>, String>),
    Progress(Option<IngestionProgress>),
    SchemaList(Result<Vec<SchemaWithState>, String>),
    SchemaKeys(Result<(Vec<KeyValue>, usize), String>),
    RecordValues(Result<Vec<serde_json::Value>, String>),
}

// --- Tab enum ---

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    FolderSync,
    Ingestion,
    AiQuery,
    Search,
    Schemas,
}

impl Tab {
    pub const ALL: [Tab; 6] = [
        Tab::Dashboard,
        Tab::FolderSync,
        Tab::Ingestion,
        Tab::AiQuery,
        Tab::Search,
        Tab::Schemas,
    ];

    pub fn title(&self) -> &str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::FolderSync => "Folder Sync",
            Tab::Ingestion => "Ingestion",
            Tab::AiQuery => "AI Query",
            Tab::Search => "Search",
            Tab::Schemas => "Schemas",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Tab::Dashboard => 0,
            Tab::FolderSync => 1,
            Tab::Ingestion => 2,
            Tab::AiQuery => 3,
            Tab::Search => 4,
            Tab::Schemas => 5,
        }
    }
}

// --- Per-tab state structs ---

pub struct DashboardState {
    pub schemas: Vec<SchemaWithState>,
    pub indexing_status: Option<IndexingStatus>,
    pub db_config: Option<DatabaseConfig>,
    pub public_key: Option<String>,
    pub loading: bool,
}

#[derive(PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
}

pub struct PathCompletionState {
    pub candidates: Vec<String>,
    pub selected: usize,
}

pub struct FolderSyncState {
    pub path_input: String,
    pub cursor_pos: usize,
    pub input_mode: InputMode,
    pub completion: Option<PathCompletionState>,
    pub scan_result: Option<SmartFolderScanResponse>,
    pub selected_file: usize,
    pub ingesting_index: Option<usize>,
    pub ingestion_results: Vec<(String, Result<IngestionResponse, String>)>,
    pub loading: bool,
    pub status_message: Option<String>,
    pub progress: Option<IngestionProgress>,
}

pub struct IngestionState {
    pub path_input: String,
    pub cursor_pos: usize,
    pub input_mode: InputMode,
    pub completion: Option<PathCompletionState>,
    pub result: Option<Result<IngestionResponse, String>>,
    pub loading: bool,
    pub progress: Option<IngestionProgress>,
}

pub enum ChatRole {
    User,
    Assistant,
    ToolUse,
}

pub struct ChatMessage {
    pub role: ChatRole,
    pub text: String,
    pub tool_calls: Vec<ToolCallRecord>,
}

pub struct AiQueryState {
    pub query_input: String,
    pub cursor_pos: usize,
    pub input_mode: InputMode,
    pub messages: Vec<ChatMessage>,
    pub scroll: usize,
    pub loading: bool,
}

pub struct SearchState {
    pub input: String,
    pub cursor_pos: usize,
    pub input_mode: InputMode,
    pub results: Vec<IndexResult>,
    pub selected: usize,
    pub loading: bool,
}

pub struct SchemasState {
    pub schemas: Vec<SchemaWithState>,
    pub selected: usize,
    pub keys: Vec<KeyValue>,
    pub keys_total: usize,
    pub keys_offset: usize,
    pub keys_loading: bool,
    pub selected_key: usize,
    pub record: Option<serde_json::Value>,
    pub record_loading: bool,
    pub loading: bool,
    /// 0 = schema list, 1 = keys list, 2 = record view
    pub focus: usize,
}

pub struct LogState {
    pub visible: bool,
    pub lines: VecDeque<String>,
    pub scroll: usize,
    pub max_lines: usize,
    log_rx: Option<broadcast::Receiver<String>>,
}

// --- Main App ---

pub struct App {
    pub processor: Arc<OperationProcessor>,
    pub user_hash: String,
    pub current_tab: Tab,
    pub should_quit: bool,

    pub dashboard: DashboardState,
    pub folder_sync: FolderSyncState,
    pub ingestion: IngestionState,
    pub ai_query: AiQueryState,
    pub search: SearchState,
    pub schemas_state: SchemasState,
    pub log_state: LogState,

    progress_tracker: ProgressTracker,
    result_tx: mpsc::UnboundedSender<AsyncResult>,
    result_rx: mpsc::UnboundedReceiver<AsyncResult>,
}

impl App {
    pub fn new(
        processor: Arc<OperationProcessor>,
        user_hash: String,
        log_rx: Option<broadcast::Receiver<String>>,
    ) -> Self {
        let (result_tx, result_rx) = mpsc::unbounded_channel();

        let db_config = Some(processor.get_database_config());
        let public_key = Some(processor.get_node_public_key());
        let progress_tracker: ProgressTracker = Arc::new(InMemoryProgressStore::new());

        Self {
            processor,
            user_hash,
            current_tab: Tab::Dashboard,
            should_quit: false,
            dashboard: DashboardState {
                schemas: vec![],
                indexing_status: None,
                db_config,
                public_key,
                loading: false,
            },
            folder_sync: FolderSyncState {
                path_input: String::new(),
                cursor_pos: 0,
                input_mode: InputMode::Normal,
                completion: None,
                scan_result: None,
                selected_file: 0,
                ingesting_index: None,
                ingestion_results: vec![],
                loading: false,
                status_message: None,
                progress: None,
            },
            ingestion: IngestionState {
                path_input: String::new(),
                cursor_pos: 0,
                input_mode: InputMode::Normal,
                completion: None,
                result: None,
                loading: false,
                progress: None,
            },
            ai_query: AiQueryState {
                query_input: String::new(),
                cursor_pos: 0,
                input_mode: InputMode::Normal,
                messages: vec![],
                scroll: 0,
                loading: false,
            },
            search: SearchState {
                input: String::new(),
                cursor_pos: 0,
                input_mode: InputMode::Normal,
                results: vec![],
                selected: 0,
                loading: false,
            },
            schemas_state: SchemasState {
                schemas: vec![],
                selected: 0,
                keys: vec![],
                keys_total: 0,
                keys_offset: 0,
                keys_loading: false,
                selected_key: 0,
                record: None,
                record_loading: false,
                loading: false,
                focus: 0,
            },
            log_state: LogState {
                visible: false,
                lines: VecDeque::new(),
                scroll: 0,
                max_lines: 500,
                log_rx,
            },
            progress_tracker,
            result_tx,
            result_rx,
        }
    }

    // --- Async data loading ---

    pub fn load_dashboard_data(&mut self) {
        self.dashboard.loading = true;
        let proc = Arc::clone(&self.processor);
        let tx = self.result_tx.clone();
        tokio::spawn(async move {
            let schemas = proc.list_schemas().await;
            let _ = tx.send(AsyncResult::Schemas(
                schemas.map_err(|e| e.to_string()),
            ));
        });

        let proc = Arc::clone(&self.processor);
        let tx = self.result_tx.clone();
        tokio::spawn(async move {
            let status = proc.get_indexing_status().await;
            let _ = tx.send(AsyncResult::IndexingStatus(
                status.map_err(|e| e.to_string()),
            ));
        });
    }

    pub fn start_folder_scan(&mut self) {
        let path_str = self.folder_sync.path_input.trim().to_string();
        if path_str.is_empty() {
            return;
        }
        let path = PathBuf::from(&path_str);
        if !path.is_dir() {
            self.folder_sync.status_message = Some(format!("Not a directory: {}", path_str));
            return;
        }
        self.folder_sync.loading = true;
        self.folder_sync.scan_result = None;
        self.folder_sync.ingestion_results.clear();
        self.folder_sync.status_message = Some("Scanning folder with AI...".to_string());
        let proc = Arc::clone(&self.processor);
        let tx = self.result_tx.clone();
        tokio::spawn(async move {
            let result = proc.smart_folder_scan(&path, 3, 200).await;
            let _ = tx.send(AsyncResult::FolderScan(
                result.map_err(|e| e.to_string()),
            ));
        });
    }

    pub fn ingest_selected_scan_file(&mut self) {
        let scan = match &self.folder_sync.scan_result {
            Some(s) => s,
            None => return,
        };
        if self.folder_sync.selected_file >= scan.recommended_files.len() {
            return;
        }
        let file = &scan.recommended_files[self.folder_sync.selected_file];
        let file_path = PathBuf::from(&file.path);
        self.folder_sync.ingesting_index = Some(self.folder_sync.selected_file);
        self.folder_sync.status_message = Some(format!("Ingesting: {}", file.path));
        self.folder_sync.progress = None;

        let proc = Arc::clone(&self.processor);
        let tx = self.result_tx.clone();
        let path_str = file.path.clone();
        let tracker = self.progress_tracker.clone();
        tokio::spawn(async move {
            let result = proc
                .ingest_single_file_with_tracker(&file_path, true, Some(tracker))
                .await;
            let _ = tx.send(AsyncResult::FileIngestion(
                result.map_err(|e| format!("{}: {}", path_str, e)),
            ));
        });
    }

    pub fn ingest_all_recommended(&mut self) {
        let scan = match &self.folder_sync.scan_result {
            Some(s) => s,
            None => return,
        };
        if scan.recommended_files.is_empty() {
            return;
        }
        self.folder_sync.loading = true;
        self.folder_sync.progress = None;
        self.folder_sync.status_message =
            Some(format!("Ingesting {} files...", scan.recommended_files.len()));

        for file in &scan.recommended_files {
            let file_path = PathBuf::from(&file.path);
            let proc = Arc::clone(&self.processor);
            let tx = self.result_tx.clone();
            let path_str = file.path.clone();
            let tracker = self.progress_tracker.clone();
            tokio::spawn(async move {
                let result = proc
                    .ingest_single_file_with_tracker(&file_path, true, Some(tracker))
                    .await;
                let _ = tx.send(AsyncResult::FileIngestion(
                    result.map_err(|e| format!("{}: {}", path_str, e)),
                ));
            });
        }
    }

    pub fn start_file_ingestion(&mut self) {
        let path_str = self.ingestion.path_input.trim().to_string();
        if path_str.is_empty() {
            return;
        }
        let path = PathBuf::from(&path_str);
        if !path.is_file() {
            self.ingestion.result = Some(Err(format!("Not a file: {}", path_str)));
            return;
        }
        self.ingestion.loading = true;
        self.ingestion.result = None;
        self.ingestion.progress = None;
        let proc = Arc::clone(&self.processor);
        let tx = self.result_tx.clone();
        let tracker = self.progress_tracker.clone();
        tokio::spawn(async move {
            let result = proc
                .ingest_single_file_with_tracker(&path, true, Some(tracker))
                .await;
            let _ = tx.send(AsyncResult::FileIngestion(
                result.map_err(|e| e.to_string()),
            ));
        });
    }

    pub fn start_ai_query(&mut self) {
        let query = self.ai_query.query_input.trim().to_string();
        if query.is_empty() {
            return;
        }
        // Add user message to conversation
        self.ai_query.messages.push(ChatMessage {
            role: ChatRole::User,
            text: query.clone(),
            tool_calls: vec![],
        });
        self.ai_query.query_input.clear();
        self.ai_query.cursor_pos = 0;
        self.ai_query.loading = true;
        // Auto-scroll to bottom
        self.ai_query.scroll = usize::MAX;

        let proc = Arc::clone(&self.processor);
        let tx = self.result_tx.clone();
        let user_hash = self.user_hash.clone();
        // llm_query internally calls execute_query_json which returns a !Send future,
        // so we run it on a dedicated thread with its own runtime.
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(proc.llm_query(&query, &user_hash, 10));
            let _ = tx.send(AsyncResult::AiQuery(
                result.map_err(|e| e.to_string()),
            ));
        });
    }

    pub fn load_schemas_list(&mut self) {
        self.schemas_state.loading = true;
        let proc = Arc::clone(&self.processor);
        let tx = self.result_tx.clone();
        tokio::spawn(async move {
            let result = proc.list_schemas().await;
            let _ = tx.send(AsyncResult::SchemaList(
                result.map_err(|e| e.to_string()),
            ));
        });
    }

    pub fn load_schema_keys(&mut self) {
        if self.schemas_state.schemas.is_empty() {
            return;
        }
        let name = self.schemas_state.schemas[self.schemas_state.selected]
            .name()
            .to_string();
        self.schemas_state.keys_loading = true;
        let proc = Arc::clone(&self.processor);
        let tx = self.result_tx.clone();
        let offset = self.schemas_state.keys_offset;
        tokio::spawn(async move {
            let result = proc.list_schema_keys(&name, offset, 50).await;
            let _ = tx.send(AsyncResult::SchemaKeys(
                result.map_err(|e| e.to_string()),
            ));
        });
    }

    pub fn load_record_values(&mut self) {
        if self.schemas_state.keys.is_empty() || self.schemas_state.schemas.is_empty() {
            return;
        }
        let sws = &self.schemas_state.schemas[self.schemas_state.selected];
        let schema_name = sws.name().to_string();
        let fields: Vec<String> = sws
            .schema
            .fields
            .clone()
            .unwrap_or_default();
        if fields.is_empty() {
            return;
        }
        let kv = &self.schemas_state.keys[self.schemas_state.selected_key];
        let filter = match (&kv.hash, &kv.range) {
            (Some(h), Some(r)) => Some(HashRangeFilter::HashRangeKey {
                hash: h.clone(),
                range: r.clone(),
            }),
            (Some(h), None) => Some(HashRangeFilter::HashKey(h.clone())),
            (None, Some(r)) => Some(HashRangeFilter::RangeKey(r.clone())),
            (None, None) => None,
        };
        self.schemas_state.record_loading = true;
        self.schemas_state.record = None;
        let proc = Arc::clone(&self.processor);
        let tx = self.result_tx.clone();
        // execute_query_json returns a !Send future, so use a dedicated thread
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let query = Query::new_with_filter(schema_name, fields, filter);
            let result = rt.block_on(proc.execute_query_json(query));
            let _ = tx.send(AsyncResult::RecordValues(
                result.map_err(|e| e.to_string()),
            ));
        });
    }

    pub fn execute_search(&mut self) {
        if self.search.input.trim().is_empty() {
            return;
        }
        self.search.loading = true;
        let proc = Arc::clone(&self.processor);
        let tx = self.result_tx.clone();
        let term = self.search.input.clone();
        tokio::spawn(async move {
            let results = proc.native_index_search(&term).await;
            let _ = tx.send(AsyncResult::SearchResults(
                results.map_err(|e| e.to_string()),
            ));
        });
    }

    // --- Process async results ---

    pub fn process_async_results(&mut self) {
        while let Ok(result) = self.result_rx.try_recv() {
            match result {
                AsyncResult::Schemas(Ok(schemas)) => {
                    self.dashboard.schemas = schemas;
                    self.dashboard.loading = false;
                }
                AsyncResult::Schemas(Err(_)) => {
                    self.dashboard.loading = false;
                }
                AsyncResult::IndexingStatus(Ok(status)) => {
                    self.dashboard.indexing_status = Some(status);
                    self.dashboard.loading = false;
                }
                AsyncResult::IndexingStatus(Err(_)) => {
                    self.dashboard.loading = false;
                }
                AsyncResult::FolderScan(Ok(scan)) => {
                    let count = scan.recommended_files.len();
                    self.folder_sync.scan_result = Some(scan);
                    self.folder_sync.selected_file = 0;
                    self.folder_sync.loading = false;
                    self.folder_sync.status_message =
                        Some(format!("Found {} files to ingest. Press 'a' to ingest all or Enter for selected.", count));
                }
                AsyncResult::FolderScan(Err(e)) => {
                    self.folder_sync.loading = false;
                    self.folder_sync.status_message = Some(format!("Scan failed: {}", e));
                }
                AsyncResult::FileIngestion(Ok(resp)) => {
                    let name = resp.schema_used.clone().unwrap_or_default();
                    let summary = format!(
                        "{}: {} mutations executed",
                        name, resp.mutations_executed
                    );
                    // Route to the correct tab
                    if self.current_tab == Tab::FolderSync {
                        self.folder_sync.ingestion_results.push((summary, Ok(resp)));
                        self.folder_sync.ingesting_index = None;
                        let done = self.folder_sync.ingestion_results.len();
                        let total = self.folder_sync.scan_result.as_ref()
                            .map(|s| s.recommended_files.len())
                            .unwrap_or(0);
                        if done >= total && self.folder_sync.loading {
                            self.folder_sync.loading = false;
                        }
                        self.folder_sync.status_message =
                            Some(format!("Ingested {}/{} files", done, total));
                    } else {
                        self.ingestion.result = Some(Ok(resp));
                        self.ingestion.loading = false;
                    }
                }
                AsyncResult::FileIngestion(Err(e)) => {
                    if self.current_tab == Tab::FolderSync {
                        self.folder_sync.ingestion_results.push((e.clone(), Err(e)));
                        self.folder_sync.ingesting_index = None;
                    } else {
                        self.ingestion.result = Some(Err(e));
                        self.ingestion.loading = false;
                    }
                }
                AsyncResult::AiQuery(Ok((answer, tool_calls))) => {
                    if !tool_calls.is_empty() {
                        self.ai_query.messages.push(ChatMessage {
                            role: ChatRole::ToolUse,
                            text: format!("{} tool calls", tool_calls.len()),
                            tool_calls: tool_calls.clone(),
                        });
                    }
                    self.ai_query.messages.push(ChatMessage {
                        role: ChatRole::Assistant,
                        text: answer,
                        tool_calls: vec![],
                    });
                    self.ai_query.loading = false;
                    self.ai_query.scroll = usize::MAX;
                }
                AsyncResult::AiQuery(Err(e)) => {
                    self.ai_query.messages.push(ChatMessage {
                        role: ChatRole::Assistant,
                        text: format!("Error: {}", e),
                        tool_calls: vec![],
                    });
                    self.ai_query.loading = false;
                    self.ai_query.scroll = usize::MAX;
                }
                AsyncResult::SearchResults(Ok(results)) => {
                    self.search.results = results;
                    self.search.selected = 0;
                    self.search.loading = false;
                }
                AsyncResult::SearchResults(Err(_)) => {
                    self.search.loading = false;
                }
                AsyncResult::SchemaList(Ok(schemas)) => {
                    self.schemas_state.schemas = schemas;
                    self.schemas_state.loading = false;
                    self.schemas_state.selected = 0;
                    self.schemas_state.keys.clear();
                    self.schemas_state.keys_total = 0;
                    self.schemas_state.keys_offset = 0;
                    self.schemas_state.selected_key = 0;
                    self.schemas_state.record = None;
                    // Auto-load keys for first schema
                    if !self.schemas_state.schemas.is_empty() {
                        self.load_schema_keys();
                    }
                }
                AsyncResult::SchemaList(Err(_)) => {
                    self.schemas_state.loading = false;
                }
                AsyncResult::SchemaKeys(Ok((keys, total))) => {
                    self.schemas_state.keys = keys;
                    self.schemas_state.keys_total = total;
                    self.schemas_state.keys_loading = false;
                }
                AsyncResult::SchemaKeys(Err(_)) => {
                    self.schemas_state.keys_loading = false;
                }
                AsyncResult::RecordValues(Ok(values)) => {
                    self.schemas_state.record = values.into_iter().next();
                    self.schemas_state.record_loading = false;
                }
                AsyncResult::RecordValues(Err(_)) => {
                    self.schemas_state.record_loading = false;
                }
                AsyncResult::Progress(progress) => {
                    if self.current_tab == Tab::Ingestion && self.ingestion.loading {
                        self.ingestion.progress = progress;
                    } else if self.current_tab == Tab::FolderSync {
                        self.folder_sync.progress = progress;
                    }
                }
            }
        }
    }

    // --- Key handling ---

    pub fn handle_key(&mut self, key: KeyEvent) {
        // Route to input handler if in editing mode
        if self.is_editing() {
            self.handle_editing_input(key);
            return;
        }

        // Global keys
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.should_quit = true;
                return;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                return;
            }
            KeyCode::Tab => {
                let idx = self.current_tab.index();
                let next = (idx + 1) % Tab::ALL.len();
                self.switch_tab(Tab::ALL[next]);
                return;
            }
            KeyCode::BackTab => {
                let idx = self.current_tab.index();
                let prev = if idx == 0 { Tab::ALL.len() - 1 } else { idx - 1 };
                self.switch_tab(Tab::ALL[prev]);
                return;
            }
            KeyCode::Char('1') => { self.switch_tab(Tab::Dashboard); return; }
            KeyCode::Char('2') => { self.switch_tab(Tab::FolderSync); return; }
            KeyCode::Char('3') => { self.switch_tab(Tab::Ingestion); return; }
            KeyCode::Char('4') => { self.switch_tab(Tab::AiQuery); return; }
            KeyCode::Char('5') => { self.switch_tab(Tab::Search); return; }
            KeyCode::Char('6') => { self.switch_tab(Tab::Schemas); return; }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                self.log_state.visible = !self.log_state.visible;
                return;
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.refresh_current_tab();
                return;
            }
            _ => {}
        }

        // Per-tab keys
        match self.current_tab {
            Tab::Dashboard => {}
            Tab::FolderSync => self.handle_folder_sync_key(key),
            Tab::Ingestion => self.handle_ingestion_key(key),
            Tab::AiQuery => self.handle_ai_query_key(key),
            Tab::Search => self.handle_search_key(key),
            Tab::Schemas => self.handle_schemas_key(key),
        }
    }

    fn is_editing(&self) -> bool {
        match self.current_tab {
            Tab::FolderSync => self.folder_sync.input_mode == InputMode::Editing,
            Tab::Ingestion => self.ingestion.input_mode == InputMode::Editing,
            Tab::AiQuery => self.ai_query.input_mode == InputMode::Editing,
            Tab::Search => self.search.input_mode == InputMode::Editing,
            _ => false,
        }
    }

    fn handle_editing_input(&mut self, key: KeyEvent) {
        match self.current_tab {
            Tab::FolderSync => self.handle_folder_sync_input(key),
            Tab::Ingestion => self.handle_ingestion_input(key),
            Tab::AiQuery => self.handle_ai_query_input(key),
            Tab::Search => self.handle_search_input(key),
            _ => {}
        }
    }

    fn switch_tab(&mut self, tab: Tab) {
        let prev = self.current_tab;
        self.current_tab = tab;
        if prev == tab {
            return;
        }
        match tab {
            Tab::Dashboard => self.load_dashboard_data(),
            Tab::Schemas => {
                if self.schemas_state.schemas.is_empty() {
                    self.load_schemas_list();
                }
            }
            _ => {}
        }
    }

    fn refresh_current_tab(&mut self) {
        match self.current_tab {
            Tab::Dashboard => self.load_dashboard_data(),
            Tab::FolderSync => self.start_folder_scan(),
            Tab::Search => self.execute_search(),
            Tab::Schemas => self.load_schemas_list(),
            _ => {}
        }
    }

    // --- Folder Sync keys ---

    fn handle_folder_sync_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter | KeyCode::Char('i') | KeyCode::Char('/') => {
                if self.folder_sync.scan_result.is_none() {
                    self.folder_sync.input_mode = InputMode::Editing;
                } else {
                    self.ingest_selected_scan_file();
                }
            }
            KeyCode::Char('a') => self.ingest_all_recommended(),
            KeyCode::Char('s') => {
                self.folder_sync.scan_result = None;
                self.folder_sync.ingestion_results.clear();
                self.folder_sync.input_mode = InputMode::Editing;
            }
            KeyCode::Up => {
                if self.folder_sync.selected_file > 0 {
                    self.folder_sync.selected_file -= 1;
                }
            }
            KeyCode::Down => {
                let max = self.folder_sync.scan_result.as_ref()
                    .map(|s| s.recommended_files.len().saturating_sub(1))
                    .unwrap_or(0);
                if self.folder_sync.selected_file < max {
                    self.folder_sync.selected_file += 1;
                }
            }
            _ => {}
        }
    }

    fn handle_folder_sync_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.folder_sync.input_mode = InputMode::Normal;
                self.folder_sync.completion = None;
            }
            KeyCode::Enter => {
                self.folder_sync.input_mode = InputMode::Normal;
                self.folder_sync.completion = None;
                self.start_folder_scan();
            }
            KeyCode::Tab => {
                complete_path(
                    &mut self.folder_sync.path_input,
                    &mut self.folder_sync.cursor_pos,
                    &mut self.folder_sync.completion,
                    true,
                );
            }
            KeyCode::BackTab => {
                if let Some(c) = &mut self.folder_sync.completion {
                    if !c.candidates.is_empty() {
                        c.selected = if c.selected == 0 { c.candidates.len() - 1 } else { c.selected - 1 };
                        apply_completion(&mut self.folder_sync.path_input, &mut self.folder_sync.cursor_pos, c);
                    }
                }
            }
            _ => {
                self.folder_sync.completion = None;
                edit_text_input(
                    &mut self.folder_sync.path_input,
                    &mut self.folder_sync.cursor_pos,
                    key.code,
                );
            }
        }
    }

    // --- Ingestion keys ---

    fn handle_ingestion_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter | KeyCode::Char('i') | KeyCode::Char('/') => {
                self.ingestion.input_mode = InputMode::Editing;
            }
            _ => {}
        }
    }

    fn handle_ingestion_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.ingestion.input_mode = InputMode::Normal;
                self.ingestion.completion = None;
            }
            KeyCode::Enter => {
                self.ingestion.input_mode = InputMode::Normal;
                self.ingestion.completion = None;
                self.start_file_ingestion();
            }
            KeyCode::Tab => {
                complete_path(
                    &mut self.ingestion.path_input,
                    &mut self.ingestion.cursor_pos,
                    &mut self.ingestion.completion,
                    false,
                );
            }
            KeyCode::BackTab => {
                if let Some(c) = &mut self.ingestion.completion {
                    if !c.candidates.is_empty() {
                        c.selected = if c.selected == 0 { c.candidates.len() - 1 } else { c.selected - 1 };
                        apply_completion(&mut self.ingestion.path_input, &mut self.ingestion.cursor_pos, c);
                    }
                }
            }
            _ => {
                self.ingestion.completion = None;
                edit_text_input(
                    &mut self.ingestion.path_input,
                    &mut self.ingestion.cursor_pos,
                    key.code,
                );
            }
        }
    }

    // --- AI Query keys ---

    fn handle_ai_query_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter | KeyCode::Char('i') | KeyCode::Char('/') => {
                self.ai_query.input_mode = InputMode::Editing;
            }
            KeyCode::Up => {
                if self.ai_query.scroll == usize::MAX {
                    // Currently auto-scrolled to bottom; snap to a concrete value
                    // We don't know exact line count here, so set a large number and
                    // the render will clamp it. Subtract 1 to scroll up.
                    self.ai_query.scroll = 10000_usize.saturating_sub(1);
                } else if self.ai_query.scroll > 0 {
                    self.ai_query.scroll -= 1;
                }
            }
            KeyCode::Down => {
                self.ai_query.scroll = self.ai_query.scroll.saturating_add(1);
            }
            KeyCode::End => {
                self.ai_query.scroll = usize::MAX;
            }
            _ => {}
        }
    }

    fn handle_ai_query_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.ai_query.input_mode = InputMode::Normal,
            KeyCode::Enter => {
                self.ai_query.input_mode = InputMode::Normal;
                self.start_ai_query();
            }
            _ => edit_text_input(
                &mut self.ai_query.query_input,
                &mut self.ai_query.cursor_pos,
                key.code,
            ),
        }
    }

    // --- Search keys ---

    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter | KeyCode::Char('i') | KeyCode::Char('/') => {
                self.search.input_mode = InputMode::Editing;
            }
            KeyCode::Up => {
                if self.search.selected > 0 {
                    self.search.selected -= 1;
                }
            }
            KeyCode::Down => {
                if self.search.selected < self.search.results.len().saturating_sub(1) {
                    self.search.selected += 1;
                }
            }
            _ => {}
        }
    }

    fn handle_search_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.search.input_mode = InputMode::Normal,
            KeyCode::Enter => {
                self.search.input_mode = InputMode::Normal;
                self.execute_search();
            }
            _ => edit_text_input(
                &mut self.search.input,
                &mut self.search.cursor_pos,
                key.code,
            ),
        }
    }

    // --- Schemas keys ---

    fn handle_schemas_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => {
                // Cycle focus: schemas -> keys -> record
                self.schemas_state.focus = (self.schemas_state.focus + 1) % 3;
            }
            KeyCode::BackTab => {
                self.schemas_state.focus = if self.schemas_state.focus == 0 { 2 } else { self.schemas_state.focus - 1 };
            }
            KeyCode::Up => {
                match self.schemas_state.focus {
                    0 => {
                        if self.schemas_state.selected > 0 {
                            self.schemas_state.selected -= 1;
                            self.schemas_state.keys_offset = 0;
                            self.schemas_state.selected_key = 0;
                            self.schemas_state.record = None;
                            self.load_schema_keys();
                        }
                    }
                    1 => {
                        if self.schemas_state.selected_key > 0 {
                            self.schemas_state.selected_key -= 1;
                            self.load_record_values();
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Down => {
                match self.schemas_state.focus {
                    0 => {
                        let max = self.schemas_state.schemas.len().saturating_sub(1);
                        if self.schemas_state.selected < max {
                            self.schemas_state.selected += 1;
                            self.schemas_state.keys_offset = 0;
                            self.schemas_state.selected_key = 0;
                            self.schemas_state.record = None;
                            self.load_schema_keys();
                        }
                    }
                    1 => {
                        let max = self.schemas_state.keys.len().saturating_sub(1);
                        if self.schemas_state.selected_key < max {
                            self.schemas_state.selected_key += 1;
                            self.load_record_values();
                        }
                    }
                    _ => {}
                }
            }
            KeyCode::Enter => {
                // When on keys panel, load the record
                if self.schemas_state.focus == 1 && !self.schemas_state.keys.is_empty() {
                    self.load_record_values();
                    self.schemas_state.focus = 2;
                }
            }
            KeyCode::Char('n') => {
                if self.schemas_state.keys_offset + 50 < self.schemas_state.keys_total {
                    self.schemas_state.keys_offset += 50;
                    self.schemas_state.selected_key = 0;
                    self.schemas_state.record = None;
                    self.load_schema_keys();
                }
            }
            KeyCode::Char('p') => {
                if self.schemas_state.keys_offset >= 50 {
                    self.schemas_state.keys_offset -= 50;
                    self.schemas_state.selected_key = 0;
                    self.schemas_state.record = None;
                    self.load_schema_keys();
                }
            }
            _ => {}
        }
    }

    // --- Tick ---

    pub fn on_tick(&mut self) {
        // Poll progress tracker if we have active ingestion
        let should_poll = (self.ingestion.loading && self.current_tab == Tab::Ingestion)
            || ((self.folder_sync.loading || self.folder_sync.ingesting_index.is_some())
                && self.current_tab == Tab::FolderSync);

        if should_poll {
            let tracker = self.progress_tracker.clone();
            let tx = self.result_tx.clone();
            tokio::spawn(async move {
                // List all jobs and find the most recent running one
                let jobs = tracker.list_by_user("cli").await.unwrap_or_default();
                let active = jobs
                    .into_iter()
                    .filter(|j| {
                        matches!(
                            j.status,
                            fold_db::progress::JobStatus::Running
                                | fold_db::progress::JobStatus::Queued
                        )
                    })
                    .max_by_key(|j| j.updated_at);
                let progress = active.map(|j| j.into());
                let _ = tx.send(AsyncResult::Progress(progress));
            });
        }

        // Drain new log messages from the broadcast channel
        if let Some(rx) = &mut self.log_state.log_rx {
            while let Ok(msg) = rx.try_recv() {
                // Parse out the message from the JSON, or use raw
                let display = if let Ok(entry) = serde_json::from_str::<serde_json::Value>(&msg) {
                    let level = entry["level"].as_str().unwrap_or("INFO");
                    let event = entry["event_type"].as_str().unwrap_or("");
                    let message = entry["message"].as_str().unwrap_or(&msg);
                    // Shorten the event_type (e.g. "fold_node::ingestion" -> "ingestion")
                    let short_event = event.rsplit("::").next().unwrap_or(event);
                    format!("[{}] [{}] {}", level, short_event, message)
                } else {
                    msg
                };
                self.log_state.lines.push_back(display);
                if self.log_state.lines.len() > self.log_state.max_lines {
                    self.log_state.lines.pop_front();
                }
                // Auto-scroll to bottom
                let total = self.log_state.lines.len();
                self.log_state.scroll = total.saturating_sub(1);
            }
        }
    }
}

/// Complete a path with Tab. On first press, compute candidates and fill the
/// longest common prefix. On repeated presses, cycle through candidates.
/// `dirs_only` restricts completions to directories (for folder sync).
fn complete_path(
    text: &mut String,
    cursor: &mut usize,
    completion: &mut Option<PathCompletionState>,
    dirs_only: bool,
) {
    if let Some(c) = completion {
        // Already have candidates — cycle forward
        if !c.candidates.is_empty() {
            c.selected = (c.selected + 1) % c.candidates.len();
            apply_completion(text, cursor, c);
        }
        return;
    }

    // First Tab press — compute candidates
    let input = expand_tilde(text.trim());
    let (dir, prefix) = split_path_for_completion(&input);

    let entries = match std::fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };

    let mut candidates: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with(&prefix) {
            continue;
        }
        // Skip hidden files unless the user typed a dot
        if name.starts_with('.') && !prefix.starts_with('.') {
            continue;
        }
        let full = if dir.ends_with('/') {
            format!("{}{}", dir, name)
        } else {
            format!("{}/{}", dir, name)
        };
        let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        if dirs_only && !is_dir {
            continue;
        }
        // Append / for directories so the user can keep tabbing deeper
        if is_dir {
            candidates.push(format!("{}/", full));
        } else {
            candidates.push(full);
        }
    }

    candidates.sort();

    if candidates.is_empty() {
        return;
    }

    if candidates.len() == 1 {
        // Unique match — fill it in, no popup needed
        *text = candidates[0].clone();
        *cursor = text.len();
        // If it's a directory, leave completion open so next Tab goes deeper
        if text.ends_with('/') {
            *completion = None; // reset so next Tab re-scans the new dir
        }
        return;
    }

    // Multiple candidates — fill longest common prefix, then show candidates
    let lcp = longest_common_prefix(&candidates);
    *text = lcp;
    *cursor = text.len();

    *completion = Some(PathCompletionState {
        candidates,
        selected: 0,
    });
}

/// Apply the currently selected completion candidate to the input.
fn apply_completion(text: &mut String, cursor: &mut usize, c: &PathCompletionState) {
    if let Some(val) = c.candidates.get(c.selected) {
        *text = val.clone();
        *cursor = text.len();
    }
}

/// Expand `~` at the start of a path to the user's home directory.
fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix('~') {
        if let Some(home) = dirs::home_dir() {
            return format!("{}{}", home.display(), rest);
        }
    }
    path.to_string()
}

/// Split an input path into (directory_to_list, prefix_to_match).
/// e.g. "/usr/lo" -> ("/usr", "lo")
///      "/usr/"   -> ("/usr/", "")
///      ""        -> (".", "")
fn split_path_for_completion(input: &str) -> (String, String) {
    if input.is_empty() {
        return (".".to_string(), String::new());
    }
    let path = PathBuf::from(input);
    if input.ends_with('/') {
        (input.to_string(), String::new())
    } else if let Some(parent) = path.parent() {
        let dir = if parent.as_os_str().is_empty() {
            ".".to_string()
        } else {
            parent.to_string_lossy().into_owned()
        };
        let prefix = path
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        (dir, prefix)
    } else {
        (".".to_string(), input.to_string())
    }
}

/// Find the longest common prefix of a set of strings.
fn longest_common_prefix(strings: &[String]) -> String {
    if strings.is_empty() {
        return String::new();
    }
    let first = &strings[0];
    let mut len = first.len();
    for s in &strings[1..] {
        len = len.min(s.len());
        for (i, (a, b)) in first.bytes().zip(s.bytes()).enumerate() {
            if a != b {
                len = len.min(i);
                break;
            }
        }
    }
    first[..len].to_string()
}

/// Shared text editing logic for input fields.
fn edit_text_input(text: &mut String, cursor: &mut usize, key: KeyCode) {
    match key {
        KeyCode::Char(c) => {
            text.insert(*cursor, c);
            *cursor += 1;
        }
        KeyCode::Backspace => {
            if *cursor > 0 {
                *cursor -= 1;
                text.remove(*cursor);
            }
        }
        KeyCode::Left => {
            if *cursor > 0 {
                *cursor -= 1;
            }
        }
        KeyCode::Right => {
            if *cursor < text.len() {
                *cursor += 1;
            }
        }
        KeyCode::Home => *cursor = 0,
        KeyCode::End => *cursor = text.len(),
        _ => {}
    }
}
