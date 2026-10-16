//! The search tool (`docs/research-runs.md`, "The launch and the runner"; `docs/archive.md`): `bmo search`, which sets
//! a question off as a run of its own, and `bmo runner search`, the run, which researches it on the search
//! line — or brings a page in whole — and hands what it found back to the chat's session. What a search
//! shares with every research run is `verbs::research`; what is a search's own stands here.
//!
//! - `request` — the depths, the options, the request file and the run's prompt.
//! - `launch` — the verb.
//! - `runner` — the run.
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
pub mod launch;
pub mod record;
pub mod request;
pub mod result;
pub mod runner;
pub mod share;
pub mod web;
