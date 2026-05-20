//! Full-screen ratatui TUI for api-anything.
//! Entry point: `run()`. Called from main when the user wants the interactive dashboard.
//!
//! This is intentionally lightweight. The heavy lifting (real registry, generate wizard,
//! streaming output panel, command palette) will grow inside `app.rs` and submodules.

pub mod app;
pub mod job;
pub mod startup;
pub mod styles;
pub mod widgets;

pub use app::GenUpdate;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;

/// Launch the full-screen TUI.
/// Restores the terminal cleanly on success or error.
pub async fn run() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app
    let app_result = app::App::new().run(&mut terminal).await;

    // Always restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    app_result
}
