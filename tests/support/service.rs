//! The service as a caller reaches it, stood up for one test: its router over a scratch database and a file
//! root of the test's own, the web read through a stand-in reader whose pages and pictures the test names,
//! every call presenting the token.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Method, Request, StatusCode};
use bmo_artifact::plumbing::http::{Standing, router};
use bmo_artifact::reader::{PageMarkup, PageReader, Picture, Preview, Reading};
use bmo_artifact::settings::Settings;
use serde_json::Value;
use tower::ServiceExt;

use super::ROOT;
use super::scratch::Scratch;

/// The token every call of the tests presents.
pub const TOKEN: &str = "service-token";
/// The console's token.
pub const CONSOLE_TOKEN: &str = "console-token";
/// The address the settings put share links under.
pub const SHARE_ADDRESS: &str = "https://documents.invalid/share";

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
  pub web: Arc<Web>,
  pub standing: Arc<Standing>,
  router: Router,
}

impl Service {
  pub async fn new() -> Self {
    Self::with_console(&[]).await
  }

  /// The service over a console bundle folder holding these files, by name.
  pub async fn with_console(bundle: &[(&str, &[u8])]) -> Self {
    let scratch = Scratch::new().await;
    let files = tempfile::tempdir().expect("a file root");
    let console = tempfile::tempdir().expect("a console bundle folder");

    for (name, bytes) in bundle {
      std::fs::write(console.path().join(name), bytes).expect("a bundle file");
    }

    let environment = HashMap::from([
      ("BMO_ARTIFACT_DATABASE__URL".to_owned(), scratch.address()),
      ("BMO_ARTIFACT_FILES__ROOT".to_owned(), files.path().display().to_string()),
      ("BMO_ARTIFACT_SERVER__TOKEN".to_owned(), TOKEN.to_owned()),
      ("BMO_ARTIFACT_CONSOLE__TOKEN".to_owned(), CONSOLE_TOKEN.to_owned()),
      ("BMO_ARTIFACT_CONSOLE__BUNDLE".to_owned(), console.path().display().to_string()),
      ("BMO_ARTIFACT_SHARE__ADDRESS".to_owned(), SHARE_ADDRESS.to_owned()),
    ]);
    let settings = Settings::read(Path::new(ROOT), Some(environment)).expect("the settings read");
    let web = Arc::new(Web::default());
    let standing =
      Standing::with_reader(settings, Arc::clone(&web) as Arc<dyn PageReader>).expect("it stands");
    let router = router(standing.clone()).expect("the routes stand");

    Self {
      scratch,
      files,
      console,
      web,
      standing: Arc::new(standing),
      router,
    }
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
