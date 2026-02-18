use crate::app::App;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::horizontal([
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(50),
    ])
    .split(area);

    render_schema_list(frame, app, chunks[0]);
    render_keys_panel(frame, app, chunks[1]);
    render_record_panel(frame, app, chunks[2]);
}

fn panel_border_style(app: &App, panel: usize) -> Style {
    if app.schemas_state.focus == panel {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

fn panel_title_style(app: &App, panel: usize) -> Style {
    if app.schemas_state.focus == panel {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    }
}

fn render_schema_list(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.schemas_state.focus == 0;
    let title = if focused {
        format!(" ▶ Schemas ({}) ", app.schemas_state.schemas.len())
    } else {
        format!("   Schemas ({}) ", app.schemas_state.schemas.len())
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(panel_border_style(app, 0))
        .title(title)
        .title_style(panel_title_style(app, 0));

    if app.schemas_state.loading {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Loading...",
                Style::default().fg(Color::Yellow),
            )),
        ];
        frame.render_widget(Paragraph::new(lines).block(block), area);
        return;
    }

    if app.schemas_state.schemas.is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No schemas.",
                Style::default().fg(Color::Gray),
            )),
            Line::from(Span::styled(
                "  r=refresh",
                Style::default().fg(Color::Gray),
            )),
        ];
        frame.render_widget(Paragraph::new(lines).block(block), area);
        return;
    }

    let inner_height = area.height.saturating_sub(2) as usize;
    let scroll = app.schemas_state.selected.saturating_sub(inner_height.saturating_sub(1));

    let lines: Vec<Line> = app
        .schemas_state
        .schemas
        .iter()
        .enumerate()
        .skip(scroll)
        .take(inner_height)
        .map(|(i, sws)| {
            let state_indicator = match sws.state {
                fold_db::schema::schema_types::SchemaState::Approved => {
                    Span::styled("[A]", Style::default().fg(Color::Green))
                }
                fold_db::schema::schema_types::SchemaState::Blocked => {
                    Span::styled("[B]", Style::default().fg(Color::Red))
                }
                fold_db::schema::schema_types::SchemaState::Available => {
                    Span::styled("[?]", Style::default().fg(Color::Yellow))
                }
            };

            let is_selected = i == app.schemas_state.selected;
            let name = sws
                .schema
                .descriptive_name
                .as_deref()
                .unwrap_or_else(|| sws.name());

            let name_style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let marker = if is_selected { ">" } else { " " };

            Line::from(vec![
                Span::raw(marker),
                state_indicator,
                Span::raw(" "),
                Span::styled(name.to_string(), name_style),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_keys_panel(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.schemas_state.focus == 1;
    let page_start = if app.schemas_state.keys.is_empty() {
        0
    } else {
        app.schemas_state.keys_offset + 1
    };
    let page_end = app.schemas_state.keys_offset + app.schemas_state.keys.len();
    let marker = if focused { " ▶" } else { "  " };
    let title = format!(
        "{} Keys {}-{}/{} ",
        marker, page_start, page_end, app.schemas_state.keys_total
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(panel_border_style(app, 1))
        .title(title)
        .title_style(panel_title_style(app, 1));

    if app.schemas_state.keys_loading {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Loading...",
                Style::default().fg(Color::Yellow),
            )),
        ];
        frame.render_widget(Paragraph::new(lines).block(block), area);
        return;
    }

    if app.schemas_state.keys.is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No keys.",
                Style::default().fg(Color::Gray),
            )),
        ];
        frame.render_widget(Paragraph::new(lines).block(block), area);
        return;
    }

    let inner_height = area.height.saturating_sub(2) as usize;
    let scroll = app
        .schemas_state
        .selected_key
        .saturating_sub(inner_height.saturating_sub(1));

    let lines: Vec<Line> = app
        .schemas_state
        .keys
        .iter()
        .enumerate()
        .skip(scroll)
        .take(inner_height)
        .map(|(i, kv)| {
            let key_str = match (&kv.hash, &kv.range) {
                (Some(h), Some(r)) => format!("{} | {}", h, r),
                (Some(h), None) => h.clone(),
                (None, Some(r)) => r.clone(),
                (None, None) => "empty".to_string(),
            };

            let is_selected = i == app.schemas_state.selected_key;
            let marker = if is_selected { ">" } else { " " };
            let style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Line::from(vec![
                Span::raw(marker),
                Span::styled(format!(" {}", key_str), style),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_record_panel(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.schemas_state.focus == 2;
    let title = if focused {
        " ▶ Record "
    } else {
        "   Record "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(panel_border_style(app, 2))
        .title(title)
        .title_style(panel_title_style(app, 2));

    if app.schemas_state.record_loading {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Loading...",
                Style::default().fg(Color::Yellow),
            )),
        ];
        frame.render_widget(Paragraph::new(lines).block(block), area);
        return;
    }

    let record = match &app.schemas_state.record {
        Some(r) => r,
        None => {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Select a key to view fields.",
                    Style::default().fg(Color::Gray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Left/Right  switch panels",
                    Style::default().fg(Color::Gray),
                )),
                Line::from(Span::styled(
                    "  Up/Down     navigate list",
                    Style::default().fg(Color::Gray),
                )),
                Line::from(Span::styled(
                    "  Enter       load record",
                    Style::default().fg(Color::Gray),
                )),
                Line::from(Span::styled(
                    "  n/p         page keys",
                    Style::default().fg(Color::Gray),
                )),
            ];
            frame.render_widget(Paragraph::new(lines).block(block), area);
            return;
        }
    };

    let mut lines = Vec::new();

    // Extract fields from the record JSON
    if let Some(fields) = record.get("fields").and_then(|f| f.as_object()) {
        for (field_name, field_data) in fields {
            // field_data is typically {"value": ..., "atom_uuid": ...}
            let value = field_data
                .get("value")
                .unwrap_or(field_data);

            lines.push(Line::from(Span::styled(
                format!("  {}:", field_name),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));

            let value_str = format_value(value);
            for line in value_str.lines() {
                lines.push(Line::from(Span::raw(format!("    {}", line))));
            }
            lines.push(Line::from(""));
        }
    } else {
        // Fallback: pretty-print the whole record
        let pretty = serde_json::to_string_pretty(record).unwrap_or_default();
        for line in pretty.lines() {
            lines.push(Line::from(Span::raw(format!("  {}", line))));
        }
    }

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn format_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        other => serde_json::to_string_pretty(other).unwrap_or_default(),
    }
}
