//! The document read API (`docs/api.md`, "Documents (HTTP)"): the list, one document, its file, its
//! attachments, the previews and pictures of the places its text points at, the catalogue of kinds, and the
//! screens that moved to `/archive`. A ref names a place in the document's record; the address behind it is
//! read from the record, never from the request.

use std::collections::HashMap;

use axum::body::Body;
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::response::Response;
use serde_json::json;

use super::files::{PRIVATE_DAY, Sending, attachment};
use crate::documents::read::{
  Asked, PAGE_MOST, PAGE_SIZE, PHRASE_MOST, RefPicture, kinds_view, listed_kind,
};
use crate::helpers::js::parse_int;
use crate::now_ms;
use crate::plumbing::http::{Call, Refused, Route, answer};

fn bad(code: &str, error: &str) -> Refused {
  Refused::new(StatusCode::BAD_REQUEST, code, error)
}

fn absent(code: &str, error: &str) -> Refused {
  Refused::new(StatusCode::NOT_FOUND, code, error)
}

// A number a road names, a whole number from 1.
fn numbered(params: &HashMap<String, String>, name: &str, code: &str, error: &str) -> Result<i64, Refused> {
  params
    .get(name)
    .and_then(|value| value.parse::<i64>().ok())
    .filter(|number| *number >= 1)
    .ok_or_else(|| bad(code, error))
}

fn document_id(params: &HashMap<String, String>) -> Result<i64, Refused> {
  numbered(params, "id", "bad_id", "the document's number is not a whole number from 1")
}

// The moment a reading is made at, in the epoch seconds the archive keeps.
fn now_seconds() -> i64 {
  now_ms() / 1000
}

/// The cursor a page carries on from: the number of the row the caller already has.
pub fn cursor(value: Option<&str>) -> Result<Option<i64>, Refused> {
  match value {
    None | Some("") => Ok(None),
    Some(value) => value
      .parse::<i64>()
      .map(Some)
      .map_err(|_| bad("bad_cursor", "the cursor is not a whole number")),
  }
}

/// How many documents a page is asked for: a whole number from 1, held to the most a page holds, and the
/// page's own size otherwise.
pub fn page_limit(value: Option<&str>) -> i64 {
  match value.and_then(parse_int) {
    Some(limit) if limit > 0 => limit.min(PAGE_MOST),
    _ => PAGE_SIZE,
  }
}

/// What a list is asked for by its address.
pub fn list_asked(query: impl Fn(&str) -> Option<String>, now: i64) -> Result<Asked, Refused> {
  Ok(Asked {
    limit: page_limit(query("limit").as_deref()),
    before: cursor(query("before").as_deref())?,
    shared: query("shared").as_deref() == Some("1"),
    kind: listed_kind(query("kind").as_deref()),
    phrase: query("q").map(|phrase| phrase.chars().take(PHRASE_MOST).collect()),
    now,
  })
}

/// `GET /api/archive`: the documents, newest first, a page at a time.
pub struct ListDocuments;

impl Route for ListDocuments {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/archive";

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let asked = list_asked(|name| call.query(name), now_seconds())?;
    let page = call.standing().library().await?.list(&asked).await?;

    Ok(answer(StatusCode::OK, json!(page)))
  }
}

crate::route!(ListDocuments);

/// `GET /api/archive/kinds`: the catalogue of kinds, each with what it shows.
pub struct ArchiveKinds;

impl Route for ArchiveKinds {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/archive/kinds";

  async fn answer(&self, _call: Call) -> Result<Response, Refused> {
    Ok(answer(StatusCode::OK, json!({ "kinds": kinds_view() })))
  }
}

crate::route!(ArchiveKinds);

/// `GET /api/archive/{id}`: one document read whole.
pub struct ReadDocument;

impl Route for ReadDocument {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/archive/{id}";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = document_id(&call.path_params().await?)?;
    let detail = call.standing().library().await?.detail(id, now_seconds()).await?;

    detail
      .map(|detail| answer(StatusCode::OK, json!(detail)))
      .ok_or_else(|| absent("document_not_found", "no such document"))
  }
}

crate::route!(ReadDocument);

fn with_headers(mut response: Response, headers: &[(header::HeaderName, String)]) -> Response {
  for (name, value) in headers {
    if let Ok(value) = HeaderValue::from_str(value) {
      response.headers_mut().insert(name.clone(), value);
    }
  }

  response
}

/// `GET /api/archive/{id}/document`: the document's file, as it was written.
pub struct DocumentFile;

impl Route for DocumentFile {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/archive/{id}/document";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = document_id(&call.path_params().await?)?;
    let path = call
      .standing()
      .library()
      .await?
      .document_file(id)
      .await?
      .ok_or_else(|| absent("document_not_found", "no such document"))?;
    let bytes = tokio::fs::read(&path).await.map_err(|why| {
      Refused::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        "file_unreadable",
        format!("the file could not be read: {why}"),
      )
    })?;

    Ok(with_headers(
      Response::new(Body::from(bytes)),
      &[
        (header::CONTENT_TYPE, "text/markdown; charset=utf-8".to_owned()),
        (header::CONTENT_DISPOSITION, format!("attachment; filename=\"search-{id}.md\"")),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_owned()),
        (header::CACHE_CONTROL, "private, no-cache".to_owned()),
      ],
    ))
  }
}

crate::route!(DocumentFile);

/// `GET /api/archive/{id}/attachments/{attachment}`: one file a document keeps beside it, looked up under the
/// document the road names and sent as it is kept.
pub struct DocumentAttachment;

impl Route for DocumentAttachment {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/archive/{id}/attachments/{attachment}";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let params = call.path_params().await?;
    let id = document_id(&params)?;
    let number = numbered(
      &params,
      "attachment",
      "bad_attachment_id",
      "the attachment's number is not a whole number from 1",
    )?;
    let file = call
      .standing()
      .library()
      .await?
      .attachment_file(id, number)
      .await?
      .ok_or_else(|| absent("attachment_not_found", "no such attachment"))?;
    let (download, scheme) = (call.query("download"), call.query("scheme"));
    let (range, cookies) = (call.header("range"), call.header("cookie"));
    let sending = Sending {
      download: download.as_deref() == Some("1"),
      range: range.as_deref(),
      scheme: scheme.as_deref(),
      cookies: cookies.as_deref(),
    };

    attachment(&file, &sending, PRIVATE_DAY).await
  }
}

crate::route!(DocumentAttachment);

fn reference(params: &HashMap<String, String>) -> String {
  params.get("ref").cloned().unwrap_or_default()
}

/// `GET /api/archive/{id}/refs/{ref}/preview`: what a source or a link of one document shows of itself.
pub struct RefPreviewRoute;

impl Route for RefPreviewRoute {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/archive/{id}/refs/{ref}/preview";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let params = call.path_params().await?;
    let id = document_id(&params)?;
    let preview = call
      .standing()
      .library()
      .await?
      .ref_preview(id, &reference(&params))
      .await?
      .ok_or_else(|| absent("no_such_source", "no such source"))?;

    Ok(with_headers(
      answer(StatusCode::OK, json!(preview)),
      &[(header::CACHE_CONTROL, "private, max-age=3600".to_owned())],
    ))
  }
}

crate::route!(RefPreviewRoute);

/// `GET /api/archive/{id}/refs/{ref}/image?as=`: a picture of one document — one its text shows, or the
/// picture or icon a source's page named — fetched by this service and handed over as a copy, labelled with
/// what its bytes are and kept from being read as anything else or running anything.
pub struct RefImageRoute;

impl Route for RefImageRoute {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/archive/{id}/refs/{ref}/image";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let params = call.path_params().await?;
    let id = document_id(&params)?;
    let asked = RefPicture::asked(call.query("as").as_deref());
    let image = call
      .standing()
      .library()
      .await?
      .ref_image(id, &reference(&params), asked)
      .await?
      .ok_or_else(|| absent("image_unavailable", "the picture is not available"))?;
    let length = image.bytes.len().to_string();

    Ok(picture(image.bytes, &image.kind, length))
  }
}

crate::route!(RefImageRoute);

/// A picture's bytes under the headers that keep it a picture.
pub fn picture(bytes: Vec<u8>, kind: &str, length: String) -> Response {
  with_headers(
    Response::new(Body::from(bytes)),
    &[
      (header::CONTENT_TYPE, kind.to_owned()),
      (header::CONTENT_LENGTH, length),
      (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_owned()),
      (header::CONTENT_SECURITY_POLICY, "default-src 'none'; sandbox".to_owned()),
      (header::CACHE_CONTROL, PRIVATE_DAY.to_owned()),
    ],
  )
}

/// Where an address that no longer names a screen of its own now stands, its query kept: the retired
/// collection screen and every address the searches were read under go to the archive screen, a document's
/// number kept. The path is the one inside the console, its base already taken off; none for every other.
pub fn moved_screen(path: &str) -> Option<String> {
  let (pathname, query) = match path.split_once('?') {
    Some((pathname, query)) => (pathname, Some(query)),
    None => (path, None),
  };
  let with_query = |target: String| match query {
    None | Some("") => target,
    Some(query) => format!("{target}?{query}"),
  };

  if pathname == "/collected" || pathname == "/collected/" {
    return Some(with_query("/archive".to_owned()));
  }

  let rest = pathname.strip_prefix("/searches")?;
  let rest = match rest {
    "" => "",
    rest => rest.strip_prefix('/')?,
  };

  Some(with_query(if rest.is_empty() {
    "/archive".to_owned()
  } else {
    format!("/archive/{rest}")
  }))
}

// The moved screen's answer: sent on for good to where its content now stands, under the console's base.
fn moved(call: &Call) -> Result<Response, Refused> {
  let uri = call.request.uri();
  let path = uri
    .path_and_query()
    .map_or_else(|| uri.path().to_owned(), |both| both.as_str().to_owned());
  let target = moved_screen(&path).ok_or_else(|| absent("not_found", "no such screen"))?;
  let base = call
    .standing()
    .settings
    .console
    .address
    .as_deref()
    .and_then(|address| url::Url::parse(address).ok())
    .map(|address| address.path().trim_end_matches('/').to_owned())
    .unwrap_or_default();
  let mut response = Response::new(Body::empty());

  *response.status_mut() = StatusCode::MOVED_PERMANENTLY;

  Ok(with_headers(response, &[(header::LOCATION, format!("{base}{target}"))]))
}

macro_rules! moved_route {
  ($name:ident, $path:literal) => {
    #[doc = concat!("`GET ", $path, "`: a screen that moved to the archive screen.")]
    pub struct $name;

    impl Route for $name {
      const METHOD: Method = Method::GET;
      const PATH: &'static str = $path;

      async fn answer(&self, call: Call) -> Result<Response, Refused> {
        moved(&call)
      }
    }

    crate::route!($name);
  };
}

moved_route!(MovedSearches, "/searches");
moved_route!(MovedSearchesFolder, "/searches/");
moved_route!(MovedSearch, "/searches/{*rest}");
moved_route!(MovedCollected, "/collected");
moved_route!(MovedCollectedFolder, "/collected/");
