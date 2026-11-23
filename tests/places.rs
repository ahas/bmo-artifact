//! Places (`docs/artifact.md`, "Places", "The setting", "The local place" and "The SharePoint place"; `docs/api.md`,
//! "Storage places"): the local place written, read in a range and let go of; a storage value judged by the kind it
//! names; a document of an owner whose chain names a SharePoint library filed, read, attached to in one call and
//! through an upload session, read in a range and let go of there, through stand-ins of the account service, the page
//! token road and Microsoft Graph; a filing refused whole where no token is lent; and a link's storage value
//! judged and written on the chain in the caller's name.

use crate::support;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{HeaderMap, Method, StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{any, get, post, put};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use bmo_artifact::places::local::LocalPlace;
use bmo_artifact::places::{Place, judged, range_of};
use serde_json::{Value, json};
use support::accounts::ACCOUNTS_TOKEN;
use support::service::{CONSOLE_TOKEN, Service};

/// The owner of the documents filed, and the login their folder is laid out by.
const OWNER: &str = "7";
const LOGIN: &str = "kim";
/// The company the owner stands in, and the unit under it they stand in.
const COMPANY: i64 = 10;
const TEAM: i64 = 11;
/// The caller token the service presents on the page token road.
const CALLER: &str = "artifact-caller";

fn library_value() -> Value {
  json!({ "place": "sharepoint", "site": "site-1", "drive": "drive-1", "folder": "04. BMO_ARTIFACT/{login}/{document}" })
}

/// What the stand-in account service holds of the storage setting, and what it was asked to write.
#[derive(Default)]
struct Chain {
  declared: Mutex<Vec<Value>>,
  value: Mutex<Option<Value>>,
  locked: Mutex<bool>,
  /// Each write: the link, the owner key, the unit named as the declaring one, and the value.
  writes: Mutex<Vec<(String, String, String, Value)>>,
}

fn admitted(headers: &HeaderMap) -> bool {
  headers.get(header::AUTHORIZATION).and_then(|value| value.to_str().ok())
    == Some(format!("Bearer {ACCOUNTS_TOKEN}").as_str())
}

fn refused(status: StatusCode, code: &str) -> Response {
  (status, Json(json!({ "code": code, "error": format!("refused: {code}") }))).into_response()
}

fn said(headers: &HeaderMap, name: &str) -> String {
  headers.get(name).and_then(|value| value.to_str().ok()).unwrap_or_default().to_owned()
}

async fn user_unit(headers: HeaderMap, Path(user): Path<String>) -> Response {
  if !admitted(&headers) || user != OWNER {
    return refused(StatusCode::NOT_FOUND, "not-found");
  }

  let unit = |id: i64, name: &str| json!({ "id": id, "name": name });

  Json(json!({
    "user": user,
    "unit": unit(TEAM, "Team"),
    "company": unit(COMPANY, "Company"),
    "path": [unit(COMPANY, "Company"), unit(TEAM, "Team")],
  }))
  .into_response()
}

async fn user(headers: HeaderMap, Path(user): Path<String>) -> Response {
  if !admitted(&headers) || user != OWNER {
    return refused(StatusCode::NOT_FOUND, "not-found");
  }

  Json(json!({ "user": user, "name": LOGIN, "fullName": "Kim", "administrator": false, "senders": [] }))
    .into_response()
}

async fn declare(State(chain): State<Arc<Chain>>, Json(kind): Json<Value>) -> Response {
  chain.declared.lock().expect("the kinds").push(kind.clone());

  Json(kind).into_response()
}

async fn resolve(
  State(chain): State<Arc<Chain>>,
  Path(key): Path<String>,
  Query(asked): Query<HashMap<String, String>>,
) -> Response {
  if chain.declared.lock().expect("the kinds").is_empty() {
    return refused(StatusCode::NOT_FOUND, "no-such-kind");
  }

  let person = asked.get("person").cloned().unwrap_or_default();

  if person != OWNER {
    return refused(StatusCode::NOT_FOUND, "not-found");
  }
  let mut answer = json!({
    "kind": key,
    "subject": format!("person:{person}"),
    "layers": [],
    "path": ["platform", format!("unit:{COMPANY}"), format!("unit:{TEAM}"), format!("person:{person}")],
  });

  if let Some(value) = chain.value.lock().expect("the value").clone() {
    answer["value"] = value;
  }

  Json(answer).into_response()
}

async fn write_value(
  State(chain): State<Arc<Chain>>,
  Path((key, link)): Path<(String, String)>,
  headers: HeaderMap,
  Json(asked): Json<Value>,
) -> Response {
  if *chain.locked.lock().expect("the lock") {
    return refused(StatusCode::CONFLICT, "locked");
  }

  chain.writes.lock().expect("the writes").push((
    link.clone(),
    said(&headers, "owner-key"),
    said(&headers, "declared-by"),
    asked["value"].clone(),
  ));

  Json(json!({ "kind": key, "link": link, "value": asked["value"], "delegation": "inherit", "rules": [], "writable": true }))
    .into_response()
}

async fn chain_listening(chain: &Arc<Chain>) -> String {
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a port");
  let address = format!("http://{}", listener.local_addr().expect("an address"));
  let router = Router::new()
    .route("/users/{user}/unit", get(user_unit))
    .route("/users/{user}", get(user))
    .route("/chain/kinds/{key}", put(declare))
    .route("/chain/kinds/{key}/resolve", get(resolve))
    .route("/chain/kinds/{key}/links/{link}/value", put(write_value))
    .with_state(Arc::clone(chain));

  tokio::spawn(async move { axum::serve(listener, router).await });
  address
}

/// What the stand-in of Microsoft Graph and the page token road holds: the library's files by their path in it,
/// the upload sessions open, the tokens lent and the parts uploaded.
#[derive(Default)]
struct Microsoft {
  files: Mutex<HashMap<String, Vec<u8>>>,
  sessions: Mutex<HashMap<String, (String, Vec<u8>)>>,
  lent: Mutex<Vec<i64>>,
  parts: Mutex<usize>,
  down: Mutex<bool>,
  /// Where the stand-in listens, which an upload session's address stands under.
  address: Mutex<String>,
}

async fn page_token(
  State(microsoft): State<Arc<Microsoft>>,
  headers: HeaderMap,
  Json(asked): Json<Value>,
) -> Response {
  if said(&headers, "authorization") != format!("Bearer {CALLER}") {
    return refused(StatusCode::UNAUTHORIZED, "unauthorized");
  }

  if *microsoft.down.lock().expect("the road") {
    return refused(StatusCode::SERVICE_UNAVAILABLE, "not-ready");
  }

  let account = asked["account"].as_i64().expect("an account");
  let scopes: Vec<&str> =
    asked["scopes"].as_array().expect("scopes").iter().filter_map(Value::as_str).collect();

  assert!(scopes.contains(&"Files.ReadWrite.All"), "{scopes:?}");
  microsoft.lent.lock().expect("the tokens").push(account);

  Json(json!({
    "token": format!("graph-{account}"),
    "scopes": "https://graph.microsoft.com/Files.ReadWrite.All",
    "expiresAt": chrono::Utc::now().timestamp_millis() + 3_600_000,
    "node": "a-node",
  }))
  .into_response()
}

// The bytes of `bytes` the Range header asks for, as a library answers them.
fn ranged(bytes: &[u8], headers: &HeaderMap) -> Response {
  let total = bytes.len() as u64;

  match range_of(headers.get(header::RANGE).and_then(|value| value.to_str().ok()), total) {
    Some((from, to)) => (
      StatusCode::PARTIAL_CONTENT,
      [(header::CONTENT_RANGE, format!("bytes {from}-{to}/{total}"))],
      bytes[from as usize..=to as usize].to_vec(),
    )
      .into_response(),
    None => (StatusCode::OK, bytes.to_vec()).into_response(),
  }
}

async fn item(
  State(microsoft): State<Arc<Microsoft>>,
  method: Method,
  Path((site, drive, rest)): Path<(String, String, String)>,
  headers: HeaderMap,
  body: Bytes,
) -> Response {
  if site != "site-1" || drive != "drive-1" || !said(&headers, "authorization").starts_with("Bearer graph-")
  {
    return refused(StatusCode::UNAUTHORIZED, "unauthorized");
  }

  let path = rest.strip_prefix("root:/").expect("a path from the library's root");
  let mut files = microsoft.files.lock().expect("the files");

  if let Some(path) = path.strip_suffix(":/content") {
    return match method {
      Method::PUT => {
        files.insert(path.to_owned(), body.to_vec());
        (StatusCode::CREATED, Json(json!({ "name": path }))).into_response()
      }
      // The library sends a read on to the file's own download address.
      _ => match files.contains_key(path) {
        true => (StatusCode::FOUND, [(header::LOCATION, format!("/download/{path}"))]).into_response(),
        false => refused(StatusCode::NOT_FOUND, "itemNotFound"),
      },
    };
  }

  if let Some(path) = path.strip_suffix(":/createUploadSession") {
    drop(files);

    let mut sessions = microsoft.sessions.lock().expect("the sessions");
    let id = format!("session-{}", sessions.len());

    sessions.insert(id.clone(), (path.to_owned(), Vec::new()));

    let address = microsoft.address.lock().expect("the address").clone();

    return Json(json!({ "uploadUrl": format!("{address}/upload/{id}") })).into_response();
  }

  match method {
    Method::DELETE => {
      let before = files.len();

      files.retain(|kept, _| kept != path && !kept.starts_with(&format!("{path}/")));

      if files.len() == before {
        refused(StatusCode::NOT_FOUND, "itemNotFound")
      } else {
        StatusCode::NO_CONTENT.into_response()
      }
    }
    _ => match files.get(path) {
      Some(bytes) => {
        Json(json!({ "size": bytes.len(), "eTag": format!("\"{}\"", bytes.len()), "file": {} }))
          .into_response()
      }
      None => refused(StatusCode::NOT_FOUND, "itemNotFound"),
    },
  }
}

async fn download(
  State(microsoft): State<Arc<Microsoft>>,
  Path(path): Path<String>,
  headers: HeaderMap,
) -> Response {
  match microsoft.files.lock().expect("the files").get(&path) {
    Some(bytes) => ranged(bytes, &headers),
    None => refused(StatusCode::NOT_FOUND, "itemNotFound"),
  }
}

async fn upload(
  State(microsoft): State<Arc<Microsoft>>,
  Path(id): Path<String>,
  headers: HeaderMap,
  body: Bytes,
) -> Response {
  assert!(said(&headers, "authorization").is_empty(), "a part goes with no token");

  let range = said(&headers, "content-range");
  let total: usize = range.rsplit('/').next().and_then(|total| total.parse().ok()).expect("a total");
  let mut sessions = microsoft.sessions.lock().expect("the sessions");
  let (path, bytes) = sessions.get_mut(&id).expect("an open session");

  *microsoft.parts.lock().expect("the parts") += 1;
  bytes.extend_from_slice(&body);

  if bytes.len() < total {
    return (
      StatusCode::ACCEPTED,
      Json(json!({ "nextExpectedRanges": [format!("{}-", bytes.len())] })),
    )
      .into_response();
  }

  microsoft.files.lock().expect("the files").insert(path.clone(), bytes.clone());

  (StatusCode::CREATED, Json(json!({ "name": path }))).into_response()
}

async fn microsoft_listening(microsoft: &Arc<Microsoft>) -> String {
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a port");
  let address = format!("http://{}", listener.local_addr().expect("an address"));
  let router = Router::new()
    .route("/page-tokens", post(page_token))
    .route("/sites/{site}/drives/{drive}/{*rest}", any(item))
    .route("/download/{*path}", get(download))
    .route("/upload/{id}", put(upload))
    .layer(DefaultBodyLimit::disable())
    .with_state(Arc::clone(microsoft));

  address.clone_into(&mut microsoft.address.lock().expect("the address"));
  tokio::spawn(async move { axum::serve(listener, router).await });
  address
}

// The service over the stand-ins, its chain naming the library for the owner.
async fn over_a_library() -> (Service, Arc<Chain>, Arc<Microsoft>) {
  let chain = Arc::new(Chain::default());

  *chain.value.lock().expect("the value") = Some(library_value());

  let accounts = chain_listening(&chain).await;
  let microsoft = Arc::new(Microsoft::default());
  let graph = microsoft_listening(&microsoft).await;
  let service = Service::with_settings(
    &accounts,
    &[
      ("BMO_ARTIFACT_SHAREPOINT__GRAPH", graph.clone()),
      ("BMO_ARTIFACT_SHAREPOINT__TOKENS", graph),
      ("BMO_ARTIFACT_SHAREPOINT__TOKEN", CALLER.to_owned()),
    ],
  )
  .await;

  (service, chain, microsoft)
}

fn as_owner() -> [(&'static str, &'static str); 1] {
  [("owner-key", OWNER)]
}

#[tokio::test]
async fn the_local_place_writes_reads_a_range_and_lets_go() {
  let root = tempfile::tempdir().expect("a root");
  let place = LocalPlace::new(root.path());

  place.write("searches/1/words.txt", b"hello world".to_vec()).await.expect("written");

  let kept = place.exists("searches/1/words.txt").await.expect("asked").expect("it stands");

  assert_eq!(kept.bytes, 11);

  let part = place.read("searches/1/words.txt", Some("bytes=6-10")).await.expect("read");

  assert_eq!((part.total, part.span), (11, Some((6, 10))));
  assert_eq!(part.whole().await.expect("the bytes"), b"world");

  let whole = place.read("searches/1/words.txt", Some("bytes=40-50")).await.expect("read");

  assert_eq!(whole.span, None, "a range the file does not hold reads the whole file");
  assert_eq!(whole.whole().await.expect("the bytes"), b"hello world");

  // A path kept whole before places were named reads where it stands.
  let kept_whole = root.path().join("searches/1/words.txt").display().to_string();

  assert_eq!(
    place.read(&kept_whole, None).await.expect("read").whole().await.expect("bytes"),
    b"hello world"
  );

  place.delete("searches/1").await.expect("let go");
  assert_eq!(place.exists("searches/1/words.txt").await.expect("asked"), None);
  assert!(place.read("searches/1/words.txt", None).await.is_err());
  assert!(place.read("../outside.txt", None).await.is_err(), "nothing is read outside the root");
}

#[test]
fn a_storage_value_is_judged_by_the_kind_it_names() {
  assert_eq!(judged(&json!({ "place": "local" })).expect("judged"), json!({ "place": "local" }));
  assert_eq!(
    judged(&json!({ "place": "sharepoint", "site": " site-1 ", "drive": "drive-1", "folder": "/04. BMO_ARTIFACT/{login}/{document}/", "account": 3 }))
      .expect("judged"),
    json!({ "place": "sharepoint", "site": "site-1", "drive": "drive-1", "folder": "04. BMO_ARTIFACT/{login}/{document}", "account": 3 })
  );

  for refused in [
    json!("local"),
    json!({}),
    json!({ "place": "tape" }),
    json!({ "place": "local", "root": "/elsewhere" }),
    json!({ "place": "sharepoint", "drive": "drive-1", "folder": "{document}" }),
    json!({ "place": "sharepoint", "site": "site-1", "drive": "a/b", "folder": "{document}" }),
    json!({ "place": "sharepoint", "site": "site-1", "drive": "drive-1", "folder": "04. BMO_ARTIFACT/{login}" }),
    json!({ "place": "sharepoint", "site": "site-1", "drive": "drive-1", "folder": "../{document}" }),
    json!({ "place": "sharepoint", "site": "site-1", "drive": "drive-1", "folder": "{document}", "account": "me" }),
  ] {
    assert!(judged(&refused).is_err(), "{refused} is refused");
  }
}

#[tokio::test]
async fn a_document_of_an_owner_whose_chain_names_a_library_is_kept_and_read_there() {
  if support::is_skipped("a_document_of_an_owner_whose_chain_names_a_library_is_kept_and_read_there") {
    return;
  }

  let (service, chain, microsoft) = over_a_library().await;
  let made = service
    .call_with(
      Method::POST,
      "/documents",
      Some(json!({ "kind": "record", "title": "Kept away", "body": "Words kept in the library.", "actorName": "tester", "labels": {} })),
      &as_owner(),
    )
    .await;

  assert_eq!(made.status, StatusCode::CREATED, "{}", made.json());

  let id = made.json()["id"].as_i64().expect("an id");
  let folder = format!("04. BMO_ARTIFACT/{LOGIN}/{id}");

  assert_eq!(
    chain.declared.lock().expect("the kinds")[0]["key"],
    "artifact.storage",
    "declared where it stood undeclared"
  );
  assert_eq!(
    microsoft
      .files
      .lock()
      .expect("the files")
      .get(&format!("{folder}/document.md"))
      .map(Vec::as_slice),
    Some(b"Words kept in the library.".as_slice())
  );
  assert!(
    !service.files.path().join(format!("searches/{id}")).exists(),
    "nothing is left on this machine"
  );
  assert_eq!(
    service
      .scratch
      .count(
        r#"SELECT COUNT(*) FROM "searches" WHERE "id" = $1 AND "company" = $2 AND "storage"->>'place' = 'sharepoint'
             AND "document" = $3"#,
        vec![id.into(), COMPANY.into(), format!("sharepoint:site-1/drive-1/{folder}/document.md").into()],
      )
      .await,
    1
  );

  let content = service
    .call_with(Method::GET, &format!("/documents/{id}/content"), None, &as_owner())
    .await;

  assert_eq!(content.status, StatusCode::OK);
  assert_eq!(content.bytes, b"Words kept in the library.");

  // A file over a single write's ceiling goes through an upload session, in parts.
  let big: Vec<u8> = (0..5 * 1024 * 1024).map(|at| (at % 251) as u8).collect();
  let attached = service
    .call_with(
      Method::POST,
      &format!("/documents/{id}/attachments"),
      Some(json!({ "name": "big.bin", "mediaType": "application/octet-stream", "content": STANDARD.encode(&big) })),
      &as_owner(),
    )
    .await;

  assert_eq!(attached.status, StatusCode::CREATED, "{}", attached.json());
  assert!(*microsoft.parts.lock().expect("the parts") >= 2);

  let attachment = attached.json()["id"].as_i64().expect("an attachment");
  let part = service
    .call_with(
      Method::GET,
      &format!("/documents/{id}/attachments/{attachment}"),
      None,
      &[("owner-key", OWNER), ("range", "bytes=10-19")],
    )
    .await;

  assert_eq!(part.status, StatusCode::PARTIAL_CONTENT);
  assert_eq!(part.header("content-range"), format!("bytes 10-19/{}", big.len()));
  assert_eq!(part.bytes, big[10..20]);

  let small = service
    .call_with(
      Method::POST,
      &format!("/documents/{id}/attachments"),
      Some(json!({ "name": "note.txt", "mediaType": "text/plain", "content": STANDARD.encode("a note") })),
      &as_owner(),
    )
    .await;

  assert_eq!(small.status, StatusCode::CREATED);
  assert!(
    microsoft
      .files
      .lock()
      .expect("the files")
      .keys()
      .any(|path| path.starts_with(&format!("{folder}/attachments/")) && path.ends_with("note.txt"))
  );
  assert_eq!(
    *microsoft.lent.lock().expect("the tokens"),
    vec![7],
    "the owner's token is borrowed once and kept"
  );

  let gone = service.call_with(Method::DELETE, &format!("/documents/{id}"), None, &as_owner()).await;

  assert_eq!(gone.status, StatusCode::NO_CONTENT);
  assert!(
    microsoft
      .files
      .lock()
      .expect("the files")
      .keys()
      .all(|path| !path.starts_with(&format!("{folder}/")))
  );
}

#[tokio::test]
async fn a_filing_no_token_is_lent_for_leaves_nothing() {
  if support::is_skipped("a_filing_no_token_is_lent_for_leaves_nothing") {
    return;
  }

  let (service, _chain, microsoft) = over_a_library().await;

  *microsoft.down.lock().expect("the road") = true;

  let made = service
    .call_with(
      Method::POST,
      "/documents",
      Some(json!({ "kind": "record", "title": "Nowhere", "body": "Words.", "actorName": "tester", "labels": {} })),
      &[("owner-key", "8")],
    )
    .await;

  // Another account stands in no unit the stand-in knows, so its document stays on this machine.
  assert_eq!(made.status, StatusCode::CREATED, "{}", made.json());

  let refused = service
    .call_with(
      Method::POST,
      "/documents",
      Some(json!({ "kind": "record", "title": "Nowhere", "body": "Words.", "actorName": "tester", "labels": {} })),
      &as_owner(),
    )
    .await;

  assert_eq!(refused.status, StatusCode::BAD_GATEWAY, "{}", refused.json());
  assert_eq!(
    service
      .scratch
      .count(r#"SELECT COUNT(*) FROM "searches" WHERE "owner" = $1"#, vec![OWNER.into()])
      .await,
    0
  );
}

#[tokio::test]
async fn a_links_storage_value_is_judged_and_written_in_the_callers_name() {
  if support::is_skipped("a_links_storage_value_is_judged_and_written_in_the_callers_name") {
    return;
  }

  let (service, chain, _microsoft) = over_a_library().await;
  let places = service.call(Method::GET, "/storage-places", None).await;
  let names: Vec<String> = places.json()["places"]
    .as_array()
    .expect("places")
    .iter()
    .filter_map(|place| place["name"].as_str().map(str::to_owned))
    .collect();

  assert_eq!(names, ["local", "sharepoint"]);

  let value = json!({ "place": "sharepoint", "site": "site-1 ", "drive": "drive-1", "folder": "04. BMO_ARTIFACT/{login}/{document}" });
  let written = service
    .call_by(
      Some(CONSOLE_TOKEN),
      Method::PUT,
      &format!("/storage/unit:{COMPANY}"),
      Some(json!({ "value": value })),
      &[("owner-role", "member"), ("owner-key", OWNER)],
    )
    .await;

  assert_eq!(written.status, StatusCode::OK, "{}", written.json());
  assert_eq!(written.json()["value"]["site"], "site-1");
  assert_eq!(
    chain.writes.lock().expect("the writes")[0],
    (format!("unit:{COMPANY}"), OWNER.to_owned(), "artifact".to_owned(), library_value())
  );

  let bad = service
    .call_with(
      Method::PUT,
      &format!("/storage/unit:{COMPANY}"),
      Some(json!({ "value": { "place": "sharepoint", "site": "site-1", "drive": "drive-1", "folder": "flat" } })),
      &as_owner(),
    )
    .await;

  assert_eq!(bad.status, StatusCode::BAD_REQUEST);
  assert_eq!(chain.writes.lock().expect("the writes").len(), 1, "nothing is written for a refusal");

  let nobody = service.call(Method::PUT, "/storage/platform", Some(json!({ "value": null }))).await;

  assert_eq!(nobody.status, StatusCode::FORBIDDEN);

  *chain.locked.lock().expect("the lock") = true;

  let locked = service
    .call_with(
      Method::PUT,
      &format!("/storage/unit:{TEAM}"),
      Some(json!({ "value": null })),
      &as_owner(),
    )
    .await;

  assert_eq!(locked.status, StatusCode::CONFLICT);
  assert_eq!(locked.json()["code"], "locked");
}
