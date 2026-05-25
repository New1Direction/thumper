//! On-disk registry (json or sqlite later).
//! Now supports real registration after generation/absorption.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegistryData {
    pub tools: Vec<RegistryTool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryTool {
    pub name: String,
    pub kind: String,
    pub output_dir: String,
    pub artifacts: Vec<String>,
    pub absorbed: bool,
    pub last_generated: String,
}

fn registry_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    home.join(".api-anything").join("registry.json") // legacy path during Thumper rename; future versions may migrate to .thump or .thumper-cli
}

pub async fn register_generated(
    name: &str,
    output_dir: &std::path::Path,
    artifact_paths: Vec<String>,
    absorbed: bool,
) -> Result<()> {
    let output_str = output_dir.display().to_string();
    super::sqlite::save_tool(
        name,
        if absorbed { "absorbed" } else { "generated" },
        &output_str,
        &artifact_paths,
        absorbed,
    )?;
    Ok(())
}

pub fn load() -> Result<RegistryData> {
    let sqlite_tools = super::sqlite::load_tools().unwrap_or_default();
    let tools = sqlite_tools
        .into_iter()
        .map(|t| RegistryTool {
            name: t.name,
            kind: t.kind,
            output_dir: t.output_dir,
            artifacts: t.artifacts,
            absorbed: t.absorbed,
            last_generated: t.last_generated,
        })
        .collect();
    Ok(RegistryData { tools })
}
