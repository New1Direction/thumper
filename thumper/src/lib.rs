//! Thumper library interface. Exposes core execution, registry, and scheduling modules.

#[cfg(feature = "acp")]
pub mod acp;
pub mod bun;
pub mod cli;
pub mod demo;
pub mod generator;
/// korg-ledger@v1 tamper-evident hash-chain + heal-loop journal writer.
pub mod ledger;
pub mod registry;
#[cfg(feature = "tui")]
pub mod tui;
