//! `bmo runner news-brief` over a scratch database, with every unit it calls stood in for
//! (`docs/news-brief.md`, "How one page is made"): what a run reads of its words, a page filed as a page
//! document of its room with a share link and its message said into the room at its time, a page stopped once
//! drawn, a run that fails telling the owner, and the writer run apart under the news task.

mod support;

use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Mutex;

use bmo_artifact::clients::agency::NewRun;
use bmo_artifact::plumbing::verb::{Shell, VerbError};
use bmo_artifact::settings::Settings;
use bmo_artifact::verbs::news_brief::calendar::moment;
use bmo_artifact::verbs::news_brief::runner::{Asked, Clock, RunnerArgs, make};
use clap::Parser;
use support::ROOT;
use support::agency::{self, Book as AgencyBook, Stand, answered};
use support::news_brief::brief_value;
use support::scratch::Scratch;
use support::units::{self, Book, Units};

const ROOM: i64 = -9_200_000_000_002;
const DAY: &str = "2026-09-28";
const CONSOLE: &str = "https://console.example/bmo";
const REQUEST: &str = "Write the news page for {{day}}. Written at: {{askedAt}}.";

// 2026-09-28 08:00 in the committed time zone, UTC.
const EIGHT: i64 = 1_790_582_400_000;

/// A clock standing at one moment, which keeps every moment it was asked to wait for and waits for none.
struct Stopped {
  now: i64,
  waited: Mutex<Vec<i64>>,
}

impl Clock for Stopped {
  fn now_ms(&self) -> i64 {
    self.now
  }

  fn wait_until(&self, at_ms: i64) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
    self.waited.lock().expect("the waits").push(at_ms);
    Box::pin(async {})
  }
}

fn stopped_at(now: i64) -> Stopped {
  Stopped {
    now,
    waited: Mutex::new(vec![]),
  }
}

// What the binary's line reads as the runner's words.
fn args(line: &[&str]) -> Option<RunnerArgs> {
  #[derive(Parser)]
  struct Line {
    #[command(flatten)]
    runner: RunnerArgs,
  }

  Line::try_parse_from(std::iter::once("runner").chain(line.iter().copied()))
    .ok()
    .map(|line| line.runner)
}

fn asked(line: &[&str]) -> Option<Asked> {
  args(line)?.read(EIGHT, chrono_tz::UTC)
}

// One runner's world: a scratch database, a file root and a prompt tree of its own, and every unit stood in for.
struct Bench {
  scratch: Scratch,
  units: Units,
  agency: Stand,
  shell: Shell,
  root: tempfile::TempDir,
}

impl Bench {
  async fn new(agency: AgencyBook) -> Self {
    let scratch = Scratch::new().await;
    let (units, address) = units::serve(Book {
      rooms: vec![(ROOM, "Morning Room".to_owned())],
      ..Book::default()
    })
    .await;
    let (stand, agency) = agency::serve(agency).await;
    let root = tempfile::tempdir().expect("a folder");
    let search = root.path().join("code/prompts/agents/search");

    std::fs::create_dir_all(&search).expect("the prompt tree");
    std::fs::write(search.join("news-brief-request.md"), REQUEST).expect("the request");

    let mut environment = HashMap::from([("BMO_ARTIFACT_DATABASE__URL".to_owned(), scratch.address())]);

    environment.extend(
      units::pointed_at(&address, &agency)
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value)),
    );
    environment.extend([
      ("BMO_ARTIFACT_FILES__ROOT".to_owned(), root.path().join("data").display().to_string()),
      ("BMO_ARTIFACT_PROMPTS__CODE".to_owned(), root.path().join("code").display().to_string()),
      ("BMO_ARTIFACT_CONSOLE__ADDRESS".to_owned(), CONSOLE.to_owned()),
    ]);

    let shell = Shell::new(Settings::read(Path::new(ROOT), Some(environment)).expect("the settings"))
      .expect("the shell");

    Self {
      scratch,
      units,
      agency: stand,
      shell,
      root,
    }
  }

  // A writer's answer already written, as a file of its own.
  fn answer(&self, text: &str) -> String {
    let path = self.root.path().join("answer.txt");

    std::fs::write(&path, text).expect("the answer is written");
    path.display().to_string()
  }

  fn folder(&self) -> PathBuf {
    self.root.path().join("data/news-brief")
  }

  async fn pages(&self) -> i64 {
    self
      .scratch
      .count(
        r#"SELECT COUNT(*) FROM "searches" WHERE "chat_id" = $1 AND "kind" = 'page'"#,
        vec![ROOM.into()],
      )
      .await
  }
}

#[test]
fn a_run_is_read_with_its_defaults_and_refused_where_it_does_not_read() {
  let plain = asked(&["--chat", "-100"]).expect("read");

  assert_eq!((plain.chat_id, plain.send_to, plain.day.as_str()), (-100, -100, DAY));
  assert_eq!(plain.share_hours, 24.0);
  assert!(plain.send_at.is_none() && plain.answer.is_none() && plain.run_id.is_none() && !plain.no_send);

  let trial = asked(&[
    "--chat",
    "-100",
    "--send-to",
    "7",
    "--day",
    "2026-09-29",
    "--send-at",
    "08:30",
    "--run",
    "101",
    "--unit",
    "bmo-news-brief-1",
    "--no-send",
  ])
  .expect("read");

  assert_eq!(trial.send_to, 7);
  assert_eq!(trial.send_at.map(|(_, at)| at), moment("2026-09-29", "08:30", chrono_tz::UTC));
  assert_eq!((trial.run_id, trial.unit.as_str()), (Some(101), "bmo-news-brief-1"));
  assert!(trial.no_send);

  for refused in [
    &["--send-to", "7"][..],
    &["--chat", "room"],
    &["--chat", "-100", "--day", "2026-9-29"],
    &["--chat", "-100", "--share-hours", "0"],
    &["--chat", "-100", "--send-at", "8:30"],
    &["--chat", "-100", "--run", "one"],
  ] {
    assert!(asked(refused).is_none(), "{refused:?}");
  }
}

#[tokio::test]
async fn a_page_is_filed_with_a_share_link_and_its_message_said_into_the_room_at_its_time() {
  let bench = Bench::new(AgencyBook::default()).await;
  let answer = bench.answer(&brief_value().to_string());
  let room = ROOM.to_string();
  let asked = asked(&[
    "--chat",
    &room,
    "--day",
    DAY,
    "--share-hours",
    "6",
    "--send-at",
    "08:30",
    "--answer",
    &answer,
  ])
  .expect("read");
  let clock = stopped_at(EIGHT);
  let lines = make(&bench.shell, &asked, &clock).await.expect("the page went out");

  assert!(
    std::fs::read_to_string(bench.folder().join(format!("news-{DAY}-{ROOM}.html")))
      .expect("the page is kept")
      .contains("First summary")
  );
  assert_eq!(bench.pages().await, 1);
  assert_eq!(
    bench
      .scratch
      .count(
        r#"SELECT ("expires_at" - "created_at") FROM "web_search_shares" s JOIN "searches" d ON d."id" = s."search_id" WHERE d."chat_id" = $1"#,
        vec![ROOM.into()],
      )
      .await,
    6 * 3600
  );
  assert_eq!(
    *clock.waited.lock().expect("the waits"),
    [moment(DAY, "08:30", chrono_tz::UTC).expect("a moment")]
  );

  let sent = bench.units.book().sent.clone();

  assert_eq!(sent.len(), 1);
  assert_eq!(
    (sent[0]["asker"]["chat"].as_i64(), sent[0]["asker"]["job"].as_bool()),
    (Some(ROOM), Some(false))
  );
  assert_eq!((sent[0]["tool"].as_str(), sent[0]["input"]["chat"].as_i64()), (Some("say"), Some(ROOM)));

  let text = sent[0]["input"]["text"].as_str().expect("a message");

  assert!(text.starts_with(CONSOLE) && text.contains("/share/"), "{text}");
  assert!(text.ends_with("• First summary\n• Second summary"), "{text}");
  assert!(
    lines[0].ends_with(&format!("sent to {ROOM}: {}", text.lines().next().expect("the link"))),
    "{lines:?}"
  );
}

#[tokio::test]
async fn a_page_asked_to_stop_once_drawn_is_neither_filed_nor_sent() {
  let bench = Bench::new(AgencyBook::default()).await;
  let answer = bench.answer(&brief_value().to_string());
  let room = ROOM.to_string();
  let asked = asked(&["--chat", &room, "--day", DAY, "--answer", &answer, "--no-send"]).expect("read");

  make(&bench.shell, &asked, &stopped_at(EIGHT)).await.expect("drawn");

  assert!(bench.folder().join(format!("news-{DAY}-{ROOM}.html")).exists());
  assert_eq!(bench.pages().await, 0);
  assert!(bench.units.book().sent.is_empty());
}

#[tokio::test]
async fn a_run_that_fails_tells_the_owner_and_sends_nothing() {
  let bench = Bench::new(AgencyBook::default()).await;
  let answer = bench.answer("no news today");
  let room = ROOM.to_string();
  let asked = asked(&["--chat", &room, "--day", DAY, "--answer", &answer, "--unit", "bmo-news-brief-1"])
    .expect("read");

  match make(&bench.shell, &asked, &stopped_at(EIGHT)).await {
    Err(VerbError::Ended { code: 1, said }) => assert!(said.contains("no JSON object"), "{said}"),
    other => panic!("not a failure: {other:?}"),
  }

  assert_eq!(bench.pages().await, 0);
  assert!(bench.units.book().sent.is_empty());

  let handed = bench.units.book().handed.clone();

  assert_eq!(handed.len(), 1);
  assert_eq!((handed[0].agent.as_deref(), handed[0].chat), (None, None));
  assert_eq!(handed[0].sort, "news_brief");
  assert!(
    handed[0].said.order.contains(&format!("room {ROOM} was sent nothing")),
    "{}",
    handed[0].said.order
  );
  assert!(handed[0].said.order.contains("bmo-news-brief-1"), "{}", handed[0].said.order);
}

#[test]
fn a_call_that_does_not_read_is_refused_with_the_usage() {
  let run = support::cli::cli(&[], &["runner", "news-brief", "--send-to", "7"], None);

  assert_eq!(run.code, Some(2));
  assert!(run.err.starts_with("usage: bmo runner news-brief"), "{}", run.err);
}

#[tokio::test]
async fn the_writer_runs_apart_on_the_search_line_under_the_news_task_and_its_row_is_closed() {
  let mut book = AgencyBook::lending();

  book.apart = answered(&brief_value().to_string());

  let bench = Bench::new(book).await;
  let opened = NewRun {
    agent: "search".to_owned(),
    chat_id: Some(ROOM),
    unit: "bmo-news-brief-1".to_owned(),
    started_at: EIGHT / 1000,
    ..NewRun::default()
  };
  let run_id = bench.shell.agency.open_run(&opened).await.expect("the run's row").to_string();
  let room = ROOM.to_string();
  let asked = asked(&[
    "--chat",
    &room,
    "--day",
    DAY,
    "--run",
    &run_id,
    "--unit",
    "bmo-news-brief-1",
    "--no-send",
  ])
  .expect("read");

  make(&bench.shell, &asked, &stopped_at(EIGHT)).await.expect("drawn");

  let started = bench.agency.book().started.clone();

  assert_eq!(started.len(), 1);
  assert_eq!(
    (started[0].line.as_str(), started[0].instructions.name.as_str()),
    ("search", "news-brief")
  );
  assert_eq!(started[0].instructions.text, None);
  assert!(started[0].prompt.starts_with(&format!("Write the news page for {DAY}. Written at: ")));
  assert_eq!(started[0].log.as_ref().map(|log| log.run.as_str()), Some("bmo-news-brief-1"));
  assert!(bench.folder().join(format!("answer-{DAY}-{ROOM}.txt")).exists());
  assert_eq!(
    bench
      .agency
      .book()
      .closes
      .iter()
      .map(|(run, close)| (*run, close.outcome.clone()))
      .collect::<Vec<_>>(),
    [(100, "done".to_owned())]
  );
}
