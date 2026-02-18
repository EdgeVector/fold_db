use crate::app::{App, Tab};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

use crate::tabs;

pub fn render(frame: &mut Frame, app: &App) {
    if app.log_state.visible {
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(12),
        ])
        .split(frame.area());

        render_tab_bar(frame, app, chunks[0]);
        render_tab_content(frame, app, chunks[1]);
        render_log_panel(frame, app, chunks[2]);
    } else {
        let chunks =
            Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(frame.area());

        render_tab_bar(frame, app, chunks[0]);
        render_tab_content(frame, app, chunks[1]);
    }
}

fn render_tab_content(frame: &mut Frame, app: &App, area: Rect) {
    match app.current_tab {
        Tab::Dashboard => tabs::dashboard::render(frame, app, area),
        Tab::FolderSync => tabs::folder_sync::render(frame, app, area),
        Tab::Ingestion => tabs::ingestion::render(frame, app, area),
        Tab::AiQuery => tabs::ai_query::render(frame, app, area),
        Tab::Search => tabs::search::render(frame, app, area),
        Tab::Schemas => tabs::schemas::render(frame, app, area),
    }
}

fn render_log_panel(frame: &mut Frame, app: &App, area: Rect) {
    let title = format!(" Logs ({}) [l=hide] ", app.log_state.lines.len());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray))
        .title(title)
        .title_style(
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        );

    let inner_height = area.height.saturating_sub(2) as usize; // borders
    let total = app.log_state.lines.len();

    // Auto-scroll: show the most recent lines
    let skip = total.saturating_sub(inner_height);

    let lines: Vec<Line> = app
        .log_state
        .lines
        .iter()
        .skip(skip)
        .take(inner_height)
        .map(|msg| {
            let style = if msg.contains("[ERROR]") {
                Style::default().fg(Color::Red)
            } else if msg.contains("[WARN]") {
                Style::default().fg(Color::Yellow)
            } else if msg.contains("[DEBUG]") {
                Style::default().fg(Color::Gray)
            } else {
                Style::default().fg(Color::Reset)
            };
            Line::from(Span::styled(format!(" {}", msg), style))
        })
        .collect();

    if lines.is_empty() {
        let placeholder = vec![Line::from(Span::styled(
            "  No logs yet. Logs appear here during ingestion, queries, etc.",
            Style::default().fg(Color::Gray),
        ))];
        frame.render_widget(Paragraph::new(placeholder).block(block), area);
    } else {
        frame.render_widget(Paragraph::new(lines).block(block), area);
    }
}

fn render_tab_bar(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .enumerate()
        .map(|(i, tab)| {
            let num = format!("{}", i + 1);
            Line::from(vec![
                Span::styled(
                    num,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(":"),
                Span::raw(tab.title()),
            ])
        })
        .collect();

    let bar_title = if app.log_state.visible {
        " FoldDB TUI [l=hide logs] "
    } else {
        " FoldDB TUI [l=show logs] "
    };

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(bar_title),
        )
        .select(app.current_tab.index())
        .style(Style::default().fg(Color::Reset))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::raw(" | "));

    frame.render_widget(tabs, area);
}
