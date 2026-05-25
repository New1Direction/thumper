use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Local error palette (matches the one in plasma_bar.rs)
const MOCHA_RED: Color = Color::Rgb(243, 139, 168);
const MOCHA_MAROON: Color = Color::Rgb(230, 69, 83);

/// Predictive recovery actions that Thumper can offer when an error is detected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PredictiveAction {
    /// Run `bun install` (covers most ENOENT, missing lockfile, dependency issues)
    RunBunInstall,
    /// Run `bun init` (when the project directory appears uninitialized)
    RunBunInit,
    #[default]
    None,
}

/// Renders a beautiful Unicode-bordered Diagnostic Error Card.
/// Designed to be appended directly under a red-pulsing plasma bar inside a ListItem.
pub fn render_diagnostic_error_card(
    diagnostics: &[String],
    available_width: usize,
) -> Vec<Line<'static>> {
    if diagnostics.is_empty() {
        return vec![];
    }

    let width = available_width.clamp(40, 80); // sensible bounds
    let inner_width = width.saturating_sub(4); // account for "│ " + " │"

    let mut lines = Vec::new();

    // Top border
    let title = "⚠ Failure";
    let title_len = title.chars().count();
    let border_len = (width - 2).max(title_len + 4);
    let top = format!(
        "┌─ {} {:─<width$}┐",
        title,
        "",
        width = border_len.saturating_sub(title_len + 3)
    );
    lines.push(Line::from(Span::styled(
        top,
        Style::default().fg(MOCHA_RED).add_modifier(Modifier::BOLD),
    )));

    // Content lines (take first 4 relevant lines)
    let mut content_lines: Vec<String> = diagnostics.iter().take(4).cloned().collect();

    // Highlight the most "error-y" line (first one containing common failure markers)
    if let Some(idx) = content_lines.iter().position(|l| {
        l.contains("ENOENT")
            || l.contains("error")
            || l.contains("failed")
            || l.contains("EACCES")
            || l.contains("command not found")
    }) {
        if let Some(line) = content_lines.get_mut(idx) {
            // We'll style it bold when rendering
        }
    }

    for (i, line) in content_lines.iter().enumerate() {
        let mut display = line.clone();
        if display.len() > inner_width {
            display.truncate(inner_width - 3);
            display.push_str("...");
        }
        let display_len = display.len();

        let style = if i == 0
            || line.contains("ENOENT")
            || line.contains("error")
            || line.contains("failed")
        {
            Style::default().fg(MOCHA_RED).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        lines.push(Line::from(vec![
            Span::styled("│ ", Style::default().fg(MOCHA_RED)),
            Span::styled(display, style),
            Span::styled(
                " ".repeat(inner_width.saturating_sub(display_len)),
                Style::default(),
            ),
            Span::styled("│", Style::default().fg(MOCHA_RED)),
        ]));
    }

    // Blank line inside card for breathing room
    if !content_lines.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("│", Style::default().fg(MOCHA_RED)),
            Span::styled(" ".repeat(inner_width), Style::default()),
            Span::styled("│", Style::default().fg(MOCHA_RED)),
        ]));
    }

    // Actionable hint + predictive recovery action
    let (hint, action) = analyze_failure(diagnostics);
    if !hint.is_empty() {
        let mut hint_display = hint.clone();
        if hint_display.len() > inner_width {
            hint_display.truncate(inner_width - 3);
            hint_display.push_str("...");
        }
        let hint_len = hint_display.len() + 2; // for emoji + space

        lines.push(Line::from(vec![
            Span::styled("│ ", Style::default().fg(MOCHA_RED)),
            Span::styled("💡 ", Style::default().fg(MOCHA_MAROON)),
            Span::styled(hint_display, Style::default().fg(Color::Gray)),
            Span::styled(
                " ".repeat(inner_width.saturating_sub(hint_len)),
                Style::default(),
            ),
            Span::styled("│", Style::default().fg(MOCHA_RED)),
        ]));
    }

    // Predictive recovery action prompt (only shown when we have a concrete action)
    if action != PredictiveAction::None {
        let (key, description) = match action {
            PredictiveAction::RunBunInstall => ("I", "run `bun install`"),
            PredictiveAction::RunBunInit => ("N", "run `bun init`"),
            PredictiveAction::None => unreachable!(),
        };

        let prompt = format!("🛠️  Action: Press [{}] to {}", key, description);
        let mut prompt_display = prompt.clone();
        if prompt_display.len() > inner_width {
            prompt_display.truncate(inner_width - 3);
            prompt_display.push_str("...");
        }
        let prompt_len = prompt_display.len();

        lines.push(Line::from(vec![
            Span::styled("│ ", Style::default().fg(MOCHA_RED)),
            Span::styled(
                prompt_display,
                Style::default().fg(MOCHA_RED).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " ".repeat(inner_width.saturating_sub(prompt_len)),
                Style::default(),
            ),
            Span::styled("│", Style::default().fg(MOCHA_RED)),
        ]));
    }

    // Bottom border
    let bottom = format!("└{:─<width$}┘", "", width = width - 2);
    lines.push(Line::from(Span::styled(
        bottom,
        Style::default().fg(MOCHA_RED).add_modifier(Modifier::BOLD),
    )));

    lines
}

/// Analyzes the diagnostics and returns both a human-readable hint and a
/// recommended PredictiveAction that the user can trigger with a key.
pub fn analyze_failure(diagnostics: &[String]) -> (String, PredictiveAction) {
    let combined = diagnostics.join(" ").to_lowercase();

    if combined.contains("enoent")
        || combined.contains("no such file")
        || combined.contains("package.json")
    {
        return (
            "Lockfile or package.json missing. Run `bun install` in the project directory."
                .to_string(),
            PredictiveAction::RunBunInstall,
        );
    }
    if combined.contains("command not found")
        || combined.contains("sh:")
        || combined.contains("not found")
    {
        return (
            "Executable is missing from $PATH. Check your Bun / Node installation.".to_string(),
            PredictiveAction::RunBunInstall,
        );
    }
    if combined.contains("eacces")
        || combined.contains("permission denied")
        || combined.contains("perm")
    {
        return (
            "Permission denied. Check file ownership and directory permissions.".to_string(),
            PredictiveAction::None,
        );
    }
    if combined.contains("lockfile") {
        return (
            "Lockfile appears corrupted or out of sync. Delete and run `bun install` again."
                .to_string(),
            PredictiveAction::RunBunInstall,
        );
    }
    if combined.contains("cannot find module") || combined.contains("module not found") {
        return (
            "Missing dependency. Make sure all packages are installed (`bun install`).".to_string(),
            PredictiveAction::RunBunInstall,
        );
    }

    (
        "See full output with `--verbose` or check the command you ran.".to_string(),
        PredictiveAction::None,
    )
}
