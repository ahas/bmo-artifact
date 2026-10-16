//! A kept file put on the wire (`docs/archive.md`, "An HTML page"; `docs/api.md`, "Documents (HTTP)"),
//! whichever road asked for it: it plays and seeks where it stands, is saved rather than shown when the
//! caller asks for a download, and nothing on it is ever let run — except an HTML page, which runs as a
//! page in a sandbox of its own origin and goes out with its prelude. A share road names a caching that ends
//! with its link.

use std::ops::Bound;

use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::Response;
use headers::{Header, Range};
use percent_encoding::utf8_percent_encode;

use super::hosted_page::{KEPT_FILE_POLICY, hosted_page, hosted_page_policy, reader_scheme};
use crate::documents::read::KeptFile;
use crate::helpers::js::URI_COMPONENT;
use crate::plumbing::http::{Call, Refused};

/// How long a reader's own cache may keep what the console's road sends.
pub const PRIVATE_DAY: &str = "private, max-age=86400";

/// The stretch of a file the Range header asks for, both ends counted in, none where it asks for none or for
/// none a file of `bytes` holds, the whole file being the answer then.
pub fn range_of(asked: Option<&str>, bytes: u64) -> Option<(u64, u64)> {
  let value = HeaderValue::from_str(asked?.trim()).ok()?;
  let range = Range::decode(&mut std::iter::once(&value)).ok()?;
  let (start, end) = range.satisfiable_ranges(bytes).next()?;
  let from = match start {
    Bound::Included(from) => from,
    Bound::Excluded(from) => from + 1,
    Bound::Unbounded => 0,
  };
  let to = match end {
    Bound::Included(to) => to.min(bytes.checked_sub(1)?),
    Bound::Excluded(to) => to.checked_sub(1)?.min(bytes.checked_sub(1)?),
    Bound::Unbounded => bytes.checked_sub(1)?,
  };

  (from <= to && from < bytes).then_some((from, to))
}

fn failed(why: impl std::fmt::Display) -> Refused {
  Refused::new(
    StatusCode::INTERNAL_SERVER_ERROR,
    "file_unreadable",
    format!("the file could not be read: {why}"),
  )
}

fn set(headers: &mut HeaderMap, name: header::HeaderName, value: &str) {
  if let Ok(value) = HeaderValue::from_str(value) {
    headers.insert(name, value);
  }
}

/// A kept file sent as it is kept, answering a range where one is asked.
pub async fn kept_file(
  file: &KeptFile,
  keep: bool,
  range: Option<&str>,
  cache: &str,
) -> Result<Response, Refused> {
  let bytes = tokio::fs::read(&file.path).await.map_err(failed)?;
  let total = bytes.len() as u64;
  let asked = range_of(range, total);
  let mut response = Response::new(Body::empty());
  let headers = response.headers_mut();

  set(headers, header::CONTENT_TYPE, &file.r#type);
  set(headers, header::ACCEPT_RANGES, "bytes");
  set(headers, header::CACHE_CONTROL, cache);
  set(
    headers,
    header::CONTENT_DISPOSITION,
    &if keep {
      format!("attachment; filename*=UTF-8''{}", utf8_percent_encode(&file.name, URI_COMPONENT))
    } else {
      "inline".to_owned()
    },
  );

  match asked {
    None => {
      set(headers, header::CONTENT_LENGTH, &total.to_string());
      *response.body_mut() = Body::from(bytes);
    }
    Some((from, to)) => {
      set(headers, header::CONTENT_LENGTH, &(to - from + 1).to_string());
      set(headers, header::CONTENT_RANGE, &format!("bytes {from}-{to}/{total}"));
      *response.status_mut() = StatusCode::PARTIAL_CONTENT;
      *response.body_mut() = Body::from(bytes[from as usize..=to as usize].to_vec());
    }
  }

  Ok(response)
}

/// A page sent whole with its prelude in it, in the reader's scheme: what goes out turns on the reader's
/// screen mode, so a copy kept for one mode is never handed to another.
pub async fn hosted_file(
  file: &KeptFile,
  scheme: Option<&str>,
  cookies: Option<&str>,
  cache: &str,
) -> Result<Response, Refused> {
  let html = tokio::fs::read_to_string(&file.path).await.map_err(failed)?;
  let mut response = Response::new(Body::from(hosted_page(&html, reader_scheme(scheme, cookies))));
  let headers = response.headers_mut();

  set(headers, header::CONTENT_TYPE, "text/html; charset=utf-8");
  set(headers, header::CACHE_CONTROL, cache);
  set(headers, header::CONTENT_DISPOSITION, "inline");
  set(headers, header::VARY, "cookie");

  Ok(response)
}

/// What a request for a kept attachment carries that decides how it is sent.
#[derive(Clone, Debug, Default)]
pub struct Sending {
  /// Whether it is asked for to keep rather than to show.
  pub download: bool,
  pub range: Option<String>,
  /// The scheme the console's frame names, and the cookies the reader's mode is kept in.
  pub scheme: Option<String>,
  pub cookies: Option<String>,
}

impl Sending {
  /// What a call asks: `download=1`, its `Range` header, the `scheme` its frame names and its cookies.
  pub fn of(call: &Call) -> Self {
    Self {
      download: call.query("download").as_deref() == Some("1"),
      range: call.header("range"),
      scheme: call.query("scheme"),
      cookies: call.header("cookie"),
    }
  }
}

/// An attachment sent under the isolation its type is let: a page as a hosted page unless it is asked for
/// to keep, and every other file as a file nothing on which may run.
pub async fn attachment(file: &KeptFile, sending: &Sending, cache: &str) -> Result<Response, Refused> {
  let page = file.r#type == "text/html";
  let mut response = if page && !sending.download {
    hosted_file(file, sending.scheme.as_deref(), sending.cookies.as_deref(), cache).await?
  } else {
    kept_file(file, sending.download, sending.range.as_deref(), cache).await?
  };
  let headers = response.headers_mut();

  let policy = if page { hosted_page_policy() } else { KEPT_FILE_POLICY.to_owned() };

  set(headers, header::CONTENT_SECURITY_POLICY, &policy);
  set(headers, header::REFERRER_POLICY, "no-referrer");
  set(headers, header::X_CONTENT_TYPE_OPTIONS, "nosniff");

  Ok(response)
}
