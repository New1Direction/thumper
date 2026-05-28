//! Agent-native introspection — emits a `korg:introspect@v1` document
//! describing every top-level callable, its argument schema, declared
//! side-effects, output mode, and stable command ID.
//!
//! The document format is shared with the rest of the korg ecosystem
//! (see `Korg/adapters/recall-mcp/src/korg_recall_mcp/introspect.py`).
//! Same shape — agents that read one can switch on the same fields when
//! reading another.
//!
//! This is the Foundry pattern: machine-readable discovery as a
//! first-class flag, with capability metadata so agents can make
//! safety decisions before invocation.
//!
//! Note: `thump` doesn't currently ship an MCP server (the natural
//! agent surface for thumper is the ACP / agent stdio mode). The
//! introspect document is still useful — it describes the CLI to any
//! agent that wants to drive `thump` as a subprocess.

use serde::Serialize;
use std::collections::BTreeMap;

pub const INTROSPECT_SCHEMA_ID: &str = "korg:introspect@v1";
pub const BINARY_NAME: &str = "thump";

#[derive(Debug, Serialize, Clone)]
pub struct Capabilities {
    pub output_mode: String,
    pub side_effects: String,
    pub requires_project: bool,
    pub long_running: bool,
    pub stateful: bool,
    pub reads_stdin: bool,
    pub supports_output_path: bool,
}

impl Capabilities {
    /// Conservative defaults — zero-effect, fast, no stdin.
    pub fn safe() -> Self {
        Self {
            output_mode: "envelope".to_string(),
            side_effects: "none".to_string(),
            requires_project: false,
            long_running: false,
            stateful: false,
            reads_stdin: false,
            supports_output_path: false,
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct Callable {
    pub command_id: String,
    pub name: String,
    pub description: String,
    pub surfaces: Vec<String>,
    pub input_schema: serde_json::Value,
    pub capabilities: Capabilities,
}

#[derive(Debug, Serialize)]
pub struct IntrospectDocument {
    pub schema: String,
    pub binary: String,
    pub version: String,
    pub callables_declared: bool,
    pub callables: Vec<Callable>,
    pub exit_codes: BTreeMap<String, String>,
}

/// Canonical exit-code table shared across the korg ecosystem.
/// Wire format keys are strings (JSON has no integer keys).
pub fn exit_codes() -> BTreeMap<String, String> {
    [
        ("0", "success"),
        ("1", "error.generic"),
        ("2", "error.usage"),
        ("3", "error.config"),
        ("4", "error.io"),
        ("5", "error.network"),
        ("6", "error.user_interrupt"),
        ("7", "error.dependency_missing"),
    ]
    .iter()
    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
    .collect()
}

/// Build the descriptor list for every top-level callable thump exposes.
pub fn callables() -> Vec<Callable> {
    vec![
        Callable {
            command_id: "thump.tui".to_string(),
            name: "tui".to_string(),
            description: "Launch the full-screen interactive Thumper TUI \
                          (default when no subcommand is given)."
                .to_string(),
            surfaces: vec!["cli".to_string()],
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "generate": {
                        "type": "string",
                        "description": "Start directly in the generate wizard for this tool name."
                    }
                }
            }),
            capabilities: Capabilities {
                output_mode: "none".to_string(),
                side_effects: "fs_read".to_string(),
                long_running: true,
                reads_stdin: true,
                ..Capabilities::safe()
            },
        },
        Callable {
            command_id: "thump.generate".to_string(),
            name: "generate".to_string(),
            description: "Generate an API wrapper + harness for a tool. \
                          Writes a new project directory; supports --stream for \
                          NDJSON progress."
                .to_string(),
            surfaces: vec!["cli".to_string()],
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Tool name (e.g. \"bettercap\")."},
                    "description": {"type": "string"},
                    "from": {
                        "type": "string",
                        "enum": ["auto", "cli", "binary", "description", "repo", "existing-harness"],
                        "default": "auto"
                    },
                    "lang": {
                        "type": "string",
                        "enum": ["python", "rust", "go", "typescript", "all"],
                        "default": "python"
                    },
                    "output": {"type": "string", "description": "Output directory path."},
                    "force": {"type": "boolean", "default": false},
                    "stream": {"type": "boolean", "default": false},
                    "absorb": {"type": "boolean", "default": false},
                    "hint": {"type": "array", "items": {"type": "string"}}
                },
                "required": ["name"]
            }),
            capabilities: Capabilities {
                output_mode: "stream".to_string(),
                side_effects: "fs_write".to_string(),
                long_running: true,
                supports_output_path: true,
                ..Capabilities::safe()
            },
        },
        Callable {
            command_id: "thump.registry".to_string(),
            name: "registry".to_string(),
            description: "Manage the local tool/API registry (list, show, add, \
                          remove, reindex). Subcommands: list, show, add, \
                          remove, reindex."
                .to_string(),
            surfaces: vec!["cli".to_string()],
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "subcommand": {
                        "type": "string",
                        "enum": ["list", "show", "add", "remove", "reindex"]
                    }
                },
                "required": ["subcommand"]
            }),
            capabilities: Capabilities {
                output_mode: "envelope".to_string(),
                side_effects: "fs_write".to_string(),
                ..Capabilities::safe()
            },
        },
        Callable {
            command_id: "thump.agent.stdio".to_string(),
            name: "agent stdio".to_string(),
            description: "Run as an ACP (Agent Client Protocol) stdio server. \
                          Long-running stateful session over stdin/stdout."
                .to_string(),
            surfaces: vec!["cli".to_string(), "acp".to_string()],
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "yolo": {
                        "type": "boolean",
                        "default": false,
                        "description": "Auto-approve all generation requests (dangerous)."
                    }
                }
            }),
            capabilities: Capabilities {
                output_mode: "session".to_string(),
                side_effects: "fs_write".to_string(),
                long_running: true,
                stateful: true,
                reads_stdin: true,
                ..Capabilities::safe()
            },
        },
        Callable {
            command_id: "thump.agent.serve".to_string(),
            name: "agent serve".to_string(),
            description: "Run as an HTTP/WebSocket ACP relay. Multiple IDEs \
                          can share one process."
                .to_string(),
            surfaces: vec!["cli".to_string(), "acp".to_string()],
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "bind": {
                        "type": "string",
                        "default": "127.0.0.1:2480",
                        "description": "Listen address."
                    }
                }
            }),
            capabilities: Capabilities {
                output_mode: "session".to_string(),
                side_effects: "network".to_string(),
                long_running: true,
                stateful: true,
                ..Capabilities::safe()
            },
        },
        Callable {
            command_id: "thump.doctor".to_string(),
            name: "doctor".to_string(),
            description: "Diagnose the local environment: python bridge, \
                          registry, templates, file permissions."
                .to_string(),
            surfaces: vec!["cli".to_string()],
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "json": {"type": "boolean", "default": false}
                }
            }),
            capabilities: Capabilities {
                output_mode: "envelope".to_string(),
                side_effects: "fs_read".to_string(),
                ..Capabilities::safe()
            },
        },
        Callable {
            command_id: "thump.bun.script.run".to_string(),
            name: "bun script run".to_string(),
            description: "Run a script from package.json via the Bun runtime. \
                          Streams stdout/stderr; emits structured events on --json."
                .to_string(),
            surfaces: vec!["cli".to_string()],
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "args": {"type": "array", "items": {"type": "string"}}
                },
                "required": ["name"]
            }),
            capabilities: Capabilities {
                output_mode: "stream".to_string(),
                side_effects: "fs_write".to_string(),
                long_running: true,
                ..Capabilities::safe()
            },
        },
        Callable {
            command_id: "thump.bun.package.add".to_string(),
            name: "bun package add".to_string(),
            description: "Add one or more packages via Bun.".to_string(),
            surfaces: vec!["cli".to_string()],
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "packages": {"type": "array", "items": {"type": "string"}},
                    "dev": {"type": "boolean", "default": false},
                    "exact": {"type": "boolean", "default": false},
                    "peer": {"type": "boolean", "default": false},
                    "optional": {"type": "boolean", "default": false}
                },
                "required": ["packages"]
            }),
            capabilities: Capabilities {
                output_mode: "stream".to_string(),
                side_effects: "network".to_string(),
                long_running: true,
                ..Capabilities::safe()
            },
        },
        Callable {
            command_id: "thump.completion".to_string(),
            name: "completion".to_string(),
            description: "Generate shell completion scripts (bash, zsh, fish, ...)."
                .to_string(),
            surfaces: vec!["cli".to_string()],
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "shell": {
                        "type": "string",
                        "enum": ["bash", "zsh", "fish", "powershell", "elvish"]
                    }
                },
                "required": ["shell"]
            }),
            capabilities: Capabilities {
                output_mode: "stream".to_string(),
                ..Capabilities::safe()
            },
        },
    ]
}

pub fn build_document(version: &str) -> IntrospectDocument {
    IntrospectDocument {
        schema: INTROSPECT_SCHEMA_ID.to_string(),
        binary: BINARY_NAME.to_string(),
        version: version.to_string(),
        callables_declared: true,
        callables: callables(),
        exit_codes: exit_codes(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_has_schema_tag() {
        let doc = build_document("0.2.0");
        assert_eq!(doc.schema, "korg:introspect@v1");
    }

    #[test]
    fn document_carries_binary_and_version() {
        let doc = build_document("1.2.3");
        assert_eq!(doc.binary, "thump");
        assert_eq!(doc.version, "1.2.3");
    }

    #[test]
    fn callables_have_unique_ids() {
        let ids: Vec<_> = callables().iter().map(|c| c.command_id.clone()).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "duplicate command_ids: {:?}", ids);
    }

    #[test]
    fn all_command_ids_use_dot_namespacing() {
        for c in callables() {
            assert!(
                c.command_id.starts_with("thump."),
                "command_id should start with 'thump.': {}",
                c.command_id
            );
            assert!(
                !c.command_id.contains(' '),
                "command_id should not contain spaces: {}",
                c.command_id
            );
        }
    }

    #[test]
    fn capabilities_have_recognized_side_effects() {
        let valid = [
            "none",
            "fs_read",
            "fs_write",
            "network",
            "ledger_write",
        ];
        for c in callables() {
            assert!(
                valid.contains(&c.capabilities.side_effects.as_str()),
                "unknown side_effects on {}: {}",
                c.command_id,
                c.capabilities.side_effects
            );
        }
    }

    #[test]
    fn capabilities_have_recognized_output_modes() {
        let valid = ["none", "stream", "envelope", "session"];
        for c in callables() {
            assert!(
                valid.contains(&c.capabilities.output_mode.as_str()),
                "unknown output_mode on {}: {}",
                c.command_id,
                c.capabilities.output_mode
            );
        }
    }

    #[test]
    fn input_schemas_are_object_typed() {
        for c in callables() {
            assert_eq!(
                c.input_schema.get("type").and_then(|v| v.as_str()),
                Some("object"),
                "input_schema must be 'type: object' for {}",
                c.command_id
            );
        }
    }

    #[test]
    fn long_running_session_implies_stateful_or_session_output() {
        // Sanity check: anything that runs indefinitely AND is stateful should
        // declare output_mode=session or stream. A long-running envelope command
        // would be a contradiction (envelope = one final wrapped result).
        for c in callables() {
            if c.capabilities.long_running && c.capabilities.stateful {
                assert!(
                    matches!(
                        c.capabilities.output_mode.as_str(),
                        "session" | "stream" | "none"
                    ),
                    "{} is long_running + stateful but output_mode is {}",
                    c.command_id,
                    c.capabilities.output_mode
                );
            }
        }
    }

    #[test]
    fn document_round_trips_through_json() {
        let doc = build_document("0.2.0");
        let blob = serde_json::to_string(&doc).expect("serialize");
        let value: serde_json::Value = serde_json::from_str(&blob).expect("parse");
        assert_eq!(value["schema"], "korg:introspect@v1");
        assert_eq!(value["binary"], "thump");
        assert_eq!(value["callables_declared"], true);
        assert!(value["callables"].is_array());
        assert!(value["exit_codes"].is_object());
    }

    #[test]
    fn exit_code_table_is_complete_and_string_keyed() {
        let codes = exit_codes();
        assert_eq!(codes.get("0").map(|s| s.as_str()), Some("success"));
        assert!(codes.contains_key("1"));
        // All keys must be strings (wire format)
        for key in codes.keys() {
            // Strings already by type, but verify parseable as int
            assert!(
                key.parse::<u32>().is_ok(),
                "exit code key not numeric: {}",
                key
            );
        }
    }

    #[test]
    fn all_capabilities_include_required_fields() {
        // Serialize a single Capabilities and verify every field is present
        // in the JSON output (we'd notice a missing #[serde] attribute).
        let blob = serde_json::to_value(&Capabilities::safe()).expect("serialize");
        for required in &[
            "output_mode",
            "side_effects",
            "requires_project",
            "long_running",
            "stateful",
            "reads_stdin",
            "supports_output_path",
        ] {
            assert!(
                blob.get(required).is_some(),
                "Capabilities is missing required field {}",
                required
            );
        }
    }
}
