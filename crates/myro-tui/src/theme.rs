use ratatui::style::{Color, Modifier, Style};

// Palette
pub const ACCENT: Color = Color::Rgb(100, 200, 200); // soft teal
pub const ACCENT_DIM: Color = Color::Rgb(60, 130, 130); // muted teal
pub const DIM: Color = Color::DarkGray;
pub const TEXT: Color = Color::White;
pub const MUTED: Color = Color::Gray;
pub const SUCCESS: Color = Color::Rgb(120, 200, 120);
pub const FAIL: Color = Color::Rgb(220, 100, 100);
pub const WARN: Color = Color::Rgb(220, 180, 80);
pub const PURPLE: Color = Color::Rgb(160, 130, 220);

pub fn accent_style() -> Style {
    Style::default().fg(ACCENT)
}

pub fn accent_bold() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub fn accent_dim_style() -> Style {
    Style::default().fg(ACCENT_DIM)
}

pub fn dim_style() -> Style {
    Style::default().fg(DIM)
}

pub fn muted_style() -> Style {
    Style::default().fg(MUTED)
}

pub fn bold_style() -> Style {
    Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
}

pub fn success_style() -> Style {
    Style::default().fg(SUCCESS)
}

pub fn fail_style() -> Style {
    Style::default().fg(FAIL)
}

pub fn warn_style() -> Style {
    Style::default().fg(WARN)
}

pub fn purple_style() -> Style {
    Style::default().fg(PURPLE)
}

// Math rendering palette
pub const MATH: Color = Color::Rgb(125, 207, 255); // light cyan for inline math

pub fn math_style() -> Style {
    Style::default().fg(MATH)
}

pub fn newline_style() -> Style {
    Style::default().fg(MATH)
}

// Coach / ghost text palette
pub const GHOST: Color = Color::Rgb(90, 90, 110);
pub const GHOST_CODE: Color = Color::Rgb(80, 140, 140);

pub fn ghost_code_style() -> Style {
    Style::default()
        .fg(GHOST_CODE)
        .add_modifier(Modifier::ITALIC)
}
