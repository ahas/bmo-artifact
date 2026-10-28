//! The routes this service serves, each registered beside its own answer and its own OpenAPI operation
//! (`docs/api.md`), and the way a kept file goes out on the wire.

pub mod attachments;
pub mod console;
pub mod document_kinds;
pub mod documents;
pub mod events;
pub mod files;
pub mod health;
pub mod hosted_page;
pub mod imports;
pub mod publications;
pub mod references;
pub mod scopes;
pub mod share_reads;
pub mod shares;
