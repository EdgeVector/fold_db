use crate::app::{App, InputMode};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let has_completion = app.folder_sync.completion.is_some();
    let chunks = if has_completion {
        Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(6),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area)
    } else {
        Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(0),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area)
    };

    render_path_input(frame, app, chunks[0]);
    if has_completion {
        render_completion(frame, app, chunks[1]);
    }
    render_status_bar(frame, app, chunks[2]);
    render_scan_results(frame, app, chunks[3]);
}

fn render_path_input(frame: &mut Frame, app: &App, area: Rect) {
    let editing = app.folder_sync.input_mode == InputMode::Editing;
    let has_completion = app.folder_sync.completion.is_some();
    let (border_style, title) = if editing {
        (
            Style::default().fg(Color::Yellow),
            if has_completion {
                " Folder Path (Tab/Shift+Tab to cycle, Enter to scan) "
            } else {
                " Folder Path (Tab to complete, Enter to scan, Esc to cancel) "
            },
        )
    } else if app.folder_sync.scan_result.is_some() {
        (
            Style::default().fg(Color::Reset),
            " Folder Path (s=new scan) ",
        )
    } else {
        (
            Style::default().fg(Color::Reset),
            " Folder Path (Enter/i to type path) ",
        )
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let display = if app.folder_sync.path_input.is_empty() && !editing {
        "~/Documents"
    } else {
        &app.folder_sync.path_input
    };

    let style = if app.folder_sync.path_input.is_empty() && !editing {
        Style::default().fg(Color::Gray)
    } else {
        Style::default()
    };

    frame.render_widget(Paragraph::new(Span::styled(display, style)).block(block), area);

    if editing {
        frame.set_cursor_position((
            area.x + 1 + app.folder_sync.cursor_pos as u16,
            area.y + 1,
        ));
    }
}

fn render_completion(frame: &mut Frame, app: &App, area: Rect) {
    let comp = match &app.folder_sync.completion {
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

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    // Show a progress gauge if we have active progress during ingestion
    if app.folder_sync.ingesting_index.is_some() {
        if let Some(progress) = &app.folder_sync.progress {
            let pct = progress.progress_percentage as f64 / 100.0;
            let label = format!(
                " {}% - {}",
                progress.progress_percentage, progress.status_message
            );
            let gauge = Gauge::default()
                .gauge_style(Style::default().fg(Color::Yellow))
                .ratio(pct.min(1.0))
                .label(Span::styled(label, Style::default().fg(Color::Reset)));
            frame.render_widget(gauge, area);
            return;
        }
    }

    let text = if app.folder_sync.loading {
        Line::from(Span::styled(
            " Scanning...",
            Style::default().fg(Color::Yellow),
        ))
    } else if let Some(msg) = &app.folder_sync.status_message {
        Line::from(Span::styled(
            format!(" {}", msg),
            Style::default().fg(Color::Green),
        ))
    } else {
        Line::from(Span::styled(
            " Enter a folder path to scan for personal data files",
            Style::default().fg(Color::Gray),
        ))
    };

    frame.render_widget(Paragraph::new(text), area);
}

fn render_scan_results(frame: &mut Frame, app: &App, area: Rect) {
    let scan = match &app.folder_sync.scan_result {
        Some(s) => s,
        None => {
            let help = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  How Folder Sync Works:",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from("  1. Enter a folder path and press Enter"),
                Line::from("  2. AI scans files and identifies personal data"),
                Line::from("  3. Review recommended files"),
                Line::from("  4. Press 'a' to ingest all, or Enter for selected file"),
                Line::from(""),
                Line::from(Span::styled(
                    "  Each file goes through: parse -> schema detection -> mutation -> indexing",
                    Style::default().fg(Color::Gray),
                )),
            ];
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Scan Results ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
            frame.render_widget(Paragraph::new(help).block(block), area);
            return;
        }
    };

    let chunks = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left: recommended files
    let items: Vec<ListItem> = scan
        .recommended_files
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let is_selected = i == app.folder_sync.selected_file;
            let ingested = app
                .folder_sync
                .ingestion_results
                .iter()
                .any(|(_, r)| r.is_ok());

            let marker = if app.folder_sync.ingesting_index == Some(i) {
                Span::styled(" > ", Style::default().fg(Color::Yellow))
            } else if ingested && i < app.folder_sync.ingestion_results.len() {
                match &app.folder_sync.ingestion_results[i].1 {
                    Ok(_) => Span::styled(" + ", Style::default().fg(Color::Green)),
                    Err(_) => Span::styled(" x ", Style::default().fg(Color::Red)),
                }
            } else {
                Span::raw("   ")
            };

            let style = if is_selected {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let short_path = if file.path.len() > 40 {
                format!("...{}", &file.path[file.path.len() - 37..])
            } else {
                file.path.clone()
            };

            ListItem::new(Line::from(vec![
                marker,
                Span::styled(short_path, style),
                Span::styled(
                    format!(" [{}]", file.category),
                    Style::default().fg(Color::Gray),
                ),
            ]))
        })
        .collect();

    let title = format!(
        " Recommended ({}) [Enter=ingest, a=all] ",
        scan.recommended_files.len()
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD));

    frame.render_widget(List::new(items).block(block), chunks[0]);

    // Right: summary + skipped
    let mut lines = vec![
        Line::from(Span::styled(
            "Summary",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  Total scanned:  ", Style::default().fg(Color::Yellow)),
            Span::raw(scan.total_files.to_string()),
        ]),
        Line::from(vec![
            Span::styled("  Personal data:  ", Style::default().fg(Color::Yellow)),
            Span::styled(
                scan.summary.get("personal_data").unwrap_or(&0).to_string(),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Media:          ", Style::default().fg(Color::Yellow)),
            Span::raw(scan.summary.get("media").unwrap_or(&0).to_string()),
        ]),
        Line::from(vec![
            Span::styled("  Config:         ", Style::default().fg(Color::Yellow)),
            Span::raw(scan.summary.get("config").unwrap_or(&0).to_string()),
        ]),
        Line::from(vec![
            Span::styled("  Work:           ", Style::default().fg(Color::Yellow)),
            Span::raw(scan.summary.get("work").unwrap_or(&0).to_string()),
        ]),
        Line::from(vec![
            Span::styled("  Skipped:        ", Style::default().fg(Color::Yellow)),
            Span::raw(scan.skipped_files.len().to_string()),
        ]),
        Line::from(""),
    ];

    // Show ingestion results
    if !app.folder_sync.ingestion_results.is_empty() {
        lines.push(Line::from(Span::styled(
            "Ingestion Results",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        for (summary, result) in &app.folder_sync.ingestion_results {
            let style = if result.is_ok() {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            };
            lines.push(Line::from(Span::styled(format!("  {}", summary), style)));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Details ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    frame.render_widget(Paragraph::new(lines).block(block), chunks[1]);
}
