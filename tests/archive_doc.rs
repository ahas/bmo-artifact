//! `bmo archive doc` as a job's shell runs it (`docs/api.md`, "The archive verb"): a document filed with its link
//! printed, a page filed as it stands, one rewritten under its number keeping its link, the files it attaches, and
//! the words it refuses.

use crate::support;

use axum::http::Method;
use serde_json::Value;
use support::cli::{Ran, cli};
use support::service::{SHARE_ADDRESS, Service, TOKEN};

fn file(root: &tempfile::TempDir, name: &str, words: &str) -> String {
  let path = root.path().join(name);

  std::fs::write(&path, words).expect("the file is written");

  path.to_str().expect("a path").to_owned()
}

async fn run(address: &str, args: &[&str]) -> Ran {
  run_for(address, None, args).await
}

// The verb run in a supervisor's program, which holds the account it works for where one is given.
async fn run_for(address: &str, account: Option<&str>, args: &[&str]) -> Ran {
  let mut env = vec![
    ("BMO_ARTIFACT_SERVER__LISTEN", address.trim_start_matches("http://").to_owned()),
    ("BMO_ARTIFACT_SERVER__TOKEN", TOKEN.to_owned()),
    ("USER", "tester".to_owned()),
  ];

  env.extend(account.map(|account| ("BMO_OWNER_ACCOUNT", account.to_owned())));

  let args: Vec<String> = ["archive", "doc"]
    .iter()
    .map(|word| (*word).to_owned())
    .chain(args.iter().map(|word| (*word).to_owned()))
    .collect();

  tokio::task::spawn_blocking(move || {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();

    cli(&env, &words, None)
  })
  .await
  .expect("the command ran")
}

fn number(ran: &Ran) -> i64 {
  ran
    .out
    .lines()
    .next()
    .and_then(|line| line.strip_prefix("document "))
    .and_then(|rest| rest.split(':').next())
    .and_then(|id| id.parse().ok())
    .unwrap_or_else(|| panic!("no number in {:?}", ran.out))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_document_is_filed_its_link_printed_and_a_rewrite_keeps_the_link() {
  let service = Service::new().await;
  let address = service.listening().await;
  let root = tempfile::tempdir().expect("a folder");
  let body = file(&root, "note.md", "# Words\n\nThe first words.\n");
  let filed = run(&address, &["--title", "A note", "--body", &body, "--chat", "56"]).await;

  assert!(filed.ok, "{}", filed.err);

  let id = number(&filed);
  let link = filed.out.lines().nth(1).expect("the link").to_owned();

  assert!(link.starts_with(SHARE_ADDRESS), "{link}");

  let document: Value = service.call(Method::GET, &format!("/documents/{id}"), None).await.json();

  assert_eq!(document["kind"], "record");
  assert_eq!(document["scope"], "56");
  assert_eq!(document["actorName"], "tester");

  let again = file(&root, "again.md", "# Words\n\nOther words.\n");
  let id_word = id.to_string();
  let rewritten = run(&address, &["--replace", &id_word, "--body", &again]).await;

  assert!(rewritten.ok, "{}", rewritten.err);
  assert_eq!(number(&rewritten), id);
  assert_eq!(
    rewritten.out.lines().nth(1),
    Some(link.as_str()),
    "the same link serves the newest words"
  );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_supervisor_of_an_account_files_as_its_user_and_rewrites_only_what_it_owns() {
  let service = Service::new().await;
  let address = service.listening().await;
  let root = tempfile::tempdir().expect("a folder");
  let body = file(&root, "note.md", "# Words\n\nThe first words.\n");
  let filed = run_for(&address, Some("31"), &["--title", "Mine", "--body", &body]).await;

  assert!(filed.ok, "{}", filed.err);

  let id = number(&filed).to_string();
  let document: Value = service.call(Method::GET, &format!("/documents/{id}"), None).await.json();

  assert_eq!(document["owner"], "31");

  let again = file(&root, "again.md", "# Words\n\nOther words.\n");
  let other = run_for(&address, Some("32"), &["--replace", &id, "--body", &again]).await;

  assert_eq!(
    other.code,
    Some(1),
    "another account's supervisor reaches no such document: {}",
    other.out
  );

  let own = run_for(&address, Some("31"), &["--replace", &id, "--body", &again]).await;

  assert!(own.ok, "{}", own.err);

  let unowned = run(&address, &["--title", "Nobody's", "--body", &body]).await;
  let document: Value = service
    .call(Method::GET, &format!("/documents/{}", number(&unowned)), None)
    .await
    .json();

  assert_eq!(document["owner"], Value::Null);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_page_is_filed_as_it_stands_and_what_it_refuses_files_nothing() {
  let service = Service::new().await;
  let address = service.listening().await;
  let root = tempfile::tempdir().expect("a folder");
  let page = file(&root, "page.html", "<!doctype html><title>A page</title><p>Words</p>");
  let filed = run(&address, &["--title", "A page", "--html", &page]).await;

  assert!(filed.ok, "{}", filed.err);

  let document: Value =
    service.call(Method::GET, &format!("/documents/{}", number(&filed)), None).await.json();

  assert_eq!(document["kind"], "page");

  let both = run(&address, &["--title", "Both", "--html", &page, "--body", &page]).await;

  assert_eq!(both.code, Some(1), "{}", both.err);

  let untitled = run(&address, &["--html", &page]).await;

  assert_eq!(untitled.code, Some(1), "{}", untitled.err);

  let gone = run(&address, &["--title", "Gone", "--body", "/no/such/file.md"]).await;

  assert_eq!(gone.code, Some(1), "{}", gone.err);
}

fn picture(root: &tempfile::TempDir, name: &str) -> String {
  let path = root.path().join(name);

  image::RgbaImage::from_pixel(40, 30, image::Rgba([200, 40, 40, 255]))
    .save_with_format(&path, image::ImageFormat::Png)
    .expect("the picture is written");

  path.to_str().expect("a path").to_owned()
}

async fn kept(service: &Service, id: i64) -> Vec<(String, String)> {
  let page: Value = service.call(Method::GET, &format!("/documents/{id}/attachments"), None).await.json();

  page["items"]
    .as_array()
    .expect("a page of attachments")
    .iter()
    .map(|one| {
      (
        one["name"].as_str().unwrap_or_default().to_owned(),
        one["kind"].as_str().unwrap_or_default().to_owned(),
      )
    })
    .collect()
}

async fn filed(service: &Service) -> usize {
  let page: Value = service.call(Method::GET, "/documents", None).await.json();

  page["items"].as_array().map_or(0, Vec::len)
}

#[tokio::test(flavor = "multi_thread")]
async fn the_files_it_is_handed_are_attached_under_their_own_names_and_a_kept_name_is_never_replaced() {
  let service = Service::new().await;
  let address = service.listening().await;
  let root = tempfile::tempdir().expect("a folder");
  let body = file(&root, "report.md", "# Report\n\n![The plan](plan.png)\n");
  let plan = picture(&root, "plan.png");
  let notes = file(&root, "notes.md", "Notes.\n");
  let filed_one =
    run(&address, &["--title", "A report", "--body", &body, "--file", &plan, "--file", &notes]).await;

  assert!(filed_one.ok, "{}", filed_one.err);

  let id = number(&filed_one);
  let lines: Vec<&str> = filed_one.out.lines().collect();

  assert_eq!(lines.len(), 4, "{}", filed_one.out);
  assert!(lines[2].starts_with("attachment ") && lines[2].ends_with(": plan.png"), "{}", lines[2]);
  assert!(lines[3].ends_with(": notes.md"), "{}", lines[3]);
  assert_eq!(
    kept(&service, id).await,
    [("plan.png".to_owned(), "image".to_owned()), ("notes.md".to_owned(), "text".to_owned())]
  );

  let id_word = id.to_string();
  let again = file(&root, "again.md", "# Report\n\n![The plan](plan.png)\n\nOther words.\n");
  let kept_name = run(
    &address,
    &["--replace", &id_word, "--title", "Renamed", "--body", &again, "--file", &plan],
  )
  .await;

  assert_eq!(kept_name.code, Some(1), "{}", kept_name.out);
  assert!(kept_name.err.contains("already keeps a file named plan.png"), "{}", kept_name.err);

  let standing: Value = service.call(Method::GET, &format!("/documents/{id}"), None).await.json();

  assert_eq!(standing["title"], "A report", "nothing is rewritten for a refusal");

  let second = picture(&root, "plan-2.png");
  let redrawn = file(&root, "redrawn.md", "# Report\n\n![The plan, again](plan-2.png)\n");
  let rewritten = run(&address, &["--replace", &id_word, "--body", &redrawn, "--file", &second]).await;

  assert!(rewritten.ok, "{}", rewritten.err);
  assert!(
    rewritten.out.lines().nth(2).is_some_and(|line| line.ends_with(": plan-2.png")),
    "{}",
    rewritten.out
  );
  assert_eq!(kept(&service, id).await.len(), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_picture_the_body_does_not_draw_is_refused_and_nothing_is_filed() {
  let service = Service::new().await;
  let address = service.listening().await;
  let root = tempfile::tempdir().expect("a folder");
  let body = file(&root, "report.md", "# Report\n\nThe plan is attached: [plan.png](plan.png).\n");
  let plan = picture(&root, "plan.png");
  let notes = file(&root, "notes.md", "Notes.\n");
  let undrawn =
    run(&address, &["--title", "A report", "--body", &body, "--file", &plan, "--file", &notes]).await;

  assert_eq!(undrawn.code, Some(1), "{}", undrawn.out);
  assert!(undrawn.err.contains("draws no picture named plan.png"), "{}", undrawn.err);
  assert!(undrawn.err.contains("![caption](name)"), "{}", undrawn.err);
  assert_eq!(filed(&service).await, 0, "nothing is filed for a refusal");

  // A file that is no picture is attached whether or not the body names it, and a page is held to nothing here.
  let plain = run(&address, &["--title", "A report", "--body", &body, "--file", &notes]).await;

  assert!(plain.ok, "{}", plain.err);

  let page = file(&root, "page.html", "<!doctype html><title>A page</title><p>Words.</p>");
  let hosted = run(&address, &["--title", "A page", "--html", &page, "--file", &plan]).await;

  assert!(hosted.ok, "{}", hosted.err);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_file_that_cannot_be_attached_files_nothing() {
  let service = Service::new().await;
  let address = service.listening().await;
  let root = tempfile::tempdir().expect("a folder");
  let body = file(&root, "note.md", "# Words\n");
  let one = file(&root, "one.txt", "One.\n");
  let other = tempfile::tempdir().expect("another folder");
  let same_name = file(&other, "one.txt", "Another.\n");
  let missing =
    run(&address, &["--title", "Missing", "--body", &body, "--file", "/no/such/picture.png"]).await;

  assert_eq!(missing.code, Some(1), "{}", missing.err);

  let twice =
    run(&address, &["--title", "Twice", "--body", &body, "--file", &one, "--file", &same_name]).await;

  assert_eq!(twice.code, Some(1), "{}", twice.err);
  assert_eq!(filed(&service).await, 0, "nothing is filed for a refusal");
}
