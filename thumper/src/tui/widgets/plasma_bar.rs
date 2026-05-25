use crate::tui::job::{Job, JobStage};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// === Truecolor Catppuccin Mocha Plasma Palette ===
/// Used for silky, velocity-reactive, stage-aware animation.
const MOCHA_SKY: Color = Color::Rgb(137, 220, 235);
const MOCHA_TEAL: Color = Color::Rgb(148, 226, 213);
const MOCHA_MAUVE: Color = Color::Rgb(203, 166, 247);
const MOCHA_PEACH: Color = Color::Rgb(250, 179, 135);
const MOCHA_ROSEWATER: Color = Color::Rgb(245, 194, 231);
const MOCHA_SAPPHIRE: Color = Color::Rgb(116, 199, 236);
const MOCHA_LAVENDER: Color = Color::Rgb(180, 190, 254);

/// Angry error palette for failed jobs (slow breathing pulse)
const MOCHA_RED: Color = Color::Rgb(243, 139, 168);
const MOCHA_MAROON: Color = Color::Rgb(230, 69, 83);

/// Smooth Hermite interpolation (0..1) — produces beautiful easing curves
#[inline]
fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Linear RGB interpolation between two Truecolor values
#[inline]
fn lerp_color(a: Color, b: Color, t: f64) -> Color {
    if let (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) = (a, b) {
        let r = (r1 as f64 + (r2 as f64 - r1 as f64) * t).round() as u8;
        let g = (g1 as f64 + (g2 as f64 - g1 as f64) * t).round() as u8;
        let b = (b1 as f64 + (b2 as f64 - b1 as f64) * t).round() as u8;
        Color::Rgb(r, g, b)
    } else {
        a
    }
}

/// Core plasma color generator.
/// Velocity modulates both frequency (speed) and amplitude (how far the color travels).
/// When `is_error` is true, the bar enters a slow, angry breathing pulse between
/// MOCHA_MAROON and MOCHA_RED (ignoring normal stage/velocity logic).
fn plasma_color(stage: JobStage, t: f64, velocity: f32, is_error: bool) -> Color {
    if is_error {
        // Slow breathing angry red — low frequency for a calm but ominous pulse
        let slow_t = t * 0.55;
        let pulse = (slow_t * 1.15).sin() * 0.5 + 0.5;
        return lerp_color(MOCHA_MAROON, MOCHA_RED, pulse);
    }

    let v = velocity.clamp(0.0, 1.35) as f64;

    // Base frequency increases with velocity (fast jobs feel "alive")
    let freq = 1.6 + v * 2.8;
    let pulse = (t * freq).sin() * 0.5 + 0.5;

    // Secondary fast shimmer for high-velocity electric stages
    let shimmer = if v > 0.65 {
        ((t * (freq * 2.3)).sin() * 0.5 + 0.5) * (v - 0.5) * 1.1
    } else {
        0.0
    };

    match stage {
        JobStage::Resolving => {
            // Cool, thoughtful breathing — Sky ↔ Teal ↔ Sapphire
            let base = lerp_color(MOCHA_SKY, MOCHA_TEAL, pulse * 0.65);
            lerp_color(base, MOCHA_SAPPHIRE, shimmer.min(0.7))
        }
        JobStage::PackageOp => {
            // Warm, chunky, satisfying — Peach ↔ Rosewater ↔ Mauve
            let base = lerp_color(MOCHA_PEACH, MOCHA_ROSEWATER, pulse);
            lerp_color(base, MOCHA_MAUVE, (v * 0.85).min(0.9))
        }
        JobStage::ScriptRunning => {
            // Sharp, electric, high-contrast — fast Mauve/Lavender/Sky shimmer
            let fast_pulse = (t * (freq * 1.6)).sin() * 0.5 + 0.5;
            let base = lerp_color(MOCHA_MAUVE, MOCHA_LAVENDER, fast_pulse);
            lerp_color(base, MOCHA_SKY, shimmer * 0.9 + (v - 0.6).max(0.0) * 0.6)
        }
        _ => lerp_color(MOCHA_TEAL, MOCHA_SKY, pulse * 0.6),
    }
}

/// Renders a fixed 6-character text engine core.
/// Now driven by continuous real elapsed time + velocity for silky-smooth Truecolor pulsing.
/// Stage personality is preserved in glyph selection while color & intensity breathe fluidly.
pub fn render_recursive_fractal_core(job: &Job) -> Vec<Span<'static>> {
    if job.stage == JobStage::Complete {
        return vec![
            Span::styled(
                "✔ ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("     ", Style::default()),
        ];
    }

    let t = job.animation_time;
    let v = job.velocity;

    // Continuous intensity (0.0 calm → 1.0+ energetic). Velocity boosts both amplitude and shimmer.
    let intensity = smoothstep(0.22, 0.92, v as f64) * 0.75
        + ((t * (1.9 + v as f64 * 2.1)).sin() * 0.18 + 0.18);

    let color = plasma_color(job.stage, t, v, job.error_diagnostics.is_some());

    // Glyph personality stays (good UX), but color is now a living 24-bit gradient.
    match job.stage {
        JobStage::Resolving => {
            let glyph = if intensity < 0.28 {
                "◦"
            } else if intensity < 0.58 {
                "·"
            } else {
                "◉"
            };
            vec![Span::styled(
                format!("{}     ", glyph),
                Style::default().fg(color),
            )]
        }
        JobStage::PackageOp => {
            let bold = if intensity > 0.52 {
                Modifier::BOLD
            } else {
                Modifier::empty()
            };
            let g = if intensity > 0.78 {
                "⟐⟐⟐⟐⟐"
            } else if intensity > 0.48 {
                "⟐⟐◉  "
            } else {
                "◉     "
            };
            vec![Span::styled(
                g,
                Style::default().fg(color).add_modifier(bold),
            )]
        }
        JobStage::ScriptRunning => {
            let bold = Modifier::BOLD;
            let g = if intensity > 0.82 {
                "⟐⚡⟐⚡⟐"
            } else if intensity > 0.58 {
                "⚡⟐◉⟐ "
            } else {
                "◉     "
            };
            vec![Span::styled(
                g,
                Style::default().fg(color).add_modifier(bold),
            )]
        }
        _ => vec![Span::styled("◉     ", Style::default().fg(color))],
    }
}

/// Renders a Braille-patterned data bar with silky Truecolor plasma driven by real elapsed time.
/// Color now smoothly lerps between rich Mocha tones; velocity affects both character choice and color travel.
pub fn render_braille_plasma_bar(
    job: &Job,
    width: usize,
    force_celebration: bool,
) -> Line<'static> {
    if job.stage == JobStage::Complete {
        return Line::from("");
    }

    let progress = job.progress.clamp(0.0, 1.0);
    let fill_len = ((progress * width as f32) as usize).min(width);
    let t = job.animation_time;
    let v = job.velocity;

    if force_celebration {
        let plasma = "⣿".repeat(fill_len);
        return Line::from(vec![
            Span::styled(
                "⟿",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                plasma,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
    }

    // Leading glyph (personality preserved)
    let (lead_glyph, lead_color) = match job.stage {
        JobStage::Resolving => ("⟹", MOCHA_SKY),
        JobStage::PackageOp => (
            "⟿",
            if v > 0.72 {
                MOCHA_PEACH
            } else {
                MOCHA_ROSEWATER
            },
        ),
        JobStage::ScriptRunning => ("⚡", if v > 0.72 { Color::White } else { MOCHA_MAUVE }),
        _ => ("⟹", MOCHA_TEAL),
    };
    let mut spans = vec![Span::styled(
        lead_glyph,
        Style::default().fg(lead_color).add_modifier(Modifier::BOLD),
    )];

    // The plasma fill now receives a living, continuously interpolated color
    let fill_color = plasma_color(job.stage, t, v, job.error_diagnostics.is_some());
    let bold = if v > 0.68 {
        Modifier::BOLD
    } else {
        Modifier::empty()
    };

    // Velocity + sine now influences braille density character as well
    let density =
        smoothstep(0.28, 0.9, v as f64) * 0.55 + ((t * (2.8 + v as f64 * 1.8)).sin() * 0.22 + 0.22);
    let plasma_char = match job.stage {
        JobStage::ScriptRunning if density > 0.72 => "⠿",
        JobStage::PackageOp if density > 0.68 => "⠿",
        _ if density > 0.58 => "⠸",
        _ if density > 0.35 => "⠰",
        _ => "⠠",
    };

    let plasma = plasma_char.repeat(fill_len);
    spans.push(Span::styled(
        plasma,
        Style::default().fg(fill_color).add_modifier(bold),
    ));

    // Inactive track
    let remaining = width.saturating_sub(fill_len + 1);
    if remaining > 0 {
        spans.push(Span::styled(
            "░".repeat(remaining),
            Style::default().fg(Color::DarkGray),
        ));
    }

    Line::from(spans)
}
