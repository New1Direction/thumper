//! BunBunny Startup Screen
//! Branded intro animation shown every time the TUI launches.
//! Inspired by Hermes Agent's logo-style startup.

use std::time::{Duration, Instant};

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

/// Run-in animation frames (BunBunny sprinting from left → center)
const BUNBUNNY_RUN: [&str; 8] = [
    r#"
      (\(\      >>>
     ( -.-)
      o_(")(")
    "#,
    r#"
       (\(\     >>>
      ( -.-)
       o_(")(")
    "#,
    r#"
        (\(\    >>
       ( -.-)
        o_(")(")
    "#,
    r#"
         (\(\   >
        ( -.-)
         o_(")(")
    "#,
    r#"
          (\(\
         ( -.-)
          o_(")(")
    "#,
    r#"
           (\(\
          (-.-)
           o_(")(")
    "#,
    r#"
            (\(\
           (-.-)
            o_(")(")
    "#,
    r#"
             (\(\
            (-.-)
             o_(")(")
    "#,
];

/// Subtle idle animation (after stopping in the center)
const BUNBUNNY_IDLE: [&str; 4] = [
    r#"
             (\(\
            (-.-)
             o_(")(")
    "#,
    r#"
             (\(\
            (O.-)
             o_(")(")
    "#,
    r#"
             (\(\
            (-.-)
             o_(")(")
    "#,
    r#"
              (\(\
             (-.-)
              o_(")(")
    "#,
];

/// Weighted arrogant taglines (biased toward cocky/arrogant ones)
const TAGLINES: [&str; 30] = [
    // God-tier arrogant (heavily weighted)
    "• Bun just violated several laws of physics. Again.",
    "• I finished before you even finished typing the command.",
    "• Your other tools are still initializing. Pathetic.",
    "• Light speed? Cute. I was already on the next frame.",
    "• The bunny didn’t even break eye contact.",
    "• Executed with extreme prejudice and zero remorse.",
    "• Clean. Violent. Efficient. You’re welcome.",
    "• That was adorable. Now watch how it’s actually done.",
    "• Zero latency. Maximum disrespect for slower runtimes.",
    "• The engine smiled. It never smiles.",
    // Strong arrogant
    "• Finished so fast it left a vapor trail of shame.",
    "• Bun said 'hold my beer' and the beer is still cold.",
    "• That was so fast it made Node.js cry in the corner.",
    "• Physics called. It’s filing a restraining order.",
    "• Bun.exe has stopped giving a fuck.",
    // Playful arrogant
    "• Respectable. For something that isn’t Bun.",
    "• Not bad. I’ve seen worse. Like, yesterday.",
    "• Executed with the grace of a caffeinated god.",
    "• Acceptable. The bunny is mildly impressed.",
    "• It works. Don’t get used to this level of competence.",
    // Milder ones (for variety)
    "• Solid. The bar is now slightly higher than the floor.",
    "• Finished. No explosions detected. That’s a win.",
    "• We did it. Barely. But we did it.",
    "• That was fast. Don’t let it go to your head.",
    "• Clean burn. No wasted cycles. Unlike your life choices.",
    "• The runtime is now judging your entire tech stack.",
    "• That was cute. Now go touch grass.",
    "• Bun just did a thing™ and it was illegal in 7 countries.",
    "• I didn’t even warm up. This was just a light stretch.",
    "• Finished before your IDE even realized what happened.",
];

/// Returns a tagline with bias toward the more arrogant ones (first 15)
pub fn get_weighted_tagline(frame: u64) -> &'static str {
    // Use frame to create deterministic but varied selection with bias toward arrogant lines
    let base = (frame / 4) as usize;

    // 75% bias toward the more arrogant first 15 lines
    if (base % 4) != 3 {
        &TAGLINES[base % 15]
    } else {
        &TAGLINES[base % TAGLINES.len()]
    }
}

/// Renders the full BunBunny startup screen
pub fn render_startup(
    f: &mut Frame,
    frame_count: u64,
    start_time: Instant,
    native_bun_available: bool,
    cwd: &str,
    version: &str,
) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(35), // Animation area
            Constraint::Length(3),      // Tagline
            Constraint::Length(1),      // Version + Status
            Constraint::Length(1),      // CWD
            Constraint::Length(6),      // Commands
            Constraint::Min(0),
        ])
        .split(area);

    // === Animation ===
    let elapsed = start_time.elapsed();
    let run_in_duration = Duration::from_millis(1400); // ~1.4s for run-in (8 frames)

    let animation_text = if elapsed < run_in_duration {
        // Run-in phase
        let progress = elapsed.as_millis() as f32 / run_in_duration.as_millis() as f32;
        let frame =
            ((progress * (BUNBUNNY_RUN.len() - 1) as f32) as usize).min(BUNBUNNY_RUN.len() - 1);
        BUNBUNNY_RUN[frame]
    } else {
        // Idle phase
        let idle_frame = ((frame_count / 6) % BUNBUNNY_IDLE.len() as u64) as usize;
        BUNBUNNY_IDLE[idle_frame]
    };

    let bunny = Paragraph::new(animation_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan));

    f.render_widget(bunny, chunks[0]);

    // === Tagline ===
    let tagline = get_weighted_tagline(frame_count);
    let tagline_para = Paragraph::new(tagline).alignment(Alignment::Center).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(tagline_para, chunks[1]);

    // === Version + Native Status (Chunk 7 delight) ===
    let status = if native_bun_available {
        "📦 Native Bun (fast path, zero Python)"
    } else {
        "Falling back to Python harness"
    };

    let version_line = format!("{}  •  {}", version, status);
    let version_para = Paragraph::new(version_line)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(version_para, chunks[2]);

    // === Current Directory ===
    let cwd_line = format!("Working in: {}", cwd);
    let cwd_para = Paragraph::new(cwd_line)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(cwd_para, chunks[3]);

    // === Key Commands ===
    let commands = vec![
        Line::from(Span::styled(
            "Main Controls",
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::raw("  :     → Open command palette")),
        Line::from(Span::raw("  b     → Context-aware Bun action")),
        Line::from(Span::raw("  g     → Generate")),
        Line::from(Span::raw("  /     → Fuzzy search")),
    ];

    let commands_para = Paragraph::new(commands)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(commands_para, chunks[4]);
}

/// Helper to decide if startup should still be shown
pub fn should_show_startup(start_time: Instant) -> bool {
    start_time.elapsed() < Duration::from_secs(8)
}
