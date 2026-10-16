//! The search line (`docs/archive.md`): what a search is asked, what it comes back with, the record it stands
//! as and the document a person reads.
//!
//! - `request` — the depths, the options, the request file and the run's prompt.
//! - `result` — what a run's result says of itself, and what the session is handed in each ending.
//! - `document`, `record` — the record a search stands as, and the document a person reads.
//! - `images` — the pictures a result names, kept as this repository's own copies.
//! - `gallery` — the pictures a run looked at, and the albums the room is sent them in.
//! - `share` — the address a long answer is handed with.
//! - `import` — a page brought in whole as a result.
//! - `web` — what the runner asks of the page reader.

pub mod document;
pub mod gallery;
pub mod images;
pub mod import;
pub mod record;
pub mod request;
pub mod result;
pub mod share;
pub mod web;
