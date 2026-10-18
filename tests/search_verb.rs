//! `bmo search` as a caller runs it (`docs/research-runs.md`, "The launch and the runner"), against a scratch
//! database and stand-ins for bmo-receiver and bmo-agency: every refusal made before a row exists,
//! a launch whose run never came up closing its own row and the search it opened and taking the question's
//! file with it, who asked read off bmo-receiver, and the runner handed the question in a file of the
//! owner's alone and never in its words.

mod support;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bmo_artifact::helpers::marks::fill_marks;
use bmo_artifact::settings::Settings;
use bmo_artifact::verbs::research::Profile;
use bmo_artifact::verbs::research::admission::Demand;
use bmo_artifact::verbs::search::launch::{
  RunnerCall, runner_args, runner_command, usage, whole, write_request,
};
use bmo_artifact::verbs::search::request::{Asker, Options, Request};
use bmo_artifact::wording::Wording;
use serde_json::{Value, json};
use support::ROOT;
use support::agency::{self, Book as AgencyBook, Stand};
use support::cli::{Ran, cli};
use support::scratch::Scratch;
use support::units::{self, Book, Units};

// The room a search is asked in, as a command line names it.
const ROOM: i64 = 42;
const ROOM_TEXT: &str = "42";

struct Space {
  scratch: Scratch,
  _units: Units,
  agency: Stand,
  // The folder the launcher writes the question in.
  temp: tempfile::TempDir,
  settings: Vec<(&'static str, String)>,
}

impl Space {
  async fn new(book: Book, ledger: AgencyBook) -> Self {
    let scratch = Scratch::new().await;
    let (units, address) = units::serve(book).await;
    let (stand, agency) = agency::serve(ledger).await;
    let temp = tempfile::tempdir().expect("a folder");
    let mut settings = units::pointed_at(&address, &agency);

    settings.push(("BMO_ARTIFACT_DATABASE__URL", scratch.address()));
    settings.push(("TMPDIR", temp.path().display().to_string()));
    // The launching process names no home and no turn of its own, whatever the test's does.
    settings.push(("CLAUDE_CONFIG_DIR", String::new()));
    settings.push(("BMO_TURN_AGENT", "agent-a".to_owned()));

    Self {
      scratch,
      _units: units,
      agency: stand,
      temp,
      settings,
    }
  }

  // The binary run on the words given, `stdin` on its standard input, off the runtime's own threads so the
  // stand-ins it calls go on answering.
  async fn bmo(&self, args: &[&str], stdin: &str) -> Ran {
    let settings = self.settings.clone();
    let args: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
    let stdin = stdin.to_owned();

    tokio::task::spawn_blocking(move || {
      let args: Vec<&str> = args.iter().map(String::as_str).collect();

      cli(&settings, &args, Some(&stdin))
    })
    .await
    .expect("the run ends")
  }

  async fn searches(&self) -> Vec<Value> {
    self
      .scratch
      .texts(
        "SELECT json_build_object('id', id, 'status', status, 'query', query, 'author', author, 'kind', kind, \
         'started_depth', started_depth, 'options', options, 'asker_id', asker_id)::text FROM searches ORDER BY id",
      )
      .await
      .iter()
      .map(|row| serde_json::from_str(row).expect("a JSON row"))
      .collect()
  }

  // What the launcher left in its folder for the unit named.
  fn left(&self, unit: &str) -> Vec<String> {
    std::fs::read_dir(self.temp.path())
      .expect("the folder")
      .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
      .filter(|name| name.starts_with(unit))
      .collect()
  }
}

// A space whose bmo-agency refuses every launch.
async fn refusing() -> Space {
  let ledger = AgencyBook {
    launch_refusal: Some("refused for the test".to_owned()),
    ..AgencyBook::lending()
  };

  Space::new(Book::default(), ledger).await
}

#[tokio::test]
async fn refuses_a_call_it_cannot_read_and_opens_no_row_for_any() {
  let space = refusing().await;
  let refusals: [(&[&str], &str, &str); 5] = [
    (&["search", "--query", "rate"], "", "usage: bmo search --chat <chatId>"),
    (&["search", "--chat", ROOM_TEXT], "   ", "usage: bmo search --chat <chatId>"),
    (
      &["search", "--chat", ROOM_TEXT, "--query", "rate", "--depth", "deep"],
      "",
      "usage: bmo search",
    ),
    (
      &["search", "--chat", ROOM_TEXT, "--query", "rate", "--demand", "required"],
      "",
      "usage: bmo search",
    ),
    (
      &["search", "--chat", ROOM_TEXT, "--query", "rate", "--region", "moon"],
      "",
      "--region must be one of domestic|global",
    ),
  ];

  for (args, stdin, said) in refusals {
    let ran = space.bmo(args, stdin).await;

    assert_eq!(ran.code, Some(1), "{}", ran.err);
    assert!(ran.err.contains(said), "{}", ran.err);
  }

  assert!(space.agency.book().opened.is_empty());
  assert!(space.searches().await.is_empty());
}

// The launcher closes its own row rather than leaving it open, takes its scratch file with it, and leaves
// nothing of a failed launch.
#[tokio::test]
async fn the_search_launcher_closes_its_own_row_and_its_search_and_takes_its_scratch_file_with_it() {
  let space = refusing().await;
  let ran = space.bmo(&["search", "--chat", ROOM_TEXT, "--by", "chat:42"], "dinner nearby").await;
  let (run, opened) = {
    let book = space.agency.book();

    (book.runs.first().expect("a run opened").clone(), book.opened[0].clone())
  };
  let searches = space.searches().await;

  assert_eq!(ran.code, Some(1), "{}", ran.err);
  assert!(ran.err.contains(&format!("{} did not come up", run.unit)), "{}", ran.err);
  assert!(ran.err.contains("refused for the test"), "{}", ran.err);
  assert_eq!(run.agent, "search");
  assert!(run.unit.starts_with("bmo-search-"));
  // Not "failed": the run never reached the work.
  assert_eq!(run.outcome.as_deref(), Some("error"));
  // The basic rung, the shallowest on the ladder.
  assert_eq!(run.depth, 0);
  assert_eq!(run.chat_id, Some(ROOM));
  // The ledger's times are epoch seconds.
  assert!(run.started_at < 10_000_000_000, "{}", run.started_at);
  assert!(run.finished_at.is_some_and(|at| at < 10_000_000_000), "{:?}", run.finished_at);
  assert_eq!(opened.cap_ms, None);
  assert_eq!(opened.started_by.as_deref(), Some("chat:42"));
  assert_eq!(opened.search_id, searches[0]["id"].as_i64());
  // The launch was the only one that search would ever have.
  assert_eq!(searches[0]["status"], "failed");
  assert_eq!(searches[0]["query"], "dinner nearby");
  assert_eq!(searches[0]["author"], "searcher");
  assert_eq!(searches[0]["kind"], "search");
  assert!(space.left(&run.unit).is_empty(), "{:?}", space.left(&run.unit));
}

// A follow-up joins the earlier search of its room and opens no row of its own, a launch that never came up
// puts that search back as it stood, and a search it cannot carry on is refused before any row is written.
#[tokio::test]
async fn a_follow_up_carries_its_rooms_earlier_search_on_and_refuses_what_it_cannot() {
  let space = refusing().await;
  let mut ids = vec![];

  for (chat, author, kind) in
    [(ROOM, "searcher", "search"), (ROOM, "bmo", "document"), (7, "searcher", "search")]
  {
    let id = space
      .scratch
      .count(
        "INSERT INTO searches (chat_id, query, started_depth, ended_depth, status, started_at, finished_at, kind, author)
           VALUES ($1, 'studios', 'basic', 'basic', 'done', 1, 2, $2, $3) RETURNING id",
        vec![chat.into(), kind.into(), author.into()],
      )
      .await;

    ids.push(id.to_string());
  }

  for refused in &ids[1..] {
    let ran = space.bmo(&["search", "--chat", ROOM_TEXT, "--into", refused], "more photos").await;

    assert_eq!(ran.code, Some(1), "{}", ran.err);
    assert!(
      ran.err.contains(&format!("search {refused} is no finished search of chat {ROOM}")),
      "{}",
      ran.err
    );
  }

  assert!(space.agency.book().opened.is_empty());

  let ran = space.bmo(&["search", "--chat", ROOM_TEXT, "--into", &ids[0]], "more photos").await;
  let joined = space.agency.book().opened[0].search_id;
  let searches = space.searches().await;

  assert_eq!(ran.code, Some(1), "{}", ran.err);
  assert_eq!(joined, ids[0].parse().ok());
  assert_eq!(searches.len(), 3);
  assert_eq!(searches[0]["status"], "done");
  assert_eq!(searches[0]["query"], "studios");
}

#[tokio::test]
async fn a_page_asked_for_whole_is_an_import_at_the_first_depth_under_its_own_question() {
  let space = refusing().await;

  space
    .bmo(
      &[
        "search",
        "--chat",
        ROOM_TEXT,
        "--import",
        "https://r.example/a",
        "--depth",
        "exhaustive",
      ],
      "",
    )
    .await;

  let search = space.searches().await.remove(0);
  let wording = Wording::english().expect("the English wording");

  assert_eq!(
    search["query"],
    fill_marks(&wording.search.import_query, &[("source", "https://r.example/a")])
  );
  assert_eq!(search["kind"], "import");
  assert_eq!(search["started_depth"], "basic");
  assert_eq!(search["options"], r#"{"source":"https://r.example/a"}"#);
}

// A launch bmo-agency takes answers that the run is on its way, leaves the question in the runner's file with
// who asked and when as bmo-receiver keeps them, and hands bmo-agency this binary on the runner's words with
// the road the question came in on and the agent whose turn asked.
#[tokio::test]
async fn a_launch_reads_who_asked_off_bmo_receiver_and_hands_bmo_agency_the_runner_on_its_words() {
  let mut book = Book::default();

  book.messages.insert(
    (ROOM, 77),
    json!({ "sender_id": 5, "sender_name": "tester", "sent_at": 1_700_000_000_123_i64, "via": "chat-road" }),
  );

  let space = Space::new(book, AgencyBook::lending()).await;
  let ran = space
    .bmo(
      &[
        "search",
        "--chat",
        ROOM_TEXT,
        "--message",
        "77",
        "--query",
        "how many times did the rate rise",
      ],
      "",
    )
    .await;
  let ((run_id, launch), unit) = {
    let book = space.agency.book();

    (book.launches[0].clone(), book.runs[0].unit.clone())
  };

  assert!(ran.ok, "{}", ran.err);
  assert!(
    ran
      .out
      .contains(&format!("search launched as {unit}; it answers chat {ROOM} by itself")),
    "{}",
    ran.out
  );
  assert_eq!(run_id, 100);
  assert_eq!(launch.agent, "search");
  assert_eq!(launch.title, "how many times did the rate rise");
  assert!(launch.command.program.ends_with("bmo-artifact-cli"), "{}", launch.command.program);
  assert_eq!(launch.command.args[..2], ["search-runner".to_owned(), ROOM_TEXT.to_owned()]);
  assert_eq!(
    launch.command.args[launch.command.args.len() - 4..],
    ["--via", "chat-road", "--asked-by", "agent-a"]
  );
  assert!(
    !launch.command.args.iter().any(|arg| arg.contains("rate rise")),
    "{:?}",
    launch.command.args
  );
  assert_eq!(launch.command.cwd, ROOT);
  assert!(launch.command.environment.contains_key("BMO_ARTIFACT_UNITS__RECEIVER_TOKEN"));
  assert!(!launch.command.environment.contains_key("BMO_TURN_AGENT"));
  assert_eq!(space.searches().await[0]["asker_id"], 5);

  let asked: Value =
    serde_json::from_str(&std::fs::read_to_string(&launch.command.args[2]).expect("the question's file"))
      .unwrap();

  assert_eq!(asked["query"], "how many times did the rate rise");
  assert_eq!(asked["asker"]["name"], "tester");
  assert_eq!(asked["askedAt"], 1_700_000_000);
  assert_eq!(space.left(&unit), [format!("{unit}.txt")]);
}

#[test]
fn a_number_is_read_off_the_front_of_its_text() {
  assert_eq!(whole("-100"), Some(-100));
  assert_eq!(whole(" 42 "), Some(42));
  assert_eq!(whole("12abc"), Some(12));
  assert_eq!(whole("abc"), None);
  assert_eq!(whole(""), None);
}

#[test]
fn the_usage_names_the_depths_regions_and_shapes() {
  let usage = usage();

  assert!(usage.starts_with("usage: bmo search --chat <chatId>"));
  assert!(usage.contains("[--depth basic|thorough|exhaustive]"));
  assert!(usage.contains("[--region domestic|global]"));
  assert!(usage.contains("[--shape brief|table|full|learning]"));
}

// The runner is handed the question in its file, and not in its words or its environment.
#[test]
fn of_a_search_hands_the_runner_the_question_in_its_file_and_not_in_its_words() {
  use std::os::unix::fs::PermissionsExt;

  let settings = Settings::read(Path::new(ROOT), Some(HashMap::new())).expect("the settings read");
  let place = tempfile::tempdir().unwrap();
  let path = place.path().join("bmo-search-1.txt");
  let question = "how many times did the base rate rise this year";
  let request = Request {
    query: question.to_owned(),
    asker: Some(Asker {
      id: 7,
      name: "tester".to_owned(),
    }),
    asked_at: Some(1_700_000_000),
    options: Options::default(),
  };
  let profile = Profile::parse(
    r#"{"key":"search:basic","capMs":null,"synthesisReserveMs":null,"childRole":"scout","maxChildren":0,"waitLimitMs":60000,"strategy":"direct-then-browser","hostIntervalMs":1000,"engineIntervalMs":3000,"setAt":1}"#,
    &settings.research,
    "search:basic",
  )
  .unwrap();

  write_request(&path, &request).unwrap();

  let args = runner_args(&RunnerCall {
    chat_id: ROOM,
    query_path: &path,
    depth: "basic",
    run_id: 3,
    unit: "bmo-search-1",
    started_by: Some("chat:42"),
    search_id: 4,
    demand: Demand::Optional,
    profile: &profile,
    via: Some("chat-road"),
    kept_on: None,
    asked_by: None,
  });
  let command = runner_command(
    args.clone(),
    [("BMO_ARTIFACT_FILES__ROOT", "/data"), ("SECRET", "no"), ("PATH", "/bin")]
      .map(|(name, value)| (name.to_owned(), value.to_owned())),
  )
  .unwrap();

  assert!(!args.iter().any(|arg| arg.contains("base rate")), "{args:?}");
  assert_eq!(args[..4], ["search-runner", ROOM_TEXT, &path.to_string_lossy(), "basic"]);
  assert_eq!(args[args.len() - 2..], ["--via", "chat-road"]);
  assert_eq!(
    command.environment.keys().map(String::as_str).collect::<Vec<_>>(),
    ["BMO_ARTIFACT_FILES__ROOT", "PATH"]
  );
  assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);

  let written: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

  assert_eq!(written["query"], question);
  assert_eq!(written["asker"]["name"], "tester");
  assert_eq!(PathBuf::from(&command.args[2]), path);
}
