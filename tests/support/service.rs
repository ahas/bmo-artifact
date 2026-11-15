//! The service as a caller reaches it, stood up for one test: its router over a scratch database and a file
//! root of the test's own, the web read through a stand-in reader whose pages and pictures the test names,
//! every call presenting the token.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Method, Request, StatusCode};
use bmo_artifact::plumbing::http::{Standing, router};
use bmo_artifact::reader::{PageMarkup, PageReader, Picture, Preview, Reading};
use bmo_artifact::settings::Settings;
use serde_json::Value;
use tower::ServiceExt;

use super::ROOT;
use super::accounts::ACCOUNTS_TOKEN;
use super::scratch::Scratch;

/// The token every call of the tests presents.
pub const TOKEN: &str = "service-token";
/// The console's token.
pub const CONSOLE_TOKEN: &str = "console-token";
/// The address the settings put share links under.
pub const SHARE_ADDRESS: &str = "https://documents.invalid/share";

/// A bundle's module, as a test's bundle folder holds it.
pub const MODULE: &[u8] = b"export default { routes: [] };\n";
/// A bundle's stylesheet.
pub const STYLESHEET: &[u8] = b".artifact-paper { color: red; }\n";
/// A bundle's words, each locale's under its code.
pub const WORDS: &[u8] = br#"{"en":{"artifact":{"title":{"screen":"Artifact"}}},"ko":{"artifact":{"title":{"screen":"Artifact"}}}}"#;

/// The bmo-ui release file the bundle's own install takes, as `client/package.json` names it.
pub fn vendored_release() -> PathBuf {
  let client = Path::new(ROOT).join("client");
  let manifest: Value = serde_json::from_str(
    &std::fs::read_to_string(client.join("package.json")).expect("the bundle's manifest"),
  )
  .expect("it reads");
  let spec = manifest["devDependencies"]["bmo-ui"]
    .as_str()
    .expect("bmo-ui is a development dependency");

  client.join(spec.strip_prefix("file:").expect("bmo-ui is taken from a file"))
}

/// The layer that release names, read from its own layer file.
pub fn release_layer() -> &'static str {
  static LAYER: OnceLock<String> = OnceLock::new();

  LAYER.get_or_init(|| {
    let read = Command::new("tar")
      .arg("-xzOf")
      .arg(vendored_release())
      .arg("package/shared/layer.json")
      .output()
      .expect("tar runs");
    let file: Value = serde_json::from_slice(&read.stdout).expect("the release's layer file");

    file["layer"].as_str().expect("a layer").to_owned()
  })
}

/// The file bmo-ui's bundle build writes beside a module, naming the layer the vendored release names.
pub fn layer_file() -> (&'static str, &'static [u8]) {
  static FILE: OnceLock<Vec<u8>> = OnceLock::new();

  (
    "layer.json",
    FILE.get_or_init(|| serde_json::json!({ "layer": release_layer() }).to_string().into_bytes()),
  )
}

/// A bundle folder's files as the build writes them: a module, a stylesheet, words and the layer file.
pub fn built_bundle() -> [(&'static str, &'static [u8]); 4] {
  [("bundle.js", MODULE), ("bundle.css", STYLESHEET), ("words.json", WORDS), layer_file()]
}

// The settings a service of a test stands on: its database, its file root, its console folder and its player
// folder, the account service where the test stands one up, and the tokens and share address every test names.
fn environment(
  database: String,
  files: &Path,
  console: &Path,
  player: &Path,
  accounts: Option<&str>,
) -> HashMap<String, String> {
  let mut environment = HashMap::from([
    ("BMO_ARTIFACT_DATABASE__URL".to_owned(), database),
    ("BMO_ARTIFACT_FILES__ROOT".to_owned(), files.display().to_string()),
    ("BMO_ARTIFACT_SERVER__TOKEN".to_owned(), TOKEN.to_owned()),
    ("BMO_ARTIFACT_CONSOLE__TOKEN".to_owned(), CONSOLE_TOKEN.to_owned()),
    ("BMO_ARTIFACT_CONSOLE__BUNDLE".to_owned(), console.display().to_string()),
    ("BMO_ARTIFACT_PLAYER__FOLDER".to_owned(), player.display().to_string()),
    ("BMO_ARTIFACT_SHARE__ADDRESS".to_owned(), SHARE_ADDRESS.to_owned()),
  ]);

  if let Some(address) = accounts {
    environment.insert("BMO_ARTIFACT_ACCOUNTS__ADDRESS".to_owned(), address.to_owned());
    environment.insert("BMO_ARTIFACT_ACCOUNTS__TOKEN".to_owned(), ACCOUNTS_TOKEN.to_owned());
  }

  environment
}

// A folder of the test's own holding these files, by name.
fn folder_of(files: &[(&str, &[u8])]) -> tempfile::TempDir {
  let folder = tempfile::tempdir().expect("a folder");

  for (name, bytes) in files {
    std::fs::write(folder.path().join(name), bytes).expect("a file");
  }

  folder
}

/// The web as the stand-in reader serves it: pages, pictures and previews by address, and every address
/// asked written down.
#[derive(Default)]
pub struct Web {
  pub pages: Mutex<HashMap<String, PageMarkup>>,
  pub pictures: Mutex<HashMap<String, Picture>>,
  pub previews: Mutex<HashMap<String, Preview>>,
  pub asked: Mutex<Vec<String>>,
}

impl Web {
  fn asked(&self, url: &str) {
    self.asked.lock().expect("the book").push(url.to_owned());
  }
}

impl PageReader for Web {
  fn page<'a>(&'a self, url: &'a str) -> Reading<'a, PageMarkup> {
    self.asked(url);

    let found = self.pages.lock().expect("the book").get(url).cloned();

    Box::pin(async move { found.ok_or_else(|| format!("{url} answered 404")) })
  }

  fn picture<'a>(&'a self, url: &'a str) -> Reading<'a, Picture> {
    self.asked(url);

    let found = self.pictures.lock().expect("the book").get(url).cloned();

    Box::pin(async move { found.ok_or_else(|| format!("{url} answered 404")) })
  }

  fn preview<'a>(&'a self, url: &'a str) -> Reading<'a, Preview> {
    self.asked(url);

    let found = self.previews.lock().expect("the book").get(url).cloned();

    Box::pin(async move { found.ok_or_else(|| format!("{url} answered 404")) })
  }
}

/// One answer: its status, its headers and its body.
pub struct Answered {
  pub status: StatusCode,
  pub headers: HeaderMap,
  pub bytes: Vec<u8>,
}

impl Answered {
  /// The body read as JSON, null where it is not.
  pub fn json(&self) -> Value {
    serde_json::from_slice(&self.bytes).unwrap_or(Value::Null)
  }

  /// One header's value.
  pub fn header(&self, name: &str) -> String {
    self
      .headers
      .get(name)
      .and_then(|value| value.to_str().ok())
      .unwrap_or_default()
      .to_owned()
  }
}

/// The service stood up for one test.
pub struct Service {
  pub scratch: Scratch,
  pub files: tempfile::TempDir,
  /// The folder the console bundle is read from.
  pub console: tempfile::TempDir,
  /// The folder the player is read from.
  pub player: tempfile::TempDir,
  pub web: Arc<Web>,
  pub standing: Arc<Standing>,
  /// The account service the settings name, where the test stands one up.
  accounts: Option<String>,
  router: Router,
}

impl Service {
  /// The service over a built bundle and an empty player folder.
  pub async fn new() -> Self {
    Self::with_folders(&built_bundle(), &[], None).await
  }

  /// The service over a built bundle, asking the account service at `accounts` the readers' units.
  pub async fn with_accounts(accounts: &str) -> Self {
    Self::with_folders(&built_bundle(), &[], Some(accounts)).await
  }

  /// The service over a console bundle folder holding these files, by name, and an empty player folder.
  pub async fn with_console(bundle: &[(&str, &[u8])]) -> Self {
    Self::with_folders(bundle, &[], None).await
  }

  /// The service over a built bundle and a player folder holding these files, by name.
  pub async fn with_player(player: &[(&str, &[u8])]) -> Self {
    Self::with_folders(&built_bundle(), player, None).await
  }

  async fn with_folders(
    bundle: &[(&str, &[u8])],
    player: &[(&str, &[u8])],
    accounts: Option<&str>,
  ) -> Self {
    let scratch = Scratch::new().await;
    let files = tempfile::tempdir().expect("a file root");
    let console = folder_of(bundle);
    let player = folder_of(player);
    let environment = environment(scratch.address(), files.path(), console.path(), player.path(), accounts);
    let settings = Settings::read(Path::new(ROOT), Some(environment)).expect("the settings read");
    let web = Arc::new(Web::default());
    let standing =
      Standing::with_reader(settings, Arc::clone(&web) as Arc<dyn PageReader>).expect("it stands");
    let router = router(standing.clone()).expect("the routes stand");

    Self {
      scratch,
      files,
      console,
      player,
      web,
      standing: Arc::new(standing),
      accounts: accounts.map(str::to_owned),
      router,
    }
  }

  /// The service stood up again over the same database and the same file root, as a stop and a start leave it,
  /// or a redeployment that keeps the store: its settings read afresh and its routes gathered again.
  pub fn restart(&mut self) {
    let settings = Settings::read(
      Path::new(ROOT),
      Some(environment(
        self.scratch.address(),
        self.files.path(),
        self.console.path(),
        self.player.path(),
        self.accounts.as_deref(),
      )),
    )
    .expect("the settings read");
    let standing =
      Standing::with_reader(settings, Arc::clone(&self.web) as Arc<dyn PageReader>).expect("it stands");

    self.router = router(standing.clone()).expect("the routes stand");
    self.standing = Arc::new(standing);
  }

  /// The service listening on a port of its own until the test ends; its address.
  pub async fn listening(&self) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a port");
    let address = format!("http://{}", listener.local_addr().expect("an address"));
    let router = self.router.clone();

    tokio::spawn(async move { axum::serve(listener, router).await });
    address
  }

  /// A call with the token, its body JSON where one is given.
  pub async fn call(&self, method: Method, uri: &str, body: Option<Value>) -> Answered {
    self.call_with(method, uri, body, &[]).await
  }

  /// A call with the token and the headers named.
  pub async fn call_with(
    &self,
    method: Method,
    uri: &str,
    body: Option<Value>,
    headers: &[(&str, &str)],
  ) -> Answered {
    self.call_by(Some(TOKEN), method, uri, body, headers).await
  }

  /// A call presenting `token`, or none, with the headers named.
  pub async fn call_by(
    &self,
    token: Option<&str>,
    method: Method,
    uri: &str,
    body: Option<Value>,
    headers: &[(&str, &str)],
  ) -> Answered {
    let mut request = Request::builder().method(method).uri(uri);

    if let Some(token) = token {
      request = request.header("authorization", format!("Bearer {token}"));
    }

    for (name, value) in headers {
      request = request.header(*name, *value);
    }

    let body = match body {
      Some(body) => {
        request = request.header("content-type", "application/json");
        Body::from(body.to_string())
      }
      None => Body::empty(),
    };
    let response = self
      .router
      .clone()
      .oneshot(request.body(body).expect("a request"))
      .await
      .expect("an answer");
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("the body reads").to_vec();

    Answered {
      status,
      headers,
      bytes,
    }
  }

  /// The first frame a stream sends, its comments passed over.
  pub async fn first_frame(&self, uri: &str, headers: &[(&str, &str)]) -> (StatusCode, String) {
    use futures_util::StreamExt;

    let mut request = Request::builder().uri(uri).header("authorization", format!("Bearer {TOKEN}"));

    for (name, value) in headers {
      request = request.header(*name, *value);
    }

    let response = self
      .router
      .clone()
      .oneshot(request.body(Body::empty()).expect("a request"))
      .await
      .expect("an answer");
    let status = response.status();
    let mut stream = response.into_body().into_data_stream();

    while let Some(Ok(chunk)) = stream.next().await {
      let text = String::from_utf8_lossy(&chunk).into_owned();

      if !text.trim().is_empty() && !text.starts_with(':') {
        return (status, text);
      }
    }

    (status, String::new())
  }

  /// Whether a stream opened at `uri` ends once the server's stop is raised, after its first frame.
  pub async fn stream_ends_on_stop(&self, uri: &str) -> bool {
    use futures_util::StreamExt;

    let request = Request::builder().uri(uri).header("authorization", format!("Bearer {TOKEN}"));
    let response = self
      .router
      .clone()
      .oneshot(request.body(Body::empty()).expect("a request"))
      .await
      .expect("an answer");
    let mut stream = response.into_body().into_data_stream();

    if stream.next().await.is_none() {
      return false;
    }

    self.standing.stop();

    while let Some(chunk) = stream.next().await {
      chunk.expect("a frame");
    }

    true
  }

  /// The kinds of the events kept, oldest first.
  pub async fn event_kinds(&self) -> Vec<String> {
    let page = self
      .call_with(Method::GET, "/events?limit=1000", None, &[("accept", "application/json")])
      .await;

    page.json()["items"]
      .as_array()
      .map(|items| items.iter().filter_map(|item| item["kind"].as_str().map(str::to_owned)).collect())
      .unwrap_or_default()
  }
}
