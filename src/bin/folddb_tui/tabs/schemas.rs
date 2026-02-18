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

fn render_schema_list(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.schemas_state.focus == 0;
    let border_style = if focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = if focused {
        format!(" >> Schemas ({}) << ", app.schemas_state.schemas.len())
    } else {
        format!("    Schemas ({})    ", app.schemas_state.schemas.len())
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title)
        .title_style(if focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    if app.schemas_state.loading {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled("  Loading...", Style::default().fg(Color::Yellow))),
            ])
            .block(block),
            area,
        );
        return;
    }

    if app.schemas_state.schemas.is_empty() {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled("  No schemas.", Style::default().fg(Color::Gray))),
                Line::from(Span::styled("  r=refresh", Style::default().fg(Color::Gray))),
            ])
            .block(block),
            area,
        );
        return;
    }

    let inner_height = area.height.saturating_sub(2) as usize;
    let scroll = app
        .schemas_state
        .selected
        .saturating_sub(inner_height.saturating_sub(1));

    let lines: Vec<Line> = app
        .schemas_state
        .schemas
        .iter()
        .enumerate()
        .skip(scroll)
        .take(inner_height)
        .map(|(i, sws)| {
            let is_selected = i == app.schemas_state.selected;
            let name = sws
                .schema
                .descriptive_name
                .as_deref()
                .unwrap_or_else(|| sws.name());

            let state_char = match sws.state {
                fold_db::schema::schema_types::SchemaState::Approved => "A",
                fold_db::schema::schema_types::SchemaState::Blocked => "B",
                fold_db::schema::schema_types::SchemaState::Available => "?",
            };
            let state_color = match sws.state {
                fold_db::schema::schema_types::SchemaState::Approved => Color::Green,
                fold_db::schema::schema_types::SchemaState::Blocked => Color::Red,
                fold_db::schema::schema_types::SchemaState::Available => Color::Yellow,
            };

            if is_selected && focused {
                // Focused + selected: reverse video highlight
                Line::from(vec![
                    Span::styled(
                        format!(" [{}] {} ", state_char, name),
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else if is_selected {
                // Selected but not focused: dim highlight
                Line::from(vec![
                    Span::styled(
                        format!(" [{}] ", state_char),
                        Style::default().fg(state_color),
                    ),
                    Span::styled(name.to_string(), Style::default().fg(Color::Gray)),
                ])
            } else {
                Line::from(vec![
                    Span::styled(
                        format!(" [{}] ", state_char),
                        Style::default().fg(state_color),
                    ),
                    Span::raw(name.to_string()),
                ])
            }
        })
        .collect();

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_keys_panel(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.schemas_state.focus == 1;
    let border_style = if focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let page_start = if app.schemas_state.keys.is_empty() {
        0
    } else {
        app.schemas_state.keys_offset + 1
    };
    let page_end = app.schemas_state.keys_offset + app.schemas_state.keys.len();

    let title = if focused {
        format!(" >> Keys {}-{}/{} << ", page_start, page_end, app.schemas_state.keys_total)
    } else {
        format!("    Keys {}-{}/{}    ", page_start, page_end, app.schemas_state.keys_total)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title)
        .title_style(if focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    if app.schemas_state.keys_loading {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled("  Loading...", Style::default().fg(Color::Yellow))),
            ])
            .block(block),
            area,
        );
        return;
    }

    if app.schemas_state.keys.is_empty() {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled("  No keys.", Style::default().fg(Color::Gray))),
            ])
            .block(block),
            area,
        );
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

            if is_selected && focused {
                Line::from(Span::styled(
                    format!(" {} ", key_str),
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
            } else if is_selected {
                Line::from(Span::styled(
                    format!(" {}", key_str),
                    Style::default().fg(Color::Gray),
                ))
            } else {
                Line::from(Span::raw(format!(" {}", key_str)))
            }
        })
        .collect();

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_record_panel(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.schemas_state.focus == 2;
    let border_style = if focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = if focused {
        " >> Record << "
    } else {
        "    Record    "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title)
        .title_style(if focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    if app.schemas_state.record_loading {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled("  Loading...", Style::default().fg(Color::Yellow))),
            ])
            .block(block),
            area,
        );
        return;
    }

    let record = match &app.schemas_state.record {
        Some(r) => r,
        None => {
            let hint = match app.schemas_state.focus {
                0 => "  Press Enter to browse keys",
                1 => "  Press Enter to view record",
                _ => "  Press Esc to go back",
            };
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(hint, Style::default().fg(Color::Gray))),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Enter  drill deeper",
                        Style::default().fg(Color::DarkGray),
                    )),
                    Line::from(Span::styled(
                        "  Esc    go back",
                        Style::default().fg(Color::DarkGray),
                    )),
                    Line::from(Span::styled(
                        "  n/p    page keys",
                        Style::default().fg(Color::DarkGray),
                    )),
                ])
                .block(block),
                area,
            );
            return;
        }
    };

    let mut lines = Vec::new();

    if let Some(fields) = record.get("fields").and_then(|f| f.as_object()) {
        for (field_name, field_data) in fields {
            let value = field_data.get("value").unwrap_or(field_data);

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
