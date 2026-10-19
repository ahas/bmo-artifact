//! The morning news page's rooms, as rows of `news_briefs`, and the holiday calendar its working days skip,
//! as rows of `holidays` (`docs/news-brief.md`, "When it runs" and "The holiday calendar"). This is the one
//! module that writes either table: the `news-brief` command adds, changes, switches and removes rooms and
//! edits the calendar through it, and the minute loop writes nothing but the day it started a row.
//!
//! A row's `created_at` is epoch seconds; a day is a calendar date written `YYYY-MM-DD`, told in the time
//! zone the settings name.

use chrono::{NaiveDate, NaiveTime};
use sea_orm::{ConnectionTrait, QueryResult};

use crate::{Refusal, all, one, run};

/// The time a room added is sent its page where the call names none.
pub const SEND_AT: &str = "08:30";
/// How many minutes before its time the writing starts where the call names none.
pub const LEAD_MINUTES: i64 = 40;
/// How many hours a page's share link lives where the call names none.
pub const SHARE_HOURS: i64 = 24;

// The bounds a lead the writer can work in and a link's life are held to.
const LEAD_RANGE: std::ops::RangeInclusive<i64> = 10..=180;
const SHARE_RANGE: std::ops::RangeInclusive<i64> = 1..=24 * 30;

/// One room's row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Brief {
  pub id: i64,
  pub chat_id: i64,
  /// The time the room is sent its page, `HH:MM`.
  pub send_at: String,
  /// How long before that the writing starts, in minutes.
  pub lead_minutes: i64,
  /// How long the page's share link lives, in hours.
  pub share_hours: i64,
  pub enabled: bool,
  /// The day a page was last started for the row; none before the first.
  pub last_day: Option<String>,
  pub created_at: i64,
}

/// What a call names of a row, each as it was written.
#[derive(Clone, Debug, Default)]
pub struct Named {
  pub chat: Option<String>,
  pub at: Option<String>,
  pub lead: Option<String>,
  pub share_hours: Option<String>,
}

/// The fields a call changes, read and checked; a field left none stays as it stands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Change {
  pub chat_id: Option<i64>,
  pub send_at: Option<String>,
  pub lead_minutes: Option<i64>,
  pub share_hours: Option<i64>,
  pub enabled: Option<bool>,
}

impl Change {
  /// The fields a call names, read as the room, the time and the numbers they are; refused, with the
  /// sentence the caller is told, where one is not what a row may hold.
  pub fn read(named: &Named) -> Result<Self, Refusal> {
    let whole = |text: &str| text.trim().parse::<i64>().ok();
    let chat_id = match &named.chat {
      None => None,
      Some(chat) => Some(whole(chat).ok_or_else(|| Refusal::bad("a room is named by its chat id"))?),
    };
    let lead_minutes = match &named.lead {
      None => None,
      Some(lead) => Some(whole(lead).ok_or_else(|| Refusal::bad(LEAD_FAULT))?),
    };
    let share_hours = match &named.share_hours {
      None => None,
      Some(share) => Some(whole(share).ok_or_else(|| Refusal::bad(SHARE_FAULT))?),
    };
    let change = Self {
      chat_id,
      send_at: named.at.clone(),
      lead_minutes,
      share_hours,
      enabled: None,
    };

    change.check()?;

    Ok(change)
  }

  /// Whether every field named is one a row may hold: a clock time, a lead the writer can work in and a
  /// link that lives at least an hour.
  pub fn check(&self) -> Result<(), Refusal> {
    if let Some(at) = &self.send_at
      && !is_clock(at)
    {
      return Err(Refusal::bad(format!("{at} is not a time; write it HH:MM")));
    }

    if self.lead_minutes.is_some_and(|minutes| !LEAD_RANGE.contains(&minutes)) {
      return Err(Refusal::bad(LEAD_FAULT));
    }

    if self.share_hours.is_some_and(|hours| !SHARE_RANGE.contains(&hours)) {
      return Err(Refusal::bad(SHARE_FAULT));
    }

    Ok(())
  }
}

const LEAD_FAULT: &str = "the lead is a whole number of minutes from 10 to 180";
const SHARE_FAULT: &str = "the share link lives a whole number of hours, from 1 to 720";

/// Whether a text is a time `HH:MM` on the 24-hour clock, both parts written with two digits.
pub fn is_clock(text: &str) -> bool {
  text.len() == 5 && NaiveTime::parse_from_str(text, "%H:%M").is_ok()
}

/// Whether a text is a calendar date written `YYYY-MM-DD`.
pub fn is_day(text: &str) -> bool {
  text.len() == 10 && NaiveDate::parse_from_str(text, "%Y-%m-%d").is_ok()
}

const COLUMNS: &str =
  r#""id", "chat_id", "send_at", "lead_minutes", "share_hours", "enabled", "last_day", "created_at""#;

fn brief_of(row: &QueryResult) -> Result<Brief, Refusal> {
  Ok(Brief {
    id: row.try_get("", "id")?,
    chat_id: row.try_get("", "chat_id")?,
    send_at: row.try_get("", "send_at")?,
    lead_minutes: row.try_get("", "lead_minutes")?,
    share_hours: row.try_get("", "share_hours")?,
    enabled: row.try_get::<i64>("", "enabled")? != 0,
    last_day: row.try_get("", "last_day")?,
    created_at: row.try_get("", "created_at")?,
  })
}

/// Every room's row, in the order they were added.
pub async fn briefs(db: &impl ConnectionTrait) -> Result<Vec<Brief>, Refusal> {
  all(db, &format!(r#"SELECT {COLUMNS} FROM "news_briefs" ORDER BY "id""#), vec![])
    .await?
    .iter()
    .map(brief_of)
    .collect()
}

/// One room's row, none where no such row stands.
pub async fn brief(db: &impl ConnectionTrait, id: i64) -> Result<Option<Brief>, Refusal> {
  one(db, &format!(r#"SELECT {COLUMNS} FROM "news_briefs" WHERE "id" = $1"#), vec![id.into()])
    .await?
    .as_ref()
    .map(brief_of)
    .transpose()
}

/// A room added at `now`, in epoch seconds, from what the change names and the defaults for the rest, off
/// unless the change switches it on: its row's number. A change naming no room is refused, and so is one
/// a row may not hold, before anything is written.
pub async fn add(db: &impl ConnectionTrait, change: &Change, now: i64) -> Result<i64, Refusal> {
  change.check()?;

  let Some(chat_id) = change.chat_id else {
    return Err(Refusal::bad("a room is named by its chat id"));
  };
  let row = one(
    db,
    r#"INSERT INTO "news_briefs" ("chat_id", "send_at", "lead_minutes", "share_hours", "enabled", "created_at")
      VALUES ($1, $2, $3, $4, $5, $6) RETURNING "id""#,
    vec![
      chat_id.into(),
      change.send_at.clone().unwrap_or_else(|| SEND_AT.to_owned()).into(),
      change.lead_minutes.unwrap_or(LEAD_MINUTES).into(),
      change.share_hours.unwrap_or(SHARE_HOURS).into(),
      i64::from(change.enabled.unwrap_or(false)).into(),
      now.into(),
    ],
  )
  .await?
  .ok_or_else(|| Refusal::bad("the room was not added"))?;

  Ok(row.try_get("", "id")?)
}

/// The fields a change names written on a row, the rest left as they stand: the row as it now stands, none
/// where no such row stands. A change a row may not hold is refused before anything is written.
pub async fn set(db: &impl ConnectionTrait, id: i64, change: &Change) -> Result<Option<Brief>, Refusal> {
  change.check()?;

  let Some(current) = brief(db, id).await? else {
    return Ok(None);
  };

  run(
    db,
    r#"UPDATE "news_briefs" SET "chat_id" = $1, "send_at" = $2, "lead_minutes" = $3, "share_hours" = $4,
      "enabled" = $5 WHERE "id" = $6"#,
    vec![
      change.chat_id.unwrap_or(current.chat_id).into(),
      change.send_at.clone().unwrap_or(current.send_at).into(),
      change.lead_minutes.unwrap_or(current.lead_minutes).into(),
      change.share_hours.unwrap_or(current.share_hours).into(),
      i64::from(change.enabled.unwrap_or(current.enabled)).into(),
      id.into(),
    ],
  )
  .await?;

  brief(db, id).await
}

/// The day a page was started for a row, written as it starts, so a restart the same morning does not
/// start a second one.
pub async fn mark_started(db: &impl ConnectionTrait, id: i64, day: &str) -> Result<(), Refusal> {
  run(
    db,
    r#"UPDATE "news_briefs" SET "last_day" = $1 WHERE "id" = $2"#,
    vec![day.into(), id.into()],
  )
  .await?;

  Ok(())
}

/// A room's row taken away: the row as it stood, none where no such row stood.
pub async fn remove(db: &impl ConnectionTrait, id: i64) -> Result<Option<Brief>, Refusal> {
  one(
    db,
    &format!(r#"DELETE FROM "news_briefs" WHERE "id" = $1 RETURNING {COLUMNS}"#),
    vec![id.into()],
  )
  .await?
  .as_ref()
  .map(brief_of)
  .transpose()
}

/// One day off: the day and its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Holiday {
  pub day: String,
  pub name: String,
}

fn holiday_of(row: &QueryResult) -> Result<Holiday, Refusal> {
  Ok(Holiday {
    day: row.try_get("", "day")?,
    name: row.try_get("", "name")?,
  })
}

/// The holiday a day is, none where it is a day like any other.
pub async fn holiday(db: &impl ConnectionTrait, day: &str) -> Result<Option<Holiday>, Refusal> {
  one(db, r#"SELECT "day", "name" FROM "holidays" WHERE "day" = $1"#, vec![day.into()])
    .await?
    .as_ref()
    .map(holiday_of)
    .transpose()
}

/// Every holiday of one year, in order.
pub async fn holidays_of(db: &impl ConnectionTrait, year: i32) -> Result<Vec<Holiday>, Refusal> {
  all(
    db,
    r#"SELECT "day", "name" FROM "holidays" WHERE "day" LIKE $1 ORDER BY "day""#,
    vec![format!("{year:04}-%").into()],
  )
  .await?
  .iter()
  .map(holiday_of)
  .collect()
}

/// A day added, or renamed where it already stands, its name trimmed: the holiday as it now stands. A day
/// that is not a calendar date and a name with no words are refused before anything is written.
pub async fn set_holiday(db: &impl ConnectionTrait, day: &str, name: &str) -> Result<Holiday, Refusal> {
  if !is_day(day) {
    return Err(Refusal::bad(format!("{day} is not a day; write it YYYY-MM-DD")));
  }

  let name = name.trim();

  if name.is_empty() {
    return Err(Refusal::bad("a holiday has a name"));
  }

  run(
    db,
    r#"INSERT INTO "holidays" ("day", "name") VALUES ($1, $2) ON CONFLICT ("day") DO UPDATE SET "name" = excluded."name""#,
    vec![day.into(), name.into()],
  )
  .await?;

  Ok(Holiday {
    day: day.to_owned(),
    name: name.to_owned(),
  })
}

/// A day taken off the calendar: whether it stood on it.
pub async fn remove_holiday(db: &impl ConnectionTrait, day: &str) -> Result<bool, Refusal> {
  Ok(run(db, r#"DELETE FROM "holidays" WHERE "day" = $1"#, vec![day.into()]).await? > 0)
}
