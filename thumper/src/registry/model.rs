//! Core registry types (Tool, ApiSpec, etc.)
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Tool {
    pub name: String,
    pub kind: String,
    pub tags: Vec<String>,
}
