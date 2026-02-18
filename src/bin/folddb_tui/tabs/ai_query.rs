use crate::app::{App, ChatRole, InputMode};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(3),
    ])
    .split(area);

    render_conversation(frame, app, chunks[0]);
    render_input(frame, app, chunks[1]);
}

fn render_input(frame: &mut Frame, app: &App, area: Rect) {
    let editing = app.ai_query.input_mode == InputMode::Editing;
    let (border_style, title) = if editing {
        (
            Style::default().fg(Color::Yellow),
            " Ask anything (Enter to send, Esc to cancel) ",
        )
    } else {
        (
            Style::default().fg(Color::Reset),
            " Ask anything (Enter/i to type) ",
        )
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let display = if app.ai_query.query_input.is_empty() && !editing {
        "What emails did I get last week?"
    } else {
        &app.ai_query.query_input
    };

    let style = if app.ai_query.query_input.is_empty() && !editing {
        Style::default().fg(Color::Gray)
    } else {
        Style::default()
    };

    frame.render_widget(Paragraph::new(Span::styled(display, style)).block(block), area);

    if editing {
        frame.set_cursor_position((
            area.x + 1 + app.ai_query.cursor_pos as u16,
            area.y + 1,
        ));
    }
}

fn render_conversation(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Conversation [Up/Down to scroll] ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    if app.ai_query.messages.is_empty() && !app.ai_query.loading {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  AI Query",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  Ask questions about your data in plain English."),
            Line::from("  The AI agent will autonomously:"),
            Line::from(""),
            Line::from(Span::styled(
                "    1. List available schemas to understand your data",
                Style::default().fg(Color::Green),
            )),
            Line::from(Span::styled(
                "    2. Search the index for relevant keywords",
                Style::default().fg(Color::Green),
            )),
            Line::from(Span::styled(
                "    3. Query specific schemas and fields",
                Style::default().fg(Color::Green),
            )),
            Line::from(Span::styled(
                "    4. Synthesize a final answer",
                Style::default().fg(Color::Green),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Requires FOLD_OPENROUTER_API_KEY to be set.",
                Style::default().fg(Color::Gray),
            )),
        ];
        frame.render_widget(Paragraph::new(lines).block(block), area);
        return;
    }

    let inner_width = area.width.saturating_sub(2) as usize;
    let mut lines: Vec<Line> = Vec::new();

    for msg in &app.ai_query.messages {
        match msg.role {
            ChatRole::User => {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  You:",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )));
                for wrapped in wrap_text(&msg.text, inner_width.saturating_sub(4)) {
                    lines.push(Line::from(Span::styled(
                        format!("    {}", wrapped),
                        Style::default().fg(Color::Reset),
                    )));
                }
            }
            ChatRole::ToolUse => {
                lines.push(Line::from(Span::styled(
                    format!("  [Agent used {} tools]", msg.tool_calls.len()),
                    Style::default().fg(Color::Gray),
                )));
                for tc in &msg.tool_calls {
                    let params_str = tc.params.to_string();
                    let params_short = if params_str.len() > 60 {
                        format!("{}...", &params_str[..60])
                    } else {
                        params_str
                    };
                    lines.push(Line::from(vec![
                        Span::styled("    ", Style::default()),
                        Span::styled(
                            &tc.tool,
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!(" {}", params_short),
                            Style::default().fg(Color::Gray),
                        ),
                    ]));
                }
            }
            ChatRole::Assistant => {
                lines.push(Line::from(""));
                let is_error = msg.text.starts_with("Error:");
                lines.push(Line::from(Span::styled(
                    "  AI:",
                    Style::default()
                        .fg(if is_error { Color::Red } else { Color::Green })
                        .add_modifier(Modifier::BOLD),
                )));
                let text_style = if is_error {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default().fg(Color::Reset)
                };
                for wrapped in wrap_text(&msg.text, inner_width.saturating_sub(4)) {
                    lines.push(Line::from(Span::styled(
                        format!("    {}", wrapped),
                        text_style,
                    )));
                }
            }
        }
    }

    // Show loading indicator at the bottom of conversation
    if app.ai_query.loading {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  AI is thinking...",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "    Searching your data with tools...",
            Style::default().fg(Color::Gray),
        )));
    }

    let inner_height = area.height.saturating_sub(2) as usize;
    let total_lines = lines.len();

    // Compute scroll: if scroll is MAX, auto-scroll to bottom
    let scroll = if app.ai_query.scroll == usize::MAX {
        total_lines.saturating_sub(inner_height)
    } else {
        app.ai_query.scroll.min(total_lines.saturating_sub(inner_height))
    };

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll as u16, 0));

    frame.render_widget(paragraph, area);
}

/// Simple word-wrap for a string to fit within `max_width` columns.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }
    let mut result = Vec::new();
    for line in text.lines() {
        if line.len() <= max_width {
            result.push(line.to_string());
        } else {
            let mut remaining = line;
            while remaining.len() > max_width {
                // Try to break at a space
                let break_at = remaining[..max_width]
                    .rfind(' ')
                    .unwrap_or(max_width);
                result.push(remaining[..break_at].to_string());
                remaining = remaining[break_at..].trim_start();
            }
            if !remaining.is_empty() {
                result.push(remaining.to_string());
            }
        }
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}
