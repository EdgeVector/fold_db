use crate::app::{App, InputMode};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let chunks =
        Layout::vertical([Constraint::Length(3), Constraint::Length(1), Constraint::Min(0)])
            .split(area);

    render_input(frame, app, chunks[0]);
    render_status_bar(frame, app, chunks[1]);
    render_results(frame, app, chunks[2]);
}

fn render_input(frame: &mut Frame, app: &App, area: Rect) {
    let (border_style, title) = if app.search.input_mode == InputMode::Editing {
        (
            Style::default().fg(Color::Yellow),
            " Search (editing - Enter to search, Esc to cancel) ",
        )
    } else {
        (
            Style::default().fg(Color::Reset),
            " Search (Enter/i to edit) ",
        )
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let input = Paragraph::new(app.search.input.as_str()).block(block);
    frame.render_widget(input, area);

    // Show cursor when editing
    if app.search.input_mode == InputMode::Editing {
        frame.set_cursor_position((
            area.x + 1 + app.search.cursor_pos as u16,
            area.y + 1,
        ));
    }
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let text = if app.search.loading {
        Line::from(Span::styled(
            " Searching...",
            Style::default().fg(Color::Yellow),
        ))
    } else {
        Line::from(vec![
            Span::styled(
                format!(" {} results", app.search.results.len()),
                Style::default().fg(Color::Green),
            ),
            if !app.search.results.is_empty() {
                Span::styled(
                    format!("  (selected: {})", app.search.selected + 1),
                    Style::default().fg(Color::Gray),
                )
            } else {
                Span::raw("")
            },
        ])
    };

    let paragraph = Paragraph::new(text);
    frame.render_widget(paragraph, area);
}

fn render_results(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from("Schema").style(Style::default().fg(Color::Yellow)),
        Cell::from("Field").style(Style::default().fg(Color::Yellow)),
        Cell::from("Key").style(Style::default().fg(Color::Yellow)),
        Cell::from("Value").style(Style::default().fg(Color::Yellow)),
    ])
    .height(1);

    let rows: Vec<Row> = app
        .search
        .results
        .iter()
        .enumerate()
        .map(|(i, result)| {
            let key_str = format_key_value(&result.key_value);
            let value_str = truncate_value(&result.value, 40);

            let style = if i == app.search.selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(result.schema_name.clone()),
                Cell::from(result.field.clone()),
                Cell::from(key_str),
                Cell::from(value_str),
            ])
            .style(style)
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Results ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(25),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(35),
        ],
    )
    .header(header)
    .block(block);

    frame.render_widget(table, area);
}

fn format_key_value(kv: &fold_db::schema::types::key_value::KeyValue) -> String {
    match (&kv.hash, &kv.range) {
        (Some(h), Some(r)) => format!("{}_{}", h, r),
        (Some(h), None) => h.clone(),
        (None, Some(r)) => format!("_{}", r),
        (None, None) => "empty".to_string(),
    }
}

fn truncate_value(value: &serde_json::Value, max_len: usize) -> String {
    let s = match value {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s
    }
}
