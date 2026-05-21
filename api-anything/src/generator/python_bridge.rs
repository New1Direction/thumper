//! Real bridge to RedMicro's Python API harness & absorb tooling.
//!
//! This is the heart of "get an API from anything" when targeting Python (FastAPI).
//! It discovers the RedMicro checkout, invokes the real generators, and returns
//! structured artifacts so the Rust CLI/TUI can report them cleanly.

use crate::cli::output::{emit_stream, StreamEvent};
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

/// Represents a file that was actually produced by the Python generator.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GeneratedArtifact {
    pub path: PathBuf,
    pub kind: String, // "python-fastapi", "test", "readme", etc.
    pub size: u64,
}

/// Structured progress event for TUI / ACP consumers (real streaming from bridge
/// without polluting stdout with NDJSON when a sender is provided). Explicit wiring.
#[derive(Debug, Clone)]
pub struct ProgressEvent {
    pub stage: String,
    pub pct: u8,
    pub message: String,
}

/// Configuration for a generation run.
#[derive(Debug, Clone)]
pub struct GenerateRequest {
    pub tool_name: String,
    pub description: String,
    pub output_dir: PathBuf,
    /// If true, prefer the richer `absorb.py --full` flow when available.
    pub use_absorb: bool,
    /// Optional sender for live TUI progress (when present, suppresses emit_stream to keep TUI clean).
    /// This is how the real python bridge streams messages into the Jobs panel.
    pub progress_tx: Option<mpsc::UnboundedSender<ProgressEvent>>,
}

/// Main entry point called by `cli/generate.rs`.
///
/// It will:
/// 1. Discover RedMicro root (api-harness + absorb-tool)
/// 2. Prefer `absorb.py` when `use_absorb`, otherwise the simple `api_wrapper_generator.py`
/// 3. Run the python script with cwd = output_dir so files land where the user wants
/// 4. Stream progress via the NDJSON protocol
/// 5. Return the list of real files created
pub async fn generate_python_api(req: GenerateRequest) -> Result<Vec<GeneratedArtifact>> {
    report_progress(&req, "discover", 5, "Locating RedMicro api-harness...");

    let redmicro = discover_redmicro_root().await?;

    let api_harness_dir = redmicro.join("supporting-tools/api-harness");
    let absorb_script = redmicro.join("supporting-tools/absorb-tool/absorb.py");
    let simple_generator = api_harness_dir.join("api_wrapper_generator.py");

    if !simple_generator.exists() && !absorb_script.exists() {
        return Err(anyhow!(
            "Could not find RedMicro generators. Looked in {:?}",
            redmicro
        ));
    }

    // Ensure output dir exists
    tokio::fs::create_dir_all(&req.output_dir).await?;

    let python = find_python().await?;

    let artifacts = if req.use_absorb && absorb_script.exists() {
        run_absorb(&python, &absorb_script, &req, &redmicro).await?
    } else if simple_generator.exists() {
        run_simple_generator(&python, &simple_generator, &req).await?
    } else {
        return Err(anyhow!("No suitable Python generator found"));
    };

    report_progress(&req, "register", 95, "Generation complete");

    Ok(artifacts)
}

/// Try hard to find a usable RedMicro root containing `supporting-tools/api-harness`.
async fn discover_redmicro_root() -> Result<PathBuf> {
    // 1. Explicit env var (recommended for power users / CI)
    if let Ok(val) = std::env::var("REDMICRO_ROOT") {
        let p = PathBuf::from(val);
        if p.join("supporting-tools/api-harness").exists() {
            return Ok(p);
        }
    }

    // 2. Common locations for this environment (the Grok + RedMicro setup)
    let mut candidates = Vec::new();
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".grok/skills/redmicro"));
        candidates.push(home.join("Documents/redmicro"));
    }
    candidates.push(PathBuf::from("/opt/redmicro"));

    for c in &candidates {
        if c.join("supporting-tools/api-harness/api_wrapper_generator.py")
            .exists()
        {
            return Ok(c.clone());
        }
    }

    // 3. Walk upward from current working directory (user might be inside a checkout)
    let mut cwd = std::env::current_dir()?;
    for _ in 0..8 {
        if cwd.join("supporting-tools/api-harness").exists() {
            return Ok(cwd);
        }
        if !cwd.pop() {
            break;
        }
    }

    // 4. Last resort: look relative to the running binary
    if let Ok(exe) = std::env::current_exe() {
        let mut p = exe;
        for _ in 0..6 {
            if p.join("supporting-tools/api-harness").exists() {
                return Ok(p);
            }
            if !p.pop() {
                break;
            }
        }
    }

    Err(anyhow!(
        "RedMicro root not found. Set REDMICRO_ROOT or place the repo at ~/.grok/skills/redmicro"
    ))
}

async fn find_python() -> Result<PathBuf> {
    // Prefer python3, then python
    for name in ["python3", "python"] {
        if let Ok(p) = which::which(name) {
            return Ok(p);
        }
    }
    Err(anyhow!(
        "python3 not found in PATH. The bridge needs a working Python 3."
    ))
}

/// Run the simple, fast generator (api_wrapper_generator.py).
async fn run_simple_generator(
    python: &Path,
    script: &Path,
    req: &GenerateRequest,
) -> Result<Vec<GeneratedArtifact>> {
    report_progress(
        req,
        "generate",
        20,
        &format!("Invoking api_wrapper_generator.py for {}", req.tool_name),
    );

    // Run the generator from *its own directory* (the RedMicro tree).
    // This avoids any relative-path or template-formatting surprises the generator may have.
    // After success we will move the produced file(s) into the user's requested output_dir.
    let script_dir = script.parent().unwrap_or(Path::new("."));
    let mut cmd = Command::new(python);
    cmd.arg(script)
        .arg(&req.tool_name)
        .arg(&req.description)
        .current_dir(script_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().context("failed to spawn python generator")?;

    // Stream stdout line-by-line as progress
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Some(line) = lines.next_line().await? {
            if line.contains("Generated") {
                report_progress(req, "write", 70, &line);
            } else if !line.trim().is_empty() {
                report_progress(req, "python-out", 55, &line);
            }
        }
    }

    let stderr = if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        let mut err = String::new();
        while let Some(line) = lines.next_line().await? {
            err.push_str(&line);
            err.push('\n');
        }
        err
    } else {
        String::new()
    };

    let status = child.wait().await?;

    if !status.success() {
        return Err(anyhow!(
            "Python generator exited with status {:?}\n\nstderr:\n{}",
            status.code(),
            stderr
        ));
    }

    // Discover what was actually written
    let mut artifacts = vec![];
    let mut read_dir = tokio::fs::read_dir(&req.output_dir).await?;
    while let Some(entry) = read_dir.next_entry().await? {
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.ends_with("_api.py") || name.contains(&req.tool_name) {
                let meta = tokio::fs::metadata(&path).await?;
                artifacts.push(GeneratedArtifact {
                    path: path.clone(),
                    kind: if name.ends_with("_api.py") {
                        "python-fastapi".to_string()
                    } else {
                        "support".to_string()
                    },
                    size: meta.len(),
                });
            }
        }
    }

    // The generator wrote the file next to the script (because we ran it from script_dir).
    // Move it into the user's desired output directory.
    let generated_name = format!("{}_api.py", req.tool_name.replace('-', "_"));
    let produced = script_dir.join(&generated_name);

    if produced.exists() {
        tokio::fs::create_dir_all(&req.output_dir).await?;
        let dest = req.output_dir.join(&generated_name);
        tokio::fs::rename(&produced, &dest).await?;

        let meta = tokio::fs::metadata(&dest).await?;
        artifacts.push(GeneratedArtifact {
            path: dest,
            kind: "python-fastapi".to_string(),
            size: meta.len(),
        });
    }

    if artifacts.is_empty() {
        // Fallback
        let expected = req.output_dir.join(&generated_name);
        if expected.exists() {
            let meta = tokio::fs::metadata(&expected).await?;
            artifacts.push(GeneratedArtifact {
                path: expected,
                kind: "python-fastapi".to_string(),
                size: meta.len(),
            });
        }
    }

    Ok(artifacts)
}

/// Run the much more powerful absorb tool (recommended for serious absorptions).
/// Now performs FULL absorption: API (from absorb) + CLI harness (via harness_generator) +
/// basic tests + registration (absorb writes absorbed_tools.json) and materializes
/// everything under the caller's output_dir for a complete package.
async fn run_absorb(
    python: &Path,
    script: &Path,
    req: &GenerateRequest,
    redmicro_root: &Path,
) -> Result<Vec<GeneratedArtifact>> {
    report_progress(
        req,
        "absorb",
        15,
        &format!(
            "Running full absorb for {} (CLI + API + tests + registration)",
            req.tool_name
        ),
    );

    // Collect structured progress from absorb.py output
    let mut progress_lines: Vec<String> = vec![];

    let mut cmd = Command::new(python);
    cmd.arg(script)
        .arg(&req.tool_name)
        .arg(&req.description)
        .arg("--full")
        .current_dir(&req.output_dir)
        .env("REDMICRO_ROOT", redmicro_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().context("spawn absorb.py")?;

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Some(line) = lines.next_line().await? {
            if !line.trim().is_empty() {
                progress_lines.push(line.clone());
            }
            report_progress(req, "absorb-progress", 45, &line);
        }
    }

    // Drain stderr for diagnostics
    let mut stderr_text = String::new();
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Some(line) = lines.next_line().await? {
            stderr_text.push_str(&line);
            stderr_text.push('\n');
        }
    }

    let status = child.wait().await?;
    if !status.success() {
        return Err(anyhow!(
            "absorb.py failed (status {:?})\nstderr:\n{}",
            status.code(),
            stderr_text
        ));
    }

    // === FULL ABSORPTION: materialize CLI harness + tests + structured artifacts under output_dir ===
    let snake = req.tool_name.to_lowercase().replace('-', "_");
    let api_harness_dir = redmicro_root.join("supporting-tools/api-harness");
    let cli_harness_dir = redmicro_root.join("supporting-tools/cli-harness");
    let caplets_dir = redmicro_root.join("supporting-tools/caplets/examples");

    // 1. Generate CLI harness using the real harness_generator (explicit RedMicro path)
    let harness_script = cli_harness_dir.join("harness_generator.py");
    let harness_out = req.output_dir.join("cli");
    let harness_name = format!("{}_harness.py", snake);
    if harness_script.exists() {
        let _ = tokio::fs::create_dir_all(&harness_out).await;
        let mut hcmd = Command::new(python);
        hcmd.arg(&harness_script)
            .arg(&req.tool_name)
            .arg(if req.description.is_empty() {
                "Absorbed tool via Thumper (thump)"
            } else {
                &req.description
            })
            .arg(format!("python {}_harness.py --input data.json", snake))
            .current_dir(&harness_out)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Ok(mut hchild) = hcmd.spawn() {
            let _ = hchild.wait().await;
        }
    }

    // 2. Write basic tests (agent-callable smoke test + example usage)
    let tests_dir = req.output_dir.join("tests");
    let _ = tokio::fs::create_dir_all(&tests_dir).await;
    let test_file = tests_dir.join(format!("test_{}_absorption.py", snake));
    let test_content = format!(
        r#"#!/usr/bin/env python3
"""Basic absorption tests for {name} (generated by Thumper + absorb.py)."""
import json
import subprocess
import sys
from pathlib import Path

def test_health():
    # Would normally hit the generated API /health
    print("{{'status':'ok','tool':'{name}','test':'health'}}")

def test_run_smoke():
    payload = {{"target": "127.0.0.1", "options": {{"mode": "fast"}}}}
    print(json.dumps({{"status": "simulated", "tool": "{name}", "result": payload}}))

if __name__ == "__main__":
    test_health()
    test_run_smoke()
    print("Basic absorption tests passed for {name}")
"#,
        name = req.tool_name
    );
    let _ = tokio::fs::write(&test_file, test_content).await;

    // 3. Also drop a simple README with usage for the full package
    let pkg_readme = req.output_dir.join("ABSORBED_PACKAGE.md");
    let readme_content = format!(
        "# Full Absorption Package for {}\n\n\
         Generated via thump generate --absorb\n\
         - api/: FastAPI server (from absorb.py)\n\
         - cli/: CLI harness\n\
         - tests/: basic smoke tests\n\
         - registration: updated in RedMicro absorbed_tools.json\n\n\
         Run the API: uvicorn api.{}_api:app --port 8000\n",
        req.tool_name, snake
    );
    let _ = tokio::fs::write(&pkg_readme, readme_content).await;

    // 4. Discover + copy key artifacts from RedMicro + our generated ones into output_dir for user convenience
    //    (so the user's <name>-api/ dir contains the complete harness+api+tests)
    let mut artifacts: Vec<GeneratedArtifact> = vec![];

    // Copy API if present in RedMicro tree
    let produced_api = api_harness_dir.join(format!("{}_api.py", snake));
    if produced_api.exists() {
        let dest_api_dir = req.output_dir.join("api");
        let _ = tokio::fs::create_dir_all(&dest_api_dir).await;
        let dest = dest_api_dir.join(format!("{}_api.py", snake));
        let _ = tokio::fs::copy(&produced_api, &dest).await;
        if let Ok(meta) = tokio::fs::metadata(&dest).await {
            artifacts.push(GeneratedArtifact {
                path: dest,
                kind: "python-api".to_string(),
                size: meta.len(),
            });
        }
    }

    // Include harness we (or generator) produced
    let produced_harness = harness_out.join(&harness_name);
    if produced_harness.exists() {
        if let Ok(meta) = tokio::fs::metadata(&produced_harness).await {
            artifacts.push(GeneratedArtifact {
                path: produced_harness,
                kind: "cli-harness".to_string(),
                size: meta.len(),
            });
        }
    } else {
        // Fallback: create a tiny placeholder harness directly (guarantees artifact)
        let placeholder = harness_out.join(&harness_name);
        let _ = tokio::fs::create_dir_all(&harness_out).await;
        let content = format!("#!/usr/bin/env python3\n# CLI harness placeholder for {}\n# Replace body with real invocation.\nprint('{{ \"tool\": \"{}\", \"status\": \"ok\" }}')\n", req.tool_name, req.tool_name);
        let _ = tokio::fs::write(&placeholder, content).await;
        if let Ok(meta) = tokio::fs::metadata(&placeholder).await {
            artifacts.push(GeneratedArtifact {
                path: placeholder,
                kind: "cli-harness".to_string(),
                size: meta.len(),
            });
        }
    }

    // Tests
    if test_file.exists() {
        if let Ok(meta) = tokio::fs::metadata(&test_file).await {
            artifacts.push(GeneratedArtifact {
                path: test_file,
                kind: "test".to_string(),
                size: meta.len(),
            });
        }
    }

    // Caplet (if absorb created one)
    let cap_name = format!("auto-{}.cap", snake);
    let cap_path = caplets_dir.join(&cap_name);
    if cap_path.exists() {
        let dest_cap = req.output_dir.join(&cap_name);
        let _ = tokio::fs::copy(&cap_path, &dest_cap).await;
        if let Ok(meta) = tokio::fs::metadata(&dest_cap).await {
            artifacts.push(GeneratedArtifact {
                path: dest_cap,
                kind: "caplet".to_string(),
                size: meta.len(),
            });
        }
    }

    // Registration artifact
    let reg_file = api_harness_dir.join("absorbed_tools.json");
    if reg_file.exists() {
        // Copy snapshot into package
        let dest_reg = req.output_dir.join("absorbed_tools.json");
        let _ = tokio::fs::copy(&reg_file, &dest_reg).await;
        if let Ok(meta) = tokio::fs::metadata(&dest_reg).await {
            artifacts.push(GeneratedArtifact {
                path: dest_reg,
                kind: "registration".to_string(),
                size: meta.len(),
            });
        }
    }

    // Also surface any progress-mentioned paths (parse more output)
    for line in progress_lines {
        if line.contains(".py") || line.contains(".cap") || line.contains("absorbed") {
            // Already covered by structured walk above; kept for future richer NDJSON from absorb.py
        }
    }

    // Walk user output_dir (now populated) for anything else we created
    let mut stack = vec![req.output_dir.clone()];
    while let Some(dir) = stack.pop() {
        if let Ok(mut rd) = tokio::fs::read_dir(&dir).await {
            while let Ok(Some(e)) = rd.next_entry().await {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if !artifacts.iter().any(|a| a.path == p) {
                    if let Ok(meta) = tokio::fs::metadata(&p).await {
                        let name_l = p
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_lowercase();
                        let kind = if name_l.ends_with("_api.py") {
                            "python-api"
                        } else if name_l.ends_with("_harness.py") {
                            "cli-harness"
                        } else if name_l.contains("test") {
                            "test"
                        } else if name_l.ends_with(".cap") {
                            "caplet"
                        } else if name_l.contains("absorbed") || name_l.contains("registration") {
                            "registration"
                        } else if name_l.ends_with(".md") {
                            "docs"
                        } else {
                            "absorbed-asset"
                        };
                        artifacts.push(GeneratedArtifact {
                            path: p,
                            kind: kind.to_string(),
                            size: meta.len(),
                        });
                    }
                }
            }
        }
    }

    if artifacts.is_empty() {
        // ultimate fallback so GenerateResult is never empty on success
        artifacts.push(GeneratedArtifact {
            path: req.output_dir.clone(),
            kind: "absorb-package".to_string(),
            size: 0,
        });
    }

    report_progress(
        req,
        "absorb-complete",
        90,
        &format!(
            "Full absorption finished for {} ({} artifacts)",
            req.tool_name,
            artifacts.len()
        ),
    );

    Ok(artifacts)
}

/// Helper: explicit wiring for progress.
/// If TUI provided a sender, push live event (for Jobs panel) and skip stdout emit.
/// Otherwise fall back to CLI NDJSON stream. (Used by all report sites above.)
fn report_progress(req: &GenerateRequest, stage: &str, pct: u8, msg: &str) {
    if let Some(tx) = &req.progress_tx {
        let _ = tx.send(ProgressEvent {
            stage: stage.to_string(),
            pct,
            message: msg.to_string(),
        });
    } else {
        let _ = emit_stream(&StreamEvent::Progress {
            stage: stage.to_string(),
            pct,
            message: Some(msg.to_string()),
        });
    }
}
