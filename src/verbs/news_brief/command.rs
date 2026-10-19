//! `news-brief` (`docs/news-brief.md`, "The commands"): the rooms that get the morning news page, each with its
//! own time, listed, added, changed, switched and removed; one room's page started at once; and the holiday
//! calendar's days listed, added and taken off. A room added is off until it is switched on, so a page that
//! starts going out every morning is switched on as a step of its own, after a trial.

use std::collections::HashMap;

use chrono::Datelike;
use clap::Args;
use sea_orm::DatabaseConnection;

use super::calendar::{date_of, day_of};
use super::launch::{Start, launch};
use crate::plumbing::verb::{Answer, Shell, Verb, VerbError};
use crate::store::news_briefs::{self, Brief, Change, Named, holidays_of, remove_holiday, set_holiday};

/// What `news-brief` is given: one action, and the fields it names.
#[derive(Args, Clone, Debug, Default)]
#[command(allow_negative_numbers = true)]
pub struct CommandArgs {
  /// Adds a room.
  #[arg(long)]
  pub add: bool,
  /// The room, by its chat id.
  #[arg(long)]
  pub chat: Option<String>,
  /// The time its page is sent, HH:MM.
  #[arg(long)]
  pub at: Option<String>,
  /// How many minutes before that the writing starts.
  #[arg(long)]
  pub lead: Option<String>,
  /// How many hours its page's share link lives.
  #[arg(long)]
  pub share_hours: Option<String>,
  /// Switches a room on by its number, or a room added on.
  #[arg(long, num_args = 0..=1, value_name = "ID")]
  pub on: Option<Option<String>>,
  /// Switches a room off by its number.
  #[arg(long, value_name = "ID")]
  pub off: Option<String>,
  /// Changes the fields named on a room by its number.
  #[arg(long, value_name = "ID")]
  pub set: Option<String>,
  /// Takes a room away by its number.
  #[arg(long, value_name = "ID")]
  pub remove: Option<String>,
  /// Makes a room's page of today at once.
  #[arg(long, value_name = "ID")]
  pub run: Option<String>,
  /// The room a page made at once goes to instead.
  #[arg(long)]
  pub send_to: Option<String>,
  /// Lists the days off of a year, this year's where none is named.
  #[arg(long, num_args = 0..=1, value_name = "YEAR")]
  pub holidays: Option<Option<String>>,
  /// Adds a day off, or renames it.
  #[arg(long, num_args = 2, value_names = ["YYYY-MM-DD", "NAME"])]
  pub holiday_add: Option<Vec<String>>,
  /// Takes a day off the calendar.
  #[arg(long, value_name = "YYYY-MM-DD")]
  pub holiday_remove: Option<String>,
}

fn refused(why: impl std::fmt::Display) -> VerbError {
  VerbError::Refused(format!("news-brief: {why}"))
}

fn number(text: &str) -> Result<i64, VerbError> {
  text.trim().parse::<i64>().map_err(|_| refused(format!("{text} is not a row's number")))
}

impl CommandArgs {
  fn named(&self) -> Named {
    Named {
      chat: self.chat.clone(),
      at: self.at.clone(),
      lead: self.lead.clone(),
      share_hours: self.share_hours.clone(),
    }
  }
}

// What a room is called by: its name with its number, or its number alone where bmo-receiver names none or
// cannot be asked.
fn room(names: &HashMap<i64, String>, chat_id: i64) -> String {
  names
    .get(&chat_id)
    .map_or_else(|| chat_id.to_string(), |name| format!("{name} ({chat_id})"))
}

/// One row as the command shows it.
pub fn line(row: &Brief, names: &HashMap<i64, String>) -> String {
  format!(
    "#{} {} · {} · weekdays at {} (writing starts {} minutes before) · link lives {} hours · last {}",
    row.id,
    if row.enabled { "on" } else { "off" },
    room(names, row.chat_id),
    row.send_at,
    row.lead_minutes,
    row.share_hours,
    row.last_day.as_deref().unwrap_or("never")
  )
}

async fn must_exist(db: &DatabaseConnection, id: i64) -> Result<Brief, VerbError> {
  news_briefs::brief(db, id)
    .await?
    .ok_or_else(|| refused(format!("no morning page #{id}")))
}

/// The rooms, and how many holidays the year of `today` and the next hold.
pub async fn list(
  db: &DatabaseConnection,
  names: &HashMap<i64, String>,
  today: &str,
) -> Result<Vec<String>, VerbError> {
  let mut lines: Vec<String> = news_briefs::briefs(db).await?.iter().map(|row| line(row, names)).collect();

  if lines.is_empty() {
    lines.push("no room gets the morning news page".to_owned());
  }

  let year = date_of(today).map_or(0, |date| date.year());
  let this = holidays_of(db, year).await?.len();
  let next = holidays_of(db, year + 1).await?.len();

  lines.push(format!("holidays: {this} in {year}, {next} in {}", year + 1));

  Ok(lines)
}

/// What the command answers on what it was given, at `now_ms`.
pub async fn answer(shell: &Shell, asked: &CommandArgs, now_ms: i64) -> Result<Vec<String>, VerbError> {
  let db = shell.documents().await?;
  let today = day_of(now_ms, shell.settings.time.zone);
  // The rooms' names are read only for the lines that show a room.
  let names = || async {
    shell.receiver.room_names().await.unwrap_or_else(|why| {
      eprintln!("news-brief: the rooms' names could not be read: {why}");
      HashMap::new()
    })
  };

  if let Some(holiday) = &asked.holiday_add {
    let (day, name) = (&holiday[0], &holiday[1]);
    let added = set_holiday(db, day, name).await?;

    return Ok(vec![format!("{} {}: added", added.day, added.name)]);
  }

  if let Some(day) = &asked.holiday_remove {
    if !remove_holiday(db, day).await? {
      return Err(refused(format!("{day} is not in the calendar")));
    }

    return Ok(vec![format!("{day}: taken off")]);
  }

  if let Some(year) = &asked.holidays {
    let year = match year {
      Some(year) => year.trim().parse::<i32>().map_err(|_| refused(format!("{year} is not a year")))?,
      None => date_of(&today).map_or(0, |date| date.year()),
    };

    return Ok(
      holidays_of(db, year)
        .await?
        .into_iter()
        .map(|day| format!("{} {}", day.day, day.name))
        .collect(),
    );
  }

  if let Some(id) = &asked.run {
    let row = must_exist(db, number(id)?).await?;
    let send_to = match &asked.send_to {
      Some(named) => {
        Some(named.trim().parse::<i64>().map_err(|_| refused("a room is named by its chat id"))?)
      }
      None => None,
    };
    let start = Start {
      row: &row,
      day: &today,
      send_at: None,
      send_to,
    };
    let launched = launch(&*shell.agency, &start, now_ms).await?;

    return Ok(vec![format!(
      "the page of #{} for {today} runs as {} (run {}); it is sent to {} when it is done",
      row.id,
      launched.unit,
      launched.run_id,
      send_to.unwrap_or(row.chat_id)
    )]);
  }

  if asked.add {
    let mut change = Change::read(&asked.named())?;

    if change.chat_id.is_none() {
      return Err(refused("--add takes --chat <chatId>"));
    }

    change.enabled = Some(matches!(asked.on, Some(None)));

    let id = news_briefs::add(db, &change, now_ms / 1000).await?;

    return Ok(vec![line(&must_exist(db, id).await?, &names().await)]);
  }

  // A row changed: switched on or off, or the fields named set, each by its number.
  let changed = match (&asked.on, &asked.off, &asked.set) {
    (Some(Some(id)), _, _) => Some((id, Some(true))),
    (_, Some(id), _) => Some((id, Some(false))),
    (_, _, Some(id)) => Some((id, None)),
    _ => None,
  };

  if let Some((id, enabled)) = changed {
    let change = Change {
      enabled,
      ..Change::read(&asked.named())?
    };
    let row = news_briefs::set(db, number(id)?, &change)
      .await?
      .ok_or_else(|| refused(format!("no morning page #{id}")))?;

    return Ok(vec![line(&row, &names().await)]);
  }

  if let Some(id) = &asked.remove {
    let id = number(id)?;
    let row = news_briefs::remove(db, id)
      .await?
      .ok_or_else(|| refused(format!("no morning page #{id}")))?;

    return Ok(vec![format!("{}: removed", line(&row, &names().await))]);
  }

  list(db, &names().await, &today).await
}

/// `news-brief`.
pub struct NewsBriefCommand;

impl Verb for NewsBriefCommand {
  const NAME: &'static str = "news-brief";
  const ABOUT: &'static str = "The rooms that get the morning news page and the holiday calendar their working \
                               days skip: listed, added, changed, switched, removed, and one room's page made at \
                               once.";

  type Args = CommandArgs;

  async fn run(&self, asked: CommandArgs, shell: &Shell) -> Result<Answer, VerbError> {
    answer(shell, &asked, crate::now_ms()).await.map(Answer::from)
  }
}

crate::verb!(NewsBriefCommand);
