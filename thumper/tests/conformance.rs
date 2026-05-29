//! korg-ledger@v1 cross-language conformance — thumper as an independent producer.
//!
//! Thumper is a standalone repo with no korg dependency; it vendors its own copy
//! of the korg-ledger@v1 chain emitter. This test proves that copy reproduces the
//! SAME frozen tip hashes as the Python reference (korgex) and the Rust core
//! (korg-registry) — i.e. thumper is a genuine third independent implementation
//! of the spec, not a fork that silently drifted. Vectors are vendored under
//! tests/conformance/ (canonical source: korgex spec/korg-ledger-v1/).

use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use thumper_cli::ledger::chain::{chain_hash, verify_chain, GENESIS_HASH};

fn cdir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/conformance"))
}

fn read_jsonl(name: &str) -> Vec<Value> {
    fs::read_to_string(cdir().join(name))
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[test]
fn genesis_hash_is_64_zeros() {
    assert_eq!(GENESIS_HASH, "0".repeat(64));
}

#[test]
fn conformance_vectors_reproduce_the_frozen_oracle() {
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(cdir().join("conformance.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["spec_version"], "korg-ledger@v1");
    for v in manifest["vectors"].as_array().unwrap() {
        let file = v["file"].as_str().unwrap();
        let events = read_jsonl(file);
        let key_owned = v["key"].as_str().map(|s| s.as_bytes().to_vec());
        let key = key_owned.as_deref();
        let errors = verify_chain(&events, key);
        match v["verify"].as_str().unwrap() {
            "intact" => {
                assert!(errors.is_empty(), "{file}: expected intact, got {errors:?}");
                let tip = chain_hash(events.last().unwrap(), key);
                assert_eq!(
                    tip,
                    v["tip_entry_hash"].as_str().unwrap(),
                    "{file}: thumper chain_hash must reproduce the frozen tip"
                );
            }
            _ => {
                assert!(
                    !errors.is_empty(),
                    "{file}: expected tampered, verified clean"
                );
                let needle = v["error_contains"].as_str().unwrap();
                assert!(
                    errors.iter().any(|e| e.contains(needle)),
                    "{file}: errors {errors:?} missing {needle:?}"
                );
            }
        }
    }
}

#[test]
fn hmac_chain_fails_without_the_key() {
    let events = read_jsonl("hmac-intact.jsonl");
    assert!(
        !verify_chain(&events, None).is_empty(),
        "keyed chain wrongly verified without the key"
    );
}
