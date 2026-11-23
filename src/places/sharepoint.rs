//! The SharePoint place (`docs/artifact.md`, "The SharePoint place"): a document's files in a SharePoint document
//! library, reached through Microsoft Graph with a person's Graph token, borrowed through
//! an optional page-token service's page token road. A location is `<site>/<drive>/<path in the library>`.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use axum::http::{HeaderMap, StatusCode, header};
use futures_util::StreamExt as _;
use headers::{ContentLength, ContentRange, HeaderMapExt as _};
use reqwest::{Method, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use super::{Grounds, Kept, Pending, Place, PlaceKind, Reading, field, text, unknown_field};
use crate::Refusal;
use crate::now_ms;
use crate::wire::{StorageField, StorageFieldShape};

/// The SharePoint place's name, which its paths begin with.
pub const NAME: &str = "sharepoint";

/// The mark a folder pattern names the owner's login by.
pub const LOGIN: &str = "{login}";
/// The mark a folder pattern names the document's number by, which it must name.
pub const DOCUMENT: &str = "{document}";

/// The most a file written in one call may weigh; a larger one goes through an upload session.
pub const SIMPLE_WRITE: usize = 4 * 1024 * 1024;

/// One part of an upload session: a multiple of the 320 KiB Graph asks parts in.
pub const UPLOAD_PART: usize = 12 * 320 * 1024;

/// The Graph scopes a token that writes a library holds one of.
pub const SCOPES: [&str; 2] = ["Files.ReadWrite.All", "Sites.ReadWrite.All"];

/// How long before a token's end it is asked for afresh, in milliseconds.
const FRESH_MS: i64 = 60_000;

/// The kind of the SharePoint place.
pub struct SharePoint;

// A setting's value refused where it holds a slash, which would move it into another segment of a location.
fn segment(settings: &Map<String, Value>, key: &str) -> Result<String, String> {
  let value = text(settings, key)?.ok_or_else(|| format!("a SharePoint place names its {key}"))?;

  if value.contains('/') {
    return Err(format!("{key} holds no /"));
  }

  Ok(value)
}

// The folder pattern judged: a path inside the library, each part named, naming the document's number.
fn folder(settings: &Map<String, Value>) -> Result<String, String> {
  let pattern = text(settings, "folder")?.ok_or("a SharePoint place names its folder")?;
  let parts: Vec<&str> = pattern.trim_matches('/').split('/').map(str::trim).collect();

  if parts.iter().any(|part| part.is_empty() || *part == "." || *part == "..") {
    return Err("folder is a path inside the library, each of its parts named".to_owned());
  }

  if !pattern.contains(DOCUMENT) {
    return Err(format!("folder names the document's number as {DOCUMENT}"));
  }

  Ok(parts.join("/"))
}

impl PlaceKind for SharePoint {
  fn name(&self) -> &'static str {
    NAME
  }

  fn description(&self) -> &'static str {
    "The files are kept in a SharePoint document library through Microsoft Graph, with a person's \
     token lent by a page-token service."
  }

  fn fields(&self) -> Vec<StorageField> {
    vec![
      field("site", StorageFieldShape::Text, true),
      field("drive", StorageFieldShape::Text, true),
      field("folder", StorageFieldShape::Text, true),
      field("account", StorageFieldShape::Number, false),
    ]
  }

  fn judge(&self, settings: &Map<String, Value>) -> Result<Map<String, Value>, String> {
    unknown_field(settings, &self.fields())?;

    let mut judged = Map::new();

    judged.insert("site".to_owned(), Value::from(segment(settings, "site")?));
    judged.insert("drive".to_owned(), Value::from(segment(settings, "drive")?));
    judged.insert("folder".to_owned(), Value::from(folder(settings)?));

    match settings.get("account") {
      None | Some(Value::Null) => {}
      Some(account) => {
        let number = account
          .as_i64()
          .filter(|number| *number > 0)
          .ok_or("account is a whole number from 1")?;

        judged.insert("account".to_owned(), Value::from(number));
      }
    }

    Ok(judged)
  }

  fn open(
    &self,
    settings: &Map<String, Value>,
    grounds: &Grounds,
    owner: Option<&str>,
  ) -> Result<Arc<dyn Place>, Refusal> {
    let judged = self.judge(settings).map_err(Refusal::failed)?;
    let said = |key: &str| judged.get(key).and_then(Value::as_str).unwrap_or_default().to_owned();
    let account = match judged.get("account").and_then(Value::as_i64) {
      Some(account) => Some(account),
      None => owner.and_then(|owner| owner.parse().ok()),
    };

    Ok(Arc::new(Library {
      site: said("site"),
      drive: said("drive"),
      folder: said("folder"),
      account,
      grounds: grounds.clone(),
    }))
  }
}

/// One document library, opened for one document's owner.
pub struct Library {
  site: String,
  drive: String,
  folder: String,
  /// The account the token is lent for, none where nobody's is.
  account: Option<i64>,
  grounds: Grounds,
}

/// What the page token road answers: the token and the moment it ends.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageToken {
  token: String,
  expires_at: i64,
}

/// What the page token road takes.
#[derive(Serialize)]
struct PageTokenAsk<'a> {
  account: i64,
  scopes: &'a [&'a str],
}

/// The road's refusal, as its wire carries it.
#[derive(Deserialize)]
struct RoadRefusal {
  #[serde(default)]
  error: String,
}

/// Whom a token was lent for: the road it came from and the account.
type Lender = (String, i64);

/// A token lent, and the moment it ends.
type Lent = (String, i64);

/// The tokens borrowed, each by whom it was lent for; kept in memory alone.
static TOKENS: LazyLock<Mutex<HashMap<Lender, Lent>>> = LazyLock::new(Mutex::default);

/// An item of the library, as Graph answers it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Item {
  #[serde(default)]
  size: u64,
  #[serde(default)]
  e_tag: String,
  #[serde(default)]
  file: Option<Value>,
}

/// An upload session Graph opened.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadSession {
  upload_url: String,
}

fn upstream(why: impl std::fmt::Display) -> Refusal {
  Refusal::upstream(format!("the SharePoint library was not reached: {why}"))
}

// A location split into its site, its drive and its path in the library.
fn parts(at: &str) -> Result<(&str, &str, &str), Refusal> {
  let mut parts = at.splitn(3, '/');

  match (parts.next(), parts.next(), parts.next()) {
    (Some(site), Some(drive), Some(path)) if !site.is_empty() && !drive.is_empty() && !path.is_empty() => {
      Ok((site, drive, path))
    }
    _ => Err(Refusal::failed(format!("{at} is no location of a SharePoint library"))),
  }
}

// What a failed Graph answer said, for a refusal's sentence.
async fn said(answer: reqwest::Response) -> String {
  let status = answer.status();
  let body = answer.text().await.unwrap_or_default();
  let error = serde_json::from_str::<Value>(&body)
    .ok()
    .and_then(|body| body["error"]["message"].as_str().map(str::to_owned))
    .unwrap_or_default();

  format!("Graph answered {status} {error}").trim().to_owned()
}

impl Library {
  fn sharepoint(&self) -> &crate::settings::SharePoint {
    &self.grounds.settings.sharepoint
  }

  // The address of an item at `path` in the library, with `suffix` after it: `content` or an action.
  fn address(&self, site: &str, drive: &str, path: &str, suffix: Option<&str>) -> Result<Url, Refusal> {
    let mut url =
      Url::parse(&self.sharepoint().graph).map_err(|why| upstream(format!("Graph's address: {why}")))?;
    let names: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();

    {
      let mut segments = url.path_segments_mut().map_err(|()| upstream("Graph's address takes no path"))?;

      segments.pop_if_empty().extend(["sites", site, "drives", drive, "root:"]);

      for (at, name) in names.iter().enumerate() {
        match (at + 1 == names.len(), suffix) {
          (true, Some(_)) => segments.push(&format!("{name}:")),
          _ => segments.push(name),
        };
      }

      if let Some(suffix) = suffix {
        segments.push(suffix);
      }
    }

    Ok(url)
  }

  // A live token for the account the place borrows from, borrowed afresh a minute before the kept one ends.
  async fn token(&self) -> Result<String, Refusal> {
    let road = self.sharepoint();
    let account = self
      .account
      .ok_or_else(|| Refusal::upstream("the SharePoint place names no account a token is lent for"))?;

    if road.tokens.trim().is_empty() {
      return Err(Refusal::upstream("this service names no page token road for a SharePoint place"));
    }

    let key = (road.tokens.clone(), account);
    let now = now_ms();

    if let Some((token, ends)) = TOKENS.lock().ok().and_then(|tokens| tokens.get(&key).cloned())
      && ends - FRESH_MS > now
    {
      return Ok(token);
    }

    let address = format!("{}/page-tokens", road.tokens.trim_end_matches('/'));
    let answer = self
      .grounds
      .http
      .post(address)
      .bearer_auth(&road.token)
      .json(&PageTokenAsk {
        account,
        scopes: &SCOPES,
      })
      .send()
      .await
      .map_err(|why| Refusal::upstream(format!("the page token road was not reached: {why}")))?;

    if !answer.status().is_success() {
      let status = answer.status();
      let refusal: Option<RoadRefusal> = answer.json().await.ok();

      return Err(Refusal::upstream(format!(
        "the page token road refused a token for account {account} with {status}: {}",
        refusal.map(|refusal| refusal.error).unwrap_or_default()
      )));
    }

    let lent: PageToken = answer
      .json()
      .await
      .map_err(|why| Refusal::upstream(format!("the page token road's answer does not read: {why}")))?;

    if let Ok(mut tokens) = TOKENS.lock() {
      tokens.insert(key, (lent.token.clone(), lent.expires_at));
    }

    Ok(lent.token)
  }

  // The token borrowed for this place let go of, so the next call borrows a fresh one.
  fn forget(&self) {
    if let (Some(account), Ok(mut tokens)) = (self.account, TOKENS.lock()) {
      tokens.remove(&(self.sharepoint().tokens.clone(), account));
    }
  }

  // A call to Graph with the borrowed token, a token Graph refuses forgotten.
  async fn call(&self, request: RequestBuilder) -> Result<reqwest::Response, Refusal> {
    let answer = request.bearer_auth(self.token().await?).send().await.map_err(upstream)?;

    if answer.status() == StatusCode::UNAUTHORIZED {
      self.forget();
    }

    Ok(answer)
  }

  fn request(&self, method: Method, url: Url) -> RequestBuilder {
    self.grounds.http.request(method, url)
  }

  // A file written through an upload session, part by part.
  async fn upload(&self, site: &str, drive: &str, path: &str, bytes: Vec<u8>) -> Result<(), Refusal> {
    let url = self.address(site, drive, path, Some("createUploadSession"))?;
    let opened = self
      .call(self.request(Method::POST, url).json(&json!({
        "item": { "@microsoft.graph.conflictBehavior": "replace" }
      })))
      .await?;

    if !opened.status().is_success() {
      return Err(upstream(said(opened).await));
    }

    let session: UploadSession = opened.json().await.map_err(upstream)?;
    let total = bytes.len();

    for (index, part) in bytes.chunks(UPLOAD_PART).enumerate() {
      let from = index * UPLOAD_PART;
      let to = from + part.len() - 1;
      // The session's address carries its own credential, so no token goes with a part.
      let answer = self
        .grounds
        .http
        .put(&session.upload_url)
        .header(header::CONTENT_RANGE, format!("bytes {from}-{to}/{total}"))
        .body(part.to_vec())
        .send()
        .await
        .map_err(upstream)?;

      if !answer.status().is_success() {
        return Err(upstream(said(answer).await));
      }
    }

    Ok(())
  }
}

// The total size and the stretch a Graph read answered, from its headers.
fn stretch(status: StatusCode, headers: &HeaderMap) -> (u64, Option<(u64, u64)>) {
  let length = headers.typed_get::<ContentLength>().map_or(0, |length| length.0);
  let range = headers.typed_get::<ContentRange>().filter(|_| status == StatusCode::PARTIAL_CONTENT);

  match range.and_then(|range| Some((range.bytes_len()?, range.bytes_range()?))) {
    Some((total, span)) => (total, Some(span)),
    None => (length, None),
  }
}

impl Place for Library {
  fn kind(&self) -> &'static str {
    NAME
  }

  fn folder<'a>(&'a self, document: i64, owner: Option<&'a str>) -> Pending<'a, String> {
    Box::pin(async move {
      let mut laid = self.folder.replace(DOCUMENT, &document.to_string());

      if laid.contains(LOGIN) {
        let owner = owner
          .ok_or_else(|| Refusal::conflict("a document with no owner has no login to lay its folder by"))?;
        let login = self
          .grounds
          .accounts
          .login_of(owner)
          .await?
          .ok_or_else(|| Refusal::upstream(format!("the account service knows no account {owner}")))?;

        laid = laid.replace(LOGIN, &login.replace('/', "_"));
      }

      Ok(format!("{}/{}/{laid}", self.site, self.drive))
    })
  }

  fn write<'a>(&'a self, at: &'a str, bytes: Vec<u8>) -> Pending<'a, ()> {
    Box::pin(async move {
      let (site, drive, path) = parts(at)?;

      if bytes.len() > SIMPLE_WRITE {
        return self.upload(site, drive, path, bytes).await;
      }

      let url = self.address(site, drive, path, Some("content"))?;
      let answer = self
        .call(
          self
            .request(Method::PUT, url)
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .body(bytes),
        )
        .await?;

      if answer.status().is_success() { Ok(()) } else { Err(upstream(said(answer).await)) }
    })
  }

  fn read<'a>(&'a self, at: &'a str, range: Option<&'a str>) -> Pending<'a, Reading> {
    Box::pin(async move {
      let (site, drive, path) = parts(at)?;
      let url = self.address(site, drive, path, Some("content"))?;
      let asked = |range: Option<&str>| {
        let request = self.request(Method::GET, url.clone());

        match range {
          Some(range) => request.header(header::RANGE, range),
          None => request,
        }
      };
      let mut answer = self.call(asked(range)).await?;

      // A range the file does not hold is answered with the whole file, as the local place answers it.
      if answer.status() == StatusCode::RANGE_NOT_SATISFIABLE {
        answer = self.call(asked(None)).await?;
      }

      match answer.status() {
        StatusCode::NOT_FOUND => return Err(Refusal::absent("the file is not in its library")),
        status if !status.is_success() => return Err(upstream(said(answer).await)),
        _ => {}
      }

      let (total, span) = stretch(answer.status(), answer.headers());
      let body = answer.bytes_stream().map(|part| part.map_err(std::io::Error::other)).boxed();

      Ok(Reading { total, span, body })
    })
  }

  fn exists<'a>(&'a self, at: &'a str) -> Pending<'a, Option<Kept>> {
    Box::pin(async move {
      let (site, drive, path) = parts(at)?;
      let answer = self.call(self.request(Method::GET, self.address(site, drive, path, None)?)).await?;

      match answer.status() {
        StatusCode::NOT_FOUND => return Ok(None),
        status if !status.is_success() => return Err(upstream(said(answer).await)),
        _ => {}
      }

      let item: Item = answer.json().await.map_err(upstream)?;

      Ok(item.file.is_some().then_some(Kept {
        bytes: item.size,
        version: item.e_tag,
      }))
    })
  }

  fn delete<'a>(&'a self, at: &'a str) -> Pending<'a, ()> {
    Box::pin(async move {
      let (site, drive, path) = parts(at)?;
      let answer = self.call(self.request(Method::DELETE, self.address(site, drive, path, None)?)).await?;

      match answer.status() {
        StatusCode::NOT_FOUND => Ok(()),
        status if status.is_success() => Ok(()),
        _ => Err(upstream(said(answer).await)),
      }
    })
  }
}
