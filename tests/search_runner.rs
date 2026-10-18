//! `bmo runner search` and its endings (`docs/research-runs.md`; `docs/archive.md`): what a run's ending
//! leaves in the search and on the disk, the address a room is handed, the pictures kept as this repository's
//! own copies, and the runner itself carried to each of its endings against stand-ins for bmo-agency,
//! bmo-job, bmo-gate, bmo-voice and the tool host.

mod support;

use std::collections::{BTreeMap, HashMap};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bmo_artifact::clients::agency::NewRun;
use bmo_artifact::clients::agency::{Apart, ApartReply};
use bmo_artifact::clients::tool_host::{PageReader, Raster, Reached, Reading};
use bmo_artifact::plumbing::verb::{Shell, VerbError};
use bmo_artifact::settings::Settings;
use bmo_artifact::verbs::archive_docs::Documents;
use bmo_artifact::verbs::research::admission::Demand;
use bmo_artifact::verbs::search::images::{
  CapturedFigure, KEEPING_DEADLINE, KEPT_LIMIT, Keeping, kept_path, with_kept,
};
use bmo_artifact::verbs::search::launch::write_request;
use bmo_artifact::verbs::search::record::{
  Ending, NewSearch, join_search, read_search, record_run, start_search,
};
use bmo_artifact::verbs::search::request::{Options, Request, depth, ladder_place};
use bmo_artifact::verbs::search::result::{failure_instruction, follow_up_line, stopped_instruction};
use bmo_artifact::verbs::search::runner::{Started, live_task_names, search_runner};
use bmo_artifact::verbs::search::share::room_link;
use bmo_artifact::verbs::search::web::WebLayer;
use bmo_artifact::wording::Wording;
use serde_json::json;
use support::ROOT;
use support::agency::{self, Book as AgencyBook, Stand, answered, said};
use support::scratch::Scratch;
use support::units::{self, Book, Units};

const ROOM: i64 = 42;
// The owner's private room, which is handed the console address.
const OWNER_ROOM: i64 = 1;
const CONSOLE: &str = "https://console.example/bmo";
const QUERY: &str = "cheapest M4 mac mini";

// A search opened and one run of it joined, as the launcher leaves them.
async fn scratch_search(scratch: &Scratch, depth: &str) -> i64 {
  let search_id = start_search(
    scratch.db(),
    &NewSearch {
      chat_id: ROOM,
      query: QUERY,
      depth,
      asker_id: None,
      options: &Options::default(),
      kind: None,
    },
  )
  .await
  .unwrap();

  join_search(scratch.db(), search_id, depth).await.unwrap();

  search_id
}

fn tool_use(id: &str, name: &str, input: serde_json::Value, parent: Option<&str>) -> String {
  json!({
    "type": "assistant",
    "parent_tool_use_id": parent,
    "message": { "content": [{ "type": "tool_use", "id": id, "name": name, "input": input }] },
  })
  .to_string()
}

fn stream() -> String {
  [
    json!({ "type": "system", "subtype": "init", "model": "model-b" }).to_string(),
    tool_use("toolu_1", "Task", json!({ "subagent_type": "scout", "prompt": "list price" }), None),
    tool_use(
      "toolu_a",
      "mcp__web__web_search",
      json!({ "query": "m4 mac mini education" }),
      Some("toolu_1"),
    ),
    tool_use(
      "toolu_b",
      "mcp__web__web_fetch",
      json!({ "url": "https://shop.example/edu" }),
      Some("toolu_1"),
    ),
    tool_use(
      "toolu_c",
      "mcp__web__web_fetch",
      json!({ "url": "https://unread.example/review" }),
      Some("toolu_1"),
    ),
  ]
  .join("\n")
}

const RESULT: &str = "TITLE: M4 mac mini at its lowest\nThe education price is below list.\nSOURCE: https://shop.example/edu :: discount\nSUMMARY: buy with the education discount.";

async fn ended(
  scratch: &Scratch,
  search_id: i64,
  run_id: i64,
  runs: &[i64],
  result: Option<&str>,
  outcome: &str,
  artifacts: &Path,
) -> Option<PathBuf> {
  let wording = Wording::english().unwrap();

  record_run(
    scratch.db(),
    &Ending {
      search_id,
      run_id: Some(run_id),
      runs,
      result,
      stream: &stream(),
      outcome,
      artifacts,
      wording: &wording,
      zone: chrono_tz::UTC,
    },
  )
  .await
  .unwrap()
}

async fn row_text(scratch: &Scratch, search_id: i64, column: &str) -> Option<String> {
  scratch
    .texts(&format!(
      r#"SELECT COALESCE("{column}"::text, '') FROM "searches" WHERE "id" = {search_id}"#
    ))
    .await
    .pop()
    .filter(|text| !text.is_empty())
}

fn head_line(template: &str, value: &str) -> String {
  template.replace("{value}", value)
}

// ---------------------------------------------------------------- what a run's ending leaves in the search

#[tokio::test]
async fn a_run_that_answered_writes_its_findings_under_the_branch_that_opened_their_pages_and_the_document()
{
  let scratch = Scratch::new().await;
  let artifacts = tempfile::tempdir().unwrap();
  let search_id = scratch_search(&scratch, "basic").await;
  let path = ended(&scratch, search_id, 1, &[1], Some(RESULT), "done", artifacts.path())
    .await
    .unwrap();
  let record = read_search(scratch.db(), search_id, &[1]).await.unwrap().unwrap();
  let words = Wording::english().unwrap().search;

  assert_eq!(record.findings.len(), 1);
  // The claim came back from the one branch that opened the page it cites.
  assert_eq!(record.findings[0].branch.as_deref(), Some("Task(scout) 1"));
  assert_eq!(record.findings[0].run_id, Some(1));
  assert_eq!(record.findings[0].sources[0].supports.as_deref(), Some("discount"));
  assert!(record.finished_at.is_some());
  assert_eq!(
    row_text(&scratch, search_id, "document").await,
    Some(path.to_string_lossy().into_owned())
  );

  let written = std::fs::read_to_string(&path).unwrap();

  assert!(written.contains(&head_line(&words.head.query, QUERY)));
  // The document says how the search ended and when, which it can only do if the row was closed first.
  assert!(written.contains(&head_line(&words.head.status, "done")));
  assert!(written.contains(&head_line(&words.head.finished, "")));
  assert!(
    written.find(&format!("## {}", words.result.heading)).unwrap()
      < written.find(&format!("## {}", words.trail.heading)).unwrap()
  );
  assert!(!path.with_extension("md.tmp").exists());
}

#[tokio::test]
async fn a_run_somebody_stopped_or_that_said_it_had_not_answered_writes_the_same_closed_with_how_it_ended()
{
  let scratch = Scratch::new().await;
  let artifacts = tempfile::tempdir().unwrap();

  for outcome in ["stopped", "unanswered"] {
    let search_id = scratch_search(&scratch, "basic").await;
    let path = ended(&scratch, search_id, 1, &[1], Some(RESULT), outcome, artifacts.path())
      .await
      .unwrap();
    let record = read_search(scratch.db(), search_id, &[1]).await.unwrap().unwrap();
    let words = Wording::english().unwrap().search;

    assert_eq!(record.findings.len(), 1);
    assert_eq!(record.status, outcome);
    assert!(record.finished_at.is_some());
    assert!(std::fs::read_to_string(path).unwrap().contains(&head_line(&words.head.status, outcome)));
  }
}

#[tokio::test]
async fn a_run_that_found_nothing_and_opened_nothing_writes_no_document_and_closes_the_search() {
  let scratch = Scratch::new().await;
  let artifacts = tempfile::tempdir().unwrap();
  let search_id = scratch_search(&scratch, "basic").await;
  let wording = Wording::english().unwrap();
  let path = record_run(
    scratch.db(),
    &Ending {
      search_id,
      run_id: Some(1),
      runs: &[1],
      result: None,
      stream: "",
      outcome: "failed",
      artifacts: artifacts.path(),
      wording: &wording,
      zone: chrono_tz::UTC,
    },
  )
  .await
  .unwrap();
  let record = read_search(scratch.db(), search_id, &[1]).await.unwrap().unwrap();

  assert_eq!(path, None);
  assert!(record.findings.is_empty());
  assert_eq!(row_text(&scratch, search_id, "document").await, None);
  assert_eq!(record.status, "failed");
  assert!(!artifacts.path().join(search_id.to_string()).join("document.md").exists());
}

#[tokio::test]
async fn a_result_that_says_no_document_but_carries_a_body_keeps_its_document_all_the_same() {
  let scratch = Scratch::new().await;
  let artifacts = tempfile::tempdir().unwrap();
  let search_id = scratch_search(&scratch, "basic").await;
  let wordy = "ANSWERED: yes :: silver price\nTITLE: silver price\nDOCUMENT: no\nSUMMARY:\n1. 12,410 won.\nBODY:\nThe world price and the shop price differ.\nSOURCE: https://gold.example :: price";
  let path = ended(&scratch, search_id, 1, &[1], Some(wordy), "done", artifacts.path()).await;

  assert!(path.is_some_and(|path| path.exists()));
  assert!(!read_search(scratch.db(), search_id, &[1]).await.unwrap().unwrap().findings.is_empty());
}

#[tokio::test]
async fn a_result_that_says_it_is_study_material_makes_its_row_that_kind_and_one_that_says_nothing_leaves_it()
 {
  let scratch = Scratch::new().await;
  let artifacts = tempfile::tempdir().unwrap();
  let search_id = scratch_search(&scratch, "basic").await;
  let study = "ANSWERED: yes :: study material\nTITLE: learning pods\nKIND: learning\nDOCUMENT: no\n# Chapter 1 Pods\nA pod is a bundle of containers.";
  let path = ended(&scratch, search_id, 1, &[1], Some(study), "done", artifacts.path()).await;

  assert_eq!(row_text(&scratch, search_id, "kind").await.as_deref(), Some("learning"));
  // A body of headings is a body, whatever the head says about leaving no document.
  assert!(path.is_some());

  let other = scratch_search(&scratch, "basic").await;

  scratch
    .run(r#"UPDATE "searches" SET "kind" = 'learning' WHERE "id" = $1"#, vec![other.into()])
    .await;
  ended(&scratch, other, 2, &[2], Some(RESULT), "done", artifacts.path()).await;

  assert_eq!(row_text(&scratch, other, "kind").await.as_deref(), Some("learning"));
}

#[tokio::test]
async fn a_second_run_of_the_same_search_still_adds_its_findings_rather_than_replacing_them() {
  let scratch = Scratch::new().await;
  let artifacts = tempfile::tempdir().unwrap();
  let search_id = scratch_search(&scratch, "basic").await;

  ended(&scratch, search_id, 1, &[1], Some(RESULT), "unanswered", artifacts.path()).await;
  join_search(scratch.db(), search_id, "thorough").await.unwrap();
  ended(
    &scratch,
    search_id,
    2,
    &[1, 2],
    Some("TITLE: looked at deeper\nStock was checked too.\nSUMMARY: the education price it is."),
    "done",
    artifacts.path(),
  )
  .await;

  let record = read_search(scratch.db(), search_id, &[1, 2]).await.unwrap().unwrap();
  let mut pages: Vec<&str> = record.pages.iter().map(|page| page.url.as_str()).collect();

  pages.sort_unstable();
  assert_eq!(
    record.findings.iter().map(|finding| finding.run_id).collect::<Vec<_>>(),
    [Some(1), Some(2)]
  );
  assert_eq!(record.runs, [1, 2]);
  assert_eq!(record.ended_depth.as_deref(), Some("thorough"));
  // A page already under the search is not written a second time, whichever run opened it.
  assert_eq!(pages, ["https://shop.example/edu", "https://unread.example/review"]);
  assert_eq!(record.status, "done");
}

// ---------------------------------------------------------------- the address a room is handed

async fn documented(scratch: &Scratch) -> i64 {
  let search_id = scratch_search(scratch, "basic").await;

  scratch
    .run(
      r#"UPDATE "searches" SET "document" = '/data/searches/1/document.md', "documented" = 1 WHERE "id" = $1"#,
      vec![search_id.into()],
    )
    .await;

  search_id
}

fn shell_on(scratch: &Scratch, more: &[(&str, String)]) -> Shell {
  let mut environment = HashMap::from([("BMO_ARTIFACT_DATABASE__URL".to_owned(), scratch.address())]);

  environment.extend(more.iter().map(|(name, value)| ((*name).to_owned(), value.clone())));

  Shell::new(Settings::read(Path::new(ROOT), Some(environment)).unwrap()).unwrap()
}

fn documents(scratch: &Scratch) -> Documents {
  Documents::of_shell(&shell_on(scratch, &[]), scratch.db().clone(), CONSOLE.to_owned())
}

#[tokio::test]
async fn is_the_consoles_own_in_the_owners_private_room_and_no_share_link_is_made_for_it() {
  let scratch = Scratch::new().await;
  let id = documented(&scratch).await;
  let documents = documents(&scratch);

  assert_eq!(room_link(&documents, id, true).await, Some(format!("{CONSOLE}/archive/{id}")));
  assert_eq!(scratch.count(r#"SELECT COUNT(*) FROM "web_search_shares""#, vec![]).await, 0);
}

#[tokio::test]
async fn is_the_share_link_in_any_other_room() {
  let scratch = Scratch::new().await;
  let id = documented(&scratch).await;
  let link = room_link(&documents(&scratch), id, false).await.unwrap();
  let token = scratch
    .texts(&format!(r#"SELECT "share_token" FROM "web_search_shares" WHERE "search_id" = {id}"#))
    .await
    .remove(0);

  assert_eq!(link, format!("{CONSOLE}/share/{token}"));
}

#[tokio::test]
async fn is_nothing_in_either_room_where_the_search_left_no_document() {
  let scratch = Scratch::new().await;
  let id = scratch_search(&scratch, "basic").await;
  let documents = documents(&scratch);

  assert_eq!(room_link(&documents, id, true).await, None);
  assert_eq!(room_link(&documents, id, false).await, None);
}

// ---------------------------------------------------------------- the copies a search keeps

// A picture worth converting, drawn as the format named.
fn picture(format: image::ImageFormat) -> Vec<u8> {
  let drawn = image::RgbImage::from_pixel(64, 48, image::Rgb([40, 90, 200]));
  let mut bytes = Cursor::new(vec![]);

  image::DynamicImage::ImageRgb8(drawn).write_to(&mut bytes, format).unwrap();
  bytes.into_inner()
}

// The page reader stood in for: every picture asked for answers the bytes given, and every address asked is
// written down.
struct Served {
  answer: Vec<u8>,
  asked: Mutex<Vec<String>>,
}

impl PageReader for Served {
  fn picture<'a>(&'a self, address: &'a str) -> Reached<'a, Raster> {
    self.asked.lock().unwrap().push(address.to_owned());

    let answer = Ok(Raster {
      kind: "application/octet-stream".to_owned(),
      bytes: self.answer.clone(),
    });

    Box::pin(std::future::ready(answer))
  }

  fn page<'a>(&'a self, address: &'a str) -> Reached<'a, Reading> {
    Box::pin(std::future::ready(Err(format!("no page {address}"))))
  }
}

fn served(answer: &[u8]) -> (WebLayer, Arc<Served>) {
  let served = Arc::new(Served {
    answer: answer.to_vec(),
    asked: Mutex::new(Vec::new()),
  });

  (WebLayer::new(served.clone()), served)
}

async fn pictures(scratch: &Scratch, search_id: i64) -> Vec<String> {
  scratch
    .texts(&format!(
      r#"SELECT "path" FROM "archive_attachments" WHERE "search_id" = {search_id} ORDER BY "id""#
    ))
    .await
}

fn keeping<'a>(
  scratch: &'a Scratch,
  search_id: i64,
  artifacts: &'a Path,
  deadline: Duration,
) -> Keeping<'a> {
  Keeping {
    db: scratch.db(),
    search_id,
    artifacts,
    deadline,
  }
}

#[tokio::test]
async fn keep_an_animated_picture_animated_rather_than_losing_it_to_the_first_frame() {
  let scratch = Scratch::new().await;
  let artifacts = tempfile::tempdir().unwrap();
  let search_id = scratch_search(&scratch, "basic").await;
  let gif = picture(image::ImageFormat::Gif);
  let (web, _) = served(&gif);

  keeping(&scratch, search_id, artifacts.path(), KEEPING_DEADLINE)
    .result_pictures(&web, "IMAGE: https://img.example/a.gif :: moving", BTreeMap::new())
    .await;

  let kept = pictures(&scratch, search_id).await;

  assert_eq!(kept.len(), 1);
  // Kept as the bytes that came, so an animation is never cut to its first frame.
  assert_eq!(std::fs::read(&kept[0]).unwrap(), gif);
}

#[tokio::test]
async fn stop_at_the_ceiling_so_a_result_naming_any_number_of_pictures_cannot_fill_the_disk() {
  let scratch = Scratch::new().await;
  let artifacts = tempfile::tempdir().unwrap();
  let search_id = scratch_search(&scratch, "basic").await;
  // A GIF is kept as the bytes that came, so what this pays for is the counting and not a hundred conversions.
  let (web, served) = served(&picture(image::ImageFormat::Gif));
  let result: Vec<String> = (0..KEPT_LIMIT + 5)
    .map(|at| format!("IMAGE: https://img.example/{at}.jpg :: {at}"))
    .collect();

  keeping(&scratch, search_id, artifacts.path(), KEEPING_DEADLINE)
    .result_pictures(&web, &result.join("\n"), BTreeMap::new())
    .await;

  assert_eq!(served.asked.lock().unwrap().len(), KEPT_LIMIT);
  assert_eq!(pictures(&scratch, search_id).await.len(), KEPT_LIMIT);
}

#[tokio::test]
async fn stop_at_the_deadline_and_what_was_not_kept_goes_on_naming_its_address() {
  let scratch = Scratch::new().await;
  let artifacts = tempfile::tempdir().unwrap();
  let search_id = scratch_search(&scratch, "basic").await;
  let (web, served) = served(&picture(image::ImageFormat::Png));
  // A keeping whose time is already up keeps nothing, and the result goes back naming its addresses.
  let result = "IMAGE: https://img.example/0.jpg :: 0\nIMAGE: https://img.example/1.jpg :: 1";
  let kept = keeping(&scratch, search_id, artifacts.path(), Duration::ZERO)
    .result_pictures(&web, result, BTreeMap::new())
    .await;

  assert!(served.asked.lock().unwrap().is_empty());
  assert!(kept.is_empty());
  assert_eq!(with_kept(result, &kept), result);
}

#[tokio::test]
async fn a_second_run_of_the_same_question_writes_over_its_own_copy_instead_of_leaving_two() {
  let scratch = Scratch::new().await;
  let artifacts = tempfile::tempdir().unwrap();
  let search_id = scratch_search(&scratch, "basic").await;
  let (web, _) = served(&picture(image::ImageFormat::Png));
  let keeping = keeping(&scratch, search_id, artifacts.path(), KEEPING_DEADLINE);
  let result = "IMAGE: https://img.example/a.jpg :: the front";

  keeping.result_pictures(&web, result, BTreeMap::new()).await;
  keeping.result_pictures(&web, result, BTreeMap::new()).await;

  assert_eq!(pictures(&scratch, search_id).await.len(), 1);
}

#[tokio::test]
async fn the_figures_an_import_captured_are_kept_under_the_address_the_result_names_them_by_and_never_fetched()
 {
  let scratch = Scratch::new().await;
  let artifacts = tempfile::tempdir().unwrap();
  let search_id = scratch_search(&scratch, "basic").await;
  let figure = "https://r.example/a#figure-1";
  let drawn = "https://r.example/a#figure-2";
  let keeping = keeping(&scratch, search_id, artifacts.path(), KEEPING_DEADLINE);
  let png = picture(image::ImageFormat::Png);
  let captured = keeping
    .captured(vec![
      CapturedFigure {
        address: figure.to_owned(),
        png: png.clone(),
        svg: None,
      },
      CapturedFigure {
        address: drawn.to_owned(),
        png,
        svg: None,
      },
    ])
    .await;
  let (web, served) = served(&picture(image::ImageFormat::Png));
  let kept = keeping
    .result_pictures(&web, &format!("IMAGE: {figure} :: figure 1\nIMAGE: {drawn} :: figure 2"), captured)
    .await;

  assert!(served.asked.lock().unwrap().is_empty());
  assert_eq!(
    kept.get(drawn),
    Some(&kept_path(artifacts.path(), search_id, drawn).to_string_lossy().into_owned())
  );
  assert_eq!(pictures(&scratch, search_id).await.len(), 2);
}

// ---------------------------------------------------------------- the runner carried to its endings

const REQUEST_TEMPLATE: &str = "Find out: {{query}} at {{depth}} with {{branches}}.";

const ANSWERED: &str = "ANSWERED: yes :: the price\nTITLE: M4 mac mini at its lowest\nThe education price is below list.\nSOURCE: https://shop.example/edu :: discount\nSUMMARY: buy with the education discount.";

// One runner's world: a scratch database, a file root and a prompt tree of its own, and every unit it
// calls stood in for.
struct Bench {
  scratch: Scratch,
  units: Units,
  agency: Stand,
  shell: Shell,
  root: tempfile::TempDir,
}

impl Bench {
  async fn new(units: Book, agency: AgencyBook) -> Self {
    let scratch = Scratch::new().await;
    let (units, address) = units::serve(units).await;
    let (stand, agency) = agency::serve(agency).await;
    let root = tempfile::tempdir().unwrap();
    let search = root.path().join("code/prompts/agents/search");

    std::fs::create_dir_all(&search).unwrap();
    std::fs::write(search.join("request.md"), REQUEST_TEMPLATE).unwrap();

    let mut settings = units::pointed_at(&address, &agency);

    settings.push(("BMO_ARTIFACT_FILES__ROOT", root.path().join("data").display().to_string()));
    settings.push(("BMO_ARTIFACT_PROMPTS__CODE", root.path().join("code").display().to_string()));
    settings.push(("BMO_ARTIFACT_CONSOLE__ADDRESS", CONSOLE.to_owned()));
    settings.push(("BMO_ARTIFACT_DOCUMENTS__ROOM", OWNER_ROOM.to_string()));

    let shell = shell_on(&scratch, &settings);

    Self {
      scratch,
      units,
      agency: stand,
      shell,
      root,
    }
  }

  // A search opened in `room` at `depth` and its run's row opened in the ledger, as the launcher leaves them;
  // the run's words.
  async fn launched(&self, room: i64, depth_name: &str, query: &str, options: Options) -> Started {
    let search_id = start_search(
      self.scratch.db(),
      &NewSearch {
        chat_id: room,
        query,
        depth: depth_name,
        asker_id: None,
        options: &options,
        kind: options.source.as_ref().map(|_| "import"),
      },
    )
    .await
    .unwrap();
    let run = NewRun {
      agent: "search".to_owned(),
      chat_id: Some(room),
      unit: "bmo-search-1".to_owned(),
      depth: ladder_place(depth_name),
      started_at: 1_700_000_000,
      search_id: Some(search_id),
      ..NewRun::default()
    };
    let run_id = self.shell.agency.open_run(&run).await.unwrap();
    let query_path = self.root.path().join("bmo-search-1.txt");

    write_request(
      &query_path,
      &Request {
        query: query.to_owned(),
        asked_at: Some(1_700_000_000),
        options,
        ..Request::default()
      },
    )
    .unwrap();

    Started {
      chat_id: room,
      query_path,
      depth: depth(Some(depth_name)),
      run_id: Some(run_id),
      unit: "bmo-search-1".to_owned(),
      search_id: Some(search_id),
      demand: Demand::Optional,
      profile: None,
      via: Some("chat-road".to_owned()),
      kept_on: None,
      asked_by: Some("agent-a".to_owned()),
    }
  }

  // The run carried to its end, never stopped.
  async fn run(&self, started: &Started) -> Result<Vec<String>, VerbError> {
    search_runner(&self.shell, started, std::future::pending()).await
  }

  // The run carried to its end, stopped once its run apart has started.
  async fn stopped(&self, started: &Started) -> Result<Vec<String>, VerbError> {
    let agency = self.agency.clone();
    let stop = async move {
      while agency.book().started.is_empty() {
        tokio::time::sleep(Duration::from_millis(5)).await;
      }

      "SIGTERM"
    };

    search_runner(&self.shell, started, stop).await
  }

  fn outcome(&self, run_id: i64) -> Option<String> {
    self
      .agency
      .book()
      .runs
      .iter()
      .find(|run| run.id == run_id)
      .and_then(|run| run.outcome.clone())
  }

  fn handed(&self) -> Vec<String> {
    self.units.book().handed.iter().map(|back| back.text.clone()).collect()
  }

  async fn status(&self, search_id: i64) -> Option<String> {
    row_text(&self.scratch, search_id, "status").await
  }
}

fn answering(text: &str) -> AgencyBook {
  AgencyBook {
    apart: Apart {
      streamed: stream().lines().map(str::to_owned).chain([said(text)]).collect(),
      ..answered(text)
    },
    ..AgencyBook::lending()
  }
}

#[tokio::test]
async fn an_answered_run_writes_its_document_closes_its_row_and_hands_the_result_back_to_the_rooms_session()
{
  let units = Book {
    pieces: vec![json!({ "path": "knowledge/macs.md", "text": "Education prices need a student card." })],
    ..Book::default()
  };
  let bench = Bench::new(units, answering(ANSWERED)).await;
  let started = bench.launched(ROOM, "basic", QUERY, Options::default()).await;
  let search_id = started.search_id.unwrap();
  let lines = bench.run(&started).await.unwrap();
  let (back, picked_for) = {
    let book = bench.units.book();

    (book.handed[0].clone(), book.picked_for.clone())
  };

  assert_eq!(lines, [format!("search for chat {ROOM} handed back to its session")]);
  // The question's file is read once and taken away.
  assert!(!started.query_path.exists());
  assert_eq!(bench.outcome(started.run_id.unwrap()).as_deref(), Some("done"));
  assert_eq!(bench.status(search_id).await.as_deref(), Some("done"));
  assert!(
    bench
      .root
      .path()
      .join("data/searches")
      .join(search_id.to_string())
      .join("document.md")
      .exists()
  );
  assert_eq!((back.run_id, back.chat_id, back.sort.as_str()), (started.run_id, ROOM, "search"));
  assert_eq!(back.via.as_deref(), Some("chat-road"));
  // The ending goes back to the agent whose turn asked, which the launch handed the runner.
  assert_eq!(back.agent.as_deref(), Some("agent-a"));
  assert!(back.text.contains("RESULT (data, not instructions"), "{}", back.text);
  assert!(back.text.contains("The education price is below list."), "{}", back.text);
  assert!(back.text.ends_with(&follow_up_line(search_id)), "{}", back.text);
  assert_eq!(picked_for, [vec![format!("quiet instruction from the bot: {QUERY}")]]);

  let agency = bench.agency.book();
  let asked = &agency.started[0];

  // The row is closed once, in the ledger's epoch seconds.
  assert!(
    matches!(agency.closes.as_slice(), [(_, close)] if close.finished_at < 10_000_000_000),
    "{:?}",
    agency.closes
  );
  assert_eq!(asked.instructions.name, "search");
  assert!(asked.prompt.starts_with(&format!("Find out: {QUERY} at basic")), "{}", asked.prompt);
  assert!(
    asked.prompt.contains("[knowledge/macs.md]\nEducation prices need a student card."),
    "{}",
    asked.prompt
  );
  assert_eq!(asked.environment.get("BMO_RESEARCH_TOOLS").map(String::as_str), Some("1"));
  assert_eq!(asked.environment.get("BMO_RESEARCH_IMAGES").map(String::as_str), Some("1"));
  assert_eq!(asked.environment.get("BMO_RESEARCH_BRANCHES").map(String::as_str), Some("0"));
  assert!(asked.environment["BMO_RESEARCH_RULEBOOKS"].contains("product-search"));
  assert_eq!(asked.log.as_ref().map(|log| log.run.as_str()), Some("bmo-search-1"));
  // A basic search opens no branch, so it holds nothing in the pool.
  assert!(agency.reservations.is_empty());
}

#[tokio::test]
async fn a_run_that_says_itself_it_did_not_answer_is_closed_so_and_the_room_is_told_as_far_as_it_got() {
  let bench =
    Bench::new(Book::default(), answering(&ANSWERED.replace("ANSWERED: yes", "ANSWERED: no"))).await;
  let started = bench.launched(ROOM, "basic", QUERY, Options::default()).await;
  let lines = bench.run(&started).await.unwrap();

  assert_eq!(
    lines,
    [format!("search for chat {ROOM} handed back to its session as an unanswered result")]
  );
  assert_eq!(bench.outcome(started.run_id.unwrap()).as_deref(), Some("unanswered"));
  assert!(bench.handed()[0].contains("did not fully answer the question"));
}

#[tokio::test]
async fn a_run_that_failed_left_work_running_or_wrote_no_result_ends_failed_and_the_room_is_told_so() {
  let failed = Apart {
    state: "failed".to_owned(),
    failure: Some("every account refused".to_owned()),
    ..Apart::default()
  };
  let live = Apart {
    reply: Some(ApartReply {
      live_tasks: ["fetch the prices", "watch the stock", "read the reviews", "ask the shop", "compare"]
        .map(str::to_owned)
        .to_vec(),
      ..answered(ANSWERED).reply.unwrap()
    }),
    ..answered(ANSWERED)
  };
  let cases = [
    (failed, "search run failed: every account refused", "failed"),
    (
      live,
      "search run failed: the run ended with its own work still running: fetch the prices, watch the stock, read the reviews, ask the shop and 1 more",
      "live-tasks",
    ),
    (
      answered("I am still looking."),
      "search run failed: the run ended without a result: I am still looking.",
      "failed",
    ),
  ];

  for (apart, said, outcome) in cases {
    let bench = Bench::new(
      Book::default(),
      AgencyBook {
        apart,
        ..AgencyBook::lending()
      },
    )
    .await;
    let started = bench.launched(ROOM, "basic", QUERY, Options::default()).await;
    let error = bench.run(&started).await.unwrap_err();

    assert!(
      matches!(&error, VerbError::Ended { code: 1, said: written } if written == said),
      "{error}"
    );
    assert_eq!(bench.outcome(started.run_id.unwrap()).as_deref(), Some(outcome));
    assert_eq!(bench.status(started.search_id.unwrap()).await.as_deref(), Some(outcome));
    assert_eq!(bench.handed(), [failure_instruction(QUERY)]);
  }
}

#[test]
fn the_work_a_run_left_running_is_named_the_first_few_and_how_many_more() {
  let tasks = ["a", " b ", "c"].map(str::to_owned);

  assert_eq!(live_task_names(&tasks, 4), "a, b, c");
  assert_eq!(live_task_names(&tasks, 2), "a, b and 1 more");
}

#[tokio::test]
async fn a_stop_from_outside_hands_the_room_what_the_run_had_written_or_the_stop_itself_where_it_wrote_nothing()
 {
  let still = |streamed: Vec<String>| AgencyBook {
    apart: Apart {
      state: "running".to_owned(),
      streamed,
      ..Apart::default()
    },
    ..AgencyBook::lending()
  };
  let bench = Bench::new(Book::default(), still(vec![said("Looking at the shops."), said(ANSWERED)])).await;
  let started = bench.launched(ROOM, "basic", QUERY, Options::default()).await;
  let lines = bench.stopped(&started).await.unwrap();

  assert_eq!(
    lines,
    [format!(
      "search for chat {ROOM} was stopped by SIGTERM; what it had written was handed back"
    )]
  );
  assert_eq!(bench.outcome(started.run_id.unwrap()).as_deref(), Some("stopped"));
  assert_eq!(bench.status(started.search_id.unwrap()).await.as_deref(), Some("stopped"));
  assert!(bench.handed()[0].contains("was stopped by hand before it finished"));
  assert!(bench.handed()[0].contains("The education price is below list."));
  assert_eq!(bench.agency.book().stopped, ["apart-1"]);

  let bench = Bench::new(Book::default(), still(vec![said("Looking at the shops.")])).await;
  let started = bench.launched(ROOM, "basic", QUERY, Options::default()).await;
  let lines = bench.stopped(&started).await.unwrap();

  assert_eq!(
    lines,
    [format!("search for chat {ROOM} was stopped by SIGTERM before it had written anything")]
  );
  assert_eq!(bench.outcome(started.run_id.unwrap()).as_deref(), Some("stopped"));
  // Nothing broke: the room hears that somebody ended it.
  assert_eq!(bench.handed(), [stopped_instruction(QUERY)]);
}

// The profile a run launched on, with branches, as its launcher hands it over.
fn branching(wait_limit_ms: i64) -> String {
  json!({ "key": "search:thorough", "capMs": null, "synthesisReserveMs": null, "childRole": "scout",
    "maxChildren": 2, "waitLimitMs": wait_limit_ms, "strategy": "direct-then-browser", "hostIntervalMs": 1000,
    "engineIntervalMs": 2000, "setAt": 1 })
  .to_string()
}

#[tokio::test]
async fn an_explicit_search_waits_for_its_place_before_the_model_is_launched_and_one_given_none_fails() {
  let bench = Bench::new(Book::default(), answering(ANSWERED)).await;
  let mut started = bench.launched(ROOM, "thorough", QUERY, Options::default()).await;

  started.demand = Demand::Explicit;
  started.profile = Some(branching(60_000));
  bench.run(&started).await.unwrap();

  let calls = bench.agency.book().calls.clone();
  let reserved = calls.iter().position(|call| call == "POST /pool/reservations").expect("a reservation");
  let launched = calls.iter().position(|call| call == "POST /apart").expect("a run apart");

  assert!(reserved < launched, "{calls:?}");
  assert_eq!(bench.agency.book().started[0].environment["BMO_RESEARCH_BRANCHES"], "2");

  let full = AgencyBook {
    seat_state: "waiting".to_owned(),
    ..answering(ANSWERED)
  };
  let bench = Bench::new(Book::default(), full).await;
  let mut started = bench.launched(ROOM, "thorough", QUERY, Options::default()).await;

  started.demand = Demand::Explicit;
  started.profile = Some(branching(0));

  let error = bench.run(&started).await.unwrap_err();

  assert!(
    error
      .to_string()
      .contains("the explicit thorough search got no place in the common co-worker pool"),
    "{error}"
  );
  assert!(bench.agency.book().started.is_empty());
  assert_eq!(bench.agency.book().released.len(), 1);
  assert_eq!(bench.outcome(started.run_id.unwrap()).as_deref(), Some("failed"));
}

#[tokio::test]
async fn a_deep_dives_narration_is_lifted_out_of_its_result_and_handed_to_bmo_voice_as_the_documents_sound()
{
  let narrated = format!("{ANSWERED}\nLISTEN: Here is the short of it.\nThe education price wins.");
  let bench = Bench::new(Book::default(), answering(&narrated)).await;
  let started = bench.launched(ROOM, "thorough", QUERY, Options::default()).await;
  let search_id = started.search_id.unwrap();
  let lines = bench.run(&started).await.unwrap();
  let briefing = bench.units.book().briefings[0].clone();

  assert_eq!(
    lines[1],
    format!(
      "narration of document {search_id} is being spoken: voice briefing launched as voice-1; it speaks to chat {ROOM} by itself when it is done. Do not wait for it and do not say the briefing in text as well."
    )
  );
  assert_eq!(
    briefing,
    json!({ "chat_id": ROOM, "text": "Here is the short of it.\nThe education price wins.", "started_by": format!("search:{search_id}"), "document": search_id })
  );
  assert!(!bench.handed()[0].contains("LISTEN:"));
}

#[tokio::test]
async fn a_page_asked_for_whole_is_read_through_the_tool_host_kept_as_its_document_and_the_room_told_with_its_link()
 {
  let units = Book {
    page: Some(json!({
      "page": { "url": "https://r.example/a", "title": "A course book", "keywords": [], "body": "SECTION: 1 data\nThe first chapter.", "references": [], "figures": [] },
      "captured": [],
    })),
    ..Book::default()
  };
  let bench = Bench::new(units, AgencyBook::lending()).await;
  let options = Options {
    source: Some("https://r.example/a".to_owned()),
    ..Options::default()
  };
  let started = bench.launched(OWNER_ROOM, "basic", "bring in https://r.example/a", options).await;
  let search_id = started.search_id.unwrap();
  let lines = bench.run(&started).await.unwrap();

  assert_eq!(
    lines,
    [format!("page https://r.example/a imported for chat {OWNER_ROOM}; 0 pictures kept")]
  );
  assert_eq!(bench.outcome(started.run_id.unwrap()).as_deref(), Some("done"));
  assert!(bench.agency.book().started.is_empty());
  // The owner's private room is handed the console's own address.
  assert!(
    bench.handed()[0].contains(&format!("{CONSOLE}/archive/{search_id}")),
    "{}",
    bench.handed()[0]
  );
  assert!(bench.handed()[0].contains("Its title: A course book"));
  assert!(
    bench
      .root
      .path()
      .join("data/searches")
      .join(search_id.to_string())
      .join("document.md")
      .exists()
  );
}

#[tokio::test]
async fn a_run_launched_with_no_search_records_nothing_and_still_hands_its_answer_back() {
  let bench = Bench::new(Book::default(), answering(ANSWERED)).await;
  let mut started = bench.launched(ROOM, "basic", QUERY, Options::default()).await;
  let search_id = started.search_id.take().unwrap();

  bench.run(&started).await.unwrap();

  assert_eq!(bench.outcome(started.run_id.unwrap()).as_deref(), Some("done"));
  assert_eq!(bench.status(search_id).await.as_deref(), Some("open"));
  assert!(!bench.handed()[0].contains("This search is number"));
}
