//! Documents kept, searched and shared: records, learning books, imported pages and HTML pages, with their
//! attachments, references and share links, served over HTTP, reached through the service's own client and
//! offered to a harness by its MCP server (`docs/api.md`, `docs/archive.md`).

pub mod client;
pub mod commands;
pub mod design;
pub mod documents;
pub mod events;
pub mod helpers;
pub mod http;
pub mod kinds;
pub mod mcp;
pub mod openapi;
pub mod plumbing;
pub mod pool;
pub mod reader;
pub mod settings;
pub mod store;
pub mod wire;
pub mod wording;

pub use plumbing::refusal::Refusal;
pub use store::{all, now_ms, one, run};

/// The repository's version, as `VERSION` holds it.
pub const VERSION: &str = env!("BMO_ARTIFACT_VERSION");
