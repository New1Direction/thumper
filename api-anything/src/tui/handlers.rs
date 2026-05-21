use crate::tui::app::App;
use crate::tui::state::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Input handling returns Option<Action> (explicit wiring per AGENTS.md).
/// Search mode captures typing for filter; normal mode dispatches the rest.
pub fn handle_key(app: &mut App, key: KeyEvent) -> Option<Action> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(Action::Quit);
    }

    // Search mode: collect filter chars
    if app.in_search_mode {
        match key.code {
            KeyCode::Char(c) => return Some(Action::AppendFilter(c)),
            KeyCode::Backspace => return Some(Action::BackspaceFilter),
            KeyCode::Enter => return Some(Action::CommitFilter),
            KeyCode::Esc => return Some(Action::CancelFilter),
            _ => return None,
        }
    }

    // Bun Command Palette mode (":add hono --dev", ":script run dev", etc.)
    if app.in_bun_command_mode {
        // Any keypress while the palette is open clears the transient inline error (high-visibility feedback UX)
        app.palette_error_message = None;

        match key.code {
            KeyCode::Char(c) => {
                app.bun_history_index = None;
                app.completion_state = None;
                // Insert at cursor position (real mid-string editing)
                app.bun_command_buffer.insert(app.bun_cursor_index, c);
                app.bun_cursor_index += 1;
                app.status_message = format!("Bun: {}", app.bun_command_buffer);
                return None;
            }
            KeyCode::Backspace => {
                app.bun_history_index = None;
                app.completion_state = None;
                if app.bun_cursor_index > 0 {
                    app.bun_command_buffer.remove(app.bun_cursor_index - 1);
                    app.bun_cursor_index -= 1;
                }
                app.status_message = if app.bun_command_buffer.is_empty() {
                    "Bun command (Esc to cancel)".to_string()
                } else {
                    format!("Bun: {}", app.bun_command_buffer)
                };
                return None;
            }
            KeyCode::Delete => {
                app.bun_history_index = None;
                app.completion_state = None;
                if app.bun_cursor_index < app.bun_command_buffer.len() {
                    app.bun_command_buffer.remove(app.bun_cursor_index);
                }
                app.status_message = format!("Bun: {}", app.bun_command_buffer);
                return None;
            }
            KeyCode::Left => {
                app.bun_history_index = None;
                if app.bun_cursor_index > 0 {
                    app.bun_cursor_index -= 1;
                }
                app.status_message = format!("Bun: {}", app.bun_command_buffer);
                return None;
            }
            KeyCode::Right => {
                app.bun_history_index = None;
                if app.bun_cursor_index < app.bun_command_buffer.len() {
                    app.bun_cursor_index += 1;
                }
                app.completion_state = None;
                app.status_message = format!("Bun: {}", app.bun_command_buffer);
                return None;
            }
            KeyCode::Home | KeyCode::Char('a')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                app.bun_history_index = None;
                app.bun_cursor_index = 0;
                app.completion_state = None;
                app.status_message = format!("Bun: {}", app.bun_command_buffer);
                return None;
            }
            KeyCode::End | KeyCode::Char('e')
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                app.bun_history_index = None;
                app.bun_cursor_index = app.bun_command_buffer.len();
                app.completion_state = None;
                app.status_message = format!("Bun: {}", app.bun_command_buffer);
                return None;
            }
            KeyCode::Up => {
                app.completion_state = None;
                app.navigate_bun_history(true);
                return None;
            }
            KeyCode::Down => {
                app.completion_state = None;
                app.navigate_bun_history(false);
                return None;
            }
            KeyCode::Tab => {
                app.handle_tab_completion(false);
                return None;
            }
            KeyCode::BackTab => {
                // Shift+Tab
                app.handle_tab_completion(true);
                return None;
            }
            KeyCode::Enter => return Some(Action::ExecuteBunCommand),
            KeyCode::Esc => return Some(Action::CancelBunCommand),
            _ => return None,
        }
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),

        KeyCode::Char('g') => Some(Action::GenerateSelected),

        KeyCode::Char('a') => Some(Action::ToggleAbsorb),

        KeyCode::Char('/') => Some(Action::StartSearch),

        KeyCode::Char('d') => Some(Action::OpenDirForSelected),

        KeyCode::Char('r') => Some(Action::Refresh),

        KeyCode::Down | KeyCode::Char('j') => Some(Action::MoveDown),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::MoveUp),

        KeyCode::Enter => Some(Action::ActivateItem),

        KeyCode::Char('?') => Some(Action::ShowHelp),

        KeyCode::Char('b') => Some(Action::TriggerBunJob),

        // Colon enters Bun command palette (very powerful for power users)
        KeyCode::Char(':') => Some(Action::OpenBunCommandPalette),

        // Predictive recovery keys — only meaningful when a failed job with diagnostics exists
        KeyCode::Char('i') | KeyCode::Char('I') => Some(Action::ExecutePredictiveRecovery),
        KeyCode::Char('n') | KeyCode::Char('N') => Some(Action::ExecutePredictiveRecovery),

        _ => None,
    }
}

/// Execute the action (all side effects and real work dispatch here).
pub fn handle_action(app: &mut App, action: Action) {
    match action {
        Action::Quit => {
            app.should_quit = true;
        }
        Action::GenerateSelected => {
            app.trigger_generation();
        }
        Action::ToggleAbsorb => {
            app.absorb_mode = !app.absorb_mode;
            app.status_message = format!(
                "Absorb mode: {} (richer models+tests via absorb.py)",
                if app.absorb_mode { "ON" } else { "OFF" }
            );
        }
        Action::StartSearch => {
            app.in_search_mode = true;
            app.filter.clear();
            app.status_message =
                "Search: type to filter registry (fuzzy). Enter/Esc to finish.".to_string();
        }
        Action::AppendFilter(c) => {
            app.filter.push(c);
            app.status_message = format!("Filter: /{}", app.filter);
            app.adjust_selection_after_filter();
        }
        Action::BackspaceFilter => {
            app.filter.pop();
            app.status_message = if app.filter.is_empty() {
                "Filter: (cleared)".to_string()
            } else {
                format!("Filter: /{}", app.filter)
            };
            app.adjust_selection_after_filter();
        }
        Action::CommitFilter | Action::CancelFilter => {
            app.in_search_mode = false;
            if matches!(action, Action::CancelFilter) {
                app.filter.clear();
            }
            app.status_message = if app.filter.is_empty() {
                "Filter cleared".to_string()
            } else {
                format!(
                    "Filter active: {} items shown",
                    app.visible_indices().len()
                )
            };
        }
        Action::OpenDirForSelected => {
            app.open_selected_dir();
        }
        Action::Refresh => {
            app.status_message = "Registry refreshed (in-memory + dynamic gens)".to_string();
        }
        Action::ShowHelp => {
            app.status_message = "Keys: ↑↓/jk nav • g generate • a absorb • b bun (context) • : palette (add/run/install) • / filter • d open-dir • r refresh • ? help • q quit".to_string();
        }
        Action::MoveUp => app.move_previous(),
        Action::MoveDown => app.move_next(),
        Action::ActivateItem => {
            if let Some(sel) = app.selected {
                if let Some(item) = app.items.get(sel) {
                    app.status_message = format!(
                        "Selected: {} (kind={}). Press g to generate.",
                        item.name, item.kind
                    );
                }
            }
        }
        Action::TriggerBunJob => {
            app.trigger_bun_job();
        }
        Action::ExecutePredictiveRecovery => {
            app.execute_predictive_recovery();
        }
        Action::OpenBunCommandPalette => {
            app.in_bun_command_mode = true;
            app.bun_history_index = None;

            // Smart pre-fill based on current selection
            if let Some(sel) = app.selected {
                if sel < app.items.len() {
                    let item = &app.items[sel];
                    let name = &item.name;
                    let kind = &item.kind;
                    let tags = &item.tags;

                    if kind.contains("web")
                        || kind.contains("framework")
                        || kind.contains("server")
                        || tags
                            .iter()
                            .any(|t| t.contains("web") || t.contains("framework"))
                    {
                        app.bun_command_buffer = format!("add {}", name);
                    } else if kind.contains("cli")
                        || tags.iter().any(|t| t == "cli" || t == "tool")
                    {
                        app.bun_command_buffer = format!("add {} --dev", name);
                    } else {
                        app.bun_command_buffer = format!("add {}", name);
                    }
                } else {
                    app.bun_command_buffer.clear();
                }
            } else {
                app.bun_command_buffer.clear();
            }

            app.status_message = if app.bun_command_buffer.is_empty() {
                "Bun command: (e.g. add hono --dev, script run dev, install)".to_string()
            } else {
                format!("Bun: {}", app.bun_command_buffer)
            };
        }
        Action::ExecuteBunCommand => {
            app.execute_bun_command_from_palette();
        }
        Action::CancelBunCommand => {
            app.in_bun_command_mode = false;
            app.bun_command_buffer.clear();
            app.bun_cursor_index = 0;
            app.bun_history_index = None;
            app.completion_state = None;
            app.palette_error_message = None;
            app.status_message = "Bun command cancelled".to_string();
        }
    }
}
