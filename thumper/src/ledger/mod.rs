//! korg-ledger@v1 tamper-evident ledger for thumper.
//!
//! `chain` is the vendored hash-chain reference (proven against the frozen
//! conformance vectors in `tests/conformance/`); `journal` is the `HealLedger`
//! writer the self-healing recovery loop uses to emit a verifiable, replayable
//! forensic trail of every error caught, repair attempted, and exit.

pub mod chain;
pub mod journal;
