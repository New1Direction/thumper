//! Thumper Flight Deck — Interactive Onboarding Experience
//!
//! Current status (as of this cleanup):
//! - Two-phase experience: beautiful inline StreamIgnition → full ratatui Hermes cockpit.
//! - Station 01: Accelerated Plasma Showcase (real render_braille_plasma_bar, stage cycling).
//! - Station 02: Self-Healing Drill (live error card + [I] recovery animation + success bunny).
//! - Stations 03 (Ancestry Live) and 04 (Sandbox) are placeholders for future expansion.
//!
//! The demo is intentionally self-contained and reuses the real production widgets
//! (plasma bar, error card, Job model) with accelerated timing for showcase purposes.

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Station {
    PlasmaShowcase,
    SelfHealingDrill,
    // AncestryLive and Sandbox are planned future stations (currently placeholders)
    AncestryLive,
    Sandbox,
}

/// Internal phase for the Self-Healing Drill station
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DrillPhase {
    Failed,
    Recovering,
    Recovered,
}

/// Entry point called from main.rs when `--demo` is passed
pub async fn run_demo() -> anyhow::Result<()> {
    // Phase 0: Grok-style stream ignition
    run_stream_ignition()?;

    // If user pressed Space through the entire stream, enter the cockpit
    run_hermes_cockpit().await
}

/// === PHASE 0: Grok-Style Inline Ignition Stream ===
fn run_stream_ignition() -> anyhow::Result<()> {
    let mut step = 0u8;

    loop {
        match step {
            0 => {
                println!("\n◆ Booting Thumper Flight Deck Kernel........... OK");
                println!("◆ Initializing continuous wall-clock animation thread.... OK");
                println!("◆ Loading zero-subprocess multi-platform ancestry engine.... OK");
                println!();
                println!("   (\\(\\");
                println!("   ( >.<)   [ FLIGHT DECK ONLINE ]");
                println!("   o_(\")(\")");
                println!();
                println!("   Thumper Flight Deck engaged. All core visual engines are live.");
                println!();
                println!("   [Space] Ignite Cockpit    [Esc] Abort    [?] Controls");
            }
            1 => {
                println!("\n   Press [Space] to enter the Hermes Operations Dashboard...");
            }
            _ => {
                println!("\n   Flight Deck ignition complete.");
                break;
            }
        }

        // Simple raw key wait (no full ratatui yet)
        if wait_for_key_in_stream()? {
            step += 1;
        } else {
            // User pressed Esc or quit
            println!("\n   Flight aborted. Returning to terminal.");
            return Ok(());
        }
    }

    Ok(())
}

/// Very lightweight key waiter for the stream phase
fn wait_for_key_in_stream() -> anyhow::Result<bool> {
    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => return Ok(false),
                        KeyCode::Char(' ') => return Ok(true),
                        KeyCode::Char('?') => {
                            println!("\n   Controls: Space = advance | Esc = exit | I/N = recovery (in drill)");
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

/// === PHASE 1: Hermes Cockpit - Full Ratatui Dashboard ===
async fn run_hermes_cockpit() -> anyhow::Result<()> {
    use crossterm::{
        event::{self, Event, KeyCode, KeyEventKind},
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    };
    use ratatui::backend::CrosstermBackend;
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Paragraph};
    use ratatui::Terminal;

    use crate::tui::widgets::error_card::render_diagnostic_error_card;
    use std::io::stdout;
    use std::time::{Duration, Instant};

    // Clean handoff from stream ignition
    println!("\n[Transition] Entering Hermes Operations Dashboard...");
    std::thread::sleep(Duration::from_millis(400)); // brief dramatic pause

    // Setup full ratatui terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    // Demo state
    let mut current_station = Station::PlasmaShowcase;
    let mut tick: u64 = 0;
    let mut last_tick = Instant::now();

    // Self-Healing Drill state
    let mut drill_phase = DrillPhase::Failed;
    let mut recovery_start_tick: u64 = 0;
    let drill_diagnostics: Vec<String> = vec![
        "ENOENT: no such file or directory, open 'package.json'".to_string(),
        "error: failed to resolve dependencies".to_string(),
        "bun install exited with code 1".to_string(),
    ];

    // Mock job for plasma rendering (we'll drive its animation_time fast)
    let mut demo_job = crate::tui::job::Job {
        id: 1,
        command: "plasma.showcase".to_string(),
        stage: crate::tui::job::JobStage::Resolving,
        start_time: std::time::Instant::now(),
        elapsed: Duration::from_secs(0),
        progress: 0.65,
        velocity: 0.8,
        animation_time: 0.0,
        completion_time: None,
        performance_score: Some(92.0),
        error_diagnostics: None,
    };

    let result = (|| -> anyhow::Result<()> {
        loop {
            // Tick the demo
            let now = Instant::now();
            if now.duration_since(last_tick) > Duration::from_millis(33) {
                // ~30 FPS for smooth animation
                tick = tick.wrapping_add(1);
                last_tick = now;

                // Accelerate animation dramatically for the showcase
                demo_job.animation_time = (tick as f64) * 0.18; // much faster than real TUI

                // Per-station behavior
                match current_station {
                    Station::PlasmaShowcase => {
                        // Original accelerated showcase: cycle through all three personalities
                        let stage_index = ((tick / 45) % 3) as usize;
                        demo_job.stage = match stage_index {
                            0 => crate::tui::job::JobStage::Resolving,     // calm cyan
                            1 => crate::tui::job::JobStage::PackageOp,     // warm dense amber
                            _ => crate::tui::job::JobStage::ScriptRunning, // electric white pulse
                        };
                        demo_job.error_diagnostics = None;
                        demo_job.velocity = 0.6 + ((tick as f64 * 0.03).sin() * 0.25).abs() as f32;
                    }
                    Station::SelfHealingDrill => {
                        match drill_phase {
                            DrillPhase::Failed => {
                                // Angry red pulsing plasma + live error card
                                demo_job.error_diagnostics = Some(drill_diagnostics.clone());
                                demo_job.stage = crate::tui::job::JobStage::PackageOp;
                                demo_job.velocity = 0.25; // sluggish, unhealthy
                            }
                            DrillPhase::Recovering => {
                                // Healing personality: calm teal/cyan waves + lively recovery pulse
                                demo_job.error_diagnostics = None;
                                demo_job.stage = crate::tui::job::JobStage::Resolving;
                                let breathe = ((tick as f64 * 0.045).sin() * 0.35).abs();
                                demo_job.velocity = 0.65 + breathe as f32;
                                // Auto-complete recovery after ~3 seconds of animation
                                if tick.saturating_sub(recovery_start_tick) > 95 {
                                    drill_phase = DrillPhase::Recovered;
                                }
                            }
                            DrillPhase::Recovered => {
                                demo_job.error_diagnostics = None;
                                demo_job.stage = crate::tui::job::JobStage::Complete;
                                demo_job.velocity = 0.95;
                            }
                        }
                    }
                    _ => {
                        // Placeholder for unimplemented future stations (AncestryLive / Sandbox)
                        demo_job.error_diagnostics = None;
                        demo_job.stage = crate::tui::job::JobStage::Resolving;
                        demo_job.velocity = 0.7;
                    }
                }
            }

            // Draw — station-aware layout
            terminal.draw(|f| {
                let area = f.area();
                let is_drill = current_station == Station::SelfHealingDrill;

                // Dynamic layout: extra room for error card / recovery status on the drill
                let constraints = if is_drill {
                    vec![
                        Constraint::Length(3),  // Header
                        Constraint::Length(6),  // Plasma
                        Constraint::Min(9),     // Error card or recovery/success panel
                        Constraint::Length(4),  // Footer
                    ]
                } else {
                    vec![
                        Constraint::Length(3),  // Header
                        Constraint::Min(9),     // Plasma
                        Constraint::Length(4),  // Footer
                    ]
                };

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(constraints)
                    .split(area);

                // Station-aware header
                let station_title = match current_station {
                    Station::PlasmaShowcase => "STATION 01: ACCELERATED PLASMA SHOWCASE",
                    Station::SelfHealingDrill => "STATION 02: SELF-HEALING DRILL",
                    _ => "FUTURE STATION (not yet implemented)",
                };
                let header = Paragraph::new(vec![
                    Line::from(format!("THUMPER FLIGHT DECK  •  {}", station_title)),
                ])
                .style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(ratatui::widgets::BorderType::Rounded)
                        .title(" Hermes Operations "),
                );
                f.render_widget(header, chunks[0]);

                // Plasma (always present, reuses the real engine with current job state)
                let plasma_width = (chunks[1].width as usize).saturating_sub(4);
                let plasma = crate::tui::widgets::plasma_bar::render_braille_plasma_bar(
                    &demo_job,
                    plasma_width,
                    false,
                );
                let plasma_title = match (current_station, drill_phase) {
                    (Station::SelfHealingDrill, DrillPhase::Failed) => " Plasma Engine — FAILURE (red pulse) ".to_string(),
                    (Station::SelfHealingDrill, DrillPhase::Recovering) => " Plasma Engine — HEALING (calm teal) ".to_string(),
                    (Station::SelfHealingDrill, DrillPhase::Recovered) => " Plasma Engine — RECOVERED (green) ".to_string(),
                    _ => format!(" Plasma Engine — {:?} ", demo_job.stage),
                };
                let plasma_block = Paragraph::new(plasma)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(ratatui::widgets::BorderType::Rounded)
                            .title(plasma_title),
                    );
                f.render_widget(plasma_block, chunks[1]);

                // Status area (drill-only for now)
                if is_drill {
                    let status_chunk = chunks[2];
                    let status_lines: Vec<Line> = match drill_phase {
                        DrillPhase::Failed => {
                            // Live error card using the real production widget
                            let mut lines = render_diagnostic_error_card(&drill_diagnostics, status_chunk.width as usize);
                            // Add a small "demo hint" line at the bottom of the card area
                            lines.push(Line::from(Span::styled(
                                "   → Press [I] to trigger predictive self-healing recovery",
                                Style::default().fg(Color::Gray),
                            )));
                            lines
                        }
                        DrillPhase::Recovering => {
                            vec![
                                Line::from(""),
                                Line::from(Span::styled(
                                    "🛠️  PREDICTIVE RECOVERY IN PROGRESS",
                                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                                )),
                                Line::from(Span::styled(
                                    "   Plasma stabilizing to healthy teal/cyan waves...",
                                    Style::default().fg(Color::Gray),
                                )),
                                Line::from(Span::styled(
                                    "   Clearing error diagnostics • Re-validating dependencies",
                                    Style::default().fg(Color::Gray),
                                )),
                                Line::from(""),
                            ]
                        }
                        DrillPhase::Recovered => {
                            // Success bunny + confirmation (matching the ignition branding)
                            vec![
                                Line::from(""),
                                Line::from(Span::styled(
                                    "   (\\(\\     ✓  SELF-HEALING COMPLETE",
                                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                                )),
                                Line::from(Span::styled(
                                    "   ( >.<)     Dependencies resolved. Environment healthy.",
                                    Style::default().fg(Color::Green),
                                )),
                                Line::from(Span::styled(
                                    "   o_(\")(\")   Thumper Flight Deck ready for launch.",
                                    Style::default().fg(Color::Green),
                                )),
                                Line::from(""),
                            ]
                        }
                    };

                    let status_block = Paragraph::new(status_lines)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .border_type(ratatui::widgets::BorderType::Rounded)
                                .title(match drill_phase {
                                    DrillPhase::Failed => " Diagnostic Error Card ",
                                    DrillPhase::Recovering => " Recovery Telemetry ",
                                    DrillPhase::Recovered => " Mission Success ",
                                }),
                        );
                    f.render_widget(status_block, status_chunk);
                }

                // Footer (instructions adapt to station + drill phase)
                let footer_lines = if current_station == Station::SelfHealingDrill {
                    match drill_phase {
                        DrillPhase::Failed => vec![
                            Line::from("I: Initiate Recovery    Space: Next Station    Esc/q: Return to Shell"),
                            Line::from("Watch the MOCHA_RED breathing pulse react to the live error diagnostics."),
                        ],
                        DrillPhase::Recovering => vec![
                            Line::from("Recovery running...    Space: Skip to next station    Esc/q: Exit"),
                            Line::from("Calm green/teal waves + velocity pulse = healthy healing personality."),
                        ],
                        DrillPhase::Recovered => vec![
                            Line::from("✓ Healing successful    Space: Next Station    Esc/q: Return to Shell"),
                            Line::from("The plasma now renders in the green Complete state with celebration glyph."),
                        ],
                    }
                } else {
                    vec![
                        Line::from("Space: Cycle Stations    Esc / q: Return to Shell    (Flight Deck v0.1)"),
                        Line::from("Stations 01 & 02 implemented • real plasma + error card + self-healing demo"),
                    ]
                };

                let footer = Paragraph::new(footer_lines)
                    .style(Style::default().fg(Color::Gray))
                    .block(Block::default().borders(Borders::ALL).border_type(ratatui::widgets::BorderType::Rounded));
                let footer_idx = if is_drill { 3 } else { 2 };
                f.render_widget(footer, chunks[footer_idx]);
            })?;

            // Input handling (cockpit-wide + station-specific)
            if event::poll(Duration::from_millis(10))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('q') => break,

                            // Space: cycle between the two implemented stations.
                            // The remaining stations are future work and intentionally not reachable yet.
                            KeyCode::Char(' ') => {
                                current_station = match current_station {
                                    Station::PlasmaShowcase => Station::SelfHealingDrill,
                                    Station::SelfHealingDrill => Station::PlasmaShowcase,
                                    _ => Station::PlasmaShowcase,
                                };

                                // Seed appropriate demo state when entering a station
                                tick = 0;
                                last_tick = Instant::now();
                                demo_job.animation_time = 0.0;

                                match current_station {
                                    Station::PlasmaShowcase => {
                                        demo_job.error_diagnostics = None;
                                        demo_job.stage = crate::tui::job::JobStage::Resolving;
                                        demo_job.velocity = 0.75;
                                    }
                                    Station::SelfHealingDrill => {
                                        drill_phase = DrillPhase::Failed;
                                        recovery_start_tick = 0;
                                        demo_job.error_diagnostics =
                                            Some(drill_diagnostics.clone());
                                        demo_job.stage = crate::tui::job::JobStage::PackageOp;
                                        demo_job.velocity = 0.25;
                                    }
                                    _ => {
                                        // Should not be reached while we only cycle between 01 and 02
                                        demo_job.error_diagnostics = None;
                                        demo_job.stage = crate::tui::job::JobStage::Resolving;
                                        demo_job.velocity = 0.7;
                                    }
                                }
                            }

                            // [I] — manual recovery trigger inside the Self-Healing Drill
                            KeyCode::Char('i') | KeyCode::Char('I') => {
                                if current_station == Station::SelfHealingDrill
                                    && drill_phase == DrillPhase::Failed
                                {
                                    drill_phase = DrillPhase::Recovering;
                                    recovery_start_tick = tick;
                                    // Immediately clear diagnostics so the plasma stops the red pulse
                                    // and switches to the calm healing (Resolving + high velocity) look
                                    demo_job.error_diagnostics = None;
                                }
                            }

                            _ => {}
                        }
                    }
                }
            }
        }
        Ok(())
    })();

    // Cleanup
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}
