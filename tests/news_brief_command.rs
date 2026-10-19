//! `bmo news-brief` over a scratch database (`docs/news-brief.md`, "The commands"): rooms added off on the
//! defaults, changed, switched and removed; what a row may not hold refused before anything is written; the
//! holiday calendar edited; one room's page started at once through bmo-agency; and the binary as a shell
//! runs it.

mod support;

use std::collections::HashMap;
use std::path::Path;

use bmo_artifact::plumbing::verb::{Shell, VerbError};
use bmo_artifact::settings::Settings;
use bmo_artifact::verbs::news_brief::command::{CommandArgs, answer};
use clap::Parser;
use support::ROOT;
use support::agency::{self, Book as AgencyBook, Stand};
use support::cli::cli;
use support::scratch::Scratch;
use support::units::{self, Book};

const ROOM: i64 = -100;

// 2026-10-07 09:00 in the committed time zone, UTC.
const NOW: i64 = 1_791_363_600_000;

fn line(words: &[&str]) -> CommandArgs {
  #[derive(Parser)]
  struct Line {
    #[command(flatten)]
    command: CommandArgs,
  }

  Line::try_parse_from(std::iter::once("news-brief").chain(words.iter().copied()))
    .expect("the line reads")
    .command
}

async fn bench(agency: AgencyBook) -> (Scratch, Shell, Stand) {
  let scratch = Scratch::new().await;
  let (_, address) = units::serve(Book {
    rooms: vec![(ROOM, "Morning Room".to_owned())],
    ..Book::default()
  })
  .await;
  let (stand, agency) = agency::serve(agency).await;
  let mut environment = HashMap::from([("BMO_ARTIFACT_DATABASE__URL".to_owned(), scratch.address())]);

  environment.extend(
    units::pointed_at(&address, &agency)
      .into_iter()
      .map(|(name, value)| (name.to_owned(), value)),
  );

  let shell = Shell::new(Settings::read(Path::new(ROOT), Some(environment)).expect("the settings"))
    .expect("the shell");

  (scratch, shell, stand)
}

async fn said(shell: &Shell, words: &[&str]) -> Result<Vec<String>, VerbError> {
  answer(shell, &line(words), NOW).await
}

#[tokio::test]
async fn a_room_is_added_off_on_the_defaults_then_changed_switched_and_removed() {
  let (_scratch, shell, _) = bench(AgencyBook::default()).await;
  let room = ROOM.to_string();
  let added = said(&shell, &["--add", "--chat", &room]).await.expect("added");

  assert_eq!(
    added,
    [
      "#1 off · Morning Room (-100) · weekdays at 08:30 (writing starts 40 minutes before) · link lives 24 hours · last never"
    ]
  );

  let changed = said(&shell, &["--set", "1", "--at", "07:50", "--lead", "30", "--share-hours", "6"])
    .await
    .expect("set");

  assert!(
    changed[0].starts_with("#1 off · ") && changed[0].contains("at 07:50 (writing starts 30 minutes"),
    "{changed:?}"
  );
  assert!(said(&shell, &["--on", "1"]).await.expect("on")[0].starts_with("#1 on · "));
  assert!(said(&shell, &["--off", "1"]).await.expect("off")[0].starts_with("#1 off · "));

  let listed = said(&shell, &[]).await.expect("listed");

  assert_eq!(listed.len(), 2);
  assert_eq!(listed[1], "holidays: 0 in 2026, 0 in 2027");
  assert!(said(&shell, &["--remove", "1"]).await.expect("removed")[0].ends_with(": removed"));
  assert_eq!(said(&shell, &[]).await.expect("listed")[0], "no room gets the morning news page");

  let switched_on = said(&shell, &["--add", "--chat", "7", "--on"]).await.expect("added");

  assert!(switched_on[0].starts_with("#2 on · 7 · "), "{switched_on:?}");
}

#[tokio::test]
async fn what_a_row_may_not_hold_is_refused_before_anything_is_written() {
  let (scratch, shell, _) = bench(AgencyBook::default()).await;
  let room = ROOM.to_string();
  let refusal = |answered: Result<Vec<String>, VerbError>| match answered {
    Err(VerbError::Refused(said)) => said,
    Err(VerbError::Archive(refusal)) => refusal.detail,
    other => panic!("not refused: {other:?}"),
  };

  said(&shell, &["--add", "--chat", &room]).await.expect("added");

  assert!(refusal(said(&shell, &["--add", "--chat", &room, "--at", "8:30"]).await).contains("HH:MM"));
  assert!(refusal(said(&shell, &["--set", "1", "--lead", "5"]).await).contains("lead"));
  assert!(refusal(said(&shell, &["--set", "1", "--share-hours", "0"]).await).contains("hours"));
  assert!(refusal(said(&shell, &["--add", "--chat", "room"]).await).contains("chat id"));
  assert!(refusal(said(&shell, &["--add"]).await).contains("--chat"));
  assert!(refusal(said(&shell, &["--set", "9", "--lead", "30"]).await).contains("no morning page #9"));
  assert_eq!(scratch.count(r#"SELECT COUNT(*) FROM "news_briefs""#, vec![]).await, 1);
  assert_eq!(scratch.count(r#"SELECT "lead_minutes" FROM "news_briefs""#, vec![]).await, 40);
}

#[tokio::test]
async fn a_holiday_is_added_renamed_listed_and_taken_off() {
  let (_scratch, shell, _) = bench(AgencyBook::default()).await;

  assert_eq!(
    said(&shell, &["--holiday-add", "2026-10-09", "  Hangeul Day "]).await.expect("added"),
    ["2026-10-09 Hangeul Day: added"]
  );
  said(&shell, &["--holiday-add", "2026-10-09", "Alphabet Day"]).await.expect("renamed");
  said(&shell, &["--holiday-add", "2027-01-01", "New Year's Day"]).await.expect("added");

  assert_eq!(said(&shell, &["--holidays"]).await.expect("listed"), ["2026-10-09 Alphabet Day"]);
  assert_eq!(
    said(&shell, &["--holidays", "2027"]).await.expect("listed"),
    ["2027-01-01 New Year's Day"]
  );
  assert_eq!(
    said(&shell, &[]).await.expect("listed").last().map(String::as_str),
    Some("holidays: 1 in 2026, 1 in 2027")
  );
  assert_eq!(
    said(&shell, &["--holiday-remove", "2026-10-09"]).await.expect("removed"),
    ["2026-10-09: taken off"]
  );
  assert!(said(&shell, &["--holiday-remove", "2026-10-09"]).await.is_err());
  assert!(said(&shell, &["--holiday-add", "2026-13-01", "Nothing"]).await.is_err());
}

#[tokio::test]
async fn a_rooms_page_is_started_at_once_through_bmo_agency() {
  let (_scratch, shell, stand) = bench(AgencyBook::default()).await;
  let room = ROOM.to_string();

  said(&shell, &["--add", "--chat", &room, "--share-hours", "6"]).await.expect("added");

  let started = said(&shell, &["--run", "1", "--send-to", "7"]).await.expect("started");
  let unit = format!("bmo-news-brief-{NOW}");

  assert_eq!(
    started,
    [format!(
      "the page of #1 for 2026-10-07 runs as {unit} (run 100); it is sent to 7 when it is done"
    )]
  );

  let (opened, launches) = {
    let book = stand.book();

    (book.opened.clone(), book.launches.clone())
  };

  assert_eq!(
    (opened[0].agent.as_str(), opened[0].chat_id, opened[0].depth, opened[0].search_id),
    ("search", Some(ROOM), 0, None)
  );
  assert_eq!(opened[0].started_by.as_deref(), Some("news-brief:1"));
  assert_eq!(launches[0].0, 100);
  assert_eq!(
    launches[0].1.command.args,
    [
      "news-brief-runner",
      "--chat",
      "-100",
      "--day",
      "2026-10-07",
      "--share-hours",
      "6",
      "--send-to",
      "7",
      "--run",
      "100",
      "--unit",
      &unit
    ]
  );

  let (_scratch, shell, stand) = bench(AgencyBook {
    launch_refusal: Some("no host".to_owned()),
    ..AgencyBook::default()
  })
  .await;

  said(&shell, &["--add", "--chat", &room]).await.expect("added");

  let refused = said(&shell, &["--run", "1"]).await.expect_err("refused").to_string();

  assert!(refused.contains("did not come up") && refused.contains("no host"), "{refused}");
  assert_eq!(
    stand.book().closes.iter().map(|(_, close)| close.outcome.clone()).collect::<Vec<_>>(),
    ["error"]
  );
}

#[tokio::test]
async fn the_binary_lists_and_refuses_as_a_shell_runs_it() {
  let scratch = Scratch::new().await;
  let data = tempfile::tempdir().expect("a folder");
  let env = [
    ("BMO_ARTIFACT_DATABASE__URL", scratch.address()),
    ("BMO_ARTIFACT_FILES__ROOT", data.path().display().to_string()),
  ];
  let listed = cli(&env, &["news-brief"], None);

  assert!(listed.ok, "{}", listed.err);
  assert!(
    listed.out.starts_with("no room gets the morning news page\nholidays: "),
    "{}",
    listed.out
  );

  let refused = cli(&env, &["news-brief", "--add", "--chat", "-100", "--at", "8:30"], None);

  assert_eq!(refused.code, Some(1));
  assert!(refused.err.contains("8:30 is not a time; write it HH:MM"), "{}", refused.err);
  assert_eq!(scratch.count(r#"SELECT COUNT(*) FROM "news_briefs""#, vec![]).await, 0);
}
