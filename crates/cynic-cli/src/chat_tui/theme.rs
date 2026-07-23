//! GrokNight-class palette (presentation only).

use ratatui::style::Color;

pub(super) struct Theme;
impl Theme {
    pub(super) const FG: Color = Color::White;
    pub(super) const MUTED: Color = Color::DarkGray;
    pub(super) const ACCENT: Color = Color::Magenta;
    pub(super) const ACCENT_DIM: Color = Color::LightMagenta;
    pub(super) const USER: Color = Color::Green;
    pub(super) const ASSISTANT: Color = Color::Cyan;
    pub(super) const TOOL: Color = Color::Yellow;
    pub(super) const ERROR: Color = Color::Red;
    pub(super) const BORDER: Color = Color::Rgb(60, 60, 70);
    pub(super) const PROMPT_BORDER: Color = Color::Magenta;
    pub(super) const HEADER_BG: Color = Color::Rgb(40, 30, 50);
}
