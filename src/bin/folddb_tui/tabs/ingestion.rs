use crate::app::{App, InputMode};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let has_completion = app.ingestion.completion.is_some();
    let chunks = if has_completion {
        Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(6),
            Constraint::Min(0),
        ])
        .split(area)
    } else {
        Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(0),
            Constraint::Min(0),
        ])
        .split(area)
    };

    render_file_input(frame, app, chunks[0]);
    if has_completion {
        render_completion(frame, app, chunks[1]);
    }
    render_result(frame, app, chunks[2]);
}

fn render_file_input(frame: &mut Frame, app: &App, area: Rect) {
    let editing = app.ingestion.input_mode == InputMode::Editing;
    let has_completion = app.ingestion.completion.is_some();

    let (border_style, title) = if editing {
        (
            Style::default().fg(Color::Yellow),
            if has_completion {
                " File Path (Tab/Shift+Tab to cycle, Enter to confirm) "
            } else {
                " File Path (Tab to complete, Enter to ingest, Esc to cancel) "
            },
        )
    } else {
        (
            Style::default().fg(Color::Reset),
            " File Path (Enter/i to type path) ",
        )
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let display = if app.ingestion.path_input.is_empty() && !editing {
        "/path/to/file.json"
    } else {
        &app.ingestion.path_input
    };

    let style = if app.ingestion.path_input.is_empty() && !editing {
        Style::default().fg(Color::Gray)
    } else {
        Style::default()
    };

    frame.render_widget(Paragraph::new(Span::styled(display, style)).block(block), area);

    if editing {
        frame.set_cursor_position((
            area.x + 1 + app.ingestion.cursor_pos as u16,
            area.y + 1,
        ));
    }
}

fn render_completion(frame: &mut Frame, app: &App, area: Rect) {
    let comp = match &app.ingestion.completion {
        Some(c) => c,
        None => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(format!(" Completions ({}) ", comp.candidates.len()))
        .title_style(Style::default().fg(Color::Yellow));

    let lines: Vec<Line> = comp
        .candidates
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let is_selected = i == comp.selected;
            let display = path
                .rsplit('/')
                .next()
                .unwrap_or(path);
            if is_selected {
                Line::from(Span::styled(
                    format!("  > {}", display),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(
                    format!("    {}", display),
                    Style::default().fg(Color::Reset),
                ))
            }
        })
        .collect();

    frame.render_widget(
        Paragraph::new(lines).block(block).scroll((
            comp.selected.saturating_sub(3) as u16,
            0,
        )),
        area,
    );
}

fn render_result(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Ingestion Result ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    if app.ingestion.loading {
        let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Progress bar
        if let Some(progress) = &app.ingestion.progress {
            let pct = progress.progress_percentage as f64 / 100.0;
            let label = format!(
                "{}% - {}",
                progress.progress_percentage, progress.status_message
            );
            let gauge = Gauge::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Progress ")
                        .title_style(
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                )
                .gauge_style(Style::default().fg(Color::Yellow))
                .ratio(pct.min(1.0))
                .label(Span::styled(label, Style::default().fg(Color::Reset)));
            frame.render_widget(gauge, chunks[0]);
        } else {
            let gauge = Gauge::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Progress ")
                        .title_style(
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                )
                .gauge_style(Style::default().fg(Color::Yellow))
                .ratio(0.0)
                .label(Span::styled(
                    "Starting...",
                    Style::default().fg(Color::Gray),
                ));
            frame.render_widget(gauge, chunks[0]);
        }

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Pipeline: parse -> AI schema -> mutation -> index",
                Style::default().fg(Color::Gray),
            )),
        ];
        frame.render_widget(Paragraph::new(lines).block(block), chunks[1]);
        return;
    }

    match &app.ingestion.result {
        Some(Ok(resp)) => {
            let status_style = if resp.success {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            };

            let mut lines = vec![
                Line::from(vec![
                    Span::styled("  Status:     ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        if resp.success { "Success" } else { "Failed" },
                        status_style,
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  Schema:     ", Style::default().fg(Color::Yellow)),
                    Span::raw(resp.schema_used.as_deref().unwrap_or("none")),
                    if resp.new_schema_created {
                        Span::styled(" (new)", Style::default().fg(Color::Cyan))
                    } else {
                        Span::raw("")
                    },
                ]),
                Line::from(vec![
                    Span::styled("  Mutations:  ", Style::default().fg(Color::Yellow)),
                    Span::raw(format!(
                        "{} generated, {} executed",
                        resp.mutations_generated, resp.mutations_executed
                    )),
                ]),
            ];

            if let Some(id) = &resp.progress_id {
                lines.push(Line::from(vec![
                    Span::styled("  Progress:   ", Style::default().fg(Color::Yellow)),
                    Span::styled(id, Style::default().fg(Color::Gray)),
                ]));
            }

            if !resp.errors.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Errors:",
                    Style::default().fg(Color::Red),
                )));
                for err in &resp.errors {
                    lines.push(Line::from(Span::styled(
                        format!("    - {}", err),
                        Style::default().fg(Color::Red),
                    )));
                }
            }

            frame.render_widget(Paragraph::new(lines).block(block), area);
        }
        Some(Err(e)) => {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  Error: {}", e),
                    Style::default().fg(Color::Red),
                )),
            ];
            frame.render_widget(Paragraph::new(lines).block(block), area);
        }
        None => {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Single File Ingestion",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from("  Enter a file path to ingest it into FoldDB."),
                Line::from(""),
                Line::from("  Supported formats:"),
                Line::from(Span::styled(
                    "    Native:  JSON, CSV, TXT, Markdown, JS/Twitter",
                    Style::default().fg(Color::Green),
                )),
                Line::from(Span::styled(
                    "    Via AI:  PDF, images, YAML, and more",
                    Style::default().fg(Color::Yellow),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  The file will be parsed, a schema determined by AI,",
                    Style::default().fg(Color::Gray),
                )),
                Line::from(Span::styled(
                    "  mutations generated and executed, then keywords indexed.",
                    Style::default().fg(Color::Gray),
                )),
            ];
            frame.render_widget(Paragraph::new(lines).block(block), area);
        }
    }
}
