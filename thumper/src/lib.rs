//! Thumper library interface. Exposes core execution, registry, and scheduling modules.

#[cfg(feature = "acp")]
pub mod acp;
pub mod bun;
pub mod cli;
pub mod demo;
pub mod generator;
pub mod registry;
#[cfg(feature = "tui")]
pub mod tui;
