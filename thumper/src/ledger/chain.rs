//! korg-ledger@v1 — tamper-evident hash-chain (vendored into thumper).
//!
//! Normative source: korgex `spec/korg-ledger-v1/SPEC.md`. Thumper is a
//! standalone repo with no korg dependency, so it vendors this copy; the
//! conformance vectors in `tests/conformance/` prove it reproduces the frozen
//! tip hashes shared with the Python reference (korgex) and the Rust core
//! (korg-registry). If the spec revises, this copy must be re-synced and the
//! vectors re-checked.
//!
//! Guarantee: events are hash-chained — each carries `prev_hash` (the previous
//! event's `entry_hash`, GENESIS for the first) and `entry_hash` (hash of its
//! own canonical preimage). Any edit/delete/insert/reorder breaks the chain and
//! is localized to a `seq_id`. With an HMAC key the chain is tamper-PROOF.

use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// The chain anchor: `prev_hash` of the first event (64 zero hex chars).
pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Fields excluded from the preimage (they ARE the hash).
const HASH_FIELDS: &[&str] = &["entry_hash"];

/// Canonical byte encoding (korg-ledger@v1 §2): reproduces Python
/// `json.dumps(value, sort_keys=True, separators=(",",":"))` with
/// `ensure_ascii=True` — sorted keys, no whitespace, non-ASCII as `\uXXXX`.
pub fn canonicalize(value: &Value) -> Vec<u8> {
    let mut s = String::new();
    write_canonical(value, &mut s);
    s.into_bytes()
}

fn write_canonical(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => write_json_string(s, out),
        Value::Array(arr) => {
            out.push('[');
            for (i, e) in arr.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(e, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json_string(k, out);
                out.push(':');
                write_canonical(&map[*k], out);
            }
            out.push('}');
        }
    }
}

fn write_json_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if ('\u{20}'..='\u{7e}').contains(&c) => out.push(c),
            c => {
                let cp = c as u32;
                if cp > 0xFFFF {
                    let v = cp - 0x10000;
                    out.push_str(&format!("\\u{:04x}", 0xD800 + (v >> 10)));
                    out.push_str(&format!("\\u{:04x}", 0xDC00 + (v & 0x3FF)));
                } else {
                    out.push_str(&format!("\\u{:04x}", cp));
                }
            }
        }
    }
    out.push('"');
}

/// Compute an event's `entry_hash` (korg-ledger@v1 §3). Preimage = the event
/// minus `entry_hash` (`prev_hash` kept). SHA-256, or HMAC-SHA256 with a key.
pub fn chain_hash(event: &Value, key: Option<&[u8]>) -> String {
    let mut obj = event.as_object().cloned().unwrap_or_default();
    for f in HASH_FIELDS {
        obj.remove(*f);
    }
    let data = canonicalize(&Value::Object(obj));
    match key {
        Some(k) => {
            let mut mac = Hmac::<Sha256>::new_from_slice(k).expect("HMAC takes any key length");
            mac.update(&data);
            hex::encode(mac.finalize().into_bytes())
        }
        None => {
            let mut h = Sha256::new();
            h.update(&data);
            hex::encode(h.finalize())
        }
    }
}

/// Recompute the chain and report tampering (korg-ledger@v1 §5). `[]` == intact.
pub fn verify_chain(events: &[Value], key: Option<&[u8]>) -> Vec<String> {
    let mut errors = Vec::new();
    let mut expected_prev = GENESIS_HASH.to_string();
    for e in events {
        let sid = e
            .get("seq_id")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "?".to_string());
        match e.get("entry_hash").and_then(|v| v.as_str()) {
            None => {
                errors.push(format!(
                    "seq {sid}: missing entry_hash (event is not chained)"
                ));
                expected_prev = String::new();
            }
            Some(stored) => {
                let prev = e.get("prev_hash").and_then(|v| v.as_str()).unwrap_or("");
                if prev != expected_prev {
                    errors.push(format!(
                        "seq {sid}: prev_hash breaks the chain \
                         (an event was inserted, deleted, or reordered)"
                    ));
                }
                if chain_hash(e, key) != stored {
                    errors.push(format!(
                        "seq {sid}: entry_hash mismatch (content was tampered)"
                    ));
                }
                expected_prev = stored.to_string();
            }
        }
    }
    errors
}

/// Check the causal DAG (korg-ledger@v1 §5): unique `seq_id`s, every
/// `triggered_by` references an existing, strictly-earlier `seq_id`.
pub fn verify_dag(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    let seqs: Vec<i64> = events
        .iter()
        .filter_map(|e| e.get("seq_id").and_then(|v| v.as_i64()))
        .collect();
    let seqset: std::collections::HashSet<i64> = seqs.iter().copied().collect();
    if seqset.len() != seqs.len() {
        errors.push("duplicate seq_id present".to_string());
    }
    for e in events {
        let tb = match e.get("triggered_by").and_then(|v| v.as_i64()) {
            Some(tb) => tb,
            None => continue,
        };
        let sid = e.get("seq_id").and_then(|v| v.as_i64());
        if !seqset.contains(&tb) {
            errors.push(format!("seq {sid:?}: triggered_by {tb} does not exist"));
        } else if let Some(sid) = sid {
            if tb >= sid {
                errors.push(format!(
                    "seq {sid}: triggered_by {tb} is not strictly earlier"
                ));
            }
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonicalize_sorts_keys_and_is_compact() {
        assert_eq!(
            canonicalize(&json!({"z":[3,2],"a":{"y":1,"x":2}})),
            b"{\"a\":{\"x\":2,\"y\":1},\"z\":[3,2]}"
        );
    }

    #[test]
    fn canonicalize_escapes_non_ascii() {
        assert_eq!(
            canonicalize(&json!({"a":"é"})),
            b"{\"a\":\"\\u00e9\"}".to_vec()
        );
    }

    #[test]
    fn entry_hash_excludes_itself_but_keeps_prev_hash() {
        let ev = json!({"seq_id":1,"tool_name":"x","prev_hash":GENESIS_HASH});
        let h = chain_hash(&ev, None);
        let mut with = ev.clone();
        with["entry_hash"] = json!("anything");
        assert_eq!(chain_hash(&with, None), h);
        let mut other = ev.clone();
        other["prev_hash"] = json!("ff");
        assert_ne!(chain_hash(&other, None), h);
    }
}
