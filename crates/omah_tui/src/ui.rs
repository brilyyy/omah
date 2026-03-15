use std::time::Instant;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, TableState},
};

use crate::app::{App, ConfirmKind, Screen};

const ART: &[&str] = &[
    " ██████╗ ███╗   ███╗  █████╗  ██╗  ██╗",
    "██╔═══██╗████╗ ████║ ██╔══██╗ ██║  ██║",
    "██║   ██║██╔████╔██║ ███████║ ███████║",
    "██║   ██║██║╚██╔╝██║ ██╔══██║ ██╔══██║",
    "╚██████╔╝██║ ╚═╝ ██║ ██║  ██║ ██║  ██║",
    " ╚═════╝ ╚═╝     ╚═╝ ╚═╝  ╚═╝ ╚═╝  ╚═╝",
];
const ART_COLORS: &[(u8, u8, u8)] = &[
    (0, 100, 160),
    (0, 130, 190),
    (0, 160, 215),
    (0, 190, 235),
    (0, 215, 248),
    (0, 235, 255),
];

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();
    match &app.screen {
        Screen::Splash(start) => render_splash(f, start, area),
        Screen::Edit => render_edit(f, app, area),
        Screen::Settings => render_settings(f, app, area),
        _ => {
            render_list(f, app, area);
            match &app.screen {
                Screen::AddForm => render_add_form(f, app, area),
                Screen::Confirm(kind) => render_confirm(f, app, *kind, area),
                _ => {}
            }
        }
    }
}

// ── Splash screen ──────────────────────────────────────────────────────────

fn render_splash(f: &mut Frame, start: &Instant, area: Rect) {
    let elapsed = start.elapsed().as_millis();
    // reveal one line every ~250 ms
    let lines_revealed = ((elapsed / 250) as usize + 1).min(ART.len());

    let art_height = ART.len() as u16 + 3; // art + subtitle + blank
    let top_pad = area.height.saturating_sub(art_height) / 2;
    let art_area = Rect::new(area.x, area.y + top_pad, area.width, art_height.min(area.height));

    let constraints: Vec<Constraint> = (0..ART.len())
        .map(|_| Constraint::Length(1))
        .chain([Constraint::Length(1), Constraint::Length(1)])
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(art_area);

    for (i, line) in ART.iter().enumerate() {
        let (r, g, b) = ART_COLORS[i];
        let style = if i < lines_revealed {
            Style::default().fg(Color::Rgb(r, g, b)).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(30, 50, 65))
        };
        let x_pad = area.width.saturating_sub(line.len() as u16) / 2;
        let line_area = Rect::new(area.x + x_pad, chunks[i].y, chunks[i].width, 1);
        f.render_widget(Paragraph::new(*line).style(style), line_area);
    }

    if lines_revealed >= ART.len() {
        let subtitle = "panggonan kanggo nyimpen backup";
        let x_pad = area.width.saturating_sub(subtitle.len() as u16) / 2;
        let sub_area = Rect::new(area.x + x_pad, chunks[ART.len()].y, chunks[ART.len()].width, 1);
        f.render_widget(
            Paragraph::new(subtitle).style(Style::default().fg(Color::DarkGray)),
            sub_area,
        );
        let hint = "press any key to continue";
        let hx = area.width.saturating_sub(hint.len() as u16) / 2;
        let hint_area = Rect::new(area.x + hx, chunks[ART.len() + 1].y, chunks[ART.len() + 1].width, 1);
        f.render_widget(
            Paragraph::new(hint).style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM)),
            hint_area,
        );
    }
}

// ── List screen ────────────────────────────────────────────────────────────

fn render_list(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Paragraph::new(format!("  vault: {}", app.config.vault_path))
        .block(Block::default().borders(Borders::ALL).title(" omah "))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(title, chunks[0]);

    let header = Row::new(vec![
        Cell::from("Name").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Source").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Status").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Flags").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .style(Style::default().bg(Color::DarkGray))
    .height(1);

    let rows: Vec<Row> = app
        .statuses
        .iter()
        .map(|s| {
            let (status_text, status_color) = if !s.source_exists {
                ("source missing", Color::Red)
            } else if s.backed_up {
                ("backed up", Color::Green)
            } else {
                ("not backed up", Color::Yellow)
            };
            let flags = if s.symlinked { "symlink" } else { "" };
            Row::new(vec![
                Cell::from(s.name.clone()),
                Cell::from(s.source.clone()),
                Cell::from(status_text).style(Style::default().fg(status_color)),
                Cell::from(flags),
            ])
        })
        .collect();

    let mut table_state = TableState::default().with_selected(Some(app.selected));
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(20),
            Constraint::Percentage(45),
            Constraint::Percentage(20),
            Constraint::Percentage(15),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(" Dotfiles "))
    .row_highlight_style(Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD))
    .highlight_symbol("▶ ");

    f.render_stateful_widget(table, chunks[1], &mut table_state);

    if let Some((msg, is_error)) = &app.message {
        let color = if *is_error { Color::Red } else { Color::Green };
        f.render_widget(
            Paragraph::new(format!(" {msg}")).style(Style::default().fg(color)),
            chunks[2],
        );
    }

    f.render_widget(
        Paragraph::new(
            " j/k: navigate  e: edit  b: backup  B: backup all  r: restore  R: restore all  n: new  S: settings  q: quit",
        )
        .style(Style::default().fg(Color::DarkGray)),
        chunks[3],
    );
}

// ── Edit screen ────────────────────────────────────────────────────────────

fn render_edit(f: &mut Frame, app: &App, area: Rect) {
    let edit = &app.edit;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Length(10), // fields
            Constraint::Min(5),     // setup steps
            Constraint::Length(6),  // excludes
            Constraint::Length(1),  // help
        ])
        .split(area);

    // Header
    let dot_name = app.config.dots.get(edit.dot_idx).map(|d| d.name.as_str()).unwrap_or("?");
    let header = Paragraph::new(format!("  Editing: {dot_name}"))
        .block(Block::default().borders(Borders::ALL).title(" omah — edit "))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(header, chunks[0]);

    // ── Fields section ─────────────────────────────────────────────────────
    let field_block = Block::default()
        .borders(Borders::ALL)
        .title(" Fields ")
        .border_style(if edit.focus < 4 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });
    let fields_inner = field_block.inner(chunks[1]);
    f.render_widget(field_block, chunks[1]);

    let field_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(fields_inner.inner(Margin { horizontal: 1, vertical: 1 }));

    let active = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let inactive = Style::default();

    // Name
    let name_label = if edit.focus == 0 { "▶ Name   " } else { "  Name   " };
    f.render_widget(
        Paragraph::new(format!("{name_label} {}", edit.name))
            .style(if edit.focus == 0 { active } else { inactive }),
        field_rows[0],
    );
    // Source
    let src_label = if edit.focus == 1 { "▶ Source " } else { "  Source " };
    f.render_widget(
        Paragraph::new(format!("{src_label} {}", edit.source))
            .style(if edit.focus == 1 { active } else { inactive }),
        field_rows[1],
    );
    // Symlink
    let check = if edit.symlink { "[x]" } else { "[ ]" };
    let sym_label = if edit.focus == 2 { "▶ Symlink" } else { "  Symlink" };
    f.render_widget(
        Paragraph::new(format!("{sym_label} {check} replace source with symlink"))
            .style(if edit.focus == 2 { active } else { inactive }),
        field_rows[2],
    );
    // Deps
    let deps_label = if edit.focus == 3 { "▶ Deps   " } else { "  Deps   " };
    f.render_widget(
        Paragraph::new(format!("{deps_label} {}", edit.deps))
            .style(if edit.focus == 3 { active } else { inactive }),
        field_rows[3],
    );

    // Cursor in active text field
    let fi = fields_inner.inner(Margin { horizontal: 1, vertical: 1 });
    match edit.focus {
        0 => f.set_cursor_position((fi.x + 10 + edit.name.len() as u16, fi.y)),
        1 => f.set_cursor_position((fi.x + 10 + edit.source.len() as u16, fi.y + 1)),
        3 => f.set_cursor_position((fi.x + 10 + edit.deps.len() as u16, fi.y + 3)),
        _ => {}
    }

    // ── Setup steps section ────────────────────────────────────────────────
    let steps_block = Block::default()
        .borders(Borders::ALL)
        .title(" Setup Steps — [a] add  [d] delete ")
        .border_style(if edit.focus == 4 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });
    let steps_inner = steps_block.inner(chunks[2]);
    f.render_widget(steps_block, chunks[2]);

    if edit.steps.is_empty() {
        f.render_widget(
            Paragraph::new("  — no setup steps —").style(Style::default().fg(Color::DarkGray)),
            steps_inner.inner(Margin { horizontal: 1, vertical: 1 }),
        );
    } else {
        let items: Vec<ListItem> = edit
            .steps
            .iter()
            .map(|s| {
                let check_part = if s.check.trim().is_empty() {
                    "(no check)".to_string()
                } else {
                    format!("check: {}", s.check)
                };
                ListItem::new(format!("  {check_part}  →  {}", s.install))
            })
            .collect();

        let mut list_state = ListState::default().with_selected(Some(edit.step_sel));
        let list = List::new(items)
            .highlight_style(Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ ");
        f.render_stateful_widget(
            list,
            steps_inner.inner(Margin { horizontal: 1, vertical: 1 }),
            &mut list_state,
        );
    }

    // ── Excludes section ───────────────────────────────────────────────────
    let excl_block = Block::default()
        .borders(Borders::ALL)
        .title(" Exclude Patterns — [a] add  [d] delete ")
        .border_style(if edit.focus == 5 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });
    let excl_inner = excl_block.inner(chunks[3]);
    f.render_widget(excl_block, chunks[3]);

    if edit.excludes.is_empty() {
        f.render_widget(
            Paragraph::new("  — no exclude patterns —").style(Style::default().fg(Color::DarkGray)),
            excl_inner.inner(Margin { horizontal: 1, vertical: 1 }),
        );
    } else {
        let items: Vec<ListItem> = edit
            .excludes
            .iter()
            .map(|p| ListItem::new(format!("  {p}")))
            .collect();
        let mut list_state = ListState::default().with_selected(Some(edit.exclude_sel));
        let list = List::new(items)
            .highlight_style(Style::default().bg(Color::Blue).add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ ");
        f.render_stateful_widget(
            list,
            excl_inner.inner(Margin { horizontal: 1, vertical: 1 }),
            &mut list_state,
        );
    }

    // Help bar
    f.render_widget(
        Paragraph::new(" Tab: switch focus   j/k: navigate   s: save   Esc: cancel")
            .style(Style::default().fg(Color::DarkGray)),
        chunks[4],
    );

    // ── Add-step sub-form (modal overlay) ──────────────────────────────────
    if edit.step_form.is_some() {
        render_step_form(f, app, area);
    }

    // ── Add-exclude input (modal overlay) ──────────────────────────────────
    if edit.exclude_input.is_some() {
        render_exclude_input(f, app, area);
    }

    // Message
    if let Some((msg, is_error)) = &app.message {
        let color = if *is_error { Color::Red } else { Color::Green };
        let msg_area = Rect::new(area.x, area.y + area.height.saturating_sub(2), area.width, 1);
        f.render_widget(
            Paragraph::new(format!(" {msg}")).style(Style::default().fg(color)),
            msg_area,
        );
    }
}

fn render_step_form(f: &mut Frame, app: &App, area: Rect) {
    let form = match app.edit.step_form.as_ref() {
        Some(f) => f,
        None => return,
    };

    let popup_area = centered_rect(60, 9, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Add Setup Step ")
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let inner = inner.inner(Margin { horizontal: 1, vertical: 0 });
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(inner);

    let active = Style::default().fg(Color::Yellow);
    let inactive = Style::default();

    f.render_widget(
        Paragraph::new(form.check.as_str()).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Check path (optional — skip step if this path exists)")
                .style(if form.field == 0 { active } else { inactive }),
        ),
        rows[0],
    );
    f.render_widget(
        Paragraph::new(form.install.as_str()).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Install command")
                .style(if form.field == 1 { active } else { inactive }),
        ),
        rows[1],
    );
    f.render_widget(
        Paragraph::new("Tab/Enter: next field   Enter on last: add step   Esc: cancel")
            .style(Style::default().fg(Color::DarkGray)),
        rows[2],
    );

    if form.field == 0 {
        f.set_cursor_position((rows[0].x + 1 + form.check.len() as u16, rows[0].y + 1));
    } else {
        f.set_cursor_position((rows[1].x + 1 + form.install.len() as u16, rows[1].y + 1));
    }
}

fn render_exclude_input(f: &mut Frame, app: &App, area: Rect) {
    let buf = match app.edit.exclude_input.as_ref() {
        Some(b) => b,
        None => return,
    };

    let popup_area = centered_rect(55, 7, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Add Exclude Pattern ")
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let inner = inner.inner(Margin { horizontal: 1, vertical: 0 });
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    f.render_widget(
        Paragraph::new(buf.as_str()).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Pattern (e.g. *.log, .git)")
                .style(Style::default().fg(Color::Yellow)),
        ),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new("Enter: add   Esc: cancel").style(Style::default().fg(Color::DarkGray)),
        chunks[1],
    );

    f.set_cursor_position((chunks[0].x + 1 + buf.len() as u16, chunks[0].y + 1));
}

// ── Add-dotfile modal ──────────────────────────────────────────────────────

fn render_add_form(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(60, 13, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Add Dotfile ")
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let inner = inner.inner(Margin { horizontal: 1, vertical: 0 });
    let form = &app.form;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(inner);

    let active = Style::default().fg(Color::Yellow);
    let inactive = Style::default();

    f.render_widget(
        Paragraph::new(form.name.as_str()).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Name")
                .style(if form.field == 0 { active } else { inactive }),
        ),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(form.source.as_str()).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Source path")
                .style(if form.field == 1 { active } else { inactive }),
        ),
        chunks[1],
    );

    let check = if form.symlink { "[x]" } else { "[ ]" };
    f.render_widget(
        Paragraph::new(format!("{check} Replace source with symlink"))
            .style(if form.field == 2 { active } else { inactive }),
        chunks[2],
    );
    f.render_widget(
        Paragraph::new("Tab: next   Space: toggle symlink   Enter: save   Esc: cancel")
            .style(Style::default().fg(Color::DarkGray)),
        chunks[3],
    );

    if form.field == 0 {
        f.set_cursor_position((chunks[0].x + 1 + form.name.len() as u16, chunks[0].y + 1));
    } else if form.field == 1 {
        f.set_cursor_position((chunks[1].x + 1 + form.source.len() as u16, chunks[1].y + 1));
    }
}

// ── Confirm modal ──────────────────────────────────────────────────────────

fn render_confirm(f: &mut Frame, app: &App, kind: ConfirmKind, area: Rect) {
    let dot_name = app.statuses.get(app.selected).map(|s| s.name.as_str()).unwrap_or("?");
    let msg = match kind {
        ConfirmKind::RestoreOne => {
            format!("Restore '{dot_name}'? This will overwrite the source file.")
        }
        ConfirmKind::RestoreAll => {
            "Restore all dotfiles? This will overwrite all source files.".to_string()
        }
    };

    let popup_area = centered_rect(55, 7, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Confirm Restore ")
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let inner = inner.inner(Margin { horizontal: 1, vertical: 0 });
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(2), Constraint::Length(1)])
        .split(inner);

    f.render_widget(
        Paragraph::new(msg.as_str()).style(Style::default().fg(Color::Yellow)),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new("y / Enter: confirm   n / Esc: cancel")
            .style(Style::default().fg(Color::DarkGray)),
        chunks[1],
    );
}

// ── Settings screen ────────────────────────────────────────────────────────

fn render_settings(f: &mut Frame, app: &App, area: Rect) {
    let s = &app.settings;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Length(12), // fields
            Constraint::Min(0),
            Constraint::Length(1),  // help
        ])
        .split(area);

    let header = Paragraph::new("  Global Settings")
        .block(Block::default().borders(Borders::ALL).title(" omah — settings "))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(header, chunks[0]);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Configuration ");
    let inner = block.inner(chunks[1]);
    f.render_widget(block, chunks[1]);

    let inner = inner.inner(Margin { horizontal: 1, vertical: 1 });
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

    let active = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let hint_style = Style::default().fg(Color::DarkGray);

    // OS field
    let os_label = if s.focus == 0 { "▶ OS             " } else { "  OS             " };
    f.render_widget(
        Paragraph::new(format!("{os_label} {}", s.os))
            .style(if s.focus == 0 { active } else { Style::default() }),
        rows[0],
    );
    f.render_widget(
        Paragraph::new("                   values: auto | macos | linux").style(hint_style),
        rows[1],
    );

    // Package manager field
    let pm_label = if s.focus == 1 { "▶ Package Manager" } else { "  Package Manager" };
    f.render_widget(
        Paragraph::new(format!("{pm_label} {}", s.pkg_manager))
            .style(if s.focus == 1 { active } else { Style::default() }),
        rows[2],
    );
    f.render_widget(
        Paragraph::new("                   values: auto | brew | apt-get | pacman | dnf | zypper")
            .style(hint_style),
        rows[3],
    );

    // Cursor
    if s.focus == 0 {
        f.set_cursor_position((inner.x + 18 + s.os.len() as u16, inner.y));
    } else {
        f.set_cursor_position((inner.x + 18 + s.pkg_manager.len() as u16, inner.y + 2));
    }

    f.render_widget(
        Paragraph::new(" Tab: switch field   s: save   Esc: cancel")
            .style(Style::default().fg(Color::DarkGray)),
        chunks[3],
    );

    // Message
    if let Some((msg, is_error)) = &app.message {
        let color = if *is_error { Color::Red } else { Color::Green };
        let msg_area = Rect::new(area.x, area.y + area.height.saturating_sub(2), area.width, 1);
        f.render_widget(
            Paragraph::new(format!(" {msg}")).style(Style::default().fg(color)),
            msg_area,
        );
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let w = (area.width as u32 * percent_x as u32 / 100).min(area.width as u32) as u16;
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, w, height.min(area.height))
}
