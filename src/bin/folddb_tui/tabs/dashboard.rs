use crate::app::App;
use fold_db::{fold_db_core::orchestration::IndexingState, schema::schema_types::SchemaState, DatabaseConfig};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let chunks =
        Layout::vertical([Constraint::Length(8), Constraint::Length(8), Constraint::Min(0)])
            .split(area);

    render_node_info(frame, app, chunks[0]);
    render_indexing_status(frame, app, chunks[1]);
    render_schemas_summary(frame, app, chunks[2]);
}

fn render_node_info(frame: &mut Frame, app: &App, area: Rect) {
    let db_type = match &app.dashboard.db_config {
        Some(DatabaseConfig::Local { path }) => format!("Local ({})", path.display()),
        #[cfg(feature = "aws-backend")]
        Some(DatabaseConfig::Cloud(_)) => "Cloud (DynamoDB + S3)".to_string(),
        Some(DatabaseConfig::Exemem { api_url, .. }) => format!("Exemem ({})", api_url),
        None => "Unknown".to_string(),
    };

    let pub_key = app.dashboard.public_key.as_deref().unwrap_or("...");
    let pub_key_display = if pub_key.len() > 24 {
        format!("{}...{}", &pub_key[..12], &pub_key[pub_key.len() - 12..])
    } else {
        pub_key.to_string()
    };

    let schema_count = app.dashboard.schemas.len();

    let lines = vec![
        Line::from(vec![
            Span::styled("  Storage:    ", Style::default().fg(Color::Yellow)),
            Span::raw(&db_type),
        ]),
        Line::from(vec![
            Span::styled("  Public Key: ", Style::default().fg(Color::Yellow)),
            Span::raw(&pub_key_display),
        ]),
        Line::from(vec![
            Span::styled("  User Hash:  ", Style::default().fg(Color::Yellow)),
            Span::raw(&app.user_hash),
        ]),
        Line::from(vec![
            Span::styled("  Schemas:    ", Style::default().fg(Color::Yellow)),
            Span::raw(schema_count.to_string()),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Node Info ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_indexing_status(frame: &mut Frame, app: &App, area: Rect) {
    let lines = if let Some(status) = &app.dashboard.indexing_status {
        let state_span = match status.state {
            IndexingState::Idle => Span::styled("Idle", Style::default().fg(Color::Green)),
            IndexingState::Indexing => Span::styled("Indexing", Style::default().fg(Color::Yellow)),
        };

        vec![
            Line::from(vec![
                Span::styled("  State:       ", Style::default().fg(Color::Yellow)),
                state_span,
            ]),
            Line::from(vec![
                Span::styled("  In Progress: ", Style::default().fg(Color::Yellow)),
                Span::raw(status.operations_in_progress.to_string()),
                Span::raw("    "),
                Span::styled("Queued: ", Style::default().fg(Color::Yellow)),
                Span::raw(status.operations_queued.to_string()),
            ]),
            Line::from(vec![
                Span::styled("  Processed:   ", Style::default().fg(Color::Yellow)),
                Span::raw(status.total_operations_processed.to_string()),
                Span::raw("    "),
                Span::styled("Throughput: ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{:.1} ops/s", status.operations_per_second)),
            ]),
        ]
    } else if app.dashboard.loading {
        vec![Line::from("  Loading...")]
    } else {
        vec![Line::from("  Press 'r' to refresh")]
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Indexing ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_schemas_summary(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .dashboard
        .schemas
        .iter()
        .map(|schema| {
            let state_indicator = match schema.state {
                SchemaState::Approved => Span::styled(" [A] ", Style::default().fg(Color::Green)),
                SchemaState::Available => Span::styled(" [?] ", Style::default().fg(Color::Yellow)),
                SchemaState::Blocked => Span::styled(" [B] ", Style::default().fg(Color::Red)),
            };

            let field_count = schema.schema.fields.as_ref().map(|f| f.len()).unwrap_or(0);

            ListItem::new(Line::from(vec![
                state_indicator,
                Span::raw(schema.name()),
                Span::styled(
                    format!("  ({} fields)", field_count),
                    Style::default().fg(Color::Gray),
                ),
            ]))
        })
        .collect();

    let title = format!(" Schemas ({}) ", items.len());
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    frame.render_widget(List::new(items).block(block), area);
}
