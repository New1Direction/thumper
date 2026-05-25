//! Thumper (thump) — The delightful native Bun TUI & CLI with rich telemetry.
//! Native Rust execution, living plasma visualization, four aliases, and full ACP / headless support.

#[cfg(feature = "acp")]
mod acp;
mod bun; // thump (formerly cli_anything_bun) Python harness adapter (NDJSON streaming + subprocess)
mod cli;
mod demo;
mod generator; // will contain python_bridge + engine later
mod registry;
#[cfg(feature = "tui")]
mod tui;

use anyhow::Result;
use clap::Parser;
use cli::definition::{AgentCommands, Commands, RegistryCommands};
use cli::output;
use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize standard SQLite database tables and run schema/data migration
    crate::registry::sqlite::init_db().ok();

    let cli = cli::definition::Cli::parse();

    if cli.debug_ancestry {
        println!("{}", crate::bun::get_ancestry_diagnostics());
        return Ok(());
    }

    if cli.demo {
        return crate::demo::run_demo().await;
    }

    // Logging setup — respect -v / -q / RUST_LOG
    let level = if cli.quiet {
        Level::WARN
    } else {
        match cli.verbose {
            0 => Level::INFO,
            1 => Level::DEBUG,
            _ => Level::TRACE,
        }
    };
    let filter = EnvFilter::from_default_env().add_directive(level.into());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    if let Some(profile) = &cli.profile {
        info!("using profile: {}", profile);
    }

    let use_json = cli.json || matches!(cli.output_format, cli::definition::OutputFormat::Json);

    match &cli.command {
        None | Some(Commands::Tui { .. }) => {
            // Full-screen TUI path (the primary interactive experience)
            // Never launch interactive TUI when the user asked for machine output
            if use_json {
                output::print_json(&serde_json::json!({
                    "mode": "tui",
                    "status": "interactive_only",
                    "hint": "run without --json"
                }))?;
                return Ok(());
            }

            #[cfg(feature = "tui")]
            {
                // Launch the real ratatui TUI
                if let Err(e) = tui::run().await {
                    eprintln!("TUI exited with error: {}", e);
                    std::process::exit(1);
                }
            }

            #[cfg(not(feature = "tui"))]
            {
                if use_json {
                    output::print_json(&serde_json::json!({
                        "mode": "tui",
                        "status": "disabled",
                        "hint": "rebuild with --features tui"
                    }))?;
                } else {
                    println!("api-anything TUI is not compiled in this build.");
                    println!("Rebuild with: cargo run --features tui");
                    println!("Headless usage still works:");
                    println!("  api-anything generate <name> --json --stream");
                }
            }
        }

        Some(Commands::Generate {
            name,
            description,
            from,
            lang,
            output,
            force,
            stream,
            absorb,
            hint,
        }) => {
            let mut hints = hint.clone();
            if let Some(desc) = description {
                hints.push(desc.to_string());
            }
            cli::generate::run(
                name.clone(),
                from.clone(),
                lang.clone(),
                output.clone(),
                *force,
                *stream,
                *absorb,
                hints,
            )
            .await?;
        }

        Some(Commands::Registry { command }) => match command {
            RegistryCommands::List { tag } => cli::registry::list(tag.clone()).await?,
            RegistryCommands::Show { name } => cli::registry::show(name.clone()).await?,
            _ => {
                eprintln!("registry subcommand not fully wired in Phase 1 stub");
            }
        },

        Some(Commands::Bun { command }) => {
            // Headless execution of the Bun harness (no TUI progress channel)
            cli::bun::run(command.clone(), None, None, None).await?;
        }

        Some(Commands::Internal { command }) => match command {
            cli::definition::InternalCommands::RunBun { command } => {
                // Internal path used by the Python bridge / absorb tooling.
                // This goes through the smart native-first selector.
                cli::bun::run(command.clone(), None, None, None).await?;
            }
            cli::definition::InternalCommands::DebugAncestry { json } => {
                if *json {
                    if let Ok(report) =
                        serde_json::to_string_pretty(&crate::bun::get_ancestry_report())
                    {
                        println!("{}", report);
                    } else {
                        println!("{{\"error\": \"failed to serialize ancestry report\"}}");
                    }
                } else {
                    println!("{}", crate::bun::get_ancestry_diagnostics());
                }
            }
        },

        Some(Commands::Agent { command }) => match command {
            AgentCommands::Stdio { yolo } => {
                #[cfg(feature = "acp")]
                {
                    if let Err(e) = acp::run_stdio_server(*yolo).await {
                        eprintln!("ACP server error: {}", e);
                    }
                }
                #[cfg(not(feature = "acp"))]
                {
                    println!("acp feature not enabled. Rebuild with --features acp");
                }
            }
            _ => eprintln!("other agent modes (serve) not fully wired yet"),
        },

        Some(Commands::Doctor { json }) => {
            let report = serde_json::json!({
                "rust": "ok",
                "cargo": "ok",
                "python_bridge": "stub (will call RedMicro api_wrapper_generator.py)",
                "registry_dir": dirs::home_dir().map(|h| h.join(".api-anything")).unwrap_or_default(),
                "phase": 1,
            });
            if *json {
                output::print_json(&report)?;
            } else {
                println!("{:#?}", report);
            }
        }

        Some(Commands::Completion { shell }) => {
            use clap::CommandFactory;
            let mut cmd = cli::definition::Cli::command();
            clap_complete::generate(*shell, &mut cmd, "api-anything", &mut std::io::stdout());
        }
    }

    Ok(())
}
