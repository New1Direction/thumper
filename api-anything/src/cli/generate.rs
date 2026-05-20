//! `api-anything generate` — real implementation backed by the Python bridge
//! (and later native Rust/Go emitters).

use crate::cli::definition::{SourceKind, TargetLang};
use crate::cli::output::{emit_stream, print_json, StreamEvent};
use crate::generator::python_bridge::{generate_python_api, GenerateRequest, GeneratedArtifact};
use anyhow::Result;
use chrono::Utc;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, serde::Serialize)]
pub struct GenerateResult {
    pub id: String,
    pub name: String,
    pub source: SourceKind,
    pub lang: TargetLang,
    pub output_dir: PathBuf,
    pub artifacts: Vec<Artifact>,
    pub duration_ms: u64,
    pub status: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Artifact {
    pub path: String,
    pub kind: String,
    pub size: u64,
}

impl From<GeneratedArtifact> for Artifact {
    fn from(a: GeneratedArtifact) -> Self {
        Self {
            path: a.path.display().to_string(),
            kind: a.kind,
            size: a.size,
        }
    }
}

pub async fn run(
    name: String,
    from: SourceKind,
    lang: TargetLang,
    output: Option<PathBuf>,
    _force: bool,
    stream: bool,
    absorb: bool,
    hints: Vec<String>,
) -> Result<()> {
    let start = Instant::now();
    let id = format!("gen-{}", Utc::now().format("%Y%m%d-%H%M%S"));

    let out_dir = output.unwrap_or_else(|| PathBuf::from(format!("{}-api", name)));
    let description = hints.join(" ");

    if stream {
        emit_stream(&StreamEvent::Progress {
            stage: "planning".into(),
            pct: 5,
            message: Some(format!(
                "Generating {} API for {}",
                lang_as_str(&lang),
                name
            )),
        })?;
    }

    let use_absorb = absorb
        || hints
            .iter()
            .any(|h| h.to_lowercase().contains("absorb") || h.to_lowercase().contains("full"));

    // Dispatch by target language: Python uses the real RedMicro bridge (with absorb if requested);
    // Rust/Go use the new native emitters (working, immediately runnable code).
    let artifacts: Vec<Artifact> = match lang {
        TargetLang::Python => {
            let req = GenerateRequest {
                tool_name: name.clone(),
                description: description.clone(),
                output_dir: out_dir.clone(),
                use_absorb,
                progress_tx: None,
            };

            let real_artifacts = generate_python_api(req).await?;

            real_artifacts.into_iter().map(Artifact::from).collect()
        }
        TargetLang::Rust => {
            if stream {
                emit_stream(&StreamEvent::Progress {
                    stage: "native".into(),
                    pct: 10,
                    message: Some("Emitting native Rust axum server...".into()),
                })?;
            }
            let real =
                crate::generator::native::generate_rust_axum(&name, &description, &out_dir).await?;
            real.into_iter().map(Artifact::from).collect()
        }
        TargetLang::Go => {
            if stream {
                emit_stream(&StreamEvent::Progress {
                    stage: "native".into(),
                    pct: 10,
                    message: Some("Emitting native Go HTTP server...".into()),
                })?;
            }
            let real =
                crate::generator::native::generate_go_http(&name, &description, &out_dir).await?;
            real.into_iter().map(Artifact::from).collect()
        }
        _ => {
            if stream {
                emit_stream(&StreamEvent::Progress {
                    stage: "warning".into(),
                    pct: 30,
                    message: Some(format!(
                        "{} emitter not yet implemented — falling back to Python",
                        lang_as_str(&lang)
                    )),
                })?;
            }
            let req = GenerateRequest {
                tool_name: name.clone(),
                description,
                output_dir: out_dir.clone(),
                use_absorb: false,
                progress_tx: None,
            };
            let real = generate_python_api(req).await?;
            real.into_iter().map(Artifact::from).collect()
        }
    };

    // After generation (especially full --absorb or native), update the local registry.
    // This is the explicit registry update logic wired from the generate path.
    let absorbed = use_absorb && matches!(lang, TargetLang::Python);
    let art_paths: Vec<String> = artifacts.iter().map(|a| a.path.clone()).collect();
    let _ = crate::registry::register_generated(&name, &out_dir, art_paths, absorbed).await;

    let duration = start.elapsed().as_millis() as u64;

    if stream {
        for art in &artifacts {
            emit_stream(&StreamEvent::Artifact {
                path: art.path.clone(),
                kind: art.kind.clone(),
                size: Some(art.size),
            })?;
        }
        emit_stream(&StreamEvent::End {
            status: "ok".into(),
            id: Some(id.clone()),
            duration_ms: Some(duration),
        })?;
        return Ok(());
    }

    // Non-streaming JSON path
    let result = GenerateResult {
        id,
        name: name.clone(),
        source: from,
        lang,
        output_dir: out_dir.clone(),
        artifacts: artifacts.clone(),
        duration_ms: duration,
        status: "ok".into(),
    };

    print_json(&result)?;

    // Nice human output when not in JSON mode
    if std::env::var("API_ANYTHING_QUIET").is_err() {
        eprintln!("\n✓ Generated real artifacts for {}", name);
        eprintln!("  Output directory: {}", out_dir.display());
        for a in &artifacts {
            eprintln!("    • {} ({})", a.path, a.kind);
        }
    }

    Ok(())
}

fn lang_as_str(l: &TargetLang) -> &'static str {
    match l {
        TargetLang::Python => "Python FastAPI",
        TargetLang::Rust => "Rust axum",
        TargetLang::Go => "Go",
        TargetLang::Typescript => "TypeScript",
        TargetLang::All => "all languages",
    }
}
