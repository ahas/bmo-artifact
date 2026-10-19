//! When each room's morning news page is started (`docs/news-brief.md`, "When it runs"): a loop of the
//! server's that looks every minute, and starts a row on a working day, once, from its lead before its time
//! until two hours after it, through bmo-agency as `news-brief --run` does. A calendar that holds nothing of
//! the year is told to the owner once, and the pages still go on weekdays.

use std::collections::BTreeSet;
use std::sync::Mutex;
use std::time::Duration;

use chrono::Datelike;
use chrono_tz::Tz;
use sea_orm::DatabaseConnection;

use super::calendar::{date_of, day_of, moment, weekday};
use super::launch::{Start, launch};
use super::runner::SORT;
use crate::clients::agency::Agency;
use crate::clients::receiver::Receiver;
use crate::plumbing::background::Loop;
use crate::plumbing::http::Standing;
use crate::store::news_briefs::{Brief, briefs, holiday, holidays_of, mark_started};

/// How long past its time a room's page is still started: a service that was down at the hour sends it late
/// within this, and past it the room is left alone for the day.
pub const LATE_LIMIT_MS: i64 = 2 * 60 * 60 * 1000;

/// The day a row starts on at `now_ms`, or none: a working day, `day_off` saying whether it is a holiday,
/// not started yet that day, from its lead before its time until the late limit after it, in `zone`.
pub fn due(row: &Brief, now_ms: i64, zone: Tz, day_off: bool) -> Option<String> {
  let day = day_of(now_ms, zone);
  let send_at = moment(&day, &row.send_at, zone)?;

  if !row.enabled || row.last_day.as_deref() == Some(day.as_str()) || !weekday(&day) || day_off {
    return None;
  }

  (now_ms >= send_at - row.lead_minutes * 60_000 && now_ms <= send_at + LATE_LIMIT_MS).then_some(day)
}

/// One row's start on its day, at its own time.
pub fn start_of<'a>(row: &'a Brief, day: &'a str) -> Start<'a> {
  Start {
    row,
    day,
    send_at: Some(&row.send_at),
    send_to: None,
  }
}

/// What one look stands on.
pub struct Ground<'a> {
  pub db: &'a DatabaseConnection,
  pub agency: &'a dyn Agency,
  pub receiver: &'a Receiver,
  pub zone: Tz,
}

/// The look on the clock, which keeps the years the owner was told the calendar holds nothing of, so each is
/// told once.
#[derive(Default)]
pub struct Look {
  told: Mutex<BTreeSet<i32>>,
}

impl Look {
  pub const fn new() -> Self {
    Self {
      told: Mutex::new(BTreeSet::new()),
    }
  }

  // Whether the owner is still to be told of `year`: marked told as it is asked.
  fn to_tell(&self, year: i32) -> bool {
    self.told.lock().unwrap_or_else(std::sync::PoisonError::into_inner).insert(year)
  }

  /// One look at `now_ms`: every row due marked started for its day and started; how many were.
  pub async fn look(&self, ground: &Ground<'_>, now_ms: i64) -> Result<usize, String> {
    let rows = briefs(ground.db).await.map_err(|why| why.detail)?;
    let today = day_of(now_ms, ground.zone);
    let year = date_of(&today).map_or(0, |date| date.year());

    if rows.iter().any(|row| row.enabled)
      && holidays_of(ground.db, year).await.map_err(|why| why.detail)?.is_empty()
      && self.to_tell(year)
    {
      let order = format!(
        "The holiday calendar holds no day of {year}, so the morning news pages go out on every weekday until \
         it is filled. Add the year's holidays with: bmo news-brief --holiday-add YYYY-MM-DD '<name>'."
      );

      if let Err(why) = ground.receiver.tell_owner(SORT, &order).await {
        eprintln!("news-brief: the owner could not be told the calendar is empty: {why}");
      }
    }

    let day_off = holiday(ground.db, &today).await.map_err(|why| why.detail)?.is_some();
    let mut started = 0;

    for row in &rows {
      let Some(day) = due(row, now_ms, ground.zone, day_off) else {
        continue;
      };

      mark_started(ground.db, row.id, &day).await.map_err(|why| why.detail)?;

      match launch(ground.agency, &start_of(row, &day), now_ms).await {
        Ok(launched) => {
          eprintln!("news-brief: the page of #{} for {day} runs as {}", row.id, launched.unit);
          started += 1;
        }
        Err(why) => eprintln!("news-brief: the page of #{} for {day} did not start: {why}", row.id),
      }
    }

    Ok(started)
  }
}

/// The news brief's loop of the server.
pub struct NewsBriefLook(Look);

impl Loop for NewsBriefLook {
  const NAME: &'static str = "news-brief";
  const PERIOD: Duration = Duration::from_secs(60);

  async fn pass(&self, standing: &Standing, now_ms: i64) -> Result<(), String> {
    let db = standing.archive.db().map_err(|why| why.to_string())?;
    let ground = Ground {
      db,
      agency: &*standing.agency,
      receiver: &standing.receiver,
      zone: standing.settings.time.zone,
    };

    self.0.look(&ground, now_ms).await.map(drop)
  }
}

// The one look the server runs, kept for the server's life, since what it remembers is the years told.
static NEWS_BRIEF_LOOK: NewsBriefLook = NewsBriefLook(Look::new());

crate::background!(NEWS_BRIEF_LOOK);
