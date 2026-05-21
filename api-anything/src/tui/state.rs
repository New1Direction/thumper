use std::time::Instant;

/// A simple registry item shown in the TUI (will be replaced by real RegistryStore later).
#[derive(Debug, Clone)]
pub struct RegistryItem {
    pub name: String,
    pub kind: String,
    pub tags: Vec<String>,
    pub status: String,
    pub last_generated: Option<String>,
}

impl RegistryItem {
    pub fn preview_text(&self) -> String {
        format!(
            "Tool: {}\nKind: {}\nStatus: {}\nTags: {}\nLast generated: {}",
            self.name,
            self.kind,
            self.status,
            self.tags.join(", "),
            self.last_generated.as_deref().unwrap_or("never")
        )
    }
}

/// Simple job tracking for background generations. Enhanced for activity panel.
/// Bun jobs get special treatment (icons, raw output collapsing, better labels).
#[derive(Debug, Clone)]
pub struct Job {
    pub tool: String,
    pub status: String, // "running", "done", "error"
    pub message: String,
    pub pct: u8,
    #[allow(dead_code)]
    pub started: String,
    pub output_path: Option<String>,
    /// Bun-specific: count of raw stdout/stderr lines received
    pub raw_output_count: usize,
    /// Bun-specific: the most recent raw line (for collapsed display)
    pub last_raw_line: Option<String>,

    // === Native Bun parsing state (Chunk 7) ===
    /// Current logical stage (Resolving → Downloading → Verifying → Complete)
    pub bun_stage: Option<String>,
    /// Count of packages seen via "+ " or "updated " lines
    pub bun_package_count: usize,
    /// Captured timing from [1.20s] / [42ms] style prefixes
    pub bun_timing: Option<String>,
    /// Dev server URL (Local: or Listening on)
    pub bun_server_url: Option<String>,
    /// Live speed from native parser (e.g. "2.1MB/s")
    pub bun_speed: Option<String>,

    // Chunk 8 Delight Layer
    pub completion_flash_until: Option<Instant>,
    pub performance_score: Option<f32>,

    /// Captured last lines of stderr when the job failed (for Diagnostic Error Cards)
    pub error_diagnostics: Option<Vec<String>>,
}

/// Result of parsing a single line of native Bun output (Chunk 7).
#[derive(Default)]
pub struct BunLineParse {
    pub suggested_stage: Option<String>,
    pub package_increment: usize,
    pub timing: Option<String>,
    pub server_url: Option<String>,
    pub speed: Option<String>,
    pub is_meaningful: bool,
}

/// The legendary Bun Tagline Vault (30 lines of pure arrogance and charm)
pub fn get_finishing_tagline(score: f32) -> &'static str {
    let taglines = [
        // === GOD TIER (95+) - Maximum Arrogance ===
        "• Bun just violated several laws of physics. Again.",
        "• I finished before you even finished typing the command.",
        "• Your other tools are still initializing. Pathetic.",
        "• Light speed? Cute. I was already on the next frame.",
        "• The bunny didn’t even break eye contact.",
        "• Executed with extreme prejudice and zero remorse.",
        // === ELITE (90–95) - Smartass + Cocky ===
        "• Clean. Violent. Efficient. You’re welcome.",
        "• That was adorable. Now watch how it’s actually done.",
        "• Zero latency. Maximum disrespect for slower runtimes.",
        "• The engine smiled. It never smiles.",
        "• Finished so fast it left a vapor trail of shame.",
        "• Bun said 'hold my beer' and the beer is still cold.",
        // === VERY GOOD (82–90) - Edgy + Funny ===
        "• Respectable. For something that isn’t Bun.",
        "• Not bad. I’ve seen worse. Like, yesterday.",
        "• Solid. The bar is now slightly higher than the floor.",
        "• Executed with the grace of a caffeinated god.",
        "• Clean burn. No wasted cycles. Unlike your life choices.",
        "• That was fast. Don’t let it go to your head.",
        // === GOOD (70–82) - Arrogant but Playful ===
        "• Acceptable. The bunny is mildly impressed.",
        "• It works. Don’t get used to this level of competence.",
        "• Finished. No explosions detected. That’s a win.",
        "• Task complete. Try not to be too emotional about it.",
        "• We did it. Barely. But we did it.",
        "• Solid run. The bar was on the floor and we cleared it.",
        // === CHAOTIC / UNHINGED (Any score) ===
        "• Bun.exe has stopped giving a fuck.",
        "• That was so fast it made Node.js cry in the corner.",
        "• I didn’t even warm up. This was just a light stretch.",
        "• Physics called. It’s filing a restraining order.",
        "• Finished before your IDE even realized what happened.",
        "• The runtime is now judging your entire tech stack.",
        "• That was cute. Now go touch grass.",
        "• Bun just did a thing™ and it was illegal in 7 countries.",
    ];

    let index = match score {
        s if s >= 95.0 => (s as usize) % 6,
        s if s >= 90.0 => 6 + ((s as usize) % 6),
        s if s >= 82.0 => 12 + ((s as usize) % 6),
        s if s >= 70.0 => 18 + ((s as usize) % 6),
        _ => 24 + ((score as usize) % 6),
    };

    taglines[index.min(taglines.len() - 1)]
}

/// Parses a line of output from the native Bun runner and extracts
/// high-signal information for the Jobs panel.
pub fn parse_bun_native_line(line: &str) -> BunLineParse {
    let trimmed = line.trim();
    let mut result = BunLineParse::default();

    // 1. Timing brackets: [1.20s], [42ms], etc.
    if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed[start..].find(']') {
            let candidate = &trimmed[start + 1..start + end];
            if candidate.ends_with('s') || candidate.ends_with("ms") {
                result.timing = Some(candidate.to_string());
                result.is_meaningful = true;
            }
        }
    }

    // 2. Speed from native parser output (e.g. "2.1MB/s" or "12.4 MB/s" in the line)
    if let Some(pos) = trimmed
        .find("MB/s")
        .or_else(|| trimmed.find("kB/s"))
        .or_else(|| trimmed.find("GB/s"))
    {
        let start = trimmed[..pos]
            .rfind(|c: char| c.is_whitespace())
            .map(|p| p + 1)
            .unwrap_or(0);
        let speed_str = trimmed[start..pos + 4].trim();
        if !speed_str.is_empty() && speed_str.chars().any(|c| c.is_digit(10)) {
            result.speed = Some(speed_str.to_string());
            result.is_meaningful = true;
        }
    }

    // 3. Package tracking: lines containing " + " or " updated "
    if trimmed.contains(" + ") || trimmed.contains(" updated ") {
        result.package_increment = 1;
        result.is_meaningful = true;
        if result.suggested_stage.is_none() {
            result.suggested_stage = Some("Downloading packages".to_string());
        }
    }

    // 4. Dev server URLs
    if trimmed.starts_with("Local:") || trimmed.starts_with("Listening on") {
        if let Some(url) = trimmed.split_whitespace().find(|s| s.starts_with("http")) {
            result.server_url = Some(url.to_string());
            result.suggested_stage = Some("Running dev server".to_string());
            result.is_meaningful = true;
        }
    }

    // 5. Common stage transitions
    if trimmed.starts_with("Resolving packages") {
        result.suggested_stage = Some("Resolving packages".to_string());
        result.is_meaningful = true;
    } else if trimmed.contains("Saved lockfile") {
        result.suggested_stage = Some("Lockfile saved".to_string());
        result.is_meaningful = true;
    } else if trimmed.starts_with("Checked ") && trimmed.contains("packages") {
        result.suggested_stage = Some("Verifying installation".to_string());
        result.is_meaningful = true;
    } else if trimmed.contains("Installed ") && trimmed.contains("packages") {
        result.suggested_stage = Some("Installation complete".to_string());
        result.is_meaningful = true;
    }

    result
}

/// Map rich native Bun event names (`bun.native.<verb>.<stream>`) coming from the
/// native runner (via cli/bun.rs forwarder) to a human-friendly stage label.
/// These labels power `job.bun_stage`, status bar, and the 📦 / 🚀 icon prefixes
/// in the Jobs panel.
pub fn map_native_bun_event_to_stage(event: &str) -> Option<String> {
    if event.contains(".install.") {
        Some("Installing packages".to_string())
    } else if event.contains(".add.") {
        Some("Adding packages".to_string())
    } else if event.contains(".remove.") {
        Some("Removing packages".to_string())
    } else if event.contains(".run.") {
        Some("Running script".to_string())
    } else {
        None
    }
}

/// Transient success/error toast for beautiful feedback.
#[derive(Debug, Clone)]
pub struct Toast {
    pub text: String,
    pub is_error: bool,
    pub expires_at: Instant,
}

/// Context for Tab completion in the Bun command palette
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionContext {
    TopLevelVerb,
    ScriptName,
}

/// State for cycling through completions
#[derive(Debug, Clone)]
pub struct CompletionState {
    pub matches: Vec<String>,
    pub index: usize,
    pub original_word: String, // the word we started completing
    pub context: CompletionContext,
}

#[derive(Debug, Clone)]
pub enum GenUpdate {
    Progress {
        tool: String,
        stage: String,
        pct: u8,
        msg: String,
    },
    Done {
        tool: String,
        path: String,
    },
    Error {
        tool: String,
        err: String,
        diagnostics: Vec<String>,
    },
}

/// Explicit actions returned by input handling (per AGENTS.md TUI pattern).
/// Execution performed in handle_action so event loop stays clean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Quit,
    GenerateSelected,
    ToggleAbsorb,
    StartSearch,
    AppendFilter(char),
    BackspaceFilter,
    CommitFilter,
    CancelFilter,
    OpenDirForSelected,
    Refresh,
    ShowHelp,
    MoveUp,
    MoveDown,
    ActivateItem,
    /// Trigger a demo Bun job (package install) via the Python harness.
    /// Shows live streaming in the Jobs panel.
    TriggerBunJob,
    /// Open the Bun command palette (e.g. ":add hono --dev" or ":script run dev")
    OpenBunCommandPalette,
    /// Execute the current Bun command from the palette
    ExecuteBunCommand,
    /// Cancel Bun command palette
    CancelBunCommand,
    /// Trigger a predictive self-healing action for the most recent failed job
    /// (e.g. auto-running `bun install` after an ENOENT / missing lockfile error).
    ExecutePredictiveRecovery,
}
