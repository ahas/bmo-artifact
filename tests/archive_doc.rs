//! `bmo archive doc` as a job's shell runs it (`docs/api.md`, "The archive verb"): a document filed with its link
//! printed, a page filed as it stands, one rewritten under its number keeping its link, and the words it refuses.

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
  let env = [
    ("BMO_ARTIFACT_SERVER__LISTEN", address.trim_start_matches("http://").to_owned()),
    ("BMO_ARTIFACT_SERVER__TOKEN", TOKEN.to_owned()),
    ("USER", "tester".to_owned()),
  ];
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
