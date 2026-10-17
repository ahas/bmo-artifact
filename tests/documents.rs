//! The document read API (`docs/api.md`, "Documents (HTTP)"), against a scratch database and a file root of
//! the test's own, bmo-receiver and bmo-job stood in for and every page fetch answered by a fake that writes
//! down what it was asked. What it is held to is that every address fetched is one the record holds, found
//! again from a ref the screen hands back, and that every file is read only where it truly sits inside its
//! document's folder. Carried from searches.test.ts, the API's archive.test.ts and moved-screens.test.ts.

mod support;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use bmo_artifact::clients::job::Job;
use bmo_artifact::clients::receiver::Receiver;
use bmo_artifact::clients::tool_host::Raster;
use bmo_artifact::documents::preview::PreviewCache;
use bmo_artifact::documents::read::{
  Asked, AttachmentView, FindingView, Library, RefKind, RefPicture, drawn_names, finding_outside,
  mark_drawn,
};
use bmo_artifact::http::documents::{list_asked, moved_screen, page_limit};
use bmo_artifact::plumbing::http::{Standing, router};
use bmo_artifact::settings::Settings;
use bmo_artifact::wording::Wording;
use serde_json::{Value, json};
use support::ROOT;
use support::scratch::Scratch;
use support::units::{self, Book, Units};
use support::web::{FakeWeb, PNG};
use tower::ServiceExt;

const ROOM: i64 = 42;
// The moment every reading below is made at, past every search here and past no link.
const NOW: i64 = 2_000_000_000;
const HOUR: i64 = 60 * 60;

// One archive of the test's own: four searches, as their rows and files stand.
struct Shelf {
  scratch: Scratch,
  _units: Units,
  root: tempfile::TempDir,
  web: Arc<FakeWeb>,
  library: Library,
  settings: HashMap<String, String>,
  first: i64,
  second: i64,
  third: i64,
  valued: i64,
}

async fn returning(scratch: &Scratch, sql: &str, values: Vec<sea_orm::Value>) -> i64 {
  scratch.count(sql, values).await
}

async fn search(scratch: &Scratch, chat: i64, query: &str, depth: &str, options: Option<&str>) -> i64 {
  returning(
    scratch,
    "INSERT INTO searches (chat_id, query, started_depth, ended_depth, status, started_at, options, kind, author)
       VALUES ($1, $2, $3, $3, 'open', 1, $4, 'search', 'searcher') RETURNING id",
    vec![chat.into(), query.into(), depth.into(), options.map(str::to_owned).into()],
  )
  .await
}

async fn finding(scratch: &Scratch, search_id: i64, place: i64, text: &str) -> i64 {
  returning(
    scratch,
    "INSERT INTO search_findings (search_id, place, text) VALUES ($1, $2, $3) RETURNING id",
    vec![search_id.into(), place.into(), text.into()],
  )
  .await
}

async fn source(
  scratch: &Scratch,
  search_id: i64,
  finding_id: Option<i64>,
  url: &str,
  supports: Option<&str>,
  mark: Option<&str>,
) -> i64 {
  returning(
    scratch,
    "INSERT INTO search_sources (search_id, finding_id, url, supports, mark) VALUES ($1, $2, $3, $4, $5) RETURNING id",
    vec![search_id.into(), finding_id.into(), url.into(), supports.map(str::to_owned).into(), mark.map(str::to_owned).into()],
  )
  .await
}

async fn finish(scratch: &Scratch, search_id: i64, status: &str, document: Option<&Path>) {
  scratch
    .run(
      "UPDATE searches SET status = $1, finished_at = 2, document = $2 WHERE id = $3",
      vec![status.into(), document.map(|path| path.display().to_string()).into(), search_id.into()],
    )
    .await;
}

async fn keep(
  scratch: &Scratch,
  search_id: i64,
  media: &str,
  url: Option<&str>,
  path: &Path,
  kind: &str,
  name: &str,
) -> i64 {
  returning(
    scratch,
    "INSERT INTO archive_attachments (search_id, media, url, path, type, bytes, name, kept_at)
       VALUES ($1, $2, $3, $4, $5, 3, $6, 1) RETURNING id",
    vec![
      search_id.into(),
      media.into(),
      url.map(str::to_owned).into(),
      path.display().to_string().into(),
      kind.into(),
      name.into(),
    ],
  )
  .await
}

const FIRST_TEXT: [&str; 9] = [
  "NAME: GEM12",
  "The official store https://shop.example.com/gem12. and https://cited.example.com/page for reference.",
  "IMAGE: https://cdn.example.com/p.jpg :: the unit itself",
  "![rear](https://cdn.example.com/back.png)",
  "[a review](https://review.example.com/r) and https://cdn.example.com/p.jpg",
  "MAP: https://maps.example.com/p/search/%EA%B0%80%EA%B2%8C",
  "REVIEW: 4.5 :: The fan is louder than expected. :: https://review.example.com/one",
  "REVIEW: none :: a review with no rating",
  "IMAGE: javascript:alert(1) :: fake",
];

impl Shelf {
  async fn new() -> Self {
    let scratch = Scratch::new().await;
    let (units, address) = units::serve(Book {
      rooms: vec![(ROOM, "Alpha Room".to_owned())],
      ..Book::default()
    })
    .await;
    let root = tempfile::tempdir().unwrap();
    let searches = root.path().join("searches");
    let outside = root.path().join("outside");

    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.md"), "not a search").unwrap();

    let first = search(&scratch, ROOM, "where to buy a mini PC", "basic", None).await;
    let one = finding(&scratch, first, 1, &FIRST_TEXT.join("\n")).await;

    source(&scratch, first, Some(one), "https://cited.example.com/page", Some("price"), Some("1†")).await;
    finding(&scratch, first, 2, "SUMMARY: Buy it at the official store.").await;
    source(&scratch, first, None, "https://opened.example.com/", None, None).await;
    scratch
      .run(
        r#"UPDATE searches SET title = 'Mini PCs are cheapest at the official store', topic = 'Mini PC stores',
             keywords = '["mini PC","GEM12"]' WHERE id = $1"#,
        vec![first.into()],
      )
      .await;

    let document = searches.join(first.to_string()).join("document.md");

    std::fs::create_dir_all(document.parent().unwrap()).unwrap();
    std::fs::write(&document, "# Search · mini PC\n").unwrap();
    finish(&scratch, first, "done", Some(&document)).await;

    let second = search(
      &scratch,
      ROOM,
      "a document whose link points outside",
      "thorough",
      Some(r#"{"shape":"brief"}"#),
    )
    .await;
    let pointing = searches.join(second.to_string()).join("document.md");

    std::fs::create_dir_all(pointing.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(outside.join("secret.md"), &pointing).unwrap();
    finish(&scratch, second, "failed", Some(&pointing)).await;

    let third = search(&scratch, 5, "a search still running", "exhaustive", None).await;

    // Another search's source, which a ref under the first search must never reach.
    source(&scratch, third, None, "http://169.254.169.254/latest/meta-data/", None, None).await;

    // A question after one value: answered in the room, no document kept, so the list of documents does not
    // carry it.
    let valued = search(&scratch, ROOM, "silver price today", "basic", None).await;

    scratch
      .run(
        "UPDATE searches SET title = 'Silver at 12,410 won per 3.75 g', topic = 'silver price', documented = 0 WHERE id = $1",
        vec![valued.into()],
      )
      .await;
    finish(&scratch, valued, "done", None).await;

    let web = Arc::new(FakeWeb::default());
    let previews = Arc::new(PreviewCache::new(root.path().join("web-cache"), web.clone()));
    let library = Library {
      db: scratch.db().clone(),
      root: root.path().to_path_buf(),
      words: Wording::english().unwrap().search,
      job: Job::new(Some(address.clone())),
      receiver: Receiver::new(Some(address.clone())),
      previews,
    };
    let settings = HashMap::from([
      ("BMO_ARTIFACT_DATABASE__URL".to_owned(), scratch.address()),
      ("BMO_ARTIFACT_FILES__ROOT".to_owned(), root.path().display().to_string()),
      ("BMO_ARTIFACT_UNITS__JOB".to_owned(), address.clone()),
      ("BMO_ARTIFACT_UNITS__RECEIVER".to_owned(), address),
      ("BMO_ARTIFACT_CONSOLE__ADDRESS".to_owned(), "https://console.example/bmo".to_owned()),
    ]);

    Self {
      scratch,
      _units: units,
      root,
      web,
      library,
      settings,
      first,
      second,
      third,
      valued,
    }
  }

  fn fetched(&self) -> Vec<String> {
    self.web.taken()
  }

  async fn page(&self, asked: Asked) -> bmo_artifact::documents::read::Page {
    self.library.list(&asked).await.unwrap()
  }

  async fn ids(&self, asked: Asked) -> Vec<i64> {
    self.page(asked).await.items.iter().map(|item| item.id).collect()
  }

  async fn found(&self, phrase: &str, kind: Option<&str>, limit: i64, before: Option<i64>) -> Vec<i64> {
    self.ids(asked(limit, before, false, kind, Some(phrase), NOW)).await
  }

  async fn share(&self, search_id: i64, expires_at: Option<i64>) {
    self
      .scratch
      .run(
        "INSERT INTO web_search_shares (share_token, search_id, created_by, created_at, expires_at) VALUES ($1, $2, 'tester', $3, $4)",
        vec![format!("tok-{search_id}").into(), search_id.into(), (NOW - HOUR).into(), expires_at.into()],
      )
      .await;
  }

  async fn marks(&self, now: i64) -> HashMap<i64, Value> {
    self
      .page(asked(10, None, false, None, None, now))
      .await
      .items
      .into_iter()
      .map(|item| (item.id, json!(item.shared)))
      .collect()
  }

  // The service's own router, on this archive and the fake web.
  async fn ask(
    &self,
    path: &str,
    headers: &[(&str, &str)],
  ) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let settings = Settings::read(Path::new(ROOT), Some(self.settings.clone())).unwrap();
    let mut standing = Standing::new(settings).unwrap();

    standing.previews = Arc::clone(&self.library.previews);

    let mut request = Request::get(path);

    for (name, value) in headers {
      request = request.header(*name, *value);
    }

    let response = router(standing).unwrap().oneshot(request.body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap().to_vec();

    (status, headers, body)
  }

  async fn json(&self, path: &str) -> (StatusCode, Value) {
    let (status, _, body) = self.ask(path, &[]).await;

    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
  }
}

fn asked(
  limit: i64,
  before: Option<i64>,
  shared: bool,
  kind: Option<&str>,
  phrase: Option<&str>,
  now: i64,
) -> Asked {
  Asked {
    limit,
    before,
    shared,
    kind: kind.map(str::to_owned),
    phrase: phrase.map(str::to_owned),
    now,
  }
}

fn all(now: i64) -> Asked {
  asked(10, None, false, None, None, now)
}

// A part out of the middle of a text, so what is found is found by a part of it.
fn middle(text: &str) -> String {
  let chars: Vec<char> = text.chars().collect();

  chars[2..chars.len() - 2].iter().collect()
}

// ---------------------------------------------------------------- the list

#[tokio::test]
async fn is_newest_first_carries_what_a_row_shows_and_pages_by_id() {
  let shelf = Shelf::new().await;
  let page = shelf.page(asked(2, None, false, None, None, NOW)).await;
  let rows: Vec<Value> = page.items.iter().map(|item| json!(item)).collect();

  assert!(page.has_more);
  assert_eq!(page.items.iter().map(|item| item.id).collect::<Vec<_>>(), [shelf.third, shelf.second]);
  assert_eq!(
    (&rows[1]["room"], &rows[1]["query"], &rows[1]["status"], &rows[1]["startedDepth"]),
    (
      &json!("Alpha Room"),
      &json!("a document whose link points outside"),
      &json!("failed"),
      &json!("thorough")
    )
  );
  // A search that wrote no head has none, and the options the launch gave come back as given.
  assert_eq!(
    (
      &rows[1]["title"],
      &rows[1]["topic"],
      &rows[1]["keywords"],
      &rows[1]["rule"],
      &rows[1]["options"]
    ),
    (&Value::Null, &Value::Null, &json!([]), &Value::Null, &json!({ "shape": "brief" }))
  );
  assert_eq!(
    (&rows[0]["status"], &rows[0]["finishedAt"], &rows[0]["sources"]),
    (&json!("open"), &Value::Null, &json!(1))
  );

  let rest = shelf.page(asked(2, Some(shelf.second), false, None, None, NOW)).await;

  assert!(!rest.has_more);
  assert_eq!(
    rest.items.iter().map(|item| (item.id, item.findings, item.sources)).collect::<Vec<_>>(),
    [(shelf.first, 2, 2)]
  );
}

#[tokio::test]
async fn leaves_out_a_search_that_kept_no_document_since_the_list_is_a_list_of_documents() {
  let shelf = Shelf::new().await;
  let page = shelf.page(all(NOW)).await;

  assert!(!page.items.iter().any(|item| item.id == shelf.valued));
  assert!(page.items.iter().all(|item| item.documented));
}

// Every row here is a search the searcher wrote, so held to searches the list is whole, and held to another
// kind it is empty — the holding is the reading's, not a page with rows taken out.
#[tokio::test]
async fn carries_each_documents_kind_and_author_and_is_held_to_one_kind_in_the_reading_itself() {
  let shelf = Shelf::new().await;
  let whole = shelf.page(all(NOW)).await;

  assert!(whole.items.iter().all(|item| item.kind == "search" && item.author == "searcher"));
  assert_eq!(
    shelf.ids(asked(10, None, false, Some("search"), None, NOW)).await,
    whole.items.iter().map(|item| item.id).collect::<Vec<_>>()
  );
  assert!(shelf.ids(asked(10, None, false, Some("record"), None, NOW)).await.is_empty());
}

#[tokio::test]
async fn but_the_search_itself_is_still_read_one_by_one_and_says_it_kept_none() {
  let shelf = Shelf::new().await;
  let detail = json!(shelf.library.detail(shelf.valued, NOW).await.unwrap().unwrap());

  assert_eq!(
    (
      &detail["search"]["query"],
      &detail["search"]["status"],
      &detail["search"]["documented"],
      &detail["search"]["document"]
    ),
    (&json!("silver price today"), &json!("done"), &json!(false), &json!(false))
  );
  assert_eq!(detail["findings"], json!([]));
}

// ---------------------------------------------------------------- searching the list

#[tokio::test]
async fn finds_a_part_of_the_title_the_question_the_topic_the_keywords_the_room_and_the_body_without_regard_to_case()
 {
  let shelf = Shelf::new().await;
  let (first, second) = (shelf.first, shelf.second);

  assert_eq!(
    shelf
      .found(&middle("Mini PCs are cheapest at the official store"), None, 10, None)
      .await,
    [first]
  );
  assert_eq!(
    shelf.found(&middle("a document whose link points outside"), None, 10, None).await,
    [second]
  );
  assert_eq!(shelf.found(&middle("Mini PC stores"), None, 10, None).await, [first]);
  assert_eq!(shelf.found("GEM12", None, 10, None).await, [first]);
  assert_eq!(shelf.found("Alpha Room", None, 10, None).await, [second, first]);
  assert_eq!(shelf.found("SHOP.EXAMPLE.COM/GEM", None, 10, None).await, [first]);
}

#[tokio::test]
async fn is_a_phrase_matched_whole_and_none_at_all_where_nothing_says_it() {
  let shelf = Shelf::new().await;

  assert!(shelf.found("GEM12 PC", None, 10, None).await.is_empty());
  assert!(shelf.found("100%", None, 10, None).await.is_empty());
  // A search that kept no document is left out, whatever it says.
  assert!(shelf.found("12,410", None, 10, None).await.is_empty());
}

#[tokio::test]
async fn an_empty_or_blank_phrase_is_no_search() {
  let shelf = Shelf::new().await;

  assert_eq!(shelf.found("  ", None, 10, None).await, shelf.found("", None, 10, None).await);
  assert!(shelf.found("", None, 10, None).await.len() > 1);
}

#[tokio::test]
async fn works_with_the_kind_and_pages_on_from_its_own_last_row() {
  let shelf = Shelf::new().await;

  assert_eq!(shelf.found("Alpha Room", Some("search"), 10, None).await, [shelf.second, shelf.first]);
  assert!(shelf.found("Alpha Room", Some("record"), 10, None).await.is_empty());

  let page = shelf.page(asked(1, None, false, None, Some("Alpha Room"), NOW)).await;

  assert!(page.has_more);
  assert_eq!(page.items.iter().map(|item| item.id).collect::<Vec<_>>(), [shelf.second]);
  assert_eq!(shelf.found("Alpha Room", None, 1, Some(shelf.second)).await, [shelf.first]);
}

// The row shows nothing of the body, so the words around the hit come with it; a row found by its head
// carries none, and an unsearched list never does.
#[tokio::test]
async fn hands_over_the_bodys_words_around_the_hit_only_where_the_body_said_it() {
  let shelf = Shelf::new().await;
  let searched = shelf.page(asked(10, None, false, None, Some("cited.example.com"), NOW)).await;
  let by_head = shelf
    .page(asked(10, None, false, None, Some(&middle("a document whose link points outside")), NOW))
    .await;

  assert_eq!(searched.items[0].id, shelf.first);
  assert!(
    json!(searched.items[0])["excerpt"]
      .as_str()
      .unwrap()
      .contains("https://cited.example.com/page")
  );
  assert_eq!(json!(by_head.items[0])["excerpt"], Value::Null);
  assert!(json!(by_head.items[0]).get("excerpt").is_some());
  assert!(json!(shelf.page(all(NOW)).await.items[0]).get("excerpt").is_none());
}

// ---------------------------------------------------------------- what the list says about sharing

#[tokio::test]
async fn a_search_with_no_link_is_marked_as_having_none_and_one_with_a_live_link_carries_when_it_ends() {
  let shelf = Shelf::new().await;

  assert_eq!(
    shelf.marks(NOW).await,
    HashMap::from([(shelf.first, Value::Null), (shelf.second, Value::Null), (shelf.third, Value::Null)])
  );

  shelf.share(shelf.first, Some(NOW + HOUR)).await;

  assert_eq!(shelf.marks(NOW).await[&shelf.first], json!({ "expiresAt": NOW + HOUR }));
  // A link whose time has passed is no mark at all.
  assert_eq!(shelf.marks(NOW + HOUR).await[&shelf.first], Value::Null);
}

#[tokio::test]
async fn a_link_with_no_end_is_live_and_says_it_has_none_which_is_not_the_same_as_having_no_link() {
  let shelf = Shelf::new().await;

  shelf.share(shelf.first, None).await;

  assert_eq!(
    shelf.marks(NOW + 100 * 365 * 24 * HOUR).await[&shelf.first],
    json!({ "expiresAt": null })
  );
}

#[tokio::test]
async fn holding_the_list_to_what_is_shared_leaves_out_everything_else_a_passed_link_included() {
  let shelf = Shelf::new().await;

  shelf.share(shelf.first, None).await;
  shelf.share(shelf.third, Some(NOW + HOUR)).await;

  assert_eq!(shelf.ids(asked(10, None, true, None, None, NOW)).await, [shelf.third, shelf.first]);
  assert_eq!(shelf.ids(asked(10, None, true, None, None, NOW + HOUR)).await, [shelf.first]);
}

// The holding is the reading's, so a page of it is a page of shared searches rather than what is left of a
// page of everything.
#[tokio::test]
async fn a_held_page_is_as_long_as_it_was_asked_for_and_pages_on_from_its_own_last_row() {
  let shelf = Shelf::new().await;

  for id in [shelf.first, shelf.second, shelf.third] {
    shelf.share(id, None).await;
  }

  let page = shelf.page(asked(2, None, true, None, None, NOW)).await;

  assert!(page.has_more);
  assert_eq!(page.items.iter().map(|item| item.id).collect::<Vec<_>>(), [shelf.third, shelf.second]);
  assert_eq!(shelf.ids(asked(2, Some(shelf.second), true, None, None, NOW)).await, [shelf.first]);
}

#[tokio::test]
async fn one_search_read_whole_says_the_same_about_its_link_as_the_list_does_and_its_shared_reading_nothing()
 {
  let shelf = Shelf::new().await;

  shelf.share(shelf.first, None).await;

  let first = json!(shelf.library.detail(shelf.first, NOW).await.unwrap().unwrap());
  let second = json!(shelf.library.detail(shelf.second, NOW).await.unwrap().unwrap());

  assert_eq!(first["search"]["shared"], json!({ "expiresAt": null }));
  assert_eq!(second["search"]["shared"], Value::Null);
  // What a share page answers with is the document alone, and being shared is not part of it.
  assert!(
    !json!(shelf.library.shared(shelf.first, NOW).await.unwrap())
      .to_string()
      .contains("shared")
  );
}

// ---------------------------------------------------------------- one search

#[tokio::test]
async fn puts_cited_sources_under_their_finding_and_opened_pages_apart_with_its_kind_and_outline() {
  let shelf = Shelf::new().await;
  let detail = json!(shelf.library.detail(shelf.first, NOW).await.unwrap().unwrap());
  let search = &detail["search"];

  assert_eq!(
    (
      &search["id"],
      &search["document"],
      &search["runs"],
      &search["title"],
      &search["topic"],
      &search["keywords"],
      &search["options"]
    ),
    (
      &json!(shelf.first),
      &json!(true),
      &json!([]),
      &json!("Mini PCs are cheapest at the official store"),
      &json!("Mini PC stores"),
      &json!(["mini PC", "GEM12"]),
      &json!({})
    )
  );
  assert_eq!(
    detail["findings"]
      .as_array()
      .unwrap()
      .iter()
      .map(|one| one["place"].clone())
      .collect::<Vec<_>>(),
    [json!(1), json!(2)]
  );

  let cited = &detail["findings"][0]["sources"];

  assert_eq!(cited.as_array().unwrap().len(), 1);
  assert!(
    cited[0]["ref"]
      .as_str()
      .unwrap()
      .strip_prefix('s')
      .is_some_and(|id| id.parse::<i64>().is_ok())
  );
  assert_eq!(
    (&cited[0]["url"], &cited[0]["title"], &cited[0]["supports"], &cited[0]["mark"]),
    (&json!("https://cited.example.com/page"), &Value::Null, &json!("price"), &json!("1†"))
  );
  assert_eq!(
    detail["opened"]
      .as_array()
      .unwrap()
      .iter()
      .map(|one| one["url"].clone())
      .collect::<Vec<_>>(),
    [json!("https://opened.example.com/")]
  );
  assert_eq!(detail["kind"]["id"], "search");
  assert_eq!(detail["kind"]["summary"], true);
  assert_eq!(detail["outline"]["entries"][0]["text"], "GEM12");
  assert_eq!(detail["outline"]["marks"]["1"], json!(["1-gem12"]));
  // Each finding says what it is called and where it folds, so the console reads neither rule for itself.
  assert_eq!(
    (&detail["findings"][0]["name"], &detail["findings"][0]["title"]),
    (&json!("GEM12"), &json!("GEM12"))
  );
  // Its markdown is its structure lines in the wording file's words, ready for the console to draw.
  let first = detail["findings"][0]["markdown"].as_str().unwrap();

  assert!(first.contains("\n### GEM12\n"), "{first}");
  assert!(first.contains("[a review](https://review.example.com/r)"), "{first}");
  assert!(!first.contains("NAME:") && !first.contains("IMAGE:"), "{first}");
  assert_eq!(detail["findings"][0]["foldedMarkdown"], Value::Null);
  assert_eq!(detail["findings"][1]["markdown"], "\n**Summary** Buy it at the official store.");
  assert_eq!(detail["findings"][1]["name"], Value::Null);
  // A label with words on its own line opens no part, so the finding is called by its place.
  assert_eq!(detail["findings"][1]["title"], "Part 2");
  assert!(shelf.library.detail(9999, NOW).await.unwrap().is_none());
}

// ---------------------------------------------------------------- the files a body draws

#[test]
fn a_body_draws_the_files_its_pictures_name_by_their_kept_names() {
  let names = drawn_names([
    "# Palette\n\n![01](palette-01.webp)\n\n[open the page](gallery.html) ![](<receipt photo.png>)",
    "see note.txt, and ![](cut%20out.png)",
    "```\n![](fenced.png)\n```",
  ]);
  let mut names: Vec<&str> = names.iter().map(String::as_str).collect();

  names.sort_unstable();
  // A link is not the file shown, and nothing inside a code block is a picture.
  assert_eq!(names, ["cut out.png", "palette-01.webp", "receipt photo.png"]);
}

#[test]
fn a_file_the_body_draws_is_marked_drawn_and_one_it_links_or_never_names_is_not() {
  let view = |id: i64, media: &str, name: &str| AttachmentView {
    id,
    media: media.to_owned(),
    r#type: String::new(),
    bytes: 3,
    name: name.to_owned(),
    drawn: false,
  };
  let finding = |markdown: &str, folded: Option<&str>| FindingView {
    place: 1,
    run_id: None,
    branch: None,
    text: String::new(),
    name: None,
    title: String::new(),
    markdown: markdown.to_owned(),
    folded_markdown: folded.map(str::to_owned),
    images: Vec::new(),
    links: Vec::new(),
    sources: Vec::new(),
  };
  let mut attachments = [
    view(7, "page", "gallery.html"),
    view(8, "image", "palette-01.webp"),
    view(9, "image", "receipt photo.png"),
    view(10, "text", "note.txt"),
    view(11, "image", "linked.png"),
    // A file that is not a picture stands among the attachments even where a picture names it.
    view(12, "file", "drawn.bin"),
  ];

  mark_drawn(
    &mut attachments,
    &[
      finding("![01](palette-01.webp)\n\n[the page](gallery.html) [a picture](linked.png)", None),
      finding("words", Some("![](<receipt photo.png>) ![](drawn.bin)")),
    ],
  );

  assert_eq!(
    attachments.iter().map(|one| (one.id, one.drawn)).collect::<Vec<_>>(),
    [(7, false), (8, true), (9, true), (10, false), (11, false), (12, false)]
  );
}

#[tokio::test]
async fn a_document_read_says_which_of_its_files_its_body_draws() {
  let shelf = Shelf::new().await;
  let kept = |name: &str| shelf.root.path().join(name);

  finding(&shelf.scratch, shelf.third, 1, "![01](palette-01.webp)\n\n[the page](gallery.html)").await;

  let drawn = keep(
    &shelf.scratch,
    shelf.third,
    "image",
    None,
    &kept("palette-01.webp"),
    "image/webp",
    "palette-01.webp",
  )
  .await;
  let other =
    keep(&shelf.scratch, shelf.third, "image", None, &kept("other.png"), "image/png", "other.png").await;
  let page = keep(
    &shelf.scratch,
    shelf.third,
    "page",
    None,
    &kept("gallery.html"),
    "text/html",
    "gallery.html",
  )
  .await;
  let detail = json!(shelf.library.detail(shelf.third, NOW).await.unwrap().unwrap());

  assert_eq!(
    detail["attachments"]
      .as_array()
      .unwrap()
      .iter()
      .map(|one| (one["id"].as_i64().unwrap(), one["drawn"].as_bool().unwrap()))
      .collect::<Vec<_>>(),
    [(drawn, true), (other, false), (page, false)]
  );
}

// What is folded under a finding's fold line is markdown of its own, its blocks written as the console
// reads them.
#[tokio::test]
async fn answers_a_findings_folded_part_as_markdown_of_its_own() {
  let shelf = Shelf::new().await;
  let text = [
    "SUMMARY:",
    "1. Short.",
    "MORE:",
    "IMAGE: https://cdn.example.com/a.jpg :: one",
    "IMAGE: https://cdn.example.com/b.jpg :: two",
  ]
  .join("\n");

  finding(&shelf.scratch, shelf.third, 1, &text).await;

  let detail = json!(shelf.library.detail(shelf.third, NOW).await.unwrap().unwrap());
  let open = detail["findings"][0]["markdown"].as_str().unwrap();
  let folded = detail["findings"][0]["foldedMarkdown"].as_str().unwrap();

  assert!(open.contains("#### Summary"), "{open}");
  assert!(!open.contains("cdn.example.com"), "{open}");
  assert!(
    folded.contains(
      "::: gallery\n![one](<https://cdn.example.com/a.jpg>)\n![two](<https://cdn.example.com/b.jpg>)\n:::"
    ),
    "{folded}"
  );
}

#[tokio::test]
async fn finds_the_pictures_and_the_links_a_findings_text_points_at() {
  let shelf = Shelf::new().await;
  let detail = json!(shelf.library.detail(shelf.first, NOW).await.unwrap().unwrap());
  let finding = &detail["findings"][0];

  assert_eq!(
    finding["images"],
    json!([
      { "ref": "i1.1", "url": "https://cdn.example.com/p.jpg", "caption": "the unit itself" },
      { "ref": "i1.2", "url": "https://cdn.example.com/back.png", "caption": "rear" },
    ])
  );
  // The cited page is a source and the picture's address is a picture; neither is a link. The full stop
  // after the shop's address is the sentence's.
  assert_eq!(
    finding["links"],
    json!([
      { "ref": "l1.1", "url": "https://shop.example.com/gem12", "label": null },
      { "ref": "l1.2", "url": "https://review.example.com/r", "label": "a review" },
      { "ref": "l1.3", "url": "https://maps.example.com/p/search/%EA%B0%80%EA%B2%8C", "label": "Map" },
      // The page a review stands on, taken from the review's own line rather than as a bare address in the
      // middle of somebody's words.
      { "ref": "l1.4", "url": "https://review.example.com/one", "label": "Review" },
    ])
  );
}

#[test]
fn a_text_with_nothing_outside_has_no_pictures_and_no_links() {
  let words = Wording::english().unwrap().search;

  assert_eq!(finding_outside("TITLE: none\nJust text.", 3, &HashSet::new(), &words), (vec![], vec![]));
}

// ---------------------------------------------------------------- refs

async fn cited_ref(shelf: &Shelf) -> String {
  json!(shelf.library.detail(shelf.first, NOW).await.unwrap().unwrap())["findings"][0]["sources"][0]["ref"]
    .as_str()
    .unwrap()
    .to_owned()
}

async fn foreign_ref(shelf: &Shelf) -> String {
  json!(shelf.library.detail(shelf.third, NOW).await.unwrap().unwrap())["opened"][0]["ref"]
    .as_str()
    .unwrap()
    .to_owned()
}

#[tokio::test]
async fn refs_resolve_only_to_places_this_searchs_record_holds() {
  let shelf = Shelf::new().await;
  let (cited, foreign) = (cited_ref(&shelf).await, foreign_ref(&shelf).await);
  let library = &shelf.library;

  assert_eq!(
    library.resolve_ref(shelf.first, &cited).await.unwrap(),
    Some((RefKind::Source, "https://cited.example.com/page".to_owned()))
  );
  assert_eq!(
    library.resolve_ref(shelf.first, "i1.2").await.unwrap(),
    Some((RefKind::Image, "https://cdn.example.com/back.png".to_owned()))
  );
  assert_eq!(
    library.resolve_ref(shelf.first, "l1.3").await.unwrap().map(|(kind, _)| kind),
    Some(RefKind::Link)
  );

  for reference in [
    foreign.as_str(),
    "i1.3",
    "i2.1",
    "l1.5",
    "l9.1",
    "s0",
    "x1",
    "https://evil.example/",
    "i1.1/../x",
    "",
  ] {
    assert_eq!(library.resolve_ref(shelf.first, reference).await.unwrap(), None, "{reference}");
  }

  assert_eq!(library.resolve_ref(9999, &cited).await.unwrap(), None);
}

#[tokio::test]
async fn an_index_outside_the_record_fetches_nothing() {
  let shelf = Shelf::new().await;
  let (cited, foreign) = (cited_ref(&shelf).await, foreign_ref(&shelf).await);
  let library = &shelf.library;

  assert_eq!(library.ref_preview(shelf.first, "s99999").await.unwrap(), None);
  assert_eq!(library.ref_image(shelf.first, "i1.9", RefPicture::Own).await.unwrap(), None);
  assert_eq!(library.ref_image(shelf.first, &foreign, RefPicture::Thumb).await.unwrap(), None);
  // A picture ref has no preview and no thumbnail of its own; a source is not a picture.
  assert_eq!(library.ref_preview(shelf.first, "i1.1").await.unwrap(), None);
  assert_eq!(library.ref_image(shelf.first, "i1.1", RefPicture::Thumb).await.unwrap(), None);
  assert_eq!(library.ref_image(shelf.first, &cited, RefPicture::Own).await.unwrap(), None);
  assert!(shelf.fetched().is_empty());
}

#[tokio::test]
async fn a_preview_and_its_picture_are_fetched_once_and_then_served_from_the_cache() {
  let shelf = Shelf::new().await;
  let cited = cited_ref(&shelf).await;
  let library = &shelf.library;
  let png = Some(Raster {
    kind: "image/png".to_owned(),
    bytes: PNG.to_vec(),
  });

  for _ in 0..2 {
    let preview = json!(library.ref_preview(shelf.first, &cited).await.unwrap());

    assert_eq!(
      preview,
      json!({ "title": "title cited.example.com", "thumbnail": true, "favicon": false })
    );
  }

  assert_eq!(library.ref_image(shelf.first, &cited, RefPicture::Thumb).await.unwrap(), png);
  assert_eq!(library.ref_image(shelf.first, &cited, RefPicture::Thumb).await.unwrap(), png);
  assert_eq!(library.ref_image(shelf.first, &cited, RefPicture::Favicon).await.unwrap(), None);
  assert_eq!(library.ref_image(shelf.first, "i1.1", RefPicture::Own).await.unwrap(), png);
  assert_eq!(library.ref_image(shelf.first, "i1.1", RefPicture::Own).await.unwrap(), png);
  assert_eq!(
    shelf.fetched(),
    [
      "preview https://cited.example.com/page",
      "image https://img.example.com/card.png",
      "image https://cdn.example.com/p.jpg",
    ]
  );
}

// ---------------------------------------------------------------- the copy a search kept of a picture

const KEPT: [u8; 12] = [0x00, 0x00, 0x00, 0x20, 0x66, 0x74, 0x79, 0x70, 0x61, 0x76, 0x69, 0x66];

impl Shelf {
  fn kept_file(&self, name: &str) -> PathBuf {
    let path = self.root.path().join("searches").join(self.first.to_string()).join("images").join(name);

    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, KEPT).unwrap();
    path
  }
}

#[tokio::test]
async fn the_kept_copy_is_what_comes_out_and_the_address_it_came_from_is_never_fetched_again() {
  let shelf = Shelf::new().await;
  let path = shelf.kept_file("back.avif");

  keep(
    &shelf.scratch,
    shelf.first,
    "image",
    Some("https://cdn.example.com/back.png"),
    &path,
    "image/avif",
    "back.avif",
  )
  .await;

  assert_eq!(
    shelf.library.ref_image(shelf.first, "i1.2", RefPicture::Own).await.unwrap(),
    Some(Raster {
      kind: "image/avif".to_owned(),
      bytes: KEPT.to_vec(),
    })
  );
  assert!(shelf.fetched().is_empty());
}

#[tokio::test]
async fn a_copy_gone_from_the_disk_or_pointing_outside_the_documents_falls_back_to_the_address() {
  let shelf = Shelf::new().await;
  let gone = shelf
    .root
    .path()
    .join("searches")
    .join(shelf.first.to_string())
    .join("images")
    .join("missing.avif");
  let outside = shelf.root.path().join("outside").join("secret.md");

  keep(
    &shelf.scratch,
    shelf.first,
    "image",
    Some("https://cdn.example.com/p.jpg"),
    &gone,
    "image/avif",
    "missing.avif",
  )
  .await;
  keep(
    &shelf.scratch,
    shelf.first,
    "image",
    Some("https://cdn.example.com/back.png"),
    &outside,
    "image/avif",
    "secret.md",
  )
  .await;

  for reference in ["i1.1", "i1.2"] {
    let image = shelf.library.ref_image(shelf.first, reference, RefPicture::Own).await.unwrap();

    assert_eq!(image.map(|image| image.kind), Some("image/png".to_owned()), "{reference}");
  }

  assert_eq!(
    shelf.fetched(),
    ["image https://cdn.example.com/p.jpg", "image https://cdn.example.com/back.png"]
  );
}

// ---------------------------------------------------------------- the document file

#[tokio::test]
async fn the_document_file_is_served_only_from_inside_the_documents_folder() {
  let shelf = Shelf::new().await;
  let library = &shelf.library;
  let written = shelf.root.path().join("searches").join(shelf.first.to_string()).join("document.md");

  assert_eq!(library.document_file(shelf.first).await.unwrap(), Some(written.canonicalize().unwrap()));
  assert_eq!(library.document_file(shelf.second).await.unwrap(), None);
  assert_eq!(
    json!(library.detail(shelf.second, NOW).await.unwrap().unwrap())["search"]["document"],
    false
  );
  assert_eq!(library.document_file(shelf.third).await.unwrap(), None);
  assert_eq!(library.document_file(9999).await.unwrap(), None);
}

// ---------------------------------------------------------------- one search read for somebody outside

#[tokio::test]
async fn read_for_somebody_outside_is_the_document_and_nothing_of_the_console() {
  let shelf = Shelf::new().await;
  let shared = json!(shelf.library.shared(shelf.first, NOW).await.unwrap().unwrap());
  let whole = json!(shelf.library.detail(shelf.first, NOW).await.unwrap().unwrap());
  let mut keys: Vec<&str> = shared.as_object().unwrap().keys().map(String::as_str).collect();

  keys.sort_unstable();
  assert_eq!(
    keys,
    ["attachments", "findings", "kind", "kindEntry", "opened", "outline", "runCount", "title"]
  );
  assert_eq!(shared["title"], "Mini PCs are cheapest at the official store");
  assert_eq!(shared["findings"], whole["findings"]);
  assert_eq!(shared["opened"], whole["opened"]);
  // The kind and the contents a link's reader is drawn by are the console's own.
  assert_eq!(shared["kind"], whole["search"]["kind"]);
  assert_eq!(shared["kindEntry"], whole["kind"]);
  assert_eq!(shared["outline"], whole["outline"]);

  // The question stands as the name where the search wrote none, as it does on the console.
  let second = json!(shelf.library.shared(shelf.second, NOW).await.unwrap().unwrap());

  assert_eq!(second["title"], "a document whose link points outside");

  // It carries none of the state, the room, the options, the depth or the times.
  for absent in [
    "failed",
    "thorough",
    "Alpha Room",
    "brief",
    "startedAt",
    "finishedAt",
    "chatId",
    "status",
  ] {
    assert!(!second.to_string().contains(absent), "{absent}");
  }

  assert!(shelf.library.shared(9999, NOW).await.unwrap().is_none());
}

// ---------------------------------------------------------------- the routes

#[test]
fn the_list_pages_within_its_bounds_and_is_held_only_by_what_its_address_says() {
  let query = |pairs: &'static [(&'static str, &'static str)]| {
    move |name: &str| pairs.iter().find(|(key, _)| *key == name).map(|(_, value)| (*value).to_owned())
  };
  let read = |pairs: &'static [(&'static str, &'static str)]| list_asked(query(pairs), 1_000).unwrap();

  assert_eq!(page_limit(Some("100000")), 200);
  assert_eq!(page_limit(Some("-3")), 30);
  assert_eq!(page_limit(None), 30);
  assert_eq!(read(&[("before", "40")]).before, Some(40));
  // A kind is read against the catalogue, so only a kind there is holds the list.
  assert_eq!(read(&[("kind", "learning")]).kind.as_deref(), Some("learning"));
  assert_eq!(read(&[("kind", "diary")]).kind, None);
  // Anything but the flag itself is no filter.
  assert!(read(&[("shared", "1")]).shared);
  assert!(!read(&[("shared", "true")]).shared);
  assert_eq!(read(&[("q", "mini PC")]).phrase.as_deref(), Some("mini PC"));
  assert_eq!(
    list_asked(|name| (name == "q").then(|| "a".repeat(250)), 1_000).unwrap().phrase,
    Some("a".repeat(200))
  );
  assert!(list_asked(query(&[("before", "x")]), 1_000).is_err());
}

#[tokio::test]
async fn a_search_or_a_ref_the_record_does_not_hold_is_not_found_and_a_number_that_is_none_is_refused() {
  let shelf = Shelf::new().await;
  let first = shelf.first;

  assert_eq!(shelf.json("/api/archive/9999").await.1["code"], "document_not_found");
  assert_eq!(
    shelf.json(&format!("/api/archive/{}/document", shelf.second)).await.0,
    StatusCode::NOT_FOUND
  );
  assert_eq!(
    shelf.json(&format!("/api/archive/{first}/refs/s99999/preview")).await.1["code"],
    "no_such_source"
  );
  assert_eq!(
    shelf.json(&format!("/api/archive/{first}/refs/i1.99/image")).await.1["code"],
    "image_unavailable"
  );
  assert_eq!(
    shelf.json(&format!("/api/archive/{first}/refs/i1.1/image?as=thumb")).await.0,
    StatusCode::NOT_FOUND
  );
  assert_eq!(
    shelf.json("/api/archive/abc").await,
    (StatusCode::BAD_REQUEST, shelf.json("/api/archive/0").await.1)
  );
  assert_eq!(shelf.json("/api/archive/abc").await.1["code"], "bad_id");
  assert_eq!(shelf.json("/api/archive?before=x").await.1["code"], "bad_cursor");
}

#[tokio::test]
async fn the_address_is_never_taken_from_the_request() {
  let shelf = Shelf::new().await;
  let first = shelf.first;
  let cited = cited_ref(&shelf).await;

  shelf
    .ask(
      &format!("/api/archive/{first}/refs/i1.1/image?url=http://127.0.0.1/&as=http://evil"),
      &[],
    )
    .await;
  shelf
    .ask(&format!("/api/archive/{first}/refs/{cited}/preview?url=http://127.0.0.1/"), &[])
    .await;

  assert_eq!(
    shelf.fetched(),
    ["image https://cdn.example.com/p.jpg", "preview https://cited.example.com/page"]
  );
}

#[tokio::test]
async fn a_picture_goes_out_as_the_type_its_bytes_are_not_sniffed_sandboxed_and_privately_cached() {
  let shelf = Shelf::new().await;
  let (status, headers, body) =
    shelf.ask(&format!("/api/archive/{}/refs/i1.1/image", shelf.first), &[]).await;
  let header =
    |name: &str| headers.get(name).and_then(|value| value.to_str().ok()).unwrap_or_default().to_owned();

  assert_eq!(status, StatusCode::OK);
  assert_eq!(body, PNG);
  assert_eq!(header("content-type"), "image/png");
  assert_eq!(header("content-length"), "8");
  assert_eq!(header("x-content-type-options"), "nosniff");
  assert_eq!(header("content-security-policy"), "default-src 'none'; sandbox");
  assert_eq!(header("cache-control"), "private, max-age=86400");
}

#[tokio::test]
async fn an_attachment_is_looked_up_under_the_document_the_road_names_and_sent_as_it_is_kept() {
  let shelf = Shelf::new().await;
  let (first, second) = (shelf.first, shelf.second);
  let path = shelf
    .root
    .path()
    .join("searches")
    .join(first.to_string())
    .join("attachments")
    .join("a-meeting.ogg");

  std::fs::create_dir_all(path.parent().unwrap()).unwrap();
  std::fs::write(&path, b"OggS sound").unwrap();

  let id = keep(&shelf.scratch, first, "audio", None, &path, "audio/ogg", "meeting.ogg").await;
  let (status, headers, body) = shelf.ask(&format!("/api/archive/{first}/attachments/{id}"), &[]).await;
  let header = |headers: &axum::http::HeaderMap, name: &str| {
    headers.get(name).and_then(|value| value.to_str().ok()).unwrap_or_default().to_owned()
  };

  assert_eq!((status, body.as_slice()), (StatusCode::OK, b"OggS sound".as_slice()));
  assert_eq!(header(&headers, "content-type"), "audio/ogg");
  assert_eq!(header(&headers, "accept-ranges"), "bytes");
  assert_eq!(header(&headers, "content-disposition"), "inline");
  assert!(header(&headers, "content-security-policy").contains("script-src 'none'"));

  let (status, headers, body) = shelf
    .ask(&format!("/api/archive/{first}/attachments/{id}"), &[("range", "bytes=0-3")])
    .await;

  assert_eq!((status, body.as_slice()), (StatusCode::PARTIAL_CONTENT, b"OggS".as_slice()));
  assert_eq!(header(&headers, "content-range"), "bytes 0-3/10");

  let (_, headers, _) = shelf.ask(&format!("/api/archive/{first}/attachments/{id}?download=1"), &[]).await;

  assert_eq!(header(&headers, "content-disposition"), "attachment; filename*=UTF-8''meeting.ogg");
  assert_eq!(
    shelf.json(&format!("/api/archive/{second}/attachments/{id}")).await.1["code"],
    "attachment_not_found"
  );
  assert_eq!(
    shelf.json(&format!("/api/archive/{first}/attachments/x")).await.1["code"],
    "bad_attachment_id"
  );
}

#[tokio::test]
async fn a_page_is_sent_as_a_hosted_page_in_the_readers_scheme_and_as_its_file_when_it_is_kept() {
  let shelf = Shelf::new().await;
  let page = shelf.scratch.count(
    "INSERT INTO searches (chat_id, query, started_depth, status, started_at, kind, author) VALUES (1, 'a page', 'basic', 'done', 1, 'page', 'bmo') RETURNING id",
    vec![],
  ).await;
  let path = shelf
    .root
    .path()
    .join("searches")
    .join(page.to_string())
    .join("attachments")
    .join("page.html");
  let html = "<!doctype html><html><body><p>x</p></body></html>";

  std::fs::create_dir_all(path.parent().unwrap()).unwrap();
  std::fs::write(&path, html).unwrap();

  let id = keep(&shelf.scratch, page, "page", None, &path, "text/html", "page.html").await;
  let (status, headers, body) = shelf
    .ask(&format!("/api/archive/{page}/attachments/{id}"), &[("cookie", "nuxt-color-mode=light")])
    .await;
  let served = String::from_utf8(body).unwrap();

  assert_eq!(status, StatusCode::OK);
  assert!(
    served.starts_with("<!doctype html><style data-archive-sheet>:root{--doc-paper:#f6f5f1"),
    "{served}"
  );
  assert!(served.ends_with("<html><body><p>x</p></body></html>"));
  assert!(
    headers["content-security-policy"]
      .to_str()
      .unwrap()
      .starts_with("sandbox allow-scripts")
  );
  assert_eq!(headers["vary"], "cookie");
  assert_eq!(
    shelf.library.hosted_page_file(page).await.unwrap().map(|file| file.name),
    Some("page.html".to_owned())
  );
  assert_eq!(shelf.library.hosted_page_file(shelf.first).await.unwrap(), None);

  let (_, _, kept) = shelf.ask(&format!("/api/archive/{page}/attachments/{id}?download=1"), &[]).await;

  assert_eq!(kept, html.as_bytes());
}

#[tokio::test]
async fn the_catalogue_of_kinds_is_served_in_its_order() {
  let shelf = Shelf::new().await;
  let (status, body) = shelf.json("/api/archive/kinds").await;

  assert_eq!(status, StatusCode::OK);
  assert_eq!(
    body["kinds"]
      .as_array()
      .unwrap()
      .iter()
      .map(|one| one["id"].clone())
      .collect::<Vec<_>>(),
    ["search", "import", "learning", "record", "page"].map(|id| json!(id))
  );
  assert_eq!(
    body["kinds"][4],
    json!({ "id": "page", "labelKey": "archive.kind.page", "summary": false, "hosted": true, "searched": false, "head": ["description"] })
  );
}

// ---------------------------------------------------------------- an address that has moved

#[test]
fn the_searches_list_and_one_search_land_on_the_archive_screen_the_number_kept() {
  assert_eq!(moved_screen("/searches").as_deref(), Some("/archive"));
  assert_eq!(moved_screen("/searches/").as_deref(), Some("/archive"));
  assert_eq!(moved_screen("/searches/57").as_deref(), Some("/archive/57"));
  assert_eq!(moved_screen("/searches?shared=1").as_deref(), Some("/archive?shared=1"));
  assert_eq!(moved_screen("/searches/57?from=room").as_deref(), Some("/archive/57?from=room"));
}

#[test]
fn the_retired_collection_screen_lands_on_the_archive_screen() {
  assert_eq!(moved_screen("/collected").as_deref(), Some("/archive"));
  assert_eq!(moved_screen("/collected?kind=audio").as_deref(), Some("/archive?kind=audio"));
}

#[test]
fn everything_else_stays_where_it_is_a_name_that_only_starts_the_same_way_included() {
  for path in ["/archive/57", "/searchesx", "/api/searches", "/share/abc", "/"] {
    assert_eq!(moved_screen(path), None, "{path}");
  }
}

#[tokio::test]
async fn a_moved_screen_is_sent_on_for_good_under_the_consoles_own_path() {
  let shelf = Shelf::new().await;

  for (path, target) in [
    ("/searches/57?from=room", "/bmo/archive/57?from=room"),
    ("/collected/", "/bmo/archive"),
    ("/searches", "/bmo/archive"),
  ] {
    let (status, headers, _) = shelf.ask(path, &[]).await;

    assert_eq!(
      (status, headers["location"].to_str().unwrap()),
      (StatusCode::MOVED_PERMANENTLY, target),
      "{path}"
    );
  }
}
