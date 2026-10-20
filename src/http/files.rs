//! A kept file put on the wire (`docs/archive.md`, "An HTML page"; `docs/api.md`, "Documents" and
//! "Attachments"): it plays and seeks where it stands, is saved rather than shown when the caller asks for a
//! download, and nothing on it is ever let run, except an HTML page, which runs as a page in a sandbox of
//! its own origin and goes out with its prelude.

use std::ops::Bound;

use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::Response;
use headers::{Header, Range};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

use super::hosted_page::{KEPT_FILE_POLICY, hosted_page, hosted_page_policy, reader_scheme};
use crate::Refusal;
use crate::plumbing::http::{Call, Refused};
use crate::wire::ContentQuery;

/// How long a reader's own cache may keep a kept file.
pub const PRIVATE_DAY: &str = "private, max-age=86400";
/// A file that changes under its address, never kept.
pub const NO_STORE: &str = "no-store";

// What a file's name is written with in a download's header: every byte but the unreserved ones encoded.
const NAME: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'.').remove(b'_').remove(b'~');

/// A file on the disk as it goes out: where it is, its type and the name it is saved under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptFile {
  pub path: String,
  pub media_type: String,
  pub name: String,
}

/// Whether a kept file stands inside the file root, every link resolved: a file is served from nowhere
/// else.
pub fn inside_root(root: &std::path::Path, path: &str) -> bool {
  match (std::fs::canonicalize(path), std::fs::canonicalize(root)) {
    (Ok(real), Ok(root)) => real.starts_with(root),
    _ => false,
  }
}

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

fn unread(why: std::io::Error) -> Refused {
  if why.kind() == std::io::ErrorKind::NotFound {
    Refusal::absent("the file is not on the disk").into()
  } else {
    Refusal::failed(format!("the file could not be read: {why}")).into()
  }
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
  let bytes = tokio::fs::read(&file.path).await.map_err(unread)?;
  let total = bytes.len() as u64;
  let asked = range_of(range, total);
  let mut response = Response::new(Body::empty());
  let headers = response.headers_mut();

  set(headers, header::CONTENT_TYPE, &file.media_type);
  set(headers, header::ACCEPT_RANGES, "bytes");
  set(headers, header::CACHE_CONTROL, cache);
  set(
    headers,
    header::CONTENT_DISPOSITION,
    &if keep {
      format!("attachment; filename*=UTF-8''{}", utf8_percent_encode(&file.name, NAME))
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

/// A page sent whole with its prelude in it, in the scheme asked for.
pub async fn hosted_file(file: &KeptFile, scheme: Option<&str>, cache: &str) -> Result<Response, Refused> {
  let html = tokio::fs::read_to_string(&file.path).await.map_err(unread)?;
  let mut response = Response::new(Body::from(hosted_page(&html, reader_scheme(scheme))));
  let headers = response.headers_mut();

  set(headers, header::CONTENT_TYPE, "text/html; charset=utf-8");
  set(headers, header::CACHE_CONTROL, cache);
  set(headers, header::CONTENT_DISPOSITION, "inline");

  Ok(response)
}

/// How a call asks for a kept file: its query and the range its header names.
pub struct Asking {
  pub query: ContentQuery,
  pub range: Option<String>,
}

impl Asking {
  /// What a call asks for.
  pub fn of(call: &Call) -> Result<Self, Refused> {
    Ok(Self {
      query: call.query()?,
      range: call.header("range"),
    })
  }
}

/// A kept file sent under the isolation its type is let, as it is asked for: a page as a hosted page unless
/// it is asked for to keep, and every other file as a file nothing on which may run.
pub async fn send(asking: &Asking, file: &KeptFile, cache: &str) -> Result<Response, Refused> {
  let asked = &asking.query;
  let download = asked.download == Some(true);
  let page = file.media_type.split(';').next().is_some_and(|kind| kind.trim() == "text/html");
  let mut response = if page && !download {
    hosted_file(file, asked.scheme.as_deref(), cache).await?
  } else {
    kept_file(file, download, asking.range.as_deref(), cache).await?
  };
  let headers = response.headers_mut();
  let policy = if page { hosted_page_policy() } else { KEPT_FILE_POLICY.to_owned() };

  set(headers, header::CONTENT_SECURITY_POLICY, &policy);
  set(headers, header::REFERRER_POLICY, "no-referrer");
  set(headers, header::X_CONTENT_TYPE_OPTIONS, "nosniff");

  Ok(response)
}
