//! The days and the moments a morning news page is told by, in the time zone the settings name
//! (`docs/news-brief.md`, "When it runs"): a day is a calendar date written `YYYY-MM-DD`, a send time a clock
//! time written `HH:MM`.

use chrono::{Datelike, NaiveDate, NaiveTime, TimeZone};
use chrono_tz::Tz;

/// The day a moment in epoch milliseconds falls on in `zone`.
pub fn day_of(now_ms: i64, zone: Tz) -> String {
  zone
    .timestamp_millis_opt(now_ms)
    .single()
    .map_or_else(String::new, |at| at.format("%Y-%m-%d").to_string())
}

/// The calendar date a day names, none where it names none.
pub fn date_of(day: &str) -> Option<NaiveDate> {
  (day.len() == 10).then(|| NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()).flatten()
}

/// The moment, in epoch milliseconds, a clock time stands at on a day in `zone`; the earlier where the
/// zone's clocks pass it twice, none where they skip it or either does not read.
pub fn moment(day: &str, clock: &str, zone: Tz) -> Option<i64> {
  let date = date_of(day)?;
  let time = (clock.len() == 5).then(|| NaiveTime::parse_from_str(clock, "%H:%M").ok()).flatten()?;

  zone
    .from_local_datetime(&date.and_time(time))
    .earliest()
    .map(|at| at.timestamp_millis())
}

/// Whether a day falls Monday to Friday.
pub fn weekday(day: &str) -> bool {
  date_of(day).is_some_and(|date| date.weekday().num_days_from_monday() < 5)
}
