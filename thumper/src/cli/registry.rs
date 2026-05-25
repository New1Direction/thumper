//! `api-anything registry` subcommands (list, show, add, ...)
//! Phase 1 stubs that operate on an in-memory / file-backed registry.

use crate::cli::output::print_json;
use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RegistryEntry {
    pub name: String,
    pub kind: String,
    pub tags: Vec<String>,
    pub last_generated: Option<String>,
    pub status: String,
}

pub async fn list(tag: Option<String>) -> Result<()> {
    // Real registry data (populated by generate --absorb / native paths via register_generated)
    let data = crate::registry::store::load().unwrap_or_default();
    let mut entries: Vec<RegistryEntry> = data
        .tools
        .into_iter()
        .map(|t| RegistryEntry {
            name: t.name,
            kind: t.kind,
            tags: vec![],
            last_generated: Some(t.last_generated),
            status: if t.absorbed {
                "absorbed".into()
            } else {
                "generated".into()
            },
        })
        .collect();

    if entries.is_empty() {
        // fallback demo data when registry empty
        entries = vec![RegistryEntry {
            name: "bettercap".into(),
            kind: "c2".into(),
            tags: vec!["c2".into(), "mitm".into()],
            last_generated: Some("2026-05-18".into()),
            status: "ok".into(),
        }];
    }

    if let Some(t) = tag {
        entries.retain(|e| e.tags.iter().any(|tag| tag.contains(&t)) || e.kind.contains(&t));
    }

    print_json(&entries)?;
    Ok(())
}

pub async fn show(name: String) -> Result<()> {
    // stub
    let entry = RegistryEntry {
        name,
        kind: "unknown".into(),
        tags: vec![],
        last_generated: None,
        status: "not-found (stub)".into(),
    };
    print_json(&entry)?;
    Ok(())
}
