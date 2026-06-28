use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

use crate::{app::App, theme};

// ── Dots tab — card layout ────────────────────────────────────────────

pub fn draw_dots(frame: &mut Frame, area: Rect, app: &App) {
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
        _ => {
            frame.render_widget(&block, area);
        }
    }

    // ── Card list ─────────────────────────────────────────────────────
    let filtered = app.filtered_statuses();

    // Each card is 3 lines collapsed, more when expanded
    let card_lines: Vec<Line> = build_card_lines(app, &filtered);

    let text = Text::from(card_lines);
    frame.render_widget(
        Paragraph::new(text)
            .wrap(Wrap { trim: true }),
        inner,
    );
}

fn build_card_lines<'a>(app: &'a App, filtered: &[(usize, &'a omah_lib::ops::DotStatus)]) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = Vec::new();
    let diff_map = app.diff_map();

    for (list_idx, (actual_idx, s)) in filtered.iter().enumerate() {
        let selected = list_idx == app.selected_index;
        let is_expanded = app.detail_expanded == Some(*actual_idx);

        // ── Card header: name + status badge ──────────────────────────
        let name_style = if selected {
            Style::new().fg(theme::PRIMARY_BRIGHT).add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(theme::TEXT)
        };
        let selector = if selected { "▸ " } else { "  " };

        let mut header_spans = vec![
            Span::styled(selector, theme::dim()),
            Span::styled(s.name.clone(), name_style),
            Span::raw("  "),
            status_badge(s),
        ];

        // Diff indicator
        let dot_changes = diff_map.get(s.name.as_str()).map(|v| v.len()).unwrap_or(0);
        if dot_changes > 0 {
            header_spans.push(Span::raw("  "));
            header_spans.push(Span::styled(
                format!("~{dot_changes}"),
                Style::new().fg(theme::WARNING).bold(),
            ));
            header_spans.push(Span::styled(" diff", theme::dim()));
        }

        lines.push(Line::from(header_spans));

        // ── Card summary: deps chips + setup count ────────────────────
        let mut summary_spans = vec![Span::raw("    ")];

        // Dep chips
        if !s.missing_deps.is_empty() {
            summary_spans.push(Span::styled(
                format!(" ✗{} ", s.missing_deps.len()),
                Style::new().fg(theme::ERROR),
            ));
            summary_spans.push(Span::styled("deps", theme::dim()));
            summary_spans.push(Span::raw(" "));
        } else {
            summary_spans.push(Span::styled(" ✓ ", Style::new().fg(theme::SUCCESS)));
            summary_spans.push(Span::styled("deps", theme::dim()));
            summary_spans.push(Span::raw(" "));
        }

        // Setup count
        if !s.pending_setup.is_empty() {
            summary_spans.push(Span::styled(
                format!("○{} ", s.pending_setup.len()),
                Style::new().fg(theme::WARNING),
            ));
            summary_spans.push(Span::styled("setup", theme::dim()));
        } else {
            summary_spans.push(Span::styled(" ✓ ", Style::new().fg(theme::SUCCESS)));
            summary_spans.push(Span::styled("setup", theme::dim()));
        }

        // Source
        summary_spans.push(Span::raw("  "));
        summary_spans.push(Span::styled(s.source.clone(), theme::dim()));

        lines.push(Line::from(summary_spans));

        // ── Action bar ────────────────────────────────────────────────
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled("[b]", Style::new().fg(theme::PRIMARY)),
            Span::styled("ackup ", theme::dim()),
            Span::styled("[r]", Style::new().fg(theme::PRIMARY)),
            Span::styled("estore ", theme::dim()),
            Span::styled("[e]", Style::new().fg(theme::PRIMARY)),
            Span::styled("dit ", theme::dim()),
            Span::styled("[x]", Style::new().fg(theme::PRIMARY)),
            Span::styled("remove", theme::dim()),
        ]));

        // ── Expanded detail panel ─────────────────────────────────────
        if is_expanded {
            draw_detail_lines(app, *actual_idx, &mut lines);
        }
    }

    lines
}

fn status_badge(s: &omah_lib::ops::DotStatus) -> Span<'static> {
    if s.symlinked {
        Span::styled("🔗 deployed", Style::new().fg(theme::PRIMARY_BRIGHT))
    } else if s.source_exists && s.backed_up {
        Span::styled("✓ deployed", Style::new().fg(theme::SUCCESS))
    } else if s.backed_up {
        Span::styled("○ available", Style::new().fg(theme::PRIMARY))
    } else if s.source_exists {
        Span::styled("⚠ unbacked", Style::new().fg(theme::WARNING))
    } else {
        Span::styled("✗ missing", Style::new().fg(theme::ERROR))
    }
}

fn draw_detail_lines(app: &App, dot_idx: usize, lines: &mut Vec<Line>) {
    let config = match app.config.as_ref().and_then(|c| c.dots.get(dot_idx)) {
        Some(d) => d,
        None => return,
    };
    let s = match app.statuses.get(dot_idx) {
        Some(s) => s,
        None => return,
    };

    lines.push(Line::from(Span::styled(
        " ── Locations ──",
        theme::tab_active(),
    )));

    let state_text = if s.symlinked {
        "🔗 symlinked".to_string()
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
        Span::styled("  Source:  ", theme::dim()),
        Span::styled(s.source.clone(), Style::new().fg(theme::TEXT)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  State:   ", theme::dim()),
        Span::styled(state_text, Style::new().fg(theme::TEXT)),
    ]));

    // Deps section
    if let Some(ref deps) = config.deps {
        if !deps.is_empty() {
            lines.push(Line::from(Span::styled(
                " ── Dependencies ──",
                theme::tab_active(),
            )));
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
                        if installed { "  installed" } else { "  missing" },
                        theme::dim(),
                    ),
                ]));
            }
        }
    }

    // Setup section
    if let Some(ref steps) = config.setup {
        if !steps.is_empty() {
            lines.push(Line::from(Span::styled(
                " ── Setup Steps ──",
                theme::tab_active(),
            )));

            // Check for active step execution
            let is_running = app
                .step_exec
                .as_ref()
                .map(|(n, st)| n == &config.name && st.running)
                .unwrap_or(false);
            let exec_output = app
                .step_exec
                .as_ref()
                .filter(|(n, _)| n == &config.name)
                .map(|(_, st)| st.output.clone())
                .unwrap_or_default();

            let pending = omah_lib::deps::pending_setup_steps(config);
            for step in steps {
                let is_pending = pending.iter().any(|p| p.install == step.install);
                let (icon, style) = if is_running && is_pending {
                    (" ◌", Style::new().fg(theme::PRIMARY_BRIGHT))
                } else if is_pending {
                    (" ○", Style::new().fg(theme::WARNING))
                } else {
                    (" ✓", Style::new().fg(theme::SUCCESS))
                };
                lines.push(Line::from(vec![
                    Span::styled(icon, style),
                    Span::raw(" "),
                    Span::styled(step.install.clone(), Style::new().fg(theme::TEXT)),
                ]));
            }

            // Show execution output if any
            if !exec_output.is_empty() {
                for line in &exec_output {
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(line.clone(), theme::text_hint()),
                    ]));
                }
            }

            // Action bar for setup
            if !pending.is_empty() {
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled("[r]", Style::new().fg(theme::PRIMARY)),
                    Span::styled("un all ", theme::dim()),
                    Span::styled("[s]", Style::new().fg(theme::PRIMARY)),
                    Span::styled("kip ", theme::dim()),
                    Span::styled("[i]", Style::new().fg(theme::PRIMARY)),
                    Span::styled("nstall deps", theme::dim()),
                ]));
            }
        }
    }

    lines.push(Line::from(Span::raw("")));
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

// ── Settings modal ───────────────────────────────────────────────────────

pub fn draw_settings(frame: &mut Frame, area: Rect, app: &App) {
    let sf = match &app.settings_form {
        Some(f) => f,
        None => return,
    };

    let mut lines: Vec<Line> = Vec::new();

    // Vault path
    let vault_focused = sf.focused == 0;
    let vault_style = if vault_focused {
        Style::new().fg(theme::PRIMARY_BRIGHT)
    } else {
        theme::dim()
    };
    lines.push(Line::from(vec![
        Span::styled(" Vault Path:", Style::new().fg(theme::TEXT_DIM)),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(sf.vault_path.clone(), vault_style),
    ]));
    if vault_focused {
        // Show cursor indicator
        lines.push(Line::from(Span::styled(
            "   ↑↓ to edit",
            theme::text_hint(),
        )));
    }
    lines.push(Line::from(Span::raw("")));

    // OS selector
    lines.push(Line::from(vec![
        Span::styled(" OS:", Style::new().fg(theme::TEXT_DIM)),
    ]));
    let os_line: Vec<Span> = SettingsForm::OS_OPTIONS
        .iter()
        .enumerate()
        .flat_map(|(i, opt)| {
            let selected = i == sf.os_index;
            let focused = sf.focused == 1;
            let style = if selected && focused {
                Style::new().fg(theme::PRIMARY_BRIGHT).bold()
            } else if selected {
                Style::new().fg(theme::TEXT)
            } else {
                theme::dim()
            };
            let mut spans = vec![Span::raw("  "), Span::styled(*opt, style)];
            if i < SettingsForm::OS_OPTIONS.len() - 1 {
                spans.push(Span::styled(" │", theme::dim()));
            }
            spans
        })
        .collect();
    lines.push(Line::from(os_line));
    lines.push(Line::from(Span::raw("")));

    // Package manager selector
    lines.push(Line::from(vec![
        Span::styled(" Package Manager:", Style::new().fg(theme::TEXT_DIM)),
    ]));
    let pm_line: Vec<Span> = SettingsForm::PKG_OPTIONS
        .iter()
        .enumerate()
        .flat_map(|(i, opt)| {
            let selected = i == sf.pkg_manager_index;
            let focused = sf.focused == 2;
            let style = if selected && focused {
                Style::new().fg(theme::PRIMARY_BRIGHT).bold()
            } else if selected {
                Style::new().fg(theme::TEXT)
            } else {
                theme::dim()
            };
            let mut spans = vec![Span::raw("  "), Span::styled(*opt, style)];
            if i < SettingsForm::PKG_OPTIONS.len() - 1 {
                spans.push(Span::styled(" │", theme::dim()));
            }
            spans
        })
        .collect();
    lines.push(Line::from(pm_line));
    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(Span::styled(
        " Tab:next  Enter:save  Esc:cancel ",
        theme::text_hint(),
    )));

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: true }),
        area,
    );
}

use crate::app::SettingsForm;
