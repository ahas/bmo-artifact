//! A kept file put on the wire (`docs/artifact.md`, "An HTML page"; `docs/api.md`, "Documents" and
//! "Attachments"): it plays and seeks where it stands, is saved rather than shown when the caller asks for a
//! download, and nothing on it is ever let run, except an HTML page, which runs as a page in a sandbox of
//! its own origin and goes out with its prelude.

use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::Response;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

use super::drawn_copies;
use super::hosted_page::{
  KEPT_FILE_POLICY, PageSounds, SoundFile, hosted_page, hosted_page_policy, reader_scheme, writes_sound,
};
use crate::Refusal;
use crate::helpers::picture;
use crate::places::Located;
use crate::plumbing::http::{Call, Refused, Standing};
use crate::readings::now_seconds;
use crate::store::attachments;
use crate::wire::ContentQuery;

/// How long a reader's own cache may keep a kept file.
pub const PRIVATE_DAY: &str = "private, max-age=86400";
/// A file that changes under its address, never kept.
pub const NO_STORE: &str = "no-store";

// What a file's name is written with in a download's header: every byte but the unreserved ones encoded.
const NAME: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'.').remove(b'_').remove(b'~');

/// A kept file as it goes out: where it is kept, its type and the name it is saved under.
#[derive(Clone, Debug)]
pub struct KeptFile {
  pub at: Located,
  pub media_type: String,
  pub name: String,
}

fn unread(why: Refusal) -> Refused {
  why.into()
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
  let reading = file.at.read(range).await.map_err(unread)?;
  let total = reading.total;
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

  match reading.span {
    None => set(headers, header::CONTENT_LENGTH, &total.to_string()),
    Some((from, to)) => {
      set(headers, header::CONTENT_LENGTH, &(to - from + 1).to_string());
      set(headers, header::CONTENT_RANGE, &format!("bytes {from}-{to}/{total}"));
      *response.status_mut() = StatusCode::PARTIAL_CONTENT;
    }
  }

  *response.body_mut() = Body::from_stream(reading.body);

  Ok(response)
}

fn is_picture(media_type: &str) -> bool {
  media_type.trim_start().to_ascii_lowercase().starts_with("image/")
}

/// A kept picture as it is drawn at `width` pixels across (docs/artifact.md, "A picture drawn small"): its copy
/// at the ladder's step, made once and kept in memory under the file and the moment it was written, or the file
/// as kept where it is not shrunk to that step.
pub async fn drawn_file(file: &KeptFile, width: u32, cache: &str) -> Result<Response, Refused> {
  let step = picture::drawn_step(width);
  let written = file
    .at
    .exists()
    .await
    .map_err(unread)?
    .ok_or_else(|| Refused::from(Refusal::absent("the file is not at its place")))?;
  let key = format!("{step}|{}|{}|{}", written.bytes, written.version, file.at.path());
  let copy = match drawn_copies::kept(&key) {
    Some(copy) => Some(copy),
    None => {
      let bytes = file.at.whole().await.map_err(unread)?;
      let made = tokio::task::spawn_blocking(move || picture::drawn(&bytes, step)).await.ok().flatten();

      made.map(|(bytes, media_type)| drawn_copies::keep(key, bytes, media_type))
    }
  };
  let Some(copy) = copy else {
    return kept_file(file, false, None, cache).await;
  };
  let mut response = Response::new(Body::from(copy.bytes.to_vec()));
  let headers = response.headers_mut();

  set(headers, header::CONTENT_TYPE, copy.media_type);
  set(headers, header::CACHE_CONTROL, cache);
  set(headers, header::CONTENT_DISPOSITION, "inline");
  set(headers, header::CONTENT_LENGTH, &copy.bytes.len().to_string());

  Ok(response)
}

/// Who a hosted page goes out to, which decides the addresses its sounds play from (`docs/artifact.md`, "An
/// HTML page" and "Readings").
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Opened {
  /// The console's own reader, whose page is handed a reading of its own.
  Console,
  /// The holder of a share link, by its token.
  Share(String),
  /// The holder of a reading, by its token.
  Reading(String),
}

/// Where a hosted page goes out: who it goes out to, and the route it goes out on, which every address it is
/// handed is written relative to, so it holds under whatever prefix a console relays the service at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reach {
  pub opened: Opened,
  pub route: &'static str,
}

impl Reach {
  pub fn new(opened: Opened, route: &'static str) -> Self {
    Self { opened, route }
  }

  // The way from the route's own address back to the root of the service's API.
  fn up(&self) -> String {
    "../".repeat(self.route.split('/').filter(|part| !part.is_empty()).count().saturating_sub(1))
  }
}

/// A response that answers any origin, for a sandboxed page that reads it from an origin of its own.
pub fn any_origin(mut response: Response) -> Response {
  response
    .headers_mut()
    .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));

  response
}

/// What a page of `document` going out at `reach` is handed for its sounds: each file the document keeps at an
/// address the sandboxed page loads, the player's script, and the player's words in `language`.
pub async fn page_sounds(
  standing: &Standing,
  document: i64,
  reach: &Reach,
  language: Option<&str>,
) -> Result<PageSounds, Refused> {
  let up = reach.up();
  let opened = match &reach.opened {
    Opened::Console => format!("readings/{}", standing.readings.issue(document, now_seconds())),
    Opened::Share(token) => format!("shares/{token}"),
    Opened::Reading(token) => format!("readings/{token}"),
  };
  let (kept, _) = attachments::list(standing.db().await?, document, None, i64::from(u16::MAX)).await?;
  let files = kept
    .into_iter()
    .map(|kept| {
      let play = format!("{up}{opened}/attachments/{}", kept.attachment.id);

      (
        kept.attachment.name,
        SoundFile {
          download: format!("{play}?download=true"),
          play,
        },
      )
    })
    .collect();
  let (locale, words) = standing.player.words(language);

  Ok(PageSounds {
    player: standing.player.script.as_ref().map(|script| format!("{up}player/{}", script.name)),
    files,
    words,
    locale,
  })
}

/// A page of `document` sent whole with its prelude in it, in the scheme asked for, its sounds handed the
/// addresses `reach` loads them at. A page goes out to be read once and never kept, since what it is handed
/// ends.
pub async fn hosted_file(
  standing: &Standing,
  file: &KeptFile,
  asked: &ContentQuery,
  document: i64,
  reach: &Reach,
) -> Result<Response, Refused> {
  let html = String::from_utf8_lossy(&file.at.whole().await.map_err(unread)?).into_owned();
  let sounds = if writes_sound(&html) {
    Some(page_sounds(standing, document, reach, asked.language.as_deref()).await?)
  } else {
    None
  };
  let page = hosted_page(&html, reader_scheme(asked.scheme.as_deref()), sounds.as_ref());
  let mut response = Response::new(Body::from(page));
  let headers = response.headers_mut();

  set(headers, header::CONTENT_TYPE, "text/html; charset=utf-8");
  set(headers, header::CACHE_CONTROL, NO_STORE);
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

/// A kept file of `document` sent under the isolation its type is let, as it is asked for: a page as a hosted
/// page going out at `reach` unless it is asked for to keep, and every other file as a file nothing on which
/// may run.
pub async fn send(
  standing: &Standing,
  asking: &Asking,
  file: &KeptFile,
  cache: &str,
  document: i64,
  reach: &Reach,
) -> Result<Response, Refused> {
  let asked = &asking.query;
  let download = asked.download == Some(true);
  let page = file.media_type.split(';').next().is_some_and(|kind| kind.trim() == "text/html");
  let mut response = if page && !download {
    hosted_file(standing, file, asked, document, reach).await?
  } else if let Some(width) = asked.width.filter(|_| !download && is_picture(&file.media_type)) {
    drawn_file(file, width, cache).await?
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
