use crate::tui::app::App;
use crate::tui::job::{Job as PlasmaJob, JobStage};
use crate::tui::state::get_finishing_tagline;
use crate::tui::styles;
use crate::tui::widgets::error_card::render_diagnostic_error_card;
use crate::tui::widgets::plasma_bar::{render_braille_plasma_bar, render_recursive_fractal_core};
use chrono::Utc;
use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::layout::Alignment;
use std::time::Instant;

/// Render the Bun command palette line 1 (input buffer with cursor) and line 2 (live hint or transient error).
pub fn render_bun_command_palette(app: &App, f: &mut Frame, area: Rect) {
    let palette_style = styles::bun_palette_bg();
    let prompt_style = styles::bun_prompt_style();
    let help_style = styles::bun_help_style();
    let cmd_style = Style::default()
        .fg(styles::BRIGHT_CYAN)
        .add_modifier(Modifier::BOLD);
    let arg_style = Style::default().fg(Color::White);
    let flag_style = Style::default().fg(Color::Yellow);

    // Dark charcoal background block
    let block = Block::default().style(palette_style).borders(Borders::TOP);

    f.render_widget(block, area);

    // Split the 2-line area
    let line_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    // === Line 1: Prompt + Syntax-highlighted buffer with real cursor position ===
    let mut spans: Vec<Span> = vec![Span::styled("❯ bun-cmd: ", prompt_style)];

    let buffer = &app.bun_command_buffer;
    let cursor = app.bun_cursor_index;

    if buffer.is_empty() {
        // Empty buffer → just show the cursor
        spans.push(Span::styled(
            "█",
            Style::default()
                .fg(styles::BRIGHT_CYAN)
                .add_modifier(Modifier::SLOW_BLINK),
        ));
    } else {
        // Build tokens with cursor awareness
        let mut current_pos = 0;
        let tokens: Vec<&str> = buffer.split_whitespace().collect();
        let mut token_start_positions = vec![];
        let mut pos = 0;

        // Calculate start positions of each token in the original string
        for token in &tokens {
            if let Some(start) = buffer[pos..].find(token) {
                let abs_start = pos + start;
                token_start_positions.push(abs_start);
                pos = abs_start + token.len();
            }
        }

        for (i, token) in tokens.iter().enumerate() {
            let token_start = token_start_positions.get(i).copied().unwrap_or(0);
            let token_end = token_start + token.len();

            // Insert cursor if it's inside this token (or right before it)
            if cursor > current_pos && cursor <= token_start {
                spans.push(Span::styled(
                    "█",
                    Style::default()
                        .fg(styles::BRIGHT_CYAN)
                        .add_modifier(Modifier::SLOW_BLINK),
                ));
            }

            // Determine style for this token
            let style = if i == 0 {
                cmd_style
            } else if token.starts_with("--") || token.starts_with("-") {
                flag_style
            } else {
                arg_style
            };

            // Add the token (or split it if cursor is inside)
            if cursor > token_start && cursor < token_end {
                // Cursor is inside this token
                let before = &token[..(cursor - token_start)];
                let after = &token[(cursor - token_start)..];
                if !before.is_empty() {
                    spans.push(Span::styled(before, style));
                }
                spans.push(Span::styled(
                    "█",
                    Style::default()
                        .fg(styles::BRIGHT_CYAN)
                        .add_modifier(Modifier::SLOW_BLINK),
                ));
                if !after.is_empty() {
                    spans.push(Span::styled(after, style));
                }
            } else {
                spans.push(Span::styled(*token, style));
            }

            current_pos = token_end;

            // Add space after token (except last)
            if i < tokens.len() - 1 {
                current_pos += 1; // space
                if cursor == current_pos {
                    spans.push(Span::styled(
                        "█",
                        Style::default()
                            .fg(styles::BRIGHT_CYAN)
                            .add_modifier(Modifier::SLOW_BLINK),
                    ));
                } else {
                    spans.push(Span::raw(" "));
                }
            }
        }

        // Cursor after the last token
        if cursor == current_pos {
            spans.push(Span::styled(
                "█",
                Style::default()
                    .fg(styles::BRIGHT_CYAN)
                    .add_modifier(Modifier::SLOW_BLINK),
            ));
        }
    }

    let input_line = Line::from(spans);
    let input = Paragraph::new(input_line).block(Block::default());
    f.render_widget(input, line_chunks[0]);

    // === Line 2: Dense help footer OR transient muted-red error ===
    let (line2_text, line2_style) = if let Some(ref err) = app.palette_error_message {
        (
            format!("⚠ {}", err),
            Style::default()
                .fg(styles::MUTED_RED)
                .add_modifier(Modifier::BOLD),
        )
    } else if !app.bun_command_buffer.trim().is_empty() {
        let cmd = app.bun_command_buffer.trim();
        let icon = if cmd.starts_with("add")
            || cmd.starts_with("install")
            || cmd.starts_with("remove")
        {
            "📦 "
        } else if cmd.starts_with("run") || cmd.starts_with("script") {
            "🚀 "
        } else {
            "🐰 "
        };
        let preview = format!("{}  {}  ·  native fast path", icon, cmd);
        (preview, help_style)
    } else {
        (
            "[Enter]: run  │  [↑↓]: history  │  [Tab]: complete  │  [Esc]: cancel".to_string(),
            help_style,
        )
    };

    let help = Paragraph::new(line2_text)
        .style(line2_style)
        .alignment(Alignment::Left);

    f.render_widget(help, line_chunks[1]);
}

/// Render the full TUI dashboard frame.
pub fn render_app(app: &App, f: &mut Frame) {
    // Show branded BunBunny startup screen for the first ~8 seconds
    if let Some(start) = app.startup_start {
        if crate::tui::startup::should_show_startup(start) {
            crate::tui::startup::render_startup(
                f,
                app.frame,
                start,
                app.native_bun_available,
                &std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| ".".to_string()),
                env!("CARGO_PKG_VERSION"),
            );
            return;
        }
    }

    let size = f.area();

    // When the Bun command palette is active, we expand the bottom area to exactly 2 lines.
    let bottom_height = if app.in_bun_command_mode { 2 } else { 3 };

    // Dashboard layout (polished)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),             // header
            Constraint::Min(10),               // body (list+preview)
            Constraint::Length(9),             // jobs / activity panel
            Constraint::Length(bottom_height), // status OR 2-line Bun palette
        ])
        .split(size);

    // === Contextual Header + Metric Ribbon (Atmospheric Polish) ===
    let absorb_badge = if app.absorb_mode { " [ABSORB]" } else { "" };
    let filter_badge = if !app.filter.is_empty() {
        format!(" [filter:/{}]", app.filter)
    } else if app.in_search_mode {
        " [search...]".to_string()
    } else {
        String::new()
    };

    // Find the most "alive" native Bun job for the metric ribbon
    let live_metric = app.jobs.iter().rev().find(|j| {
        j.tool.starts_with("bun:") || j.tool.contains("package:") || j.tool.contains("script:")
    });

    // Build the metric ribbon (tiny live meter using velocity + performance)
    let metric_ribbon: String = if let Some(job) = live_metric {
        let vel = job.bun_package_count as f32 / 20.0
            + if job.status == "running" { 0.3 } else { 0.0 };
        let perf = job.performance_score.unwrap_or(60.0) / 100.0;
        let energy = ((vel + perf) / 2.0).clamp(0.1, 0.95);

        let bars = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
        let idx = ((energy * (bars.len() - 1) as f32) as usize).min(bars.len() - 1);
        let base = format!(
            "  {}  {:.0}%",
            bars[idx],
            job.performance_score.unwrap_or(0.0)
        );
        if let Some(sp) = &job.bun_speed {
            format!("{}  {}", base, sp)
        } else {
            base
        }
    } else {
        let idle_tick = (app.frame / 14) % 5;
        match idle_tick {
            0 => "  · ",
            1 => " 🐰 ",
            2 => "  · ",
            3 => "  ",
            _ => "  ",
        }
        .to_string()
    };

    // Rich header line with BunBunny branding (mauve) + metric ribbon on the right
    let header_line = Line::from(vec![
        Span::styled("🐰 ", styles::bunbunny_mauve()),
        Span::styled("API ANYTHING", styles::bunbunny_mauve()),
        Span::raw("  —  Get an API from anything"),
        Span::styled(absorb_badge, styles::header_style()),
        Span::styled(filter_badge, styles::header_style()),
        Span::raw("  "),
        Span::styled(&metric_ribbon, Style::default().fg(styles::MOCHA_MAUVE)),
    ]);

    let header = Paragraph::new(header_line)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(styles::border_style())
                .title(" v0.2.0  •  native Bun delight "),
        );
    f.render_widget(header, chunks[0]);

    // Body split horizontal
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(chunks[1]);

    // Left: Registry list (dynamic + filtered)
    let vis = app.visible_indices();
    let list_items: Vec<ListItem> = vis
        .iter()
        .map(|&idx| {
            let item = &app.items[idx];
            let marker = if item.status == "generated" {
                "★ "
            } else {
                ""
            };
            let line = format!("{}{}  [{}]", marker, item.name, item.kind);
            let base_style = if item.status == "generated" {
                styles::success_style()
            } else if item.status == "ok" {
                Style::default().fg(Color::Green)
            } else {
                styles::running_style()
            };
            ListItem::new(line).style(base_style)
        })
        .collect();

    let filter_title = if app.filter.is_empty() {
        " Registry (↑↓ jk • / filter) ".to_string()
    } else {
        format!(" Registry ({} shown • /{}) ", vis.len(), app.filter)
    };

    let list = List::new(list_items)
        .block(
            Block::default()
                .title(filter_title)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(styles::border_style()),
        )
        .highlight_style(styles::list_highlight_style())
        .highlight_symbol("▶ ");

    let mut list_state = ListState::default();
    if let Some(sel) = app.selected {
        if let Some(pos) = vis.iter().position(|&v| v == sel) {
            list_state.select(Some(pos));
        } else if !vis.is_empty() {
            list_state.select(Some(0));
        }
    }
    f.render_stateful_widget(list, body_chunks[0], &mut list_state);

    // Right: Preview pane (warm style)
    let preview_text = if let Some(sel) = app.selected {
        if sel < app.items.len() {
            app.items[sel].preview_text()
        } else {
            "Select…".to_string()
        }
    } else {
        "Select an item with ↑/↓ or j/k. Press / to fuzzy-filter.".to_string()
    };

    let preview = Paragraph::new(preview_text)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" Preview / Details ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(styles::border_style()),
        )
        .style(styles::preview_style());
    f.render_widget(preview, body_chunks[1]);

    // Jobs / Activity panel (live streamed progress, status, paths)
    let job_vis: Vec<_> = app.jobs.iter().rev().take(6).collect();
    let job_items: Vec<ListItem> = job_vis
        .into_iter()
        .map(|job| {
            let sp = if job.status == "running" {
                app.spinner()
            } else {
                " "
            };
            let pct_str = if job.status == "running" && job.pct > 0 {
                format!(" {:>3}%", job.pct)
            } else {
                String::new()
            };
            let path_part = job
                .output_path
                .as_deref()
                .map(|p| format!(" → {}", p))
                .unwrap_or_default();

            let is_bun = job.tool.starts_with("bun:")
                || job.tool.contains("package:")
                || job.tool.contains("script:");

            let icon = if is_bun {
                if job.tool.contains("install")
                    || job.tool.contains("add")
                    || job.tool.contains("remove")
                    || job.tool.contains("package:")
                {
                    "📦 "
                } else if job.tool.contains("run") || job.tool.contains("script:") {
                    "🚀 "
                } else {
                    "🐰 "
                }
            } else {
                ""
            };

            let display_tool = if is_bun {
                job.tool.replace("palette: ", "bun ").to_string()
            } else {
                job.tool.clone()
            };

            let display_label = if is_bun {
                job.bun_stage
                    .clone()
                    .unwrap_or_else(|| display_tool.clone())
            } else {
                display_tool.clone()
            };

            let display_msg = if is_bun && job.raw_output_count > 1 {
                if let Some(last) = &job.last_raw_line {
                    format!(
                        "{} [+{} raw] {}",
                        job.message,
                        job.raw_output_count,
                        last.chars().take(50).collect::<String>()
                    )
                } else {
                    format!("{} [+{} raw lines]", job.message, job.raw_output_count)
                }
            } else {
                job.message.clone()
            };

            let content_lines: Vec<Line<'static>> = if is_bun {
                let is_flash = if let Some(until) = &job.completion_flash_until {
                    Instant::now() < *until && job.status == "done"
                } else {
                    false
                };

                if job.status == "done" && is_flash {
                    let score = job.performance_score.unwrap_or(75.0);
                    let tagline = get_finishing_tagline(score);

                    let sparkle = if score >= 95.0 {
                        let sparkles = ["✦", "✧", "✨", "·", "•", "◦", " "];
                        let idx = ((app.frame / 3) as usize) % sparkles.len();
                        format!(" {}", sparkles[idx])
                    } else if score >= 85.0 && job.bun_package_count >= 3 {
                        let confetti = ["✦", "·", "•", " "];
                        let idx = ((app.frame / 5) as usize) % confetti.len();
                        format!(" {}", confetti[idx])
                    } else {
                        String::new()
                    };

                    let rank = if score >= 95.0 {
                        let anim = ((app.frame / 3) as usize) % 5;
                        match anim {
                            0 => " [G",
                            1 => " [GO",
                            2 => " [GOD",
                            3 => " [GOD]",
                            _ => " [GOD]",
                        }
                    } else if score >= 90.0 {
                        " [ELITE]"
                    } else if score >= 80.0 {
                        " [PRO]"
                    } else {
                        ""
                    };

                    let success_bunny = if job.bun_package_count >= 5 && score >= 90.0 {
                        " 🐰"
                    } else {
                        ""
                    };

                    let summary = if let Some(url) = &job.bun_server_url {
                        format!(
                            "✔ Active • {}  {}{}{}{}",
                            url, tagline, rank, sparkle, success_bunny
                        )
                    } else if job.bun_package_count > 0 {
                        if let Some(t) = &job.bun_timing {
                            format!(
                                "✔ {} packages • {}  {}{}{}{}",
                                job.bun_package_count, t, tagline, rank, sparkle, success_bunny
                            )
                        } else {
                            format!(
                                "✔ {} packages  {}{}{}{}",
                                job.bun_package_count, tagline, rank, sparkle, success_bunny
                            )
                        }
                    } else {
                        format!(
                            "✔ Complete  {}{}{}{}",
                            tagline, rank, sparkle, success_bunny
                        )
                    };

                    let flash_style = Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD);

                    vec![Line::from(Span::styled(summary, flash_style))]
                } else {
                    let proxy = PlasmaJob {
                        id: 0,
                        command: display_tool.clone(),
                        stage: match job.bun_stage.as_deref() {
                            Some("Resolving packages") => JobStage::Resolving,
                            Some("Downloading packages") => JobStage::Downloading,
                            Some(s) if s.contains("Verifying") => JobStage::Verifying,
                            Some(s) if s.contains("complete") || s.contains("Active") => {
                                JobStage::Complete
                            }
                            Some(s)
                                if s.contains("Installing")
                                    || s.contains("Adding")
                                    || s.contains("Removing") =>
                            {
                                JobStage::PackageOp
                            }
                            Some(s) if s.contains("Running script") || s.contains("script") => {
                                JobStage::ScriptRunning
                            }
                            _ => {
                                if job.tool.contains("run") || job.tool.contains("script:") {
                                    JobStage::ScriptRunning
                                } else if job.tool.contains("package:")
                                    || job.tool.contains("install")
                                    || job.tool.contains("add")
                                    || job.tool.contains("remove")
                                {
                                    JobStage::PackageOp
                                } else {
                                    JobStage::Downloading
                                }
                            }
                        },
                        start_time: std::time::Instant::now()
                            - std::time::Duration::from_secs(1),
                        elapsed: std::time::Duration::from_millis((job.pct as u64) * 10),
                        progress: (job.pct as f32) / 100.0,
                        velocity: {
                            if let Some(t) = &job.bun_timing {
                                let secs = if t.ends_with("ms") {
                                    t.trim_end_matches("ms").parse::<f32>().unwrap_or(400.0)
                                        / 1000.0
                                } else {
                                    t.trim_end_matches('s').parse::<f32>().unwrap_or(1.5)
                                };
                                (1.0_f32 / (secs + 0.2_f32)).clamp(0.35_f32, 0.98_f32)
                            } else if job.bun_package_count > 8 {
                                0.92
                            } else if job.bun_package_count > 0 {
                                0.75
                            } else {
                                0.55
                            }
                        },
                        animation_time: app.start_time.elapsed().as_secs_f64(),
                        completion_time: None,
                        performance_score: job.performance_score,
                        error_diagnostics: if job.status == "error" {
                            Some(vec!["Command failed. See diagnostics below.".to_string()])
                        } else {
                            None
                        },
                    };

                    let is_celebrating = is_flash
                        && (job.performance_score.unwrap_or(0.0) >= 78.0
                            || job.bun_package_count >= 3);

                    let mut core_spans = render_recursive_fractal_core(&proxy);
                    let mut bar_line = render_braille_plasma_bar(&proxy, 12, is_celebrating);

                    let base_line = {
                        let mut row_spans: Vec<Span<'static>> = vec![
                            Span::raw(format!("{} ", sp)),
                            Span::raw(format!("{} ", icon)),
                            Span::styled(
                                format!("{} ", display_label),
                                Style::default().fg(Color::Gray),
                            ),
                        ];
                        row_spans.append(&mut core_spans);
                        row_spans.push(Span::raw(" "));
                        row_spans.append(&mut bar_line.spans);
                        Line::from(row_spans)
                    };

                    let mut visual_lines = vec![base_line];

                    if let Some(diag) = &proxy.error_diagnostics {
                        let card_lines = render_diagnostic_error_card(diag, 58);
                        visual_lines.extend(card_lines);
                    }

                    visual_lines
                }
            } else {
                vec![Line::from(format!(
                    "{}{} {} [{}] {}{}{}",
                    sp, icon, display_tool, job.status, display_msg, pct_str, path_part
                ))]
            };

            let sty = match job.status.as_str() {
                "running" => {
                    if is_bun {
                        Style::default().fg(Color::Yellow)
                    } else {
                        styles::running_style()
                    }
                }
                "done" => {
                    let is_flash = if let Some(until) = &job.completion_flash_until {
                        Instant::now() < *until && is_bun
                    } else {
                        false
                    };
                    if is_flash {
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        styles::success_style()
                    }
                }
                "error" => styles::error_style(),
                _ => styles::muted_style(),
            };
            ListItem::new(ratatui::text::Text::from(content_lines)).style(sty)
        })
        .collect();

    let has_bun_jobs = app.jobs.iter().any(|j| {
        j.tool.starts_with("bun:") || j.tool.contains("package:") || j.tool.contains("script:")
    });

    let jobs_title = if has_bun_jobs {
        let native_jobs: Vec<_> = app
            .jobs
            .iter()
            .filter(|j| {
                j.tool.starts_with("bun:")
                    || j.tool.contains("package:")
                    || j.tool.contains("script:")
            })
            .collect();

        let pkg_count: usize = native_jobs.iter().map(|j| j.bun_package_count).sum();
        let avg_time: Option<String> = native_jobs
            .iter()
            .filter_map(|j| j.bun_timing.as_deref())
            .filter_map(|t| {
                if t.ends_with('s') {
                    t.trim_end_matches('s').parse::<f32>().ok()
                } else if t.ends_with("ms") {
                    t.trim_end_matches("ms")
                        .parse::<f32>()
                        .ok()
                        .map(|ms| ms / 1000.0)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .into_iter()
            .reduce(|a, b| a + b)
            .map(|sum| {
                let avg = sum / native_jobs.len().max(1) as f32;
                format!("{:.1}s", avg)
            });

        let best_perf = native_jobs
            .iter()
            .filter_map(|j| j.performance_score)
            .fold(0.0f32, |a, b| a.max(b));

        let stats = match (pkg_count > 0, avg_time.as_deref(), best_perf > 70.0) {
            (true, Some(t), true) => {
                format!(" · {} pkgs · {} · {:.0}% peak", pkg_count, t, best_perf)
            }
            (true, Some(t), _) => format!(" · {} pkgs · {}", pkg_count, t),
            (true, None, _) => format!(" · {} pkgs", pkg_count),
            _ => String::new(),
        };

        format!(" Jobs / Activity — 📦🚀 Native Bun (live){} ", stats)
    } else {
        " Jobs / Activity — live progress from python_bridge (g immediately enqueues) "
            .to_string()
    };

    let jobs_list = List::new(job_items).block(
        Block::default()
            .title(jobs_title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(styles::border_style()),
    );
    f.render_widget(jobs_list, chunks[2]);

    // Chunk 8 surface polish: live native Bun activity in the status bar
    let live_bun_indicator = app
        .jobs
        .iter()
        .rev()
        .find(|j| {
            (j.tool.starts_with("bun:")
                || j.tool.contains("package:")
                || j.tool.contains("script:"))
                && j.status == "running"
        })
        .and_then(|j| {
            if let Some(st) = &j.bun_stage {
                let icon = if j.tool.contains("run") || j.tool.contains("script") {
                    "🚀"
                } else {
                    "📦"
                };
                Some(format!(" {} {}  ", icon, st))
            } else {
                None
            }
        })
        .unwrap_or_default();

    // Status bar (with time)
    let status_text = format!(
        "{}{}   |   {}   |   {}",
        live_bun_indicator,
        app.status_message,
        if app.last_action.is_empty() {
            "—"
        } else {
            &app.last_action
        },
        Utc::now().format("%H:%M:%S")
    );
    let status = Paragraph::new(status_text)
        .style(styles::status_style())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(styles::border_style())
                .title(
                    " Status — q/esc quit • g gen(real) • a absorb • / filter • d dir • r • ? ",
                ),
        );

    // Bottom area: either normal status bar or the sharp 2-line Bun command palette
    if app.in_bun_command_mode {
        render_bun_command_palette(app, f, chunks[3]);
    } else {
        f.render_widget(status, chunks[3]);
    }

    // stacked toasts in top-right corner
    let mut y_offset = 1u16;
    for t in &app.toasts {
        let toast_w = 48.min(size.width);
        let toast_h = 2;
        let toast_area = Rect {
            x: size.width.saturating_sub(toast_w + 1),
            y: y_offset,
            width: toast_w,
            height: toast_h,
        };

        let base_style = if t.is_error {
            styles::error_style()
        } else {
            styles::success_style()
        };

        let toast = Paragraph::new(t.text.as_str())
            .style(base_style.add_modifier(Modifier::BOLD))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(styles::MOCHA_SURFACE0))
                    .style(Style::default().bg(styles::MOCHA_SURFACE0))
                    .title(if t.is_error { " ✗ " } else { " ✓ " }),
            );
        f.render_widget(toast, toast_area);

        y_offset += toast_h + 1;
        if y_offset + 2 > size.height {
            break;
        }
    }
}
