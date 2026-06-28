use ratatui::style::{Color, Modifier, Style};

// ── Catppuccin Mocha ─────────────────────────────────────────────────────
// https://catppuccin.com/palette

pub const BG: Color = Color::Rgb(30, 30, 46); // Base
pub const SURFACE: Color = Color::Rgb(24, 24, 37); // Mantle
pub const SURFACE_LIGHT: Color = Color::Rgb(49, 50, 68); // Surface0
pub const SURFACE_SELECTED: Color = Color::Rgb(69, 71, 90); // Surface1

pub const PRIMARY: Color = Color::Rgb(137, 180, 250); // Blue
pub const PRIMARY_DIM: Color = Color::Rgb(108, 112, 134); // Overlay0
pub const PRIMARY_BRIGHT: Color = Color::Rgb(180, 190, 254); // Lavender

pub const ACCENT: Color = Color::Rgb(250, 179, 135); // Peach
pub const SUCCESS: Color = Color::Rgb(166, 227, 161); // Green
pub const ERROR: Color = Color::Rgb(243, 139, 168); // Red
pub const WARNING: Color = Color::Rgb(249, 226, 175); // Yellow
pub const MAUVE: Color = Color::Rgb(203, 166, 247); // Mauve
pub const SKY: Color = Color::Rgb(137, 220, 235); // Sky

pub const DIM: Color = Color::Rgb(108, 112, 134); // Overlay0
pub const TEXT: Color = Color::Rgb(205, 214, 244); // Text
pub const TEXT_DIM: Color = Color::Rgb(166, 173, 200); // Subtext0
pub const TEXT_HINT: Color = Color::Rgb(127, 132, 156); // Overlay1

// ── Style constructors ───────────────────────────────────────────────────

pub fn title() -> Style {
    Style::new().fg(LAVENDER).add_modifier(Modifier::BOLD)
}

pub const LAVENDER: Color = PRIMARY_BRIGHT;

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
    Style::new().fg(TEXT_DIM).bg(SURFACE)
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
