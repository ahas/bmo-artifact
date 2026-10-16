//! The archive read for a person (`docs/api.md`, "Documents (HTTP)"; `docs/archive.md`, "Where a person
//! reads it"): the list a page at a time, one document whole with its kind's outline, its files, and the
//! places its text points outside.
//!
//! What a finding's text points at outside — its pictures, the links it mentions — is found in the text
//! itself, and every such place, and every source row, is named by a ref the screen hands back. The address
//! behind a ref is read out of the record each time, so a request can never make this service fetch an
//! address of its own choosing. Every file is served only from inside its own document's folder.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};
use regex::Regex;
use sea_orm::{DatabaseConnection, QueryResult};
use serde::Serialize;
use serde_json::{Map, Value};

use super::preview::PreviewCache;
use crate::clients::job::Job;
use crate::clients::receiver::Receiver;
use crate::clients::tool_host::Raster;
use crate::helpers::web_address::web_addresses_in;
use crate::kinds::{Outline, OutlineFinding, kind};
use crate::markup::finding_markdown::picture_caption;
use crate::store::documents::SEARCHER;
use crate::wording::search::Search;
use crate::{Refusal, all, one};

/// How many documents a page of the list holds where none is asked, and the most it may hold.
pub const PAGE_SIZE: i64 = 30;
pub const PAGE_MOST: i64 = 200;
/// How long the phrase the list is searched by may be.
pub const PHRASE_MOST: usize = 200;
/// How much of a searched row's line stands before its hit, and around it.
pub const HIT_LEAD: usize = 8;
pub const HIT_RADIUS: usize = 36;

/// That a document is shared, without the link itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareMark {
  pub expires_at: Option<i64>,
}

/// One document as a row of the list shows it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
  pub id: i64,
  pub chat_id: i64,
  pub room: Option<String>,
  pub query: String,
  pub started_depth: String,
  pub ended_depth: Option<String>,
  pub status: String,
  pub started_at: i64,
  pub finished_at: Option<i64>,
  pub findings: i64,
  pub sources: i64,
  pub title: Option<String>,
  pub topic: Option<String>,
  pub keywords: Vec<String>,
  pub rule: Option<String>,
  pub options: Map<String, Value>,
  pub kind: String,
  pub author: String,
  pub documented: bool,
  pub shared: Option<ShareMark>,
  /// The body's words around the phrase a list was searched by; only a searched list carries it.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub excerpt: Option<Option<String>>,
}

/// A page of the list.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
  pub items: Vec<Summary>,
  pub has_more: bool,
}

/// A source a document cites or opened.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceView {
  pub r#ref: String,
  pub url: String,
  pub title: Option<String>,
  pub supports: Option<String>,
  pub mark: Option<String>,
}

/// A picture a finding's text shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ImageView {
  pub r#ref: String,
  pub url: String,
  pub caption: Option<String>,
}

/// A link a finding's text mentions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LinkView {
  pub r#ref: String,
  pub url: String,
  pub label: Option<String>,
}

/// One finding read whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FindingView {
  pub place: i64,
  pub run_id: Option<i64>,
  pub branch: Option<String>,
  pub text: String,
  pub images: Vec<ImageView>,
  pub links: Vec<LinkView>,
  pub sources: Vec<SourceView>,
}

/// One file a document keeps beside it, as its attachment section shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AttachmentView {
  pub id: i64,
  pub media: String,
  pub r#type: String,
  pub bytes: i64,
  pub name: String,
}

/// The document's own row read whole: the row's head, the runs that carried it, and whether its file is
/// there to read.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DetailHead {
  #[serde(flatten)]
  pub summary: Summary,
  pub runs: Vec<i64>,
  pub document: bool,
}

/// One document read whole: its row, what its kind is and how it outlines the body, its findings, the pages
/// it opened and cited under nothing, and its attachments.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Detail {
  pub search: DetailHead,
  pub kind: Value,
  pub outline: Outline,
  pub findings: Vec<FindingView>,
  pub opened: Vec<SourceView>,
  pub attachments: Vec<AttachmentView>,
}

/// One document as somebody holding its share link reads it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedDetail {
  /// Its title, or its question where it wrote none.
  pub title: String,
  pub run_count: usize,
  pub kind: String,
  pub findings: Vec<FindingView>,
  pub opened: Vec<SourceView>,
  pub attachments: Vec<AttachmentView>,
}

/// A kept file as it is served: where it is, its type, how long it is and what it is called.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptFile {
  pub path: PathBuf,
  pub r#type: String,
  pub bytes: u64,
  pub name: String,
}

/// What a ref names in a document's record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefKind {
  Source,
  Image,
  Link,
}

/// Which picture of a ref is wanted: its own, its page's representative picture, or its page's icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefPicture {
  Own,
  Thumb,
  Favicon,
}

impl RefPicture {
  /// The picture a word asks for: anything but `thumb` and `favicon` is the ref's own.
  pub fn asked(word: Option<&str>) -> Self {
    match word {
      Some("thumb") => Self::Thumb,
      Some("favicon") => Self::Favicon,
      _ => Self::Own,
    }
  }
}

/// What a source or a link shows of itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RefPreview {
  pub title: Option<String>,
  pub thumbnail: bool,
  pub favicon: bool,
}

/// What a page of the list is asked for.
#[derive(Clone, Debug, Default)]
pub struct Asked {
  pub limit: i64,
  pub before: Option<i64>,
  pub shared: bool,
  pub kind: Option<String>,
  pub phrase: Option<String>,
  /// The moment the reading is made at, in epoch seconds, which a link's liveness is read against.
  pub now: i64,
}

/// What the read stands on: the database, the file root, the wording, bmo-job for the runs that carried a
/// document, bmo-receiver for the rooms' names, and the previews the screen is shown.
#[derive(Clone)]
pub struct Library {
  pub db: DatabaseConnection,
  pub root: PathBuf,
  pub words: Search,
  pub job: Job,
  pub receiver: Receiver,
  pub previews: Arc<PreviewCache>,
}

const KIND: &str = "COALESCE(s.kind, 'search')";

fn summary_columns() -> String {
  format!(
    r#"s.id AS id, s.chat_id AS chat_id, s.query AS query, s.started_depth AS started_depth,
       s.ended_depth AS ended_depth, s.status AS status, s.started_at AS started_at, s.finished_at AS finished_at,
       s.title AS title, s.topic AS topic, s.keywords AS keywords, s.rule AS rule, s.options AS options,
       (SELECT COUNT(*) FROM search_findings f WHERE f.search_id = s.id) AS findings,
       (SELECT COUNT(*) FROM search_sources r WHERE r.search_id = s.id) AS sources,
       {KIND} AS kind, COALESCE(s.author, '{SEARCHER}') AS author, COALESCE(s.documented, 1) AS documented"#
  )
}

// A JSON column read back as the shape it was written in, and as nothing otherwise.
fn keywords_of(text: Option<String>) -> Vec<String> {
  text
    .and_then(|text| serde_json::from_str::<Vec<String>>(&text).ok())
    .unwrap_or_default()
}

fn options_of(text: Option<String>) -> Map<String, Value> {
  match text.and_then(|text| serde_json::from_str::<Value>(&text).ok()) {
    Some(Value::Object(options)) if options.values().all(Value::is_string) => options,
    _ => Map::new(),
  }
}

fn summary_of(
  row: &QueryResult,
  rooms: &HashMap<i64, String>,
  shared: Option<ShareMark>,
) -> Result<Summary, Refusal> {
  let chat_id: i64 = row.try_get("", "chat_id")?;

  Ok(Summary {
    id: row.try_get("", "id")?,
    chat_id,
    room: rooms.get(&chat_id).cloned(),
    query: row.try_get("", "query")?,
    started_depth: row.try_get("", "started_depth")?,
    ended_depth: row.try_get("", "ended_depth")?,
    status: row.try_get("", "status")?,
    started_at: row.try_get("", "started_at")?,
    finished_at: row.try_get("", "finished_at")?,
    findings: row.try_get("", "findings")?,
    sources: row.try_get("", "sources")?,
    title: row.try_get("", "title")?,
    topic: row.try_get("", "topic")?,
    keywords: keywords_of(row.try_get("", "keywords")?),
    rule: row.try_get("", "rule")?,
    options: options_of(row.try_get("", "options")?),
    kind: row.try_get("", "kind")?,
    author: row.try_get("", "author")?,
    documented: row.try_get::<i64>("", "documented")? != 0,
    shared,
    excerpt: None,
  })
}

/// The phrase a list is searched by, its ends' white space dropped; empty is no search.
pub fn search_phrase(query: Option<&str>) -> String {
  query.unwrap_or_default().trim().to_owned()
}

// A text's characters lowered one to one, so a hit found in the lowered text stands at the same place in
// the text itself.
fn folded(text: &[char]) -> Vec<char> {
  text.iter().map(|one| one.to_lowercase().next().unwrap_or(*one)).collect()
}

/// One line of a long text around its first hit, its line breaks laid flat and an ellipsis where it was cut:
/// `lead` characters before the hit and `radius` after it.
pub fn hit_snippet(text: &str, phrase: &str, radius: usize, lead: usize) -> String {
  let flat: Vec<char> = WHITE_SPACE.replace_all(text, " ").chars().collect();
  let wanted = folded(&search_phrase(Some(phrase)).chars().collect::<Vec<_>>());
  let haystack = folded(&flat);

  if wanted.is_empty() {
    return String::new();
  }

  let Some(start) = haystack.windows(wanted.len()).position(|window| window == wanted.as_slice()) else {
    return String::new();
  };
  let end = start + wanted.len();
  let from = start.saturating_sub(lead);
  let to = (end + radius).min(flat.len());
  let cut: String = flat[from..to].iter().collect();

  format!(
    "{}{}{}",
    if from > 0 { "…" } else { "" },
    cut.trim(),
    if to < flat.len() { "…" } else { "" }
  )
}

static WHITE_SPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").expect("a pattern"));
static IMAGE_LINE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"^[ \t]*IMAGE:[ \t]*(\S+)(?:[ \t]*::[ \t]*(.*?))?[ \t]*$").expect("a pattern")
});
static MAP_LINE: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"^[ \t]*(MAP|ROADVIEW):[ \t]*(\S+)[ \t]*$").expect("a pattern"));
static REVIEW_LINE: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"^[ \t]*REVIEW:[ \t]*(.*?)[ \t]*$").expect("a pattern"));
static SEPARATOR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[ \t]*::[ \t]*").expect("a pattern"));
static SENTENCE_END: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new("[.,;:!?'\"\u{300d}\u{300f}\u{300b}\u{ff09}\u{3001}\u{3002}]+$").expect("a pattern")
});

fn web_address(value: &str) -> Option<String> {
  url::Url::parse(value)
    .ok()
    .filter(|url| matches!(url.scheme(), "http" | "https"))
    .map(|url| url.to_string())
}

fn comparable(url: &str) -> String {
  let unanchored = url.split('#').next().unwrap_or(url);

  unanchored.trim_end_matches('/').to_owned()
}

// The pictures and links one line of markdown writes, with where each stands in the line, read the way
// CommonMark reads them. An address written alone in angle brackets is a bare address, not a link.
struct Marked {
  pictures: Vec<(String, String)>,
  links: Vec<(String, String)>,
  spans: Vec<(usize, usize)>,
}

fn marked(line: &str) -> Marked {
  let mut read = Marked {
    pictures: Vec::new(),
    links: Vec::new(),
    spans: Vec::new(),
  };
  // What is being read: a picture or a link, its address, its words so far and where it started.
  let mut open: Vec<(bool, String, String, usize)> = Vec::new();

  for (event, range) in Parser::new_ext(line, Options::empty()).into_offset_iter() {
    match event {
      Event::Start(Tag::Image { dest_url, .. }) => {
        open.push((true, dest_url.into_string(), String::new(), range.start))
      }
      Event::Start(Tag::Link {
        link_type,
        dest_url,
        ..
      }) if !matches!(link_type, LinkType::Autolink | LinkType::Email) => {
        open.push((false, dest_url.into_string(), String::new(), range.start));
      }
      Event::End(TagEnd::Image) | Event::End(TagEnd::Link) => {
        let Some((picture, address, words, _)) = open.pop() else { continue };

        if let Some(outer) = open.last_mut() {
          outer.2.push_str(&words);
        }

        if picture {
          read.pictures.push((address, words));
        } else {
          read.links.push((address, words));
        }

        if open.is_empty() {
          read.spans.push((range.start, range.end));
        }
      }
      Event::Text(text) | Event::Code(text) => {
        if let Some(inner) = open.last_mut() {
          inner.2.push_str(&text);
        }
      }
      _ => {}
    }
  }

  read
}

// A line with every picture and link it writes laid over with a space.
fn without(line: &str, spans: &[(usize, usize)]) -> String {
  let mut out = String::new();
  let mut at = 0;

  for (start, end) in spans {
    out.push_str(&line[at..*start]);
    out.push(' ');
    at = *end;
  }

  out.push_str(&line[at..]);
  out
}

/// The pictures a finding's text shows and the links it mentions, in the order they are written. A link that
/// is one of the document's own sources is a source and is not listed again, and the address of a picture is
/// a picture, not a link.
pub fn finding_outside(
  text: &str,
  place: i64,
  source_urls: &HashSet<String>,
  words: &Search,
) -> (Vec<ImageView>, Vec<LinkView>) {
  let mut images: Vec<ImageView> = Vec::new();
  let mut links: Vec<LinkView> = Vec::new();
  let mut pictured = HashSet::new();
  let mut listed = HashSet::new();
  let lines: Vec<&str> = text.split('\n').collect();

  let mut picture = |raw: &str, caption: Option<String>, images: &mut Vec<ImageView>| {
    if let Some(url) = web_address(raw) {
      pictured.insert(comparable(&url));
      images.push(ImageView {
        r#ref: format!("i{place}.{}", images.len() + 1),
        url,
        caption: caption.filter(|caption| !caption.is_empty()),
      });
    }
  };

  for line in &lines {
    if let Some(image) = IMAGE_LINE.captures(line) {
      picture(&image[1], image.get(2).map(|caption| picture_caption(caption.as_str())), &mut images);
    }

    for (address, alt) in marked(line).pictures {
      picture(&address, Some(alt), &mut images);
    }
  }

  let mut link = |raw: &str, label: Option<String>, links: &mut Vec<LinkView>| {
    let Some(url) = web_address(&SENTENCE_END.replace(raw, "")) else { return };
    let key = comparable(&url);

    if source_urls.contains(&key) || pictured.contains(&key) || !listed.insert(key) {
      return;
    }

    links.push(LinkView {
      r#ref: format!("l{place}.{}", links.len() + 1),
      url,
      label,
    });
  };
  let reading = &words.reading;

  for line in &lines {
    if IMAGE_LINE.is_match(line) {
      continue;
    }

    if let Some(map) = MAP_LINE.captures(line) {
      let label = if &map[1] == "MAP" { &reading.map } else { &reading.street_view };

      link(&map[2], Some(label.clone()), &mut links);
      continue;
    }

    if let Some(review) = REVIEW_LINE.captures(line) {
      if let Some(page) = SEPARATOR.split(&review[1]).nth(2) {
        link(page, Some(reading.review.clone()), &mut links);
      }

      continue;
    }

    let written = marked(line);

    for (address, label) in &written.links {
      link(address, Some(label.clone()), &mut links);
    }

    for address in web_addresses_in(&without(line, &written.spans)) {
      link(&address.url, None, &mut links);
    }
  }

  (images, links)
}

// A document's rows: its head, its file, the runs that carried it, its findings and its sources.
struct Material {
  summary: Summary,
  document: Option<String>,
  runs: Vec<i64>,
  findings: Vec<FindingRow>,
  sources: Vec<SourceRow>,
}

// One finding's row.
struct FindingRow {
  run_id: Option<i64>,
  branch: Option<String>,
  place: i64,
  text: String,
}

// One source's row, with the place of the finding that cites it, none for a page opened and cited under
// nothing.
struct SourceRow {
  id: i64,
  finding_place: Option<i64>,
  url: String,
  title: Option<String>,
  supports: Option<String>,
  mark: Option<String>,
}

fn source_view(source: &SourceRow) -> SourceView {
  SourceView {
    r#ref: format!("s{}", source.id),
    url: source.url.clone(),
    title: source.title.clone(),
    supports: source.supports.clone(),
    mark: source.mark.clone(),
  }
}

/// Whether a path truly sits inside a folder, every link resolved; the folder itself is not inside it.
pub fn inside(path: &Path, folder: &Path) -> Option<PathBuf> {
  let real = path.canonicalize().ok()?;
  let root = folder.canonicalize().ok()?;

  (real != root && real.starts_with(&root)).then_some(real)
}

impl Library {
  /// Where the documents' folders are.
  pub fn searches(&self) -> PathBuf {
    self.root.join("searches")
  }

  // What every room is called, as bmo-receiver says; none where it cannot be asked, which costs the list
  // its rooms' names and nothing else.
  async fn rooms(&self) -> HashMap<i64, String> {
    match self.receiver.room_names().await {
      Ok(rooms) => rooms,
      Err(why) => {
        eprintln!("the rooms' names could not be read: {why}");
        HashMap::new()
      }
    }
  }

  /// A page of the list, newest first (`docs/archive.md`, "The console screen"). Holding it to what is
  /// shared, to one kind and to a phrase is done in the reading itself, so a filtered page is a full page,
  /// and what the foot of the list asks for next carries on from the last of its rows. It is a list of
  /// documents, so a search that left none is not in it.
  pub async fn list(&self, asked: &Asked) -> Result<Page, Refusal> {
    let phrase = search_phrase(asked.phrase.as_deref());
    let rooms = self.rooms().await;
    let lowered = phrase.to_lowercase();
    let says = |column: &str| format!("position($6 in lower({column})) > 0");
    let named: Vec<String> = rooms
      .iter()
      .filter(|(_, name)| !lowered.is_empty() && name.to_lowercase().contains(&lowered))
      .map(|(chat_id, _)| chat_id.to_string())
      .collect();
    let in_room = if named.is_empty() {
      String::new()
    } else {
      format!(" OR s.chat_id IN ({})", named.join(", "))
    };
    let sql = format!(
      r#"SELECT {}, (w.share_token IS NOT NULL) AS shared, w.expires_at AS shared_until
         FROM searches s
         LEFT JOIN web_search_shares w ON w.search_id = s.id AND (w.expires_at IS NULL OR w.expires_at > $1)
         WHERE ($2::bigint IS NULL OR s.id < $2) AND ($3 = 0 OR w.share_token IS NOT NULL)
           AND ($4::text IS NULL OR {KIND} = $4)
           AND ($6::text IS NULL OR {}{in_room}
             OR EXISTS (SELECT 1 FROM search_findings f WHERE f.search_id = s.id AND {}))
           AND COALESCE(s.documented, 1) = 1
         ORDER BY s.id DESC LIMIT $5"#,
      summary_columns(),
      ["s.title", "s.query", "s.topic", "s.keywords", "s.rule"].map(says).join(" OR "),
      says("f.text"),
    );
    let rows = all(
      &self.db,
      &sql,
      vec![
        asked.now.into(),
        asked.before.into(),
        i64::from(asked.shared).into(),
        asked.kind.clone().into(),
        (asked.limit + 1).into(),
        (!lowered.is_empty()).then(|| lowered.clone()).into(),
      ],
    )
    .await?;
    let mut items = Vec::new();

    for row in rows.iter().take(asked.limit.max(0) as usize) {
      let shared = row.try_get::<bool>("", "shared")?.then(|| -> Result<ShareMark, Refusal> {
        Ok(ShareMark {
          expires_at: row.try_get("", "shared_until")?,
        })
      });

      items.push(summary_of(row, &rooms, shared.transpose()?)?);
    }

    if !phrase.is_empty() {
      self.with_excerpts(&mut items, &phrase).await?;
    }

    Ok(Page {
      items,
      has_more: rows.len() as i64 > asked.limit,
    })
  }

  // The body's words around the phrase, for each row the phrase was found in the body of: the first finding,
  // in the document's order, that says it.
  async fn with_excerpts(&self, items: &mut [Summary], phrase: &str) -> Result<(), Refusal> {
    if items.is_empty() {
      return Ok(());
    }

    let ids: Vec<String> = items.iter().map(|item| item.id.to_string()).collect();
    let rows = all(
      &self.db,
      &format!(
        r#"SELECT search_id, text FROM search_findings WHERE search_id IN ({}) AND position($1 in lower(text)) > 0
           ORDER BY place, id"#,
        ids.join(", ")
      ),
      vec![phrase.to_lowercase().into()],
    )
    .await?;
    let mut excerpts: HashMap<i64, String> = HashMap::new();

    for row in rows {
      let id: i64 = row.try_get("", "search_id")?;
      let text: String = row.try_get("", "text")?;

      excerpts.entry(id).or_insert_with(|| hit_snippet(&text, phrase, HIT_RADIUS, HIT_LEAD));
    }

    for item in items {
      item.excerpt = Some(excerpts.remove(&item.id));
    }

    Ok(())
  }

  /// The live share link a document holds at `now`: when it ends, none where it has none.
  pub async fn shared_mark(&self, id: i64, now: i64) -> Result<Option<ShareMark>, Refusal> {
    let row = one(
      &self.db,
      r#"SELECT expires_at FROM web_search_shares WHERE search_id = $1 AND (expires_at IS NULL OR expires_at > $2)"#,
      vec![id.into(), now.into()],
    )
    .await?;

    row
      .map(|row| {
        Ok(ShareMark {
          expires_at: row.try_get("", "expires_at")?,
        })
      })
      .transpose()
  }

  async fn material(&self, id: i64) -> Result<Option<Material>, Refusal> {
    let Some(row) = one(
      &self.db,
      &format!("SELECT {}, s.document AS document FROM searches s WHERE s.id = $1", summary_columns()),
      vec![id.into()],
    )
    .await?
    else {
      return Ok(None);
    };
    let rooms = self.rooms().await;
    let summary = summary_of(&row, &rooms, None)?;
    let document: Option<String> = row.try_get("", "document")?;
    let runs = match self.job.runs_of_search(id).await {
      Ok(runs) => runs,
      Err(why) => {
        eprintln!("the runs that carried document {id} could not be listed: {why}");
        Vec::new()
      }
    };
    let findings = all(
      &self.db,
      "SELECT run_id, branch, place, text FROM search_findings WHERE search_id = $1 ORDER BY place, id",
      vec![id.into()],
    )
    .await?
    .iter()
    .map(|row| {
      Ok(FindingRow {
        run_id: row.try_get("", "run_id")?,
        branch: row.try_get("", "branch")?,
        place: row.try_get("", "place")?,
        text: row.try_get("", "text")?,
      })
    })
    .collect::<Result<Vec<_>, sea_orm::DbErr>>()?;
    // A cited source is joined to its finding within the same search, so a row that names a finding of some
    // other search is not taken for one of this search's.
    let sources = all(
      &self.db,
      r#"SELECT r.id AS id, f.place AS finding_place, r.url AS url, r.title AS title, r.supports AS supports, r.mark AS mark
         FROM search_sources r LEFT JOIN search_findings f ON f.id = r.finding_id AND f.search_id = r.search_id
         WHERE r.search_id = $1 ORDER BY r.id"#,
      vec![id.into()],
    )
    .await?
    .iter()
    .map(|row| {
      Ok(SourceRow {
        id: row.try_get("", "id")?,
        finding_place: row.try_get("", "finding_place")?,
        url: row.try_get("", "url")?,
        title: row.try_get("", "title")?,
        supports: row.try_get("", "supports")?,
        mark: row.try_get("", "mark")?,
      })
    })
    .collect::<Result<Vec<_>, sea_orm::DbErr>>()?;

    Ok(Some(Material {
      summary,
      document,
      runs,
      findings,
      sources,
    }))
  }

  fn finding_views(&self, material: &Material) -> Vec<FindingView> {
    let source_urls: HashSet<String> =
      material.sources.iter().map(|source| comparable(&source.url)).collect();

    material
      .findings
      .iter()
      .map(|finding| {
        let (images, links) = finding_outside(&finding.text, finding.place, &source_urls, &self.words);

        FindingView {
          place: finding.place,
          run_id: finding.run_id,
          branch: finding.branch.clone(),
          text: finding.text.clone(),
          images,
          links,
          sources: material
            .sources
            .iter()
            .filter(|source| source.finding_place == Some(finding.place))
            .map(source_view)
            .collect(),
        }
      })
      .collect()
  }

  // The document file, only where it truly sits inside the documents' folder with every link resolved.
  fn document_path(&self, material: &Material) -> Option<PathBuf> {
    inside(Path::new(material.document.as_deref()?), &self.searches())
  }

  /// One document read whole, none where there is no such document.
  pub async fn detail(&self, id: i64, now: i64) -> Result<Option<Detail>, Refusal> {
    let Some(material) = self.material(id).await? else { return Ok(None) };
    let shared = self.shared_mark(id, now).await?;
    let findings = self.finding_views(&material);
    let kind = kind(Some(&material.summary.kind));
    let outlined: Vec<OutlineFinding> = findings
      .iter()
      .map(|finding| OutlineFinding {
        text: finding.text.clone(),
        place: finding.place,
      })
      .collect();
    let document = self.document_path(&material).is_some();

    Ok(Some(Detail {
      kind: kind.view(),
      outline: kind.outline(&outlined, &self.words),
      findings,
      opened: material
        .sources
        .iter()
        .filter(|source| source.finding_place.is_none())
        .map(source_view)
        .collect(),
      attachments: self.attachments(id).await?,
      search: DetailHead {
        summary: Summary {
          shared,
          ..material.summary
        },
        runs: material.runs,
        document,
      },
    }))
  }

  /// One document read for somebody holding its share link, which is less of it: its name, its kind, how
  /// many runs wrote it, what it found, the pages it stands on and its attachments — never its state, its
  /// room, its question, its options, its depth, its times or its link.
  pub async fn shared(&self, id: i64, now: i64) -> Result<Option<SharedDetail>, Refusal> {
    let Some(detail) = self.detail(id, now).await? else { return Ok(None) };
    let summary = detail.search.summary;

    Ok(Some(SharedDetail {
      title: summary.title.unwrap_or(summary.query),
      run_count: detail.search.runs.len(),
      kind: summary.kind,
      findings: detail.findings,
      opened: detail.opened,
      attachments: detail.attachments,
    }))
  }

  /// The attachments of one document that stand in its attachment section: every file but a picture a
  /// result named by its address, which the body's own line draws where it stands.
  pub async fn attachments(&self, id: i64) -> Result<Vec<AttachmentView>, Refusal> {
    let rows = all(
      &self.db,
      r#"SELECT id, media, type, bytes, name FROM archive_attachments
         WHERE search_id = $1 AND NOT (media = 'image' AND url IS NOT NULL) ORDER BY id"#,
      vec![id.into()],
    )
    .await?;

    rows
      .iter()
      .map(|row| {
        Ok(AttachmentView {
          id: row.try_get("", "id")?,
          media: row.try_get("", "media")?,
          r#type: row.try_get("", "type")?,
          bytes: row.try_get("", "bytes")?,
          name: row.try_get("", "name")?,
        })
      })
      .collect()
  }

  /// One attachment's file, only where it belongs to that document and truly sits inside that document's own
  /// folder. The attachment is looked up under the document, so a number that is another document's finds
  /// nothing, and the path is the archive's, which is not trusted to have stayed where it belongs.
  pub async fn attachment_file(&self, id: i64, attachment: i64) -> Result<Option<KeptFile>, Refusal> {
    let Some(row) = one(
      &self.db,
      "SELECT path, type, name FROM archive_attachments WHERE id = $1 AND search_id = $2",
      vec![attachment.into(), id.into()],
    )
    .await?
    else {
      return Ok(None);
    };
    let path: String = row.try_get("", "path")?;
    let Some(real) = inside(Path::new(&path), &self.searches().join(id.to_string())) else {
      return Ok(None);
    };
    let Some(stat) = std::fs::metadata(&real).ok().filter(std::fs::Metadata::is_file) else {
      return Ok(None);
    };

    Ok(Some(KeptFile {
      path: real,
      r#type: row.try_get("", "type")?,
      bytes: stat.len(),
      name: row.try_get("", "name")?,
    }))
  }

  /// The page a document is, where its kind is a page of its own: the first page it keeps, found through the
  /// same guards as any attachment. None for every other document, and for one whose page is missing.
  pub async fn hosted_page_file(&self, id: i64) -> Result<Option<KeptFile>, Refusal> {
    let Some(row) = one(
      &self.db,
      &format!("SELECT {KIND} AS kind FROM searches s WHERE s.id = $1"),
      vec![id.into()],
    )
    .await?
    else {
      return Ok(None);
    };
    let word: String = row.try_get("", "kind")?;

    if !kind(Some(&word)).hosted() {
      return Ok(None);
    }

    let Some(page) = one(
      &self.db,
      "SELECT id FROM archive_attachments WHERE search_id = $1 AND media = 'page' ORDER BY id LIMIT 1",
      vec![id.into()],
    )
    .await?
    else {
      return Ok(None);
    };

    self.attachment_file(id, page.try_get("", "id")?).await
  }

  /// A document's file, only where it truly sits inside the documents' folder.
  pub async fn document_file(&self, id: i64) -> Result<Option<PathBuf>, Refusal> {
    Ok(self.material(id).await?.and_then(|material| self.document_path(&material)))
  }

  /// The address a ref names in one document's record, none where the record has no such place.
  pub async fn resolve_ref(&self, id: i64, reference: &str) -> Result<Option<(RefKind, String)>, Refusal> {
    let Some(material) = self.material(id).await? else { return Ok(None) };

    if let Some(number) = reference
      .strip_prefix('s')
      .filter(|rest| !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit()))
    {
      let found = number
        .parse::<i64>()
        .ok()
        .and_then(|number| material.sources.iter().find(|source| source.id == number));

      return Ok(found.map(|source| (RefKind::Source, source.url.clone())));
    }

    let (picture, rest) = match reference.chars().next() {
      Some('i') => (true, &reference[1..]),
      Some('l') => (false, &reference[1..]),
      _ => return Ok(None),
    };
    let Some((place, _)) = rest.split_once('.') else { return Ok(None) };
    let numbered = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());

    if !rest.split('.').all(numbered) || rest.split('.').count() != 2 {
      return Ok(None);
    }

    let place: i64 = place.parse().unwrap_or(-1);
    let findings = self.finding_views(&material);
    let Some(finding) = findings.iter().find(|finding| finding.place == place) else {
      return Ok(None);
    };
    let hit = if picture {
      finding
        .images
        .iter()
        .find(|image| image.r#ref == reference)
        .map(|image| (RefKind::Image, image.url.clone()))
    } else {
      finding
        .links
        .iter()
        .find(|link| link.r#ref == reference)
        .map(|link| (RefKind::Link, link.url.clone()))
    };

    Ok(hit)
  }

  // The copy the document kept of one picture, where it kept one and the file is still there inside the
  // documents' folder. It is preferred over fetching the address afresh: the address is where the picture
  // was found, the copy is the picture.
  async fn kept_image(&self, id: i64, url: &str) -> Result<Option<Raster>, Refusal> {
    let Some(row) = one(
      &self.db,
      "SELECT path, type FROM archive_attachments WHERE search_id = $1 AND url = $2 ORDER BY id DESC LIMIT 1",
      vec![id.into(), url.into()],
    )
    .await?
    else {
      return Ok(None);
    };
    let path: String = row.try_get("", "path")?;
    let Some(real) = inside(Path::new(&path), &self.searches()) else { return Ok(None) };

    let Ok(bytes) = std::fs::read(real) else { return Ok(None) };

    Ok(Some(Raster {
      kind: row.try_get("", "type")?,
      bytes,
    }))
  }

  /// What a source or a link shows of itself; none for a picture's ref and for a place the record does not
  /// hold.
  pub async fn ref_preview(&self, id: i64, reference: &str) -> Result<Option<RefPreview>, Refusal> {
    let Some((found, url)) = self.resolve_ref(id, reference).await? else {
      return Ok(None);
    };

    if found == RefKind::Image || web_address(&url).is_none() {
      return Ok(None);
    }

    let preview = self.previews.preview(&url).await;

    Ok(Some(RefPreview {
      title: preview.as_ref().and_then(|preview| preview.title.clone()),
      thumbnail: preview.as_ref().is_some_and(|preview| preview.image.is_some()),
      favicon: preview.as_ref().is_some_and(|preview| preview.icon.is_some()),
    }))
  }

  /// The picture a ref stands for: a picture ref is itself one; a source or a link has the picture and the
  /// icon its page named, which are fetched only as that page's preview said.
  pub async fn ref_image(
    &self,
    id: i64,
    reference: &str,
    asked: RefPicture,
  ) -> Result<Option<Raster>, Refusal> {
    let Some((found, url)) = self.resolve_ref(id, reference).await? else {
      return Ok(None);
    };

    if web_address(&url).is_none() {
      return Ok(None);
    }

    if found == RefKind::Image {
      if asked != RefPicture::Own {
        return Ok(None);
      }

      return Ok(match self.kept_image(id, &url).await? {
        Some(kept) => Some(kept),
        None => self.previews.image(&url).await,
      });
    }

    if asked == RefPicture::Own {
      return Ok(None);
    }

    let Some(preview) = self.previews.preview(&url).await else { return Ok(None) };
    let picture = if asked == RefPicture::Thumb { preview.image } else { preview.icon };

    Ok(match picture {
      Some(picture) => self.previews.image(&picture).await,
      None => None,
    })
  }
}

/// The documents of the catalogue's kinds, as the list's filter offers them.
pub fn kinds_view() -> Vec<Value> {
  crate::kinds::KINDS.iter().map(|kind| kind.view()).collect()
}

/// The kind a word names, none where the catalogue holds no such kind: a word the catalogue does not know
/// is no filter at all, rather than an empty page.
pub fn listed_kind(word: Option<&str>) -> Option<String> {
  crate::kinds::known(word).map(|kind| kind.id().to_owned())
}
