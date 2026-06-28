use ratatui::style::{Color, Modifier, Style};

// ── Palette ──────────────────────────────────────────────────────────────
// Derived from the omah banner's blue sweep: (0,100,160) → (0,235,255)

pub const BG: Color = Color::Rgb(10, 22, 40);
pub const SURFACE: Color = Color::Rgb(20, 36, 60);
pub const SURFACE_LIGHT: Color = Color::Rgb(30, 50, 75);
pub const SURFACE_SELECTED: Color = Color::Rgb(0, 50, 90);

pub const PRIMARY: Color = Color::Rgb(0, 160, 215);
pub const PRIMARY_DIM: Color = Color::Rgb(0, 100, 160);
pub const PRIMARY_BRIGHT: Color = Color::Rgb(0, 215, 248);

pub const ACCENT: Color = Color::Rgb(245, 166, 35);
pub const SUCCESS: Color = Color::Rgb(80, 200, 120);
pub const ERROR: Color = Color::Rgb(255, 85, 85);
pub const WARNING: Color = Color::Rgb(245, 166, 35);

pub const DIM: Color = Color::Rgb(90, 106, 122);
pub const TEXT: Color = Color::Rgb(208, 216, 224);
pub const TEXT_DIM: Color = Color::Rgb(140, 155, 170);
pub const TEXT_HINT: Color = Color::Rgb(70, 90, 110);

// ── Style constructors ───────────────────────────────────────────────────

pub fn title() -> Style {
    Style::new().fg(PRIMARY_BRIGHT).add_modifier(Modifier::BOLD)
}

pub fn header() -> Style {
    Style::new().fg(TEXT).bg(SURFACE)
}

pub fn tab_active() -> Style {
    Style::new()
        .fg(PRIMARY_BRIGHT)
        .bg(SURFACE_LIGHT)
        .add_modifier(Modifier::BOLD)
}

pub fn tab_inactive() -> Style {
    Style::new().fg(TEXT_DIM).bg(BG)
}

pub fn border() -> Style {
    Style::new().fg(PRIMARY_DIM)
}

pub fn border_focused() -> Style {
    Style::new().fg(PRIMARY_BRIGHT)
}

pub fn success() -> Style {
    Style::new().fg(SUCCESS)
}

pub fn error() -> Style {
    Style::new().fg(ERROR)
}

pub fn warning() -> Style {
    Style::new().fg(WARNING)
}

pub fn dim() -> Style {
    Style::new().fg(DIM)
}

pub fn text_hint() -> Style {
    Style::new().fg(TEXT_HINT)
}
