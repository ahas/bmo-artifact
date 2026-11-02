//! The crate's one test harness (bmo-chat's docs/conventions.md, 「Rust builds」): each file of tests a
//! module of it, sharing the process.

mod support;

mod console;
mod dependencies;
mod documents;
mod hangul;
mod helpers;
mod hosted_page;
mod html_read;
mod import;
mod imports;
mod layout;
mod mcp;
mod nouns;
mod openapi;
mod page_reading;
mod personal_data;
mod picture_ground;
mod publications;
mod readings;
mod settings;
mod shares;
mod store;
mod wording;
