use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Tabs, Wrap},
};

use crate::{
    app::{App, FormField, ModalState, Tab},
    screens, theme, widgets,
};

// ── Root layout ──────────────────────────────────────────────────────────

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    if area.height < 8 || area.width < 40 {
        let msg = Paragraph::new("Terminal too small — resize to at least 40×8")
            .style(Style::new().fg(theme::WARNING));
        frame.render_widget(msg, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Length(1), // search bar
            Constraint::Length(1), // tabs
            Constraint::Min(0),    // content
            Constraint::Length(1), // help bar
        ])
        .split(area);

    draw_header(frame, chunks[0], app);
    draw_search(frame, chunks[1], app);
    draw_tabs(frame, chunks[2], app);
    draw_content(frame, chunks[3], app);
    draw_help(frame, chunks[4], app);

    // Modal overlay (drawn on top)
    if let Some(ref modal) = app.modal {
        draw_modal_overlay(frame, area, modal, app);
    }
}

// ── Header ───────────────────────────────────────────────────────────────

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let vault = app
        .config
        .as_ref()
        .map(|c| c.vault_path.as_str())
        .unwrap_or("—");

    let header = Paragraph::new(Line::from(vec![
        Span::styled(" Omah ", theme::title()),
        Span::styled("TUI ", Style::new().fg(theme::PRIMARY_DIM)),
        Span::styled(format!("│ {vault}"), theme::dim()),
    ]))
    .style(Style::new().bg(theme::SURFACE));

    frame.render_widget(header, area);
}

// ── Search bar ───────────────────────────────────────────────────────────

fn draw_search(frame: &mut Frame, area: Rect, app: &App) {
    let prefix = if app.search_focused { ">" } else { "/" };
    let style = if app.search_focused {
        Style::new().fg(theme::PRIMARY_BRIGHT).bg(theme::SURFACE_LIGHT)
    } else {
        Style::new().fg(theme::TEXT_DIM).bg(theme::SURFACE)
    };

    let line = if app.search.is_empty() && !app.search_focused {
        Line::from(Span::styled(" Search dotfiles… (press '/')", theme::dim()))
    } else if app.search_focused {
        let display = &app.search;
        let pos = app.search_cursor.min(display.len());
        let before = &display[..pos];
        let after = &display[pos..];
        let cursor_ch = after.chars().next().map(|c| c.to_string()).unwrap_or_default();
        let after_start = after.chars().next().map(|c| c.len_utf8()).unwrap_or(0);
        Line::from(vec![
            Span::styled(prefix, Style::new().fg(theme::PRIMARY_BRIGHT)),
            Span::styled(before, Style::new().fg(theme::TEXT)),
            Span::styled(
                cursor_ch,
                Style::new().fg(theme::PRIMARY_BRIGHT).bg(theme::SURFACE_SELECTED),
            ),
            Span::styled(&after[after_start..], Style::new().fg(theme::TEXT)),
        ])
    } else {
        Line::from(vec![
            Span::styled("/", theme::dim()),
            Span::styled(app.search.clone(), Style::new().fg(theme::TEXT)),
        ])
    };

    frame.render_widget(Paragraph::new(line).style(style), area);
}

// ── Tab bar ──────────────────────────────────────────────────────────────

fn draw_tabs(frame: &mut Frame, area: Rect, app: &App) {
    let titles: Vec<Line> = Tab::all()
        .iter()
        .map(|tab| {
            let active = *tab == app.active_tab;
            let style = if active { theme::tab_active() } else { theme::tab_inactive() };
            Line::from(Span::styled(format!(" {} ", tab.label()), style))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .highlight_style(theme::tab_active())
        .select(app.active_tab.index())
        .divider(" │ ");
    frame.render_widget(tabs, area);
}

// ── Content area ─────────────────────────────────────────────────────────

fn draw_content(frame: &mut Frame, area: Rect, app: &App) {
    match app.active_tab {
        Tab::Dots => screens::draw_dots(frame, area, app),
        Tab::Log => screens::draw_log(frame, area, app),
    }
}

// ── Help bar ─────────────────────────────────────────────────────────────

fn draw_help(frame: &mut Frame, area: Rect, app: &App) {
    let help_text = match app.active_tab {
        Tab::Dots => {
            if app.detail_expanded.is_some() {
                "Enter/Esc:close  i:install deps  s:skip setup  r:run setup"
            } else {
                "↑↓:nav  Enter:expand  a:add  e:edit  x:remove  b:backup  r:restore"
            }
        }
        Tab::Log => "↑↓:scroll",
    };

    let help = Paragraph::new(Line::from(vec![
        Span::styled(format!(" {help_text}"), theme::text_hint()),
        Span::styled("  │  ?:help  1-2/Tab:switch  S:settings  q:quit", theme::dim()),
    ]))
    .style(Style::new().bg(theme::SURFACE));

    frame.render_widget(help, area);
}

// ── Modal overlay ────────────────────────────────────────────────────────

fn draw_modal_overlay(frame: &mut Frame, area: Rect, modal: &ModalState, app: &App) {
    let modal_title: &str;
    let height_pct: u16;
    #[allow(clippy::type_complexity)]
    let content_fn: Box<dyn FnOnce(&mut Frame, Rect)>;

    match modal {
        ModalState::AddForm(form) | ModalState::EditForm(form, _) => {
            modal_title = &form.title;
            height_pct = 80;
            let fields = form.fields.clone();
            let focused = form.focused;
            let error = form.error.clone();
            content_fn = Box::new(move |f, a| draw_form(f, a, &fields, focused, &error));
        }
        ModalState::DepFlow(ws) => {
            modal_title = " Dependencies & Setup ";
            height_pct = 75;
            let dot = ws.dot_name.clone();
            let pm = ws.pkg_manager.clone();
            let install_cmd = ws.install_cmd.clone();
            let deps = ws.missing_deps.clone();
            let steps = ws.setup_steps.clone();
            let total = ws.total_count;
            let done = ws.done_count;
            let error = ws.error.clone();
            let all_done = ws.all_done;
            let _all_checked = ws.missing_deps.iter().all(|d| d.checked)
                && ws.setup_steps.iter().all(|s| s.checked);
            content_fn = Box::new(move |f, a| {
                draw_dep_flow(f, a, &dot, &deps, &steps, &pm, &install_cmd, _all_checked, total, done, &error, all_done)
            });
        }
        ModalState::Error(msg) => {
            modal_title = " Error ";
            height_pct = 30;
            let m = msg.clone();
            content_fn = Box::new(move |f, a| widgets::draw_error(f, a, &m));
        }
        ModalState::RemoveConfirm(name) => {
            modal_title = " Remove Dotfile ";
            height_pct = 25;
            let n = name.clone();
            content_fn = Box::new(move |f, a| {
                widgets::draw_confirm(
                    f,
                    a,
                    &format!("Remove '{n}' from config?\nVault files will NOT be deleted."),
                )
            });
        }
        ModalState::Confirm { message: _, action: _ } => {
            modal_title = " Confirm ";
            height_pct = 25;
            content_fn = Box::new(|f, a| {
                widgets::draw_confirm(f, a, "Proceed with this action?")
            });
        }
        ModalState::Settings => {
            modal_title = " Settings ";
            height_pct = 50;
            content_fn = Box::new(|f, a| screens::draw_settings(f, a, app));
        }
        ModalState::HelpOverlay(ctx) => {
            modal_title = " Help ";
            height_pct = 75;
            let ctx = *ctx;
            content_fn = Box::new(move |f, a| widgets::draw_help_overlay(f, a, ctx));
        }
    }

    widgets::draw_modal(frame, area, modal_title, 80, height_pct, content_fn);
}

// ── Form draw ────────────────────────────────────────────────────────────

fn draw_form(frame: &mut Frame, area: Rect, fields: &[FormField], focused: usize, error: &Option<String>) {
    let inner_area = area;
    let mut constraints = Vec::new();
    for field in fields.iter() {
        match field {
            FormField::Text { .. } | FormField::Toggle { .. } => {
                constraints.push(Constraint::Length(1));
            }
            FormField::SetupSteps { items, .. } => {
                let h = (items.len() + 1) as u16 + 2; // header + rows
                constraints.push(Constraint::Length(h.min(10)));
            }
        }
    }

    // Hint + error rows
    if error.is_some() {
        constraints.push(Constraint::Length(2));
    }
    constraints.push(Constraint::Length(1)); // hint

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner_area);

    let mut y = 0;
    for (_i, field) in fields.iter().enumerate() {
        let _is_focused = _i == focused;
        match field {
            FormField::Text { label, value, cursor } => {
                if y < chunks.len() {
                    widgets::draw_text_input(frame, chunks[y], label, value, *cursor, _is_focused);
                    y += 1;
                }
            }
            FormField::Toggle { label, value } => {
                if y < chunks.len() {
                    widgets::draw_toggle(frame, chunks[y], label, *value, _is_focused);
                    y += 1;
                }
            }
            FormField::SetupSteps { items, .. } => {
                if y < chunks.len() {
                    draw_setup_steps(frame, chunks[y], items, _is_focused);
                    y += 1;
                }
            }
        }
    }

    // Error line
    if let Some(err) = error
        && y < chunks.len()
    {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(format!(" ✗ {err}"), Style::new().fg(theme::ERROR)))),
            chunks[y],
        );
        y += 1;
    }

    // Hint line
    if y < chunks.len() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " Tab:next  Shift+Tab:prev  Enter:save  Esc:cancel ",
                theme::text_hint(),
            ))),
            chunks[y],
        );
    }
}

fn draw_setup_steps(frame: &mut Frame, area: Rect, items: &[crate::app::SetupFieldRow], focused: bool) {
    let bstyle = if focused { theme::border_focused() } else { theme::border() };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(bstyle)
        .title(" Setup Steps ");

    let inner = block.inner(area);
    frame.render_widget(&block, area);

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled(" Install ", Style::new().fg(theme::PRIMARY_DIM)),
            Span::raw("        "),
            Span::styled("Check", Style::new().fg(theme::PRIMARY_DIM)),
        ]),
    ];

    for row in items {
        let install_display = if row.install.is_empty() {
            "(new step)"
        } else {
            &row.install
        };
        let check_display = row.check_display();
        let check_str = if check_display.is_empty() {
            "—".to_string()
        } else {
            check_display
        };

        // Show check menu if open
        if row.show_check_menu {
            lines.push(Line::from(Span::styled(
                format!(" {}  [select check type…]", install_display),
                Style::new().fg(theme::TEXT),
            )));
            // Append check selector options inline
            for (i, (ct, label, _desc)) in crate::app::CHECK_TYPES.iter().enumerate() {
                let sel = if i == row.check_menu_index { "▸" } else { " " };
                let ct_style = if i == row.check_menu_index {
                    Style::new().fg(theme::PRIMARY_BRIGHT).bold()
                } else {
                    theme::dim()
                };
                lines.push(Line::from(vec![
                    Span::raw("     "),
                    Span::styled(format!("{sel} {ct}"), ct_style),
                    Span::raw(" "),
                    Span::styled(*label, theme::text_hint()),
                ]));
            }
        } else {
            lines.push(Line::from(vec![
                Span::styled(format!(" {}", install_display), Style::new().fg(theme::TEXT)),
                Span::raw("  "),
                Span::styled(check_str, theme::dim()),
            ]));
        }
    }

    if items.is_empty() {
        lines.push(Line::from(Span::styled(" (Press Enter to add a step)", theme::text_hint())));
    }

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: true }),
        inner,
    );
}

// ── Dep flow draw ────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn draw_dep_flow(
    frame: &mut Frame,
    area: Rect,
    dot_name: &str,
    deps: &[crate::dep_flow::DepItem],
    steps: &[crate::dep_flow::SetupItem],
    pkg_manager: &Option<String>,
    install_cmd: &Option<String>,
    _all_checked: bool,
    _total: usize,
    _done: usize,
    error: &Option<String>,
    all_done: bool,
) {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled(format!(" {} ", dot_name), Style::new().fg(theme::PRIMARY_BRIGHT).bold()),
        Span::styled(" — pre-restore checklist", theme::dim()),
    ]));
    lines.push(Line::from(Span::raw("")));

    // Package manager
    if let Some(pm) = pkg_manager {
        lines.push(Line::from(vec![
            Span::styled(" ● Package manager: ", Style::new().fg(theme::SUCCESS)),
            Span::styled(pm.clone(), Style::new().fg(theme::TEXT)),
        ]));
        if let Some(cmd) = install_cmd {
            lines.push(Line::from(vec![
                Span::styled("   Command: ", theme::dim()),
                Span::styled(cmd.clone(), theme::text_hint()),
            ]));
        }
        lines.push(Line::from(Span::raw("")));
    }

    // Dependencies
    if !deps.is_empty() {
        lines.push(Line::from(Span::styled(
            format!(" {} missing dep(s):", deps.len()),
            theme::tab_active(),
        )));
        for dep in deps {
            let check = if dep.checked { "[x]" } else { "[ ]" };
            let status = if dep.installed { " ✓" } else { "" };
            lines.push(Line::from(vec![
                Span::styled(format!("   {check} "), Style::new().fg(theme::PRIMARY)),
                Span::styled(dep.pkg.clone(), Style::new().fg(theme::TEXT)),
                Span::styled(status, Style::new().fg(theme::SUCCESS)),
            ]));
        }
        lines.push(Line::from(Span::raw("")));
    }

    // Setup steps
    if !steps.is_empty() {
        lines.push(Line::from(Span::styled(
            format!(" {} pending setup step(s):", steps.len()),
            theme::tab_active(),
        )));
        for step in steps {
            let check = if step.checked { "[x]" } else { "[ ]" };
            let status = if step.done { " ✓" } else { "" };
            lines.push(Line::from(vec![
                Span::styled(format!("   {check} "), Style::new().fg(theme::PRIMARY)),
                Span::styled(step.install.clone(), Style::new().fg(theme::TEXT)),
                Span::styled(status, Style::new().fg(theme::SUCCESS)),
            ]));
        }
        lines.push(Line::from(Span::raw("")));
    }

    // Error
    if let Some(e) = error {
        lines.push(Line::from(Span::styled(format!(" ✗ {e}"), Style::new().fg(theme::ERROR))));
        lines.push(Line::from(Span::raw("")));
    }

    if all_done {
        lines.push(Line::from(Span::styled(
            " ✓ All steps complete! Restore proceeding.",
            Style::new().fg(theme::SUCCESS),
        )));
    }

    // Shortcut legend
    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(Span::styled(
        " Space:toggle all  a:select all  Enter:run & restore  s:skip  Esc:cancel ",
        theme::text_hint(),
    )));

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: true }),
        area,
    );
}
