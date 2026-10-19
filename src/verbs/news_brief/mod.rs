//! The morning news page (`docs/news-brief.md`): one page a working morning, written by a research run,
//! drawn here with the indices' own readings and the articles' pictures, filed as a page document and sent
//! to its room as a link.
//!
//! - `brief` — the writer's answer, read and held to the limits the page is drawn to.
//! - `series` — an index's closes, and the figures the page writes of them.
//! - `page` — the page whole, and the message that hands it to a room.
//! - `compose` — the pictures and the closes read through the tool host, and the page drawn with them.
//! - `calendar` — the days and the moments a page is told by, in the settings' time zone.
//! - `runner` — `runner news-brief`, one page written, filed and sent.
//! - `launch` — one page's run opened in bmo-agency's ledger and launched.
//! - `command` — `news-brief`, the rooms and the holiday calendar.
//! - `look` — the server's minute loop that starts each room's page on its working days.

pub mod brief;
pub mod calendar;
pub mod command;
pub mod compose;
pub mod launch;
pub mod look;
pub mod page;
pub mod runner;
pub mod series;
