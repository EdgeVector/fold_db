use crate::app::{App, QueryFocus};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_query_builder(frame, app, chunks[0]);
    render_query_results(frame, app, chunks[1]);
}

fn render_query_builder(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_schema_selector(frame, app, chunks[0]);
    render_field_selector(frame, app, chunks[1]);
}

fn render_schema_selector(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.query.focus == QueryFocus::SchemaList;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };

    let items: Vec<ListItem> = app
        .query
        .schemas
        .iter()
        .enumerate()
        .map(|(i, schema)| {
            let style = if i == app.query.schema_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Span::styled(schema.name(), style))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(" Select Schema ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    if app.query.loading && app.query.schemas.is_empty() {
        let paragraph = Paragraph::new("Loading...").block(block);
        frame.render_widget(paragraph, area);
    } else {
        let list = List::new(items).block(block);
        frame.render_widget(list, area);
    }
}

fn render_field_selector(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.query.focus == QueryFocus::FieldList;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };

    let items: Vec<ListItem> = app
        .query
        .available_fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let checked = if i < app.query.selected_fields.len() && app.query.selected_fields[i] {
                "[x]"
            } else {
                "[ ]"
            };

            let style = if i == app.query.field_cursor {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    checked,
                    if i < app.query.selected_fields.len() && app.query.selected_fields[i] {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
                Span::raw(" "),
                Span::styled(field, style),
            ]))
        })
        .collect();

    let selected_count = app
        .query
        .selected_fields
        .iter()
        .filter(|&&s| s)
        .count();
    let title = format!(
        " Fields ({}/{}) [Space=toggle, Enter=execute] ",
        selected_count,
        app.query.available_fields.len()
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    if app.query.available_fields.is_empty() {
        let paragraph = Paragraph::new("Select a schema first").block(block);
        frame.render_widget(paragraph, area);
    } else {
        let list = List::new(items).block(block);
        frame.render_widget(list, area);
    }
}

fn render_query_results(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.query.focus == QueryFocus::Results;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(format!(" Results ({}) ", app.query.results.len()))
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    if app.query.loading {
        let paragraph = Paragraph::new("Executing query...").block(block);
        frame.render_widget(paragraph, area);
        return;
    }

    if app.query.results.is_empty() {
        let paragraph = Paragraph::new("No results. Select schema and fields, then press Enter.")
            .block(block);
        frame.render_widget(paragraph, area);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    for (i, result) in app.query.results.iter().enumerate() {
        let header_style = if i == app.query.result_scroll {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        };

        lines.push(Line::from(Span::styled(
            format!("--- Record {} ---", i + 1),
            header_style,
        )));

        if let serde_json::Value::Object(map) = result {
            for (key, val) in map {
                let val_str = match val {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("  {}: ", key), Style::default().fg(Color::Yellow)),
                    Span::raw(truncate(&val_str, 80)),
                ]));
            }
        } else {
            let pretty =
                serde_json::to_string_pretty(result).unwrap_or_else(|_| result.to_string());
            for line in pretty.lines() {
                lines.push(Line::from(format!("  {}", line)));
            }
        }

        lines.push(Line::from(""));
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((app.query.result_scroll as u16, 0));
    frame.render_widget(paragraph, area);
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s.to_string()
    }
}
