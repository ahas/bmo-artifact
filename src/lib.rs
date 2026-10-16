//! The document archive: research documents, HTML pages, records, attachments and share links, served
//! over HTTP and filed through the CLI verbs (`docs/archive.md`, `docs/api.md`).

pub mod plumbing;
pub mod settings;

/// The repository's version, as `VERSION` holds it.
pub const VERSION: &str = env!("BMO_ARTIFACT_VERSION");
