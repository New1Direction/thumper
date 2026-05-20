use crate::tui::job::{Job, JobStage};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Renders a fixed 6-character text engine core that cycles states without shifting columns.
/// Now stage-aware (Chunk 7 native delight): different "personalities" for
/// Resolving (cool & calm), PackageOp (warm & energetic 📦), ScriptRunning (electric ⚡ 🚀).
pub fn render_recursive_fractal_core(job: &Job) -> Vec<Span<'static>> {
    if job.stage == JobStage::Complete {
        return vec![
            Span::styled(
                "✔ ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("     ", Style::default()), // Strict padding conservation: exactly 5 spaces
        ];
    }

    // Stage-driven personality (slower phase for calm stages, faster/electric for scripts)
    let base_phase = (job.phase_offset % 4) as usize;
    let phase = match job.stage {
        JobStage::Resolving => (job.phase_offset / 2 % 4) as usize, // slower
        JobStage::ScriptRunning => ((job.phase_offset * 3) % 4) as usize, // snappier
        _ => base_phase,
    };

    let (cool, warm, electric) = (Color::Cyan, Color::Yellow, Color::White);

    match (job.stage, phase) {
        // Calm, cool, breathing Resolving (thoughtful, low density)
        (JobStage::Resolving, 0) => vec![Span::styled("◉     ", Style::default().fg(cool))],
        (JobStage::Resolving, 1) => vec![Span::styled("·     ", Style::default().fg(cool))],
        (JobStage::Resolving, 2) => vec![Span::styled("◦     ", Style::default().fg(cool))],
        (JobStage::Resolving, _) => vec![Span::styled("◉     ", Style::default().fg(cool))],

        // Dense, warm, chunky PackageOp (📦 — busy, satisfying, high visual mass)
        (JobStage::PackageOp, 0) => vec![Span::styled("◉     ", Style::default().fg(warm))],
        (JobStage::PackageOp, 1) => vec![Span::styled(
            "◉⟐⟐   ",
            Style::default().fg(warm).add_modifier(Modifier::BOLD),
        )],
        (JobStage::PackageOp, 2) => vec![
            Span::styled("⟐⟐", Style::default().fg(warm).add_modifier(Modifier::BOLD)),
            Span::styled(
                "◉",
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⟐ ", Style::default().fg(warm)),
        ],
        (JobStage::PackageOp, _) => vec![Span::styled(
            "⟐⟐⟐⟐⟐",
            Style::default().fg(warm).add_modifier(Modifier::BOLD),
        )],

        // Sharp, electric, high-contrast ScriptRunning (🚀 — fast pulse, snappy)
        (JobStage::ScriptRunning, 0) => vec![Span::styled("◉     ", Style::default().fg(electric))],
        (JobStage::ScriptRunning, 1) => vec![Span::styled(
            "⟐⚡⟐  ",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )],
        (JobStage::ScriptRunning, 2) => vec![
            Span::styled(
                "⚡⟐",
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "◉",
                Style::default().fg(electric).add_modifier(Modifier::BOLD),
            ),
            Span::styled("⟐ ", Style::default().fg(Color::Cyan)),
        ],
        (JobStage::ScriptRunning, _) => vec![Span::styled(
            "⟐⚡⟐⚡⟐",
            Style::default().fg(electric).add_modifier(Modifier::BOLD),
        )],

        // Default / Downloading
        (_, 0) => vec![Span::styled("◉     ", Style::default().fg(cool))],
        (_, 1) => vec![Span::styled(
            "◉⟐    ",
            Style::default().fg(cool).add_modifier(Modifier::BOLD),
        )],
        (_, 2) => vec![
            Span::styled("⟐", Style::default().fg(warm).add_modifier(Modifier::BOLD)),
            Span::styled("◉", Style::default().fg(cool).add_modifier(Modifier::BOLD)),
            Span::styled("⟐   ", Style::default().fg(warm)),
        ],
        (_, _) => vec![Span::styled(
            "⟐⟐⟐⟐⚡",
            Style::default().fg(warm).add_modifier(Modifier::BOLD),
        )],
    }
}

/// Renders a Braille-patterned data bar with responsive coloring based on job velocity.
pub fn render_braille_plasma_bar(
    job: &Job,
    width: usize,
    force_celebration: bool, // NEW: forces green dense bar for celebration/flash
) -> Line<'static> {
    if job.stage == JobStage::Complete {
        return Line::from("");
    }

    // Safely clip raw progress percentage bounds
    let progress = job.progress.clamp(0.0, 1.0);
    let fill_len = ((progress * width as f32) as usize).min(width);

    let velocity = job.velocity;
    let is_boost = velocity > 0.75;

    let mut spans = Vec::new();

    // 1. Render Fixed Leading Edge Engine Marker — now stage-aware (Chunk 7)
    // 📦 warm yellow for package ops, 🚀 electric for scripts, cool cyan for calm resolving
    let (lead_glyph, lead_color) = if force_celebration {
        ("⟿", Color::Green)
    } else {
        match job.stage {
            JobStage::Resolving => ("⟹", Color::Cyan),
            JobStage::PackageOp => (
                "⟿",
                if is_boost {
                    Color::LightYellow
                } else {
                    Color::Yellow
                },
            ),
            JobStage::ScriptRunning => (
                "⚡",
                if is_boost {
                    Color::White
                } else {
                    Color::LightCyan
                },
            ),
            _ => {
                if is_boost {
                    ("⟿", Color::Yellow)
                } else {
                    ("⟹", Color::Cyan)
                }
            }
        }
    };
    spans.push(Span::styled(
        lead_glyph,
        Style::default().fg(lead_color).add_modifier(Modifier::BOLD),
    ));

    // Celebration mode takes precedence
    if force_celebration {
        let plasma = "⣿".repeat(fill_len);
        spans.push(Span::styled(
            plasma,
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));

        let remaining = width.saturating_sub(fill_len + 1);
        if remaining > 0 {
            spans.push(Span::styled(
                "░".repeat(remaining),
                Style::default().fg(Color::DarkGray),
            ));
        }
        return Line::from(spans);
    }

    // Stage + velocity aware plasma texture (Chunk 7 native delight)
    let plasma_char = match job.stage {
        JobStage::Resolving => {
            if velocity > 0.6 {
                "⠸"
            } else {
                "⠰"
            }
        }
        JobStage::PackageOp => {
            if velocity > 0.8 {
                "⠿"
            } else if velocity > 0.5 {
                "⠸"
            } else {
                "⠰"
            }
        }
        JobStage::ScriptRunning => {
            if velocity > 0.75 {
                "⠿"
            } else {
                "⠹"
            }
        }
        _ => {
            if velocity > 0.8 {
                "⠿"
            } else if velocity > 0.55 {
                "⠸"
            } else if velocity > 0.35 {
                "⠰"
            } else {
                "⠠"
            }
        }
    };

    let plasma = plasma_char.repeat(fill_len);
    let plasma_style = match job.stage {
        JobStage::PackageOp => {
            if is_boost {
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Yellow)
            }
        }
        JobStage::ScriptRunning => {
            if is_boost {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::LightCyan)
            }
        }
        _ => {
            if is_boost {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            }
        }
    };
    spans.push(Span::styled(plasma, plasma_style));

    // 3. Render Tracking Inactive Track Space
    let remaining = width.saturating_sub(fill_len + 1);
    if remaining > 0 {
        spans.push(Span::styled(
            "░".repeat(remaining),
            Style::default().fg(Color::DarkGray),
        ));
    }

    Line::from(spans)
}
