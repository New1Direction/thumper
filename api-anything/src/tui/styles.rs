//! Warm industrial palette (amber/copper) for the TUI per AGENTS.md.
//! Theme tokens centralized here. Used by app.rs render.

use ratatui::style::{Color, Modifier, Style};

/// Core palette - industrial but warm (evokes copper tools, amber warning lights, old terminals)
pub const AMBER: Color = Color::Rgb(0xff, 0xbf, 0x00);
pub const COPPER: Color = Color::Rgb(0xb8, 0x73, 0x4f);
#[allow(dead_code)]
pub const WARM_BG: Color = Color::Rgb(0x1f, 0x1a, 0x17);
pub const LIGHT_WARM: Color = Color::Rgb(0xf5, 0xe8, 0xc7);
pub const MUTED_WARM: Color = Color::Rgb(0x8a, 0x7a, 0x6a);

pub fn header_style() -> Style {
    Style::default().fg(AMBER).add_modifier(Modifier::BOLD)
}

pub fn border_style() -> Style {
    Style::default().fg(COPPER)
}

#[allow(dead_code)]
pub fn title_border_style() -> Style {
    Style::default().fg(AMBER).bg(WARM_BG)
}

pub fn running_style() -> Style {
    Style::default().fg(AMBER).add_modifier(Modifier::BOLD)
}

pub fn success_style() -> Style {
    Style::default().fg(Color::Green)
}

pub fn error_style() -> Style {
    Style::default()
        .fg(Color::LightRed)
        .add_modifier(Modifier::BOLD)
}

pub fn list_highlight_style() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(AMBER)
        .add_modifier(Modifier::BOLD)
}

pub fn status_style() -> Style {
    Style::default().fg(LIGHT_WARM)
}

pub fn muted_style() -> Style {
    Style::default().fg(MUTED_WARM)
}

/// Mauve accent for BunBunny branding elements (Phase 3 atmospheric header)
pub fn bunbunny_mauve() -> Style {
    Style::default()
        .fg(MOCHA_MAUVE)
        .add_modifier(Modifier::BOLD)
}

/// Subtle surface background for toasts and ribbons
pub fn surface_style() -> Style {
    Style::default().fg(MOCHA_SURFACE0)
}

pub fn preview_style() -> Style {
    Style::default().fg(LIGHT_WARM)
}

// === Bun Command Palette (sharp, telemetry-dense, Grok Build CLI inspired) ===
pub const DARK_CHARCOAL: Color = Color::Rgb(0x1a, 0x1a, 0x1f);
pub const BRIGHT_CYAN: Color = Color::Rgb(0x00, 0xe5, 0xff);
pub const MUTED_SILVER: Color = Color::Rgb(0xa0, 0xa0, 0xa8);
/// Muted red for transient palette error messages (high-visibility but not screaming).
pub const MUTED_RED: Color = Color::Rgb(0xa8, 0x60, 0x60);

// === Catppuccin Mocha accents for Atmospheric Polish (Phase 3) ===
// Used for BunBunny logo/banner in header and toast backgrounds
pub const MOCHA_MAUVE: Color = Color::Rgb(0xcb, 0xa6, 0xf7);
pub const MOCHA_SURFACE0: Color = Color::Rgb(0x31, 0x32, 0x44);
pub const MOCHA_SURFACE1: Color = Color::Rgb(0x45, 0x47, 0x5a);

pub fn bun_palette_bg() -> Style {
    Style::default().bg(DARK_CHARCOAL)
}

pub fn bun_prompt_style() -> Style {
    Style::default()
        .fg(BRIGHT_CYAN)
        .add_modifier(Modifier::BOLD)
}

pub fn bun_help_style() -> Style {
    Style::default().fg(MUTED_SILVER)
}
