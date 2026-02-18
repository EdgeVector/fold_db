use crate::app::App;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    render_schema_list(frame, app, chunks[0]);
    render_schema_detail(frame, app, chunks[1]);
}

fn render_schema_list(frame: &mut Frame, app: &App, area: Rect) {
    let focus_style = if app.schemas_state.focus == 0 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(focus_style)
        .title(format!(" Schemas ({}) ", app.schemas_state.schemas.len()))
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

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
                "  No schemas found. Press 'r' to refresh.",
                Style::default().fg(Color::Gray),
            )),
        ];
        frame.render_widget(Paragraph::new(lines).block(block), area);
        return;
    }

    let inner_height = area.height.saturating_sub(2) as usize;
    // Scroll so that the selected item is visible
    let scroll = if app.schemas_state.selected >= inner_height {
        app.schemas_state.selected - inner_height + 1
    } else {
        0
    };

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
                    Span::styled(" [A] ", Style::default().fg(Color::Green))
                }
                fold_db::schema::schema_types::SchemaState::Blocked => {
                    Span::styled(" [B] ", Style::default().fg(Color::Red))
                }
                fold_db::schema::schema_types::SchemaState::Available => {
                    Span::styled(" [?] ", Style::default().fg(Color::Yellow))
                }
            };

            let name_style = if i == app.schemas_state.selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let marker = if i == app.schemas_state.selected {
                "> "
            } else {
                "  "
            };

            Line::from(vec![
                Span::raw(marker),
                state_indicator,
                Span::styled(sws.name().to_string(), name_style),
            ])
        })
        .collect();

    frame.render_widget(
        Paragraph::new(lines).block(block),
        area,
    );
}

fn render_schema_detail(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_fields_panel(frame, app, chunks[0]);
    render_keys_panel(frame, app, chunks[1]);
}

fn render_fields_panel(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Schema Details ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    if app.schemas_state.schemas.is_empty() {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Select a schema to view details",
                    Style::default().fg(Color::Gray),
                )),
            ])
            .block(block),
            area,
        );
        return;
    }

    let sws = &app.schemas_state.schemas[app.schemas_state.selected];
    let schema = &sws.schema;

    let mut lines = vec![
        Line::from(vec![
            Span::styled("  Name:   ", Style::default().fg(Color::Yellow)),
            Span::raw(schema.name.clone()),
        ]),
        Line::from(vec![
            Span::styled("  Type:   ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{:?}", schema.schema_type)),
        ]),
        Line::from(vec![
            Span::styled("  State:  ", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{:?}", sws.state),
                match sws.state {
                    fold_db::schema::schema_types::SchemaState::Approved => {
                        Style::default().fg(Color::Green)
                    }
                    fold_db::schema::schema_types::SchemaState::Blocked => {
                        Style::default().fg(Color::Red)
                    }
                    fold_db::schema::schema_types::SchemaState::Available => {
                        Style::default().fg(Color::Yellow)
                    }
                },
            ),
        ]),
    ];

    if let Some(key) = &schema.key {
        if let Some(h) = &key.hash_field {
            lines.push(Line::from(vec![
                Span::styled("  Hash:   ", Style::default().fg(Color::Yellow)),
                Span::raw(h.clone()),
            ]));
        }
        if let Some(r) = &key.range_field {
            lines.push(Line::from(vec![
                Span::styled("  Range:  ", Style::default().fg(Color::Yellow)),
                Span::raw(r.clone()),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Fields:",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));

    if let Some(fields) = &schema.fields {
        for field in fields {
            let topo = schema
                .field_topologies
                .get(field)
                .map(|t| format!("{:?}", t.root))
                .unwrap_or_default();
            let topo_short = if topo.len() > 40 {
                format!("{}...", &topo[..40])
            } else {
                topo
            };
            lines.push(Line::from(vec![
                Span::styled("    - ", Style::default().fg(Color::Gray)),
                Span::raw(field.clone()),
                if !topo_short.is_empty() {
                    Span::styled(format!("  ({})", topo_short), Style::default().fg(Color::Gray))
                } else {
                    Span::raw("")
                },
            ]));
        }
    }

    if let Some(tf) = &schema.transform_fields {
        if !tf.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Transforms:",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            for (name, expr) in tf {
                lines.push(Line::from(vec![
                    Span::styled("    - ", Style::default().fg(Color::Gray)),
                    Span::raw(name.clone()),
                    Span::styled(format!(" = {}", expr), Style::default().fg(Color::Gray)),
                ]));
            }
        }
    }

    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .scroll((0, 0)),
        area,
    );
}

fn render_keys_panel(frame: &mut Frame, app: &App, area: Rect) {
    let focus_style = if app.schemas_state.focus == 1 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };

    let page_start = app.schemas_state.keys_offset + 1;
    let page_end = app.schemas_state.keys_offset + app.schemas_state.keys.len();
    let title = format!(
        " Keys ({}-{} of {}) [n/p=page] ",
        page_start, page_end, app.schemas_state.keys_total
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(focus_style)
        .title(title)
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    if app.schemas_state.keys_loading {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Loading keys...",
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
                "  No keys found for this schema.",
                Style::default().fg(Color::Gray),
            )),
        ];
        frame.render_widget(Paragraph::new(lines).block(block), area);
        return;
    }

    let lines: Vec<Line> = app
        .schemas_state
        .keys
        .iter()
        .enumerate()
        .map(|(i, kv)| {
            let key_str = match (&kv.hash, &kv.range) {
                (Some(h), Some(r)) => format!("{}_{}", h, r),
                (Some(h), None) => h.clone(),
                (None, Some(r)) => format!("_{}", r),
                (None, None) => "empty".to_string(),
            };

            let num = format!("  {:>3}. ", app.schemas_state.keys_offset + i + 1);
            Line::from(vec![
                Span::styled(num, Style::default().fg(Color::Gray)),
                Span::raw(key_str),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(lines).block(block), area);
}
