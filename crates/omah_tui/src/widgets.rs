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
    let style = if focused {
        Style::new().fg(theme::PRIMARY_BRIGHT).bg(theme::SURFACE_LIGHT)
    } else {
        Style::new().fg(theme::TEXT).bg(theme::SURFACE)
    };
    let border_style = if focused {
        theme::border_focused()
    } else {
        theme::border()
    };

    let _prefix = Span::styled(format!(" {label}: "), Style::new().fg(theme::TEXT_DIM));
    let _display = if value.is_empty() {
        Span::styled("(empty)", theme::dim())
    } else {
        Span::styled(value.to_string(), style)
    };

    // Label on top border using title, value inside
    let inner = if focused {
        // Show cursor position indicator
        let cursor_pos = cursor.min(value.len());
        let before = &value[..cursor_pos];
        let at = value.chars().nth(cursor_pos).map(|c| c.to_string()).unwrap_or_default();
        let after = if cursor_pos < value.len() {
            &value[cursor_pos + 1..]
        } else {
            ""
        };
        Line::from(vec![
            Span::styled(before, Style::new().fg(theme::TEXT)),
            Span::styled(at, Style::new().fg(theme::PRIMARY_BRIGHT).bg(theme::SURFACE_SELECTED)),
            Span::styled(after, Style::new().fg(theme::TEXT)),
        ])
    } else {
        Line::from(Span::styled(value, Style::new().fg(theme::TEXT)))
    };

    frame.render_widget(
        Paragraph::new(Text::from(vec![inner]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(border_style)
                    .title(format!(" {label} ")),
            ),
        area,
    );
}

/// Draw a boolean toggle (Yes/No).
pub fn draw_toggle(frame: &mut Frame, area: Rect, label: &str, value: bool, focused: bool) {
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

    let border_style = if focused { theme::border_focused() } else { theme::border() };
    let text = Text::from(Line::from(vec![
        Span::styled(format!(" {label}: "), Style::new().fg(theme::TEXT_DIM)),
        Span::styled(if value { " ● Yes " } else { " ○ Yes " }, yes_style),
        Span::raw("   "),
        Span::styled(if !value { " ● No " } else { " ○ No " }, no_style),
    ]));
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(border_style),
            ),
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
