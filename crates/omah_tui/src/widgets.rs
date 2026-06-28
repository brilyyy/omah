use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

use crate::theme;

// ── Modal overlay ────────────────────────────────────────────────────────

/// Render a centered modal on top of the main UI.
/// `width_pct` and `height_pct` control the modal size relative to terminal.
pub fn draw_modal<F>(frame: &mut Frame, area: Rect, title: &str, width_pct: u16, height_pct: u16, content: F)
where
    F: FnOnce(&mut Frame, Rect),
{
    let w = (area.width * width_pct).max(40).min(area.width.saturating_sub(4));
    let h = (area.height * height_pct).max(6).min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(w)) / 2;
    let y = (area.height.saturating_sub(h)) / 2;
    let modal_area = Rect { x, y, width: w, height: h };

    // Dim background
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(Text::raw(""))
            .style(Style::new().bg(theme::BG).fg(theme::TEXT)),
        area,
    );

    // Modal border
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border_focused())
        .title(format!(" {title} "))
        .title_alignment(Alignment::Center)
        .style(Style::new().bg(theme::SURFACE));
    frame.render_widget(Clear, modal_area);
    frame.render_widget(&block, modal_area);

    let inner = block.inner(modal_area);
    content(frame, inner);
}

// ── Confirm dialog ───────────────────────────────────────────────────────

pub fn draw_confirm(frame: &mut Frame, area: Rect, message: &str) {
    let lines: Vec<Line> = vec![
        Line::from(Span::styled(message, Style::new().fg(theme::TEXT))),
        Line::from(Span::raw("")),
        Line::from(Span::styled("  y: Yes   n: No   Esc: Cancel", theme::text_hint())),
    ];
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: true }),
        area,
    );
}

// ── Error dialog ─────────────────────────────────────────────────────────

pub fn draw_error(frame: &mut Frame, area: Rect, message: &str) {
    let lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled(" ✗ ", Style::new().fg(theme::ERROR).bold()),
            Span::styled("Error", Style::new().fg(theme::ERROR).bold()),
        ]),
        Line::from(Span::raw("")),
        Line::from(Span::styled(message, theme::text_hint())),
        Line::from(Span::raw("")),
        Line::from(Span::styled("  Press any key to dismiss.", theme::text_hint())),
    ];
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: true }),
        area,
    );
}

// ── Form widgets ─────────────────────────────────────────────────────────

/// Draw a text input field with label and cursor.
pub fn draw_text_input(frame: &mut Frame, area: Rect, label: &str, value: &str, cursor: usize, focused: bool) {
    let bg = if focused { theme::SURFACE_LIGHT } else { theme::SURFACE };
    let fg = if focused { theme::PRIMARY_BRIGHT } else { theme::TEXT };

    let input_line = if focused && !value.is_empty() {
        let cursor_pos = cursor.min(value.len());
        let before = &value[..cursor_pos];
        let at = value.chars().nth(cursor_pos).map(|c| c.to_string()).unwrap_or_default();
        let after = if cursor_pos < value.len() {
            &value[cursor_pos + 1..]
        } else {
            ""
        };
        Line::from(vec![
            Span::styled(format!(" {label} "), Style::new().fg(theme::TEXT_DIM)),
            Span::styled(before, Style::new().fg(theme::TEXT)),
            Span::styled(at, Style::new().fg(theme::PRIMARY_BRIGHT).bg(theme::SURFACE_SELECTED)),
            Span::styled(after, Style::new().fg(theme::TEXT)),
        ])
    } else {
        let display = if value.is_empty() {
            Span::styled("(empty)", theme::dim())
        } else {
            Span::styled(value.to_string(), Style::new().fg(fg))
        };
        Line::from(vec![
            Span::styled(format!(" {label} "), Style::new().fg(theme::TEXT_DIM)),
            display,
        ])
    };

    frame.render_widget(
        Paragraph::new(Text::from(vec![input_line]))
            .style(Style::new().bg(bg).fg(fg)),
        area,
    );
}

/// Draw a boolean toggle (Yes/No).
pub fn draw_toggle(frame: &mut Frame, area: Rect, label: &str, value: bool, focused: bool) {
    let bg = if focused { theme::SURFACE_LIGHT } else { theme::SURFACE };
    let yes_style = if focused && value {
        Style::new().fg(theme::SUCCESS).bold()
    } else if value {
        Style::new().fg(theme::SUCCESS)
    } else {
        theme::dim()
    };
    let no_style = if focused && !value {
        Style::new().fg(theme::ERROR).bold()
    } else if !value {
        Style::new().fg(theme::ERROR)
    } else {
        theme::dim()
    };

    let text = Text::from(Line::from(vec![
        Span::styled(format!(" {label} "), Style::new().fg(theme::TEXT_DIM)),
        Span::styled(if value { " ● Yes " } else { " ○ Yes " }, yes_style),
        Span::styled(" / ", theme::dim()),
        Span::styled(if !value { " ● No " } else { " ○ No " }, no_style),
    ]));
    frame.render_widget(
        Paragraph::new(text).style(Style::new().bg(bg)),
        area,
    );
}

// ── Progress bar ─────────────────────────────────────────────────────────

pub fn draw_progress(frame: &mut Frame, area: Rect, current: u64, total: u64, label: &str) {
    let pct = if total > 0 { (current as f64 / total as f64).min(1.0) } else { 0.0 };
    let filled_width = (area.width as f64 * pct) as u16;
    let filled = "▓".repeat(filled_width as usize);
    let empty = "░".repeat((area.width.saturating_sub(filled_width)) as usize);
    let pct_text = format!(" {:.0}% ", pct * 100.0);

    let text = Text::from(vec![
        Line::from(vec![
            Span::styled(filled, Style::new().fg(theme::PRIMARY)),
            Span::styled(empty, theme::dim()),
            Span::raw(" "),
            Span::styled(pct_text, Style::new().fg(theme::PRIMARY_BRIGHT)),
        ]),
        Line::from(Span::styled(label, theme::text_hint())),
    ]);
    frame.render_widget(Paragraph::new(text), area);
}

// ── Badge helpers ────────────────────────────────────────────────────────

pub fn badge_installed() -> Span<'static> {
    Span::styled(" ●", Style::new().fg(theme::SUCCESS))
}

pub fn badge_missing() -> Span<'static> {
    Span::styled(" ○", Style::new().fg(theme::ERROR))
}

pub fn badge_done() -> Span<'static> {
    Span::styled(" ✓", Style::new().fg(theme::SUCCESS))
}

pub fn badge_pending() -> Span<'static> {
    Span::styled(" ○", Style::new().fg(theme::WARNING))
}

pub fn badge_running() -> Span<'static> {
    Span::styled(" ◌", Style::new().fg(theme::PRIMARY_BRIGHT))
}

pub fn badge_failed() -> Span<'static> {
    Span::styled(" ✗", Style::new().fg(theme::ERROR))
}

// ── Check selector popup ─────────────────────────────────────────────────

/// Draw the check-type selector as a List-style popup.
/// `items` is a slice of (type_key, label, description).
pub fn draw_check_selector(
    frame: &mut Frame,
    area: Rect,
    items: &[(&str, &str, &str)],
    selected: usize,
) {
    let mut lines: Vec<Line> = vec![Line::from(Span::styled(
        " Check Type ",
        Style::new().fg(theme::PRIMARY_DIM),
    ))];

    for (i, (ct, label, _desc)) in items.iter().enumerate() {
        let sel = if i == selected { "▸" } else { " " };
        let style = if i == selected {
            Style::new().fg(theme::PRIMARY_BRIGHT).bold()
        } else {
            Style::new().fg(theme::TEXT)
        };
        lines.push(Line::from(vec![
            Span::raw(" "),
            Span::styled(format!("{sel} {ct}"), style),
            Span::raw("  "),
            Span::styled(*label, theme::text_hint()),
        ]));
    }

    lines.push(Line::from(Span::styled(
        " ↑↓:nav  Enter:select  Esc:cancel ",
        theme::text_hint(),
    )));

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: true }),
        area,
    );
}

// ── Help overlay ─────────────────────────────────────────────────────────

pub fn draw_help_overlay(frame: &mut Frame, area: Rect, ctx: crate::app::HelpContext) {
    let (title, help_lines) = match ctx {
        crate::app::HelpContext::Dots => (
            " Dotfiles View ",
            vec![
                Line::from(Span::styled("  Navigation", theme::tab_active())),
                Line::from(Span::raw("")),
                Line::from(Span::styled("  ↑/↓ or j/k   — select dotfile", theme::text_hint())),
                Line::from(Span::styled("  Enter         — expand/collapse detail", theme::text_hint())),
                Line::from(Span::styled("  /             — focus search bar", theme::text_hint())),
                Line::from(Span::raw("")),
                Line::from(Span::styled("  Actions", theme::tab_active())),
                Line::from(Span::raw("")),
                Line::from(Span::styled("  a             — add new dotfile", theme::text_hint())),
                Line::from(Span::styled("  e             — edit selected", theme::text_hint())),
                Line::from(Span::styled("  x             — remove selected", theme::text_hint())),
                Line::from(Span::styled("  b             — backup selected", theme::text_hint())),
                Line::from(Span::styled("  r             — restore selected", theme::text_hint())),
                Line::from(Span::styled("  B             — backup all", theme::text_hint())),
                Line::from(Span::styled("  R             — restore all", theme::text_hint())),
                Line::from(Span::styled("  S             — open settings", theme::text_hint())),
                Line::from(Span::raw("")),
                Line::from(Span::styled("  Detail (expanded)", theme::tab_active())),
                Line::from(Span::raw("")),
                Line::from(Span::styled("  i             — install deps", theme::text_hint())),
                Line::from(Span::styled("  r             — run pending setup", theme::text_hint())),
                Line::from(Span::styled("  s             — skip setup step", theme::text_hint())),
                Line::from(Span::styled("  Enter/Esc     — close detail", theme::text_hint())),
                Line::from(Span::raw("")),
                Line::from(Span::styled("  Global", theme::tab_active())),
                Line::from(Span::raw("")),
                Line::from(Span::styled("  1-2/Tab       — switch tabs", theme::text_hint())),
                Line::from(Span::styled("  ?             — this help", theme::text_hint())),
                Line::from(Span::styled("  q/Esc         — quit", theme::text_hint())),
            ],
        ),
        crate::app::HelpContext::Log => (
            " Log View ",
            vec![
                Line::from(Span::styled("  ↑/↓ or j/k   — scroll log", theme::text_hint())),
                Line::from(Span::raw("")),
                Line::from(Span::styled("  1-2/Tab       — switch tabs", theme::text_hint())),
                Line::from(Span::styled("  ?             — this help", theme::text_hint())),
                Line::from(Span::styled("  q/Esc         — quit", theme::text_hint())),
            ],
        ),
        crate::app::HelpContext::Form => (
            " Form Help ",
            vec![
                Line::from(Span::styled("  Tab/Shift+Tab — navigate fields", theme::text_hint())),
                Line::from(Span::styled("  Enter         — save form", theme::text_hint())),
                Line::from(Span::styled("  Esc           — cancel", theme::text_hint())),
                Line::from(Span::styled("  Ctrl+d        — delete setup step", theme::text_hint())),
                Line::from(Span::raw("")),
                Line::from(Span::styled("  Setup Steps:", theme::tab_active())),
                Line::from(Span::raw("")),
                Line::from(Span::styled("  Enter on check field — open type selector", theme::text_hint())),
                Line::from(Span::styled("  ↑/↓ in check field  — select type", theme::text_hint())),
                Line::from(Span::styled("  Enter in selector   — confirm type", theme::text_hint())),
            ],
        ),
        crate::app::HelpContext::Detail => (
            " Detail View ",
            vec![
                Line::from(Span::styled("  Enter/Esc     — close detail", theme::text_hint())),
                Line::from(Span::styled("  i             — install missing deps", theme::text_hint())),
                Line::from(Span::styled("  r             — run pending setup steps", theme::text_hint())),
                Line::from(Span::styled("  s             — skip pending setup step", theme::text_hint())),
                Line::from(Span::styled("  ?             — this help", theme::text_hint())),
            ],
        ),
        crate::app::HelpContext::Settings => (
            " Settings ",
            vec![
                Line::from(Span::styled("  Tab/Shift+Tab — navigate fields", theme::text_hint())),
                Line::from(Span::styled("  ↑/↓           — change selector value", theme::text_hint())),
                Line::from(Span::styled("  Enter         — save & close", theme::text_hint())),
                Line::from(Span::styled("  Esc           — cancel", theme::text_hint())),
            ],
        ),
        crate::app::HelpContext::CheckSelector => (
            " Check Type Selector ",
            vec![
                Line::from(Span::styled("  ↑/↓           — navigate types", theme::text_hint())),
                Line::from(Span::styled("  Enter         — select type", theme::text_hint())),
                Line::from(Span::styled("  Esc           — cancel", theme::text_hint())),
            ],
        ),
    };

    let lines: Vec<Line> = std::iter::once(Line::from(Span::styled(
        format!(" {title}"),
        Style::new().fg(theme::PRIMARY_BRIGHT).bold(),
    )))
    .chain(std::iter::once(Line::from(Span::raw(""))))
    .chain(help_lines)
    .collect();

    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: true }),
        area,
    );
}

// ── Status badge ─────────────────────────────────────────────────────────

pub fn draw_status_badge(s: &omah_lib::ops::DotStatus) -> Span<'static> {
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

// ── Dep chip ─────────────────────────────────────────────────────────────

pub fn draw_dep_chip(name: &str, installed: bool) -> Span<'static> {
    if installed {
        Span::styled(
            format!(" ●{name} "),
            Style::new().fg(theme::SUCCESS).bg(theme::SURFACE_LIGHT),
        )
    } else {
        Span::styled(
            format!(" ○{name} "),
            Style::new().fg(theme::ERROR).bg(theme::SURFACE_LIGHT),
        )
    }
}

// ── Terminal output panel ────────────────────────────────────────────────

pub fn draw_terminal_panel(frame: &mut Frame, area: Rect, lines: &[String], running: bool) {
    let title = if running {
        " Running… "
    } else {
        " Output "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border())
        .title(title);

    let inner = block.inner(area);
    frame.render_widget(&block, area);

    let text_lines: Vec<Line> = lines
        .iter()
        .map(|l| Line::from(Span::styled(l.clone(), Style::new().fg(theme::TEXT))))
        .collect();

    if text_lines.is_empty() {
        frame.render_widget(
            Paragraph::new(Text::from(Line::from(Span::styled(
                " Waiting for output…",
                theme::dim(),
            ))))
            .block(block),
            area,
        );
    } else {
        frame.render_widget(
            Paragraph::new(Text::from(text_lines))
                .wrap(Wrap { trim: true }),
            inner,
        );
    }
}
