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
    home.join(".api-anything").join("registry.json")
}

pub async fn register_generated(
    name: &str,
    output_dir: &std::path::Path,
    artifact_paths: Vec<String>,
    absorbed: bool,
) -> Result<()> {
    tokio::fs::create_dir_all(registry_path().parent().unwrap())
        .await
        .ok();

    let mut data: RegistryData = if tokio::fs::try_exists(&registry_path())
        .await
        .unwrap_or(false)
    {
        let content = tokio::fs::read_to_string(&registry_path()).await?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        RegistryData::default()
    };

    let entry = RegistryTool {
        name: name.to_string(),
        kind: if absorbed {
            "absorbed".into()
        } else {
            "generated".into()
        },
        output_dir: output_dir.display().to_string(),
        artifacts: artifact_paths,
        absorbed,
        last_generated: chrono::Utc::now().to_rfc3339(),
    };

    // Replace if exists
    data.tools.retain(|t| t.name != name);
    data.tools.push(entry);

    let json = serde_json::to_string_pretty(&data)?;
    tokio::fs::write(&registry_path(), json).await?;

    Ok(())
}

pub fn load() -> Result<RegistryData> {
    // sync load for simple cases
    let p = registry_path();
    if p.exists() {
        let s = std::fs::read_to_string(p)?;
        Ok(serde_json::from_str(&s).unwrap_or_default())
    } else {
        Ok(RegistryData::default())
    }
}
