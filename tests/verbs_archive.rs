//! `archive doc` and `archive share` as a caller runs them (`docs/archive.md`, "A document the agent
//! writes"), against a scratch database and a file root of the test's own: a document is written whole —
//! its row as the writer's, its body, its files copied in and its document file — it answers with where
//! to read it, a refusal writes nothing at all, a narration is handed to bmo-voice, and a document already
//! written is handed to a room.

mod support;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use regex::Regex;
use serde_json::{Value, json};
use support::cli::{Ran, cli};
use support::scratch::Scratch;

// The owner, whose private room is the default room, and who asks where nobody is named.
const OWNER: i64 = 42;

const CONSOLE: &str = "https://console.example/bmo";

const BODY: &str =
  "# September power bill\n\n## Use\n\n| month | kWh |\n| --- | --- |\n| September | 312 |";

// A stand-in bmo-voice: every briefing it is asked for, and the answer it gives each.
#[derive(Clone)]
struct FakeVoice {
  asked: Arc<Mutex<Vec<Value>>>,
  answer: (StatusCode, Value),
}

async fn briefing(State(voice): State<FakeVoice>, Json(asked): Json<Value>) -> (StatusCode, Json<Value>) {
  voice.asked.lock().expect("the record").push(asked);

  (voice.answer.0, Json(voice.answer.1.clone()))
}

impl FakeVoice {
  // Listening on a port of its own, answering every briefing as given; its address.
  async fn start(answer: (StatusCode, Value)) -> (Self, String) {
    let voice = Self {
      asked: Arc::default(),
      answer,
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a port");
    let address = format!("http://{}", listener.local_addr().expect("an address"));
    let app = axum::Router::new().route("/briefings", post(briefing)).with_state(voice.clone());

    tokio::spawn(async move { axum::serve(listener, app).await });

    (voice, address)
  }

  fn asked(&self) -> Vec<Value> {
    self.asked.lock().expect("the record").clone()
  }
}

struct Space {
  scratch: Scratch,
  root: tempfile::TempDir,
  // The settings given over the committed ones, by their environment names.
  settings: Vec<(&'static str, String)>,
}

impl Space {
  async fn new() -> Self {
    let scratch = Scratch::new().await;
    let root = tempfile::tempdir().expect("a folder");
    let settings = vec![
      ("BMO_ARTIFACT_DATABASE__URL", scratch.address()),
      ("BMO_ARTIFACT_FILES__ROOT", root.path().join("data").display().to_string()),
      ("BMO_ARTIFACT_DOCUMENTS__ROOM", OWNER.to_string()),
      ("BMO_ARTIFACT_DOCUMENTS__ASKER", OWNER.to_string()),
      ("BMO_ARTIFACT_CONSOLE__ADDRESS", CONSOLE.to_owned()),
    ];

    Self {
      scratch,
      root,
      settings,
    }
  }

  fn set(&mut self, name: &'static str, value: &str) {
    self.settings.retain(|(set, _)| *set != name);
    self.settings.push((name, value.to_owned()));
  }

  fn unset(&mut self, name: &str) {
    self.settings.retain(|(set, _)| *set != name);
  }

  fn data(&self) -> PathBuf {
    self.root.path().join("data")
  }

  fn file(&self, name: &str, bytes: &[u8]) -> String {
    let path = self.root.path().join(name);

    std::fs::write(&path, bytes).expect("the file is written");
    path.display().to_string()
  }

  // The binary run on the words given, `stdin` on its standard input, off the runtime's own threads so a
  // stand-in it calls goes on answering.
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

  // The rows a statement answers, each read from the JSON object it builds.
  async fn rows(&self, sql: &str) -> Vec<Value> {
    self
      .scratch
      .texts(sql)
      .await
      .iter()
      .map(|row| serde_json::from_str(row).expect("a JSON row"))
      .collect()
  }

  async fn search(&self, id: i64) -> Value {
    self
      .rows(&format!(
        "SELECT json_build_object('chat_id', chat_id, 'asker_id', asker_id, 'kind', kind, 'author', author, \
         'status', status, 'title', title, 'topic', topic, 'query', query, 'document', document)::text
           FROM searches WHERE id = {id}"
      ))
      .await
      .remove(0)
  }

  async fn findings(&self, id: i64) -> Vec<String> {
    self
      .scratch
      .texts(&format!("SELECT text FROM search_findings WHERE search_id = {id} ORDER BY place"))
      .await
  }

  // Every file a document keeps, by its kind, its type, its name and where it is kept.
  async fn kept(&self, id: i64) -> Vec<(String, String, String, String)> {
    self
      .rows(&format!(
        "SELECT json_build_object('media', media, 'type', type, 'name', name, 'path', path)::text
           FROM archive_attachments WHERE search_id = {id} ORDER BY id"
      ))
      .await
      .into_iter()
      .map(|row| {
        let text = |key: &str| row[key].as_str().expect("a text").to_owned();

        (text("media"), text("type"), text("name"), text("path"))
      })
      .collect()
  }

  // The live share link of a document: its token, how long it was made to live, and who made it.
  async fn share(&self, id: i64) -> Option<(String, Option<i64>, String)> {
    self
      .rows(&format!(
        "SELECT json_build_object('token', share_token, 'life', expires_at - created_at, 'by', created_by)::text
           FROM web_search_shares WHERE search_id = {id}"
      ))
      .await
      .pop()
      .map(|row| {
        (
          row["token"].as_str().expect("a token").to_owned(),
          row["life"].as_i64(),
          row["by"].as_str().expect("a maker").to_owned(),
        )
      })
  }

  async fn documents(&self) -> i64 {
    self.scratch.count("SELECT COUNT(*) FROM searches", Vec::new()).await
  }
}

fn written(out: &str) -> i64 {
  Regex::new(r"document (\d+) written")
    .expect("a pattern")
    .captures(out)
    .unwrap_or_else(|| panic!("no document written in {out}"))[1]
    .parse()
    .expect("a number")
}

// A png of one colour, the size given.
fn png(width: u32, height: u32) -> Vec<u8> {
  let mut bytes = Vec::new();

  image::RgbImage::from_pixel(width, height, image::Rgb([200, 40, 40]))
    .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
    .expect("a png is written");
  bytes
}

#[tokio::test]
async fn is_written_whole_as_the_writers_its_files_copied_in_and_answers_with_where_to_read_it() {
  let space = Space::new().await;
  let voice = space.file("42-voice.ogg", b"OggS\0\x02\x07\x07\x07");
  let picture = space.file("receipt.png", &png(40, 30));
  let ran = space
    .bmo(
      &[
        "archive",
        "doc",
        "--chat",
        "-100",
        "--by",
        "42",
        "--title",
        "September power bill",
        "--attach",
        &voice,
        "--attach",
        &picture,
      ],
      BODY,
    )
    .await;

  assert_eq!(ran.err, "");
  assert!(ran.ok);

  let out = ran.out;
  let id = written(&out);
  let link = Regex::new(r"link: (https://console\.example/bmo/share/\S+) \(until ")
    .expect("a pattern")
    .captures(&out)
    .unwrap_or_else(|| panic!("no link in {out}"))[1]
    .to_owned();
  let search = space.search(id).await;

  // A room of other people is given the share link, made unasked, ahead of the console address.
  assert!(out.contains(&format!("give: {link}\nconsole: {CONSOLE}/archive/{id}\n")), "{out}");
  assert!(out.contains("attachments: 42-voice.ogg (audio), receipt.png (image)"), "{out}");
  assert_eq!(search["chat_id"], -100);
  assert_eq!(search["asker_id"], 42);
  assert_eq!(search["kind"], "record");
  assert_eq!(search["author"], "bmo");
  assert_eq!(search["status"], "done");
  assert_eq!(search["title"], "September power bill");
  assert_eq!(space.findings(id).await, [BODY]);

  // The document file: the facts of the document as its header, the body under it.
  let document = space.data().join("searches").join(id.to_string()).join("document.md");
  let text = std::fs::read_to_string(&document).expect("the document file");

  assert_eq!(search["document"], document.display().to_string());
  assert!(text.starts_with("# September power bill\n\n- "), "{text}");
  assert!(text.contains(": 42-voice.ogg, receipt.png"), "{text}");
  assert!(text.contains("| September | 312 |"));

  // The files are copies under the document's own folder, the picture written as AVIF, and what was handed
  // over stays.
  let kept = space.kept(id).await;
  let folder = space.data().join("searches").join(id.to_string()).join("attachments");

  assert_eq!(
    kept
      .iter()
      .map(|(media, kind, name, _)| (media.as_str(), kind.as_str(), name.as_str()))
      .collect::<Vec<_>>(),
    [("audio", "audio/ogg", "42-voice.ogg"), ("image", "image/avif", "receipt.png")]
  );
  assert!(kept.iter().all(|(_, _, _, path)| Path::new(path).starts_with(&folder)));
  assert!(Path::new(&voice).exists() && Path::new(&picture).exists());

  // The link is recorded as made by the sender the settings name.
  let (_, _, by) = space.share(id).await.expect("a share link");

  assert_eq!(by, "BMO");
}

#[tokio::test]
async fn takes_its_body_from_a_file_and_the_kind_it_is_named_and_its_link_lives_the_days_it_is_given() {
  let space = Space::new().await;
  let body = space.file("body.md", b"# Part 1 Numbers\n\n## Chapter 1 Naturals\n\n### Counting\n");
  let ran = space
    .bmo(
      &[
        "archive-doc",
        "--chat",
        "-100",
        "--by",
        "42",
        "--title",
        "Numbers",
        "--kind",
        "learning",
        "--body",
        &body,
        "--share-days",
        "2",
      ],
      "",
    )
    .await;
  let id = written(&ran.out);

  assert!(ran.ok, "{}", ran.err);
  assert_eq!(space.search(id).await["kind"], "learning");
  assert_eq!(space.share(id).await.expect("a share link").1, Some(2 * 24 * 60 * 60));
  assert!(
    space
      .bmo(&["archive-doc", "--chat", "-100", "--title", "Span", "--share-days", "0"], BODY)
      .await
      .err
      .contains("usage:")
  );
}

#[tokio::test]
async fn makes_a_link_that_never_ends_where_it_is_asked_for_one_and_refuses_a_span_beside_it() {
  let space = Space::new().await;
  let ran = space
    .bmo(&["archive", "doc", "--chat", "-100", "--title", "Endless", "--share-forever"], BODY)
    .await;
  let id = written(&ran.out);
  let (token, life, _) = space.share(id).await.expect("a share link");

  assert!(ran.ok, "{}", ran.err);
  assert_eq!(life, None);
  assert!(ran.out.contains(&format!("link: {CONSOLE}/share/{token} (never ends)")), "{}", ran.out);

  let both = space
    .bmo(
      &[
        "archive",
        "doc",
        "--chat",
        "-100",
        "--title",
        "Both",
        "--share-days",
        "2",
        "--share-forever",
      ],
      BODY,
    )
    .await;

  assert!(!both.ok);
  assert!(both.err.contains("usage:"), "{}", both.err);
}

#[tokio::test]
async fn makes_no_share_link_in_the_owners_private_room_and_names_the_console_address_as_the_one_to_give() {
  let space = Space::new().await;
  let ran = space
    .bmo(&["archive", "doc", "--chat", "42", "--by", "42", "--title", "Power bill"], BODY)
    .await;
  let id = written(&ran.out);

  assert_eq!(ran.err, "");
  assert!(ran.ok);
  assert!(
    ran
      .out
      .contains(&format!("give: {CONSOLE}/archive/{id}\nconsole: {CONSOLE}/archive/{id}\n")),
    "{}",
    ran.out
  );
  assert!(!ran.out.contains("/share/"));
  assert!(ran.out.contains("the owner's private room"), "{}", ran.out);
  assert!(space.share(id).await.is_none());
}

#[tokio::test]
async fn files_an_html_page_as_the_document_itself_its_title_and_description_read_out_of_it() {
  let space = Space::new().await;
  let page = "<!doctype html><title>Compare canvas</title><style>h1{color:teal}</style><h1>Compare canvas</h1><p>Two plans &amp; side by side</p>";
  let html = space.file("canvas.html", page.as_bytes());
  let ran = space
    .bmo(&["archive", "doc", "--chat", "-100", "--by", "42", "--html", &html], "never read")
    .await;
  let id = written(&ran.out);
  let search = space.search(id).await;
  let kept = space.kept(id).await;

  assert_eq!(ran.err, "");
  assert!(ran.ok);
  assert!(ran.out.contains("attachments: canvas.html (page)"), "{}", ran.out);
  assert_eq!(search["kind"], "page");
  assert_eq!(search["query"], "Compare canvas");
  assert_eq!(search["title"], "Compare canvas");
  assert_eq!(search["topic"], "Two plans & side by side");
  assert_eq!(space.findings(id).await, ["Two plans & side by side"]);
  assert_eq!((kept[0].0.as_str(), kept[0].1.as_str()), ("page", "text/html"));
  assert_eq!(std::fs::read_to_string(&kept[0].3).expect("the kept page"), page);
}

#[tokio::test]
async fn writes_a_document_again_under_its_number_as_a_page_with_its_link_kept() {
  let space = Space::new().await;
  let html = space.file("gallery.html", b"<!doctype html><title>Gallery</title><p>Palette</p>");
  let note = space.file("note.txt", b"first note");
  let first = space
    .bmo(
      &[
        "archive", "doc", "--chat", "-100", "--by", "42", "--title", "Gallery", "--attach", &note,
      ],
      BODY,
    )
    .await
    .out;
  let id = written(&first);
  let link = Regex::new(r"give: (\S+)").expect("a pattern").captures(&first).expect("a link")[1].to_owned();
  let old = space.kept(id).await[0].3.clone();
  let number = id.to_string();

  // A file the document already keeps is not one it can be written again from.
  assert!(
    space
      .bmo(
        &[
          "archive",
          "doc",
          "--chat",
          "-100",
          "--by",
          "42",
          "--title",
          "Gallery",
          "--replace",
          &number,
          "--attach",
          &old
        ],
        BODY
      )
      .await
      .err
      .contains("already keeps")
  );

  let again = space
    .bmo(
      &[
        "archive",
        "doc",
        "--chat",
        "-100",
        "--by",
        "42",
        "--title",
        "Palette gallery",
        "--html",
        &html,
        "--replace",
        &number,
      ],
      "",
    )
    .await;

  assert_eq!(again.err, "");
  assert!(again.out.contains(&format!("document {id} written again")), "{}", again.out);
  assert!(again.out.contains("attachments: gallery.html (page)"));
  assert!(again.out.contains(&format!("give: {link}")));

  let search = space.search(id).await;

  assert_eq!(search["kind"], "page");
  assert_eq!(search["title"], "Palette gallery");
  assert_eq!(space.findings(id).await, ["Palette"]);
  assert_eq!(space.kept(id).await.len(), 1);
  assert!(!Path::new(&old).exists());
  assert!(
    std::fs::read_to_string(search["document"].as_str().expect("a document"))
      .expect("the document file")
      .starts_with("# Palette gallery\n\n- ")
  );
}

#[tokio::test]
async fn is_written_for_the_settings_room_and_asker_where_neither_is_named_under_the_writer_they_name() {
  let mut space = Space::new().await;
  let ran = space
    .bmo(&["archive", "doc", "--title", "Design"], "```mermaid\nflowchart TD\n  a --> b\n```")
    .await;
  let search = space.search(written(&ran.out)).await;

  assert!(ran.ok, "{}", ran.err);
  assert_eq!(search["chat_id"], OWNER);
  assert_eq!(search["asker_id"], OWNER);
  assert_eq!(search["author"], "bmo");

  space.set("BMO_ARTIFACT_DOCUMENTS__WRITER", "scribe");

  let other = space.bmo(&["archive", "doc", "--title", "Other"], "body").await;

  assert_eq!(space.search(written(&other.out)).await["author"], "scribe");
}

#[tokio::test]
async fn refuses_where_the_settings_name_no_room_and_none_is_given_or_no_console_address_writing_nothing() {
  let mut space = Space::new().await;

  space.unset("BMO_ARTIFACT_DOCUMENTS__ROOM");

  let roomless = space.bmo(&["archive", "doc", "--title", "Design"], BODY).await;

  assert_eq!(roomless.code, Some(1));
  assert!(
    roomless.err.contains("archive-doc: no room is named: give --chat <chatId>"),
    "{}",
    roomless.err
  );

  space.unset("BMO_ARTIFACT_CONSOLE__ADDRESS");

  for (args, verb) in [
    (vec!["archive", "doc", "--chat", "-100", "--title", "Design"], "archive-doc"),
    (vec!["archive", "share", "1", "--chat", "-100"], "archive-share"),
  ] {
    let ran = space.bmo(&args, BODY).await;

    assert!(!ran.ok);
    assert!(ran.err.contains(&format!("{verb}: the console's address is not set")), "{}", ran.err);
  }

  assert_eq!(space.documents().await, 0);
  assert!(!space.data().join("searches").exists());
}

#[tokio::test]
async fn refuses_a_missing_title_an_empty_body_a_kind_that_is_the_searchers_a_page_given_more_than_itself_and_a_file_it_cannot_read_writing_nothing()
 {
  let space = Space::new().await;
  let page = space.file("page.html", b"<!doctype html><title>Page</title><p>words</p>");
  let untitled = space.file("untitled.html", b"<!doctype html><p>no title</p>");
  let body = space.file("body.md", b"# body");
  let missing = space.root.path().join("missing.mp3").display().to_string();
  let folder = space.root.path().display().to_string();
  let blank = space.file("blank.txt", b" \n");
  let start = ["archive", "doc", "--chat", "-100", "--by", "42"];
  let refusals: Vec<(Vec<&str>, &str, &str)> = vec![
    (vec![], BODY, "needs a title"),
    (vec!["--title", "Empty"], "  \n", "the body is empty"),
    (vec!["--title", "Search", "--kind", "search"], BODY, "--kind is one of"),
    (vec!["--title", "Missing", "--attach", &missing], BODY, "cannot be read"),
    (vec!["--title", "Folder", "--attach", &folder], BODY, "is not a file"),
    (vec!["--title", "Pageless", "--kind", "page"], BODY, "a page is filed with --html"),
    (vec!["--html", &page, "--body", &body], "", "give it no body"),
    (vec!["--html", &page, "--attach", &page], "", "give it no --attach"),
    (vec!["--html", &untitled], "", "has no <title>"),
    (vec!["--html", &body], "", "is not an HTML page"),
    (vec!["--html", &page, "--narration", &body], "", "give it no --narration"),
    (vec!["--title", "Silent", "--narration", &blank], BODY, "the narration is empty"),
    (vec!["--title", "Unread", "--narration", &missing], BODY, "narration file"),
    (vec!["--title", "Gone", "--replace", "7"], BODY, "no document 7"),
    (vec!["--title", "Word", "--replace", "seven"], BODY, "--replace names a document"),
  ];

  for (more, stdin, says) in refusals {
    let args: Vec<&str> = start.iter().copied().chain(more).collect();
    let ran = space.bmo(&args, stdin).await;

    assert!(!ran.ok, "{args:?}");
    assert!(ran.err.contains(says), "{args:?}: {}", ran.err);
  }

  assert!(
    space
      .bmo(&["archive", "doc", "--chat", "-100", "--by", "owner", "--title", "Who"], BODY)
      .await
      .err
      .contains("usage:")
  );
  assert_eq!(space.documents().await, 0);
  assert!(!space.data().join("searches").exists());
}

// A narration (`docs/api.md`, "Start a briefing of a finished document").

#[tokio::test]
async fn hands_its_narration_to_bmo_voice_as_the_documents_sound_once_the_document_is_written() {
  let mut space = Space::new().await;
  let (voice, address) =
    FakeVoice::start((StatusCode::OK, json!({ "unit": "bmo-voice-briefing-7" }))).await;

  space.set("BMO_ARTIFACT_UNITS__VOICE", &format!("{address}/"));

  let script = space.file("narration.txt", b"What the bill came to, and why.");
  let ran = space
    .bmo(
      &[
        "archive",
        "doc",
        "--chat",
        "-100",
        "--by",
        "42",
        "--title",
        "Power bill",
        "--narration",
        &script,
      ],
      BODY,
    )
    .await;
  let id = written(&ran.out);

  assert!(ran.ok, "{}", ran.err);
  assert!(ran.out.contains("narration: being spoken as bmo-voice-briefing-7"), "{}", ran.out);
  assert_eq!(
    voice.asked(),
    [json!({
      "chat_id": -100,
      "text": "What the bill came to, and why.",
      "started_by": format!("archive-doc:{id}"),
      "document": id,
    })]
  );
  // The script is no finding of the document.
  assert_eq!(space.findings(id).await, [BODY]);
}

#[tokio::test]
async fn leaves_the_document_written_and_says_why_where_the_voice_refuses_its_narration() {
  let mut space = Space::new().await;
  let (voice, address) =
    FakeVoice::start((StatusCode::CONFLICT, json!({ "error": "briefings are switched off" }))).await;
  let script = space.file("narration.txt", b"What the bill came to, and why.");
  let args = [
    "archive",
    "doc",
    "--chat",
    "-100",
    "--by",
    "42",
    "--title",
    "Power bill",
    "--narration",
    &script,
  ];

  space.set("BMO_ARTIFACT_UNITS__VOICE", &address);

  let refused = space.bmo(&args, BODY).await;
  let id = written(&refused.out);

  assert!(!refused.ok);
  assert!(refused.out.contains(&format!("give: {CONSOLE}/share/")), "{}", refused.out);
  assert!(
    refused.err.contains(&format!(
      "the narration of document {id} is not spoken: voice: briefings are switched off"
    )),
    "{}",
    refused.err
  );
  assert_eq!(space.findings(id).await, [BODY]);
  assert_eq!(voice.asked().len(), 1);

  // Where no bmo-voice is named, the document stands and the line says so.
  space.unset("BMO_ARTIFACT_UNITS__VOICE");

  let unnamed = space.bmo(&args, BODY).await;
  let other = written(&unnamed.out);

  assert!(!unnamed.ok);
  assert!(
    unnamed.err.contains(&format!(
      "the narration of document {other} is not spoken: the bmo-voice address is not set"
    )),
    "{}",
    unnamed.err
  );
  assert_eq!(space.documents().await, 2);
}

// A link for a document already written.

#[tokio::test]
async fn gives_a_room_of_other_people_the_share_link_keeps_the_one_that_stands_and_gives_the_owners_private_room_the_console_address()
 {
  let space = Space::new().await;
  let id = written(
    &space
      .bmo(
        &["archive", "doc", "--chat", "42", "--by", "42", "--title", "Benchmark"],
        "# Benchmark\n\nbody",
      )
      .await
      .out,
  );
  let number = id.to_string();
  let owner = space.bmo(&["archive", "share", &number, "--chat", "42"], "").await;

  assert!(owner.ok, "{}", owner.err);
  assert!(owner.out.contains(&format!("give: {CONSOLE}/archive/{id}\n")), "{}", owner.out);
  assert!(!owner.out.contains("/share/"));

  let first = space.bmo(&["archive-share", &number, "--chat", "-100", "--days", "3"], "").await;
  let (token, life, _) = space.share(id).await.expect("a share link");

  assert_eq!(first.err, "");
  assert!(
    first.out.starts_with(&format!("archive-share: document {id}, Benchmark\n")),
    "{}",
    first.out
  );
  assert!(first.out.contains(&format!("give: {CONSOLE}/share/{token}\n")));
  assert_eq!(life, Some(3 * 24 * 60 * 60));
  assert!(
    space
      .bmo(&["archive", "share", &number, "--chat", "-200"], "")
      .await
      .out
      .contains(&format!("give: {CONSOLE}/share/{token}\n"))
  );

  let endless = space.bmo(&["archive", "share", &number, "--chat", "-100", "--forever"], "").await;

  assert!(endless.ok, "{}", endless.err);
  assert!(endless.out.contains(&format!("link: {CONSOLE}/share/{token} (never ends)")));
  assert_eq!(space.share(id).await.map(|(token, life, _)| (token, life)), Some((token, None)));
  assert!(
    space
      .bmo(&["archive", "share", &number, "--chat", "-100", "--days", "3", "--forever"], "")
      .await
      .err
      .contains("usage:")
  );
}

#[tokio::test]
async fn refuses_a_number_that_names_no_document_a_search_that_kept_none_and_a_call_with_no_room() {
  let space = Space::new().await;

  space
    .scratch
    .run(
      "INSERT INTO searches (chat_id, query, started_depth, ended_depth, status, started_at, finished_at, asker_id, \
       title, documented, kind, author)
       VALUES (-100, 'bare', 'record', 'record', 'done', 1, 1, 42, 'bare', 1, 'record', 'bmo')",
      Vec::new(),
    )
    .await;

  let bare: i64 = space.scratch.texts("SELECT id::text FROM searches").await[0].parse().expect("a number");
  let number = bare.to_string();

  assert!(
    space
      .bmo(&["archive", "share", "999", "--chat", "-100"], "")
      .await
      .err
      .contains("no document 999")
  );
  assert!(
    space
      .bmo(&["archive", "share", &number, "--chat", "-100"], "")
      .await
      .err
      .contains("kept no document")
  );
  assert!(space.bmo(&["archive", "share", &number], "").await.err.contains("usage:"));
  assert!(space.share(bare).await.is_none());
}
