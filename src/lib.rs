//! The document archive: research documents, HTML pages, records, attachments and share links, served
//! over HTTP and filed through the CLI verbs (`docs/archive.md`, `docs/api.md`).

pub mod clients;
pub mod control;
pub mod design;
pub mod documents;
pub mod helpers;
pub mod http;
pub mod kinds;
pub mod markup;
pub mod plumbing;
pub mod pool;
pub mod settings;
pub mod store;
pub mod verbs;
pub mod wording;

pub use plumbing::refusal::Refusal;
pub use store::{all, now_ms, one, run, statement};

/// The repository's version, as `VERSION` holds it.
pub const VERSION: &str = env!("BMO_ARTIFACT_VERSION");
