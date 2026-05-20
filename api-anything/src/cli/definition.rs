//! Clap command-line interface definition.
//! This is the single source of truth for the entire CLI surface.
//! Add new subcommands here, then wire them in main.rs and their module.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "api-anything",
    version,
    about = "Get an API (and a harness) from anything — native TUI + headless + ACP",
    long_about = "API Anything turns arbitrary tools, binaries, and descriptions into typed, \
                  agent-callable JSON APIs and CLI harnesses. Full-screen TUI, headless JSON, \
                  ACP/IDE Protocol, and daemon mode.",
    after_help = "Examples:\n  api-anything                     # launch full-screen TUI\n  \
                  api-anything generate bettercap --json\n  api-anything agent stdio\n  \
                  api-anything serve --port 2481"
)]
pub struct Cli {
    /// Optional profile name (isolates registry, config, generated artifacts)
    #[arg(short = 'p', long, global = true, env = "API_ANYTHING_PROFILE")]
    pub profile: Option<String>,

    /// Path to config file (defaults to ~/.api-anything/config.toml)
    #[arg(long, global = true, env = "API_ANYTHING_CONFIG")]
    pub config: Option<PathBuf>,

    /// Increase verbosity (can be repeated)
    #[arg(short = 'v', long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Machine-readable output (alias for --output-format json)
    #[arg(long, global = true)]
    pub json: bool,

    /// Output format for structured commands
    #[arg(long, global = true, value_enum, default_value = "plain")]
    pub output_format: OutputFormat,

    /// Suppress all non-essential output (useful in agents / CI)
    #[arg(long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Plain,
    Json,
    StreamingJson,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Launch the full-screen interactive TUI (default when no subcommand)
    Tui {
        /// Start directly in the generate wizard for a specific tool
        #[arg(long)]
        generate: Option<String>,
    },

    /// Generate an API wrapper + harness for a tool/binary/description
    Generate {
        /// Name or identifier of the tool (e.g. "bettercap", "my-binary", "nmap")
        name: String,

        /// Optional free-form description / purpose of the tool (becomes part of the generated docs)
        #[arg()]
        description: Option<String>,

        /// How to discover the tool's capabilities
        #[arg(long, value_enum, default_value = "auto")]
        from: SourceKind,

        /// Target language for the generated server
        #[arg(long, value_enum, default_value = "python")]
        lang: TargetLang,

        /// Output directory for generated files (defaults to ./<name>-api)
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,

        /// Force overwrite of existing files
        #[arg(long)]
        force: bool,

        /// Stream progress as newline-delimited JSON (implies --output-format streaming-json)
        #[arg(long)]
        stream: bool,

        /// Perform full absorption (CLI harness + API + basic tests + registration) via absorb.py when available
        #[arg(long)]
        absorb: bool,

        /// Additional free-form hints or constraints for the generator
        #[arg(long, num_args = 0..)]
        hint: Vec<String>,
    },

    /// Manage the local tool/API registry
    Registry {
        #[command(subcommand)]
        command: RegistryCommands,
    },

    /// Run as an ACP (Agent Client Protocol) server for IDE integration
    Agent {
        #[command(subcommand)]
        command: AgentCommands,
    },

    /// Start the HTTP/WebSocket daemon (API surface for remote agents + web)
    #[cfg(feature = "serve")]
    Serve {
        /// Bind address
        #[arg(long, default_value = "127.0.0.1:2481")]
        bind: String,

        /// Run in background (daemonize on Unix)
        #[arg(long)]
        daemon: bool,

        /// Path to write PID file
        #[arg(long)]
        pidfile: Option<PathBuf>,
    },

    /// Diagnose environment (python bridge, registry, templates, permissions)
    Doctor {
        /// Emit structured JSON report
        #[arg(long)]
        json: bool,
    },

    /// Generate shell completions
    Completion {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    // Internal commands are defined below the main enum for clarity.

    /// Drive the Bun semantic harness (scripts, package management)
    /// Uses the Python cli-anything-bun adapter under the hood.
    Bun {
        #[command(subcommand)]
        command: BunCommands,
    },

    /// Internal commands used by the Python bridge and absorb tooling.
    /// Not intended for direct use.
    #[command(hide = true)]
    Internal {
        #[command(subcommand)]
        command: InternalCommands,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum BunCommands {
    /// Script execution (bun run <name>)
    Script {
        #[command(subcommand)]
        command: BunScriptCommands,
    },

    /// Package management (add / install / remove)
    Package {
        #[command(subcommand)]
        command: BunPackageCommands,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum BunScriptCommands {
    /// Run a script defined in package.json (e.g. "dev", "build")
    Run {
        /// Script name from package.json
        name: String,

        /// Arguments passed through to the script
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum BunPackageCommands {
    /// Add one or more packages
    Add {
        packages: Vec<String>,

        #[arg(long)]
        dev: bool,

        #[arg(long)]
        exact: bool,

        #[arg(long)]
        peer: bool,

        #[arg(long)]
        optional: bool,
    },

    /// Install dependencies (from lockfile or package.json)
    Install {
        /// Optional specific packages
        packages: Vec<String>,

        #[arg(long)]
        frozen_lockfile: bool,
    },

    /// Remove packages
    Remove { packages: Vec<String> },
}

#[derive(Subcommand, Debug)]
pub enum RegistryCommands {
    /// List all known tools and generated APIs
    List {
        /// Filter by DNA tag (e.g. c2, recon, exploitation)
        #[arg(long)]
        tag: Option<String>,
    },

    /// Show detailed info for one entry
    Show { name: String },

    /// Add or update a tool spec from a file or stdin
    Add {
        /// Path to YAML/JSON spec (or - for stdin)
        spec: String,
    },

    /// Remove an entry
    Remove {
        name: String,
        /// Also delete any generated artifacts on disk
        #[arg(long)]
        purge: bool,
    },

    /// Rebuild the searchable index
    Reindex,
}

#[derive(Subcommand, Debug)]
pub enum AgentCommands {
    /// Run as a stdio JSON-RPC ACP server (primary IDE integration)
    Stdio {
        /// Auto-approve all generation / absorption requests (dangerous)
        #[arg(long)]
        yolo: bool,
    },

    /// Start an HTTP/WebSocket ACP relay (multiple IDEs can share one process)
    Serve {
        #[arg(long, default_value = "127.0.0.1:2480")]
        bind: String,
    },
}

/// Internal commands used by the Python bridge / absorb tooling to delegate
/// work back to the native Rust binary (especially for Bun execution).
#[derive(Subcommand, Debug)]
pub enum InternalCommands {
    /// Execute a Bun command using the best available runner (native preferred).
    /// Streams events as NDJSON. Intended for use by `cli_anything_bun` when
    /// running under api-anything.
    RunBun {
        #[command(subcommand)]
        command: BunCommands,
    },
}

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub enum SourceKind {
    Auto,
    Cli,
    Binary,
    Description,
    Repo,
    ExistingHarness,
}

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub enum TargetLang {
    Python,
    Rust,
    Go,
    Typescript,
    All,
}

impl Default for Cli {
    fn default() -> Self {
        Self::parse()
    }
}
