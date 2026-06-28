use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, Wrap},
};

use crate::{app::App, theme};

// ── Status tab ───────────────────────────────────────────────────────────

pub fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border())
        .title(" Dotfiles ");

    let inner = block.inner(area);

    match &app.statuses {
        s if s.is_empty() => {
            let msg = match &app.config {
                None => " No dotfiles configured or config not loaded.",
                Some(c) if c.dots.is_empty() => " No dotfiles configured. Press 'a' to add one.",
                Some(_) => " Run 'omah backup' or 'omah restore' to populate status.",
            };
            frame.render_widget(
                Paragraph::new(Text::from(Line::from(Span::styled(msg, theme::dim()))))
                    .block(block),
                area,
            );
            return;
        }
        _statuses => {
            frame.render_widget(&block, area);
        }
    }

    // Split: table area + summary footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(inner);

    // ── Status table ───────────────────────────────────────────────────
    let header_style = Style::new()
        .fg(theme::PRIMARY_BRIGHT)
        .add_modifier(Modifier::BOLD)
        .bg(theme::SURFACE_LIGHT);

    let header_cells = ["Name", "State", "Deps", "Setup"]
        .iter()
        .map(|h| Cell::from(Span::styled(*h, header_style)));
    let header = Row::new(header_cells).height(1).style(
        Style::new().bg(theme::SURFACE_LIGHT),
    );

    let widths = [
        Constraint::Length(16),
        Constraint::Length(20),
        Constraint::Length(20),
        Constraint::Length(16),
    ];

    let rows: Vec<Row> = app
        .statuses
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let selected = i == app.selected_index;
            let row_style = if selected {
                Style::new().bg(theme::SURFACE_SELECTED)
            } else if i % 2 == 0 {
                Style::new().bg(theme::BG)
            } else {
                Style::new().bg(theme::SURFACE)
            };

            // State badge
            let state_span = if s.symlinked {
                Span::styled("🔗 deployed", Style::new().fg(theme::PRIMARY_BRIGHT))
            } else if s.source_exists && s.backed_up {
                Span::styled("✓ deployed", Style::new().fg(theme::SUCCESS))
            } else if s.backed_up {
                Span::styled("○ available", Style::new().fg(theme::PRIMARY))
            } else if s.source_exists {
                Span::styled("⚠ unbacked", Style::new().fg(theme::WARNING))
            } else {
                Span::styled("✗ missing", Style::new().fg(theme::ERROR))
            };

            // Deps badge
            let deps_text = if s.missing_deps.is_empty() {
                "—".to_string()
            } else {
                format!("✗ {} missing", s.missing_deps.len())
            };
            let deps_style = if s.missing_deps.is_empty() {
                theme::dim()
            } else {
                Style::new().fg(theme::ERROR)
            };

            // Setup badge
            let setup_text = if s.pending_setup.is_empty() {
                "—".to_string()
            } else {
                format!("○ {} pending", s.pending_setup.len())
            };
            let setup_style = if s.pending_setup.is_empty() {
                theme::dim()
            } else {
                Style::new().fg(theme::WARNING)
            };

            let name_prefix = if selected { "▸ " } else { "  " };
            let name_cell = Cell::from(Span::styled(
                format!("{}{}", name_prefix, s.name),
                if selected {
                    Style::new().fg(theme::PRIMARY_BRIGHT).add_modifier(Modifier::BOLD)
                } else {
                    Style::new().fg(theme::TEXT)
                },
            ));

            Row::new(vec![
                name_cell,
                Cell::from(state_span),
                Cell::from(Span::styled(deps_text, deps_style)),
                Cell::from(Span::styled(setup_text, setup_style)),
            ])
            .style(row_style)
            .height(1)
        })
        .collect();

    let table = Table::new(rows, widths)
        .header(header)
        .column_spacing(2);
    frame.render_widget(table, chunks[0]);

    // ── Summary footer ────────────────────────────────────────────────
    let total = app.statuses.len();
    let deployed = app.statuses.iter().filter(|s| s.source_exists && s.backed_up).count();
    let available = app.statuses.iter().filter(|s| !s.source_exists && s.backed_up).count();
    let unbacked = app.statuses.iter().filter(|s| s.source_exists && !s.backed_up).count();
    let missing = app.statuses.iter().filter(|s| !s.source_exists && !s.backed_up).count();

    let mut summary_parts = vec![
        Span::styled(format!(" {total} dotfile(s)"), Style::new().fg(theme::PRIMARY_BRIGHT)),
    ];
    if deployed > 0 {
        summary_parts.push(Span::styled(
            format!(" · {deployed} deployed"),
            Style::new().fg(theme::SUCCESS),
        ));
    }
    if available > 0 {
        summary_parts.push(Span::styled(
            format!(" · {available} available"),
            Style::new().fg(theme::PRIMARY),
        ));
    }
    if unbacked > 0 {
        summary_parts.push(Span::styled(
            format!(" · {unbacked} unbacked"),
            Style::new().fg(theme::WARNING),
        ));
    }
    if missing > 0 {
        summary_parts.push(Span::styled(
            format!(" · {missing} missing"),
            Style::new().fg(theme::ERROR),
        ));
    }

    frame.render_widget(
        Paragraph::new(Line::from(summary_parts))
            .style(Style::new().bg(theme::SURFACE)),
        chunks[1],
    );
}

// ── Details tab ──────────────────────────────────────────────────────────

pub fn draw_details(frame: &mut Frame, area: Rect, app: &App) {
    let dot = app.statuses.get(app.selected_index);
    let config_dot = app.config.as_ref().and_then(|c| c.dots.get(app.selected_index));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border())
        .title(match dot {
            Some(s) => format!(" {} ", s.name),
            None => " Dotfile ".to_string(),
        });

    let inner = block.inner(area);

    match dot {
        None => {
            frame.render_widget(
                Paragraph::new(Text::from(Line::from(Span::styled(
                    " Select a dotfile from the Status tab to view details.",
                    theme::dim(),
                ))))
                .block(block),
                area,
            );
            return;
        }
        Some(_) => {
            frame.render_widget(&block, area);
        }
    }

    let s = dot.unwrap();

    // Build detail rows
    let mut lines: Vec<Line> = Vec::new();

    // ── General info ────────────────────────────────────────────────
    lines.push(Line::from(Span::styled(" General", theme::tab_active())));
    lines.push(Line::from(Span::raw("")));

    let state_text = if s.symlinked {
        "🔗 deployed (symlink)".to_string()
    } else if s.source_exists && s.backed_up {
        "✓ deployed".to_string()
    } else if s.backed_up {
        "○ available".to_string()
    } else if s.source_exists {
        "⚠ unbacked".to_string()
    } else {
        "✗ missing".to_string()
    };
    lines.push(Line::from(vec![
        Span::styled("  Source:     ", theme::dim()),
        Span::styled(s.source.clone(), Style::new().fg(theme::TEXT)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  State:      ", theme::dim()),
        Span::styled(state_text, Style::new().fg(theme::TEXT)),
    ]));
    lines.push(Line::from(Span::raw("")));

    // ── Dependencies ────────────────────────────────────────────────
    if let Some(ref dot_cfg) = config_dot {
        if let Some(ref deps) = dot_cfg.deps {
            if !deps.is_empty() {
                lines.push(Line::from(Span::styled(" Dependencies", theme::tab_active())));
                lines.push(Line::from(Span::raw("")));
                for dep in deps {
                    let installed = omah_lib::deps::is_installed(dep);
                    let (icon, style) = if installed {
                        ("●", Style::new().fg(theme::SUCCESS))
                    } else {
                        ("○", Style::new().fg(theme::ERROR))
                    };
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {icon} "), style),
                        Span::styled(dep.clone(), Style::new().fg(theme::TEXT)),
                        Span::styled(
                            if installed { "  (installed)" } else { "  (missing)" },
                            theme::dim(),
                        ),
                    ]));
                }
                lines.push(Line::from(Span::raw("")));
            }
        }

        // ── Setup steps ──────────────────────────────────────────────
        if let Some(ref steps) = dot_cfg.setup {
            if !steps.is_empty() {
                lines.push(Line::from(Span::styled(" Setup Steps", theme::tab_active())));
                lines.push(Line::from(Span::raw("")));
                let pending = omah_lib::deps::pending_setup_steps(dot_cfg);
                for step in steps {
                    let is_pending = pending.iter().any(|p| p.install == step.install);
                    let (icon, style) = if is_pending {
                        ("○", Style::new().fg(theme::WARNING))
                    } else {
                        ("✓", Style::new().fg(theme::SUCCESS))
                    };
                    let check_info = step
                        .check
                        .as_deref()
                        .map(|c| format!("  [{c}]"))
                        .unwrap_or_default();
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {icon} "), style),
                        Span::styled(step.install.clone(), Style::new().fg(theme::TEXT)),
                        Span::styled(check_info, theme::dim()),
                        Span::styled(
                            if is_pending { "  (pending)" } else { "  (done)" },
                            theme::dim(),
                        ),
                    ]));
                }
                lines.push(Line::from(Span::raw("")));
            }
        }

        // ── Excludes ──────────────────────────────────────────────
        if let Some(ref exclude) = dot_cfg.exclude {
            if !exclude.is_empty() {
                lines.push(Line::from(Span::styled(" Exclude", theme::tab_active())));
                lines.push(Line::from(Span::raw("")));
                for pat in exclude {
                    lines.push(Line::from(vec![
                        Span::styled("  ⊘ ", Style::new().fg(theme::DIM)),
                        Span::styled(pat.clone(), theme::text_hint()),
                    ]));
                }
                lines.push(Line::from(Span::raw("")));
            }
        }
    }

    let text = Text::from(lines);
    frame.render_widget(
        Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .style(Style::new().bg(theme::BG)),
        inner,
    );
}

// ── Diff tab ─────────────────────────────────────────────────────────────

pub fn draw_diff(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border())
        .title(" Changes ");

    let inner = block.inner(area);

    if app.changes.is_empty() {
        frame.render_widget(
            Paragraph::new(Text::from(Line::from(Span::styled(
                " No changes — all dotfiles are in sync with the vault.",
                Style::new().fg(theme::SUCCESS),
            ))))
            .block(block),
            area,
        );
        return;
    }
    frame.render_widget(&block, area);

    // Group changes by dot_name
    let mut lines: Vec<Line> = Vec::new();
    let mut current_dot = String::new();

    for c in &app.changes {
        if c.dot_name != current_dot {
            current_dot = c.dot_name.clone();
            lines.push(Line::from(Span::styled(
                format!(" {}", current_dot),
                Style::new()
                    .fg(theme::PRIMARY_BRIGHT)
                    .add_modifier(Modifier::BOLD),
            )));
        }

        let (icon, style, label) = match c.kind {
            omah_lib::ops::ChangeKind::Added => ("+", Style::new().fg(theme::SUCCESS), "new in source"),
            omah_lib::ops::ChangeKind::Modified => ("~", Style::new().fg(theme::WARNING), "modified"),
            omah_lib::ops::ChangeKind::Removed => ("-", Style::new().fg(theme::ERROR), "only in vault"),
        };

        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(format!(" {icon} "), style.bold()),
            Span::styled(c.path.clone(), Style::new().fg(theme::TEXT)),
            Span::raw("  "),
            Span::styled(label, theme::dim()),
        ]));
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: true }),
        inner,
    );
}

// ── Log tab ──────────────────────────────────────────────────────────────

pub fn draw_log(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border())
        .title(" Operations Log ");

    let inner = block.inner(area);

    if app.log_entries.is_empty() {
        frame.render_widget(
            Paragraph::new(Text::from(Line::from(Span::styled(
                " No operations yet. Actions will appear here.",
                theme::dim(),
            ))))
            .block(block),
            area,
        );
        return;
    }
    frame.render_widget(&block, area);

    let lines: Vec<Line> = app
        .log_entries
        .iter()
        .map(|entry| {
            let style = match entry.kind {
                crate::app::LogKind::Info => Style::new().fg(theme::TEXT),
                crate::app::LogKind::Success => Style::new().fg(theme::SUCCESS),
                crate::app::LogKind::Error => Style::new().fg(theme::ERROR),
                crate::app::LogKind::Warning => Style::new().fg(theme::WARNING),
            };
            Line::from(Span::styled(format!(" {}", entry.text), style))
        })
        .collect();

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: true }),
        inner,
    );
}
