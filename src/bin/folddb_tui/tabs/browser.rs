use crate::app::{App, BrowserFocus};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::horizontal([
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(50),
    ])
    .split(area);

    render_schemas_panel(frame, app, chunks[0]);
    render_keys_panel(frame, app, chunks[1]);
    render_values_panel(frame, app, chunks[2]);
}

fn render_schemas_panel(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.browser.focus == BrowserFocus::Schemas;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };

    let items: Vec<ListItem> = app
        .browser
        .schemas
        .iter()
        .enumerate()
        .map(|(i, schema)| {
            let style = if i == app.browser.schema_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Span::styled(schema.name(), style))
        })
        .collect();

    let title = format!(" Schemas ({}) ", app.browser.schemas.len());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn render_keys_panel(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.browser.focus == BrowserFocus::Keys;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };

    let items: Vec<ListItem> = app
        .browser
        .keys
        .iter()
        .enumerate()
        .map(|(i, kv)| {
            let key_str = format_key_value(kv);
            let style = if i == app.browser.key_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Span::styled(key_str, style))
        })
        .collect();

    let page_info = if app.browser.total_keys > 0 {
        format!(
            " Keys ({}-{}/{}) [n/p] ",
            app.browser.page_offset + 1,
            (app.browser.page_offset + app.browser.keys.len()).min(app.browser.total_keys),
            app.browser.total_keys
        )
    } else {
        " Keys ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(page_info)
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    if app.browser.loading {
        let paragraph = Paragraph::new("Loading...").block(block);
        frame.render_widget(paragraph, area);
    } else {
        let list = List::new(items).block(block);
        frame.render_widget(list, area);
    }
}

fn render_values_panel(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.browser.focus == BrowserFocus::Values;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(" Values ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    if app.browser.values.is_empty() {
        let text = if app.browser.keys.is_empty() {
            "Select a schema and key to view data"
        } else {
            "Press Enter on a key to load values"
        };
        let paragraph = Paragraph::new(text).block(block);
        frame.render_widget(paragraph, area);
        return;
    }

    // Show the values for the selected key
    let mut lines: Vec<Line> = Vec::new();

    // Filter values to show just the selected key's data if possible
    let values_to_show = if app.browser.key_selected < app.browser.values.len() {
        &app.browser.values[app.browser.key_selected..=app.browser.key_selected]
    } else {
        &app.browser.values[..]
    };

    for value in values_to_show {
        if let serde_json::Value::Object(map) = value {
            for (key, val) in map {
                let val_str = match val {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{}: ", key),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw(truncate(&val_str, 60)),
                ]));
            }
        } else {
            let pretty = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
            for line in pretty.lines() {
                lines.push(Line::from(line.to_string()));
            }
        }
    }

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn format_key_value(kv: &fold_db::schema::types::key_value::KeyValue) -> String {
    match (&kv.hash, &kv.range) {
        (Some(h), Some(r)) => format!("{}_{}", h, r),
        (Some(h), None) => h.clone(),
        (None, Some(r)) => format!("_{}", r),
        (None, None) => "empty".to_string(),
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s.to_string()
    }
}
