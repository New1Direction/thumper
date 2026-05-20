//! ACP (Agent Client Protocol) support for api-anything.
//! This allows IDEs (Zed, Neovim, etc.) to treat `api-anything agent stdio` as a first-class
//! "API & Harness Expert" agent.
//!
//! Full implementation: handles initialize + session lifecycle, maps `session/prompt`
//! text to real generation via the python bridge, streams `agent_message_chunk` +
//! `tool_call` / `tool_call_update` events, final artifacts reported.
//! Extension methods under x.ai/api-anything/* are stubbed in the dispatch handler.

pub mod server;

pub use server::run_stdio_server;
