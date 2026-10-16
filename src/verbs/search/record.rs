//! A search as a record that stands (`docs/archive.md`, "`searches`" and "Findings and sources"): one row a
//! question, the depth of the run carrying it noted on it, and at the run's end the claims and their pages
//! put under it, its head noted, the row closed with the run's own word, and the document a person reads
//! written under the file root. A search whose answer was one value keeps its row and its head and writes
//! nothing else. Which runs carried a search is bmo-job's to keep, and is handed in by the caller.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono_tz::Tz;
use sea_orm::DatabaseConnection;

use super::document::{
  self, Finding, HeldFinding, HeldSource, OpenedPage, Record, ResultHead, attribute_branches,
  leaves_no_document, read_stream, result_findings, result_head, uncited_pages,
};
use super::request::Options;
use crate::http::rooms::Kept;
use crate::wording::Wording;
use crate::{Refusal, all, one, run};

crate::room_holding!(Kept {
  table: "searches",
  column: "chat_id",
  why: "a search is a record that stands, with its document, after the room it was asked in",
});

/// The status a search stands in while it is open.
pub const OPEN_STATUS: &str = "open";

/// Who writes a search's document: the searcher, not the agent by hand.
pub const SEARCHER_AUTHOR: &str = "searcher";

/// A search's row as its launcher opens it.
pub struct NewSearch<'a> {
  pub chat_id: i64,
  pub query: &'a str,
  pub depth: &'a str,
  pub asker_id: Option<i64>,
  pub options: &'a Options,
  /// The kind of document it is: a page imported whole, study material asked for by its shape, or a search.
  pub kind: &'a str,
}

/// The search's own row opened: its number.
pub async fn start_search(db: &DatabaseConnection, search: &NewSearch<'_>) -> Result<i64, Refusal> {
  let options =
    (!search.options.named().is_empty()).then(|| serde_json::to_string(search.options).unwrap_or_default());

  Ok(
    one(
      db,
      r#"INSERT INTO "searches" ("chat_id", "query", "started_depth", "ended_depth", "status", "started_at", "asker_id", "options", "kind", "author")
         VALUES ($1, $2, $3, $3, $4, $5, $6, $7, $8, $9) RETURNING "id""#,
      vec![
        search.chat_id.into(),
        search.query.into(),
        search.depth.into(),
        OPEN_STATUS.into(),
        (crate::now_ms() / 1000).into(),
        search.asker_id.into(),
        options.into(),
        search.kind.into(),
        SEARCHER_AUTHOR.into(),
      ],
    )
    .await?
    .ok_or_else(|| Refusal::bad("the search's row was not opened"))?
    .try_get("", "id")?,
  )
}

/// An earlier search of the room taken up again by a follow-up, which writes into its record and its document
/// under the same number and share link: opened once more, and the state it stood in answered so a launch that
/// never came up can put it back. None where it is no finished search of that room — another room's, a
/// document the agent wrote, a page imported whole, or one still running.
pub async fn reopen_search(
  db: &DatabaseConnection,
  search_id: i64,
  chat_id: i64,
) -> Result<Option<String>, Refusal> {
  let Some(standing) = one(
    db,
    r#"SELECT "status" FROM "searches" WHERE "id" = $1 AND "chat_id" = $2 AND COALESCE("author", $3) = $3
         AND COALESCE("kind", '') <> 'import' AND "status" <> $4"#,
    vec![search_id.into(), chat_id.into(), SEARCHER_AUTHOR.into(), OPEN_STATUS.into()],
  )
  .await?
  else {
    return Ok(None);
  };
  let status: String = standing.try_get("", "status")?;
  let reopened = one(
    db,
    r#"UPDATE "searches" SET "status" = $1, "finished_at" = NULL WHERE "id" = $2 AND "status" = $3 RETURNING "id""#,
    vec![OPEN_STATUS.into(), search_id.into(), status.clone().into()],
  )
  .await?;

  Ok(reopened.map(|_| status))
}

/// A run taking the search up: the search notes the run's depth as where it has got to.
pub async fn join_search(db: &DatabaseConnection, search_id: i64, depth: &str) -> Result<(), Refusal> {
  run(
    db,
    r#"UPDATE "searches" SET "ended_depth" = $1 WHERE "id" = $2"#,
    vec![depth.into(), search_id.into()],
  )
  .await?;

  Ok(())
}

/// How the search ended, in the words a run's outcome uses.
pub async fn finish_search(db: &DatabaseConnection, search_id: i64, status: &str) -> Result<(), Refusal> {
  run(
    db,
    r#"UPDATE "searches" SET "finished_at" = $1, "status" = $2 WHERE "id" = $3"#,
    vec![(crate::now_ms() / 1000).into(), status.into(), search_id.into()],
  )
  .await?;

  Ok(())
}

// The head a result gave its document, field by field: the first run's stands, since a follow-up carrying
// the search on looks at a narrower part of what the document is about, and a later run's only fills what
// is still empty.
async fn note_head(db: &DatabaseConnection, search_id: i64, head: &ResultHead) -> Result<(), Refusal> {
  let keywords =
    (!head.keywords.is_empty()).then(|| serde_json::to_string(&head.keywords).unwrap_or_default());

  run(
    db,
    r#"UPDATE "searches" SET "title" = COALESCE("title", $1), "topic" = COALESCE("topic", $2), "keywords" = COALESCE("keywords", $3),
         "rule" = COALESCE("rule", $4), "kind" = CASE WHEN $5::text IS NULL OR "title" IS NOT NULL THEN "kind" ELSE $5 END WHERE "id" = $6"#,
    vec![
      head.title.clone().into(),
      head.topic.clone().into(),
      keywords.into(),
      head.rule.clone().into(),
      head.kind.clone().into(),
      search_id.into(),
    ],
  )
  .await?;

  Ok(())
}

async fn write_source(
  db: &DatabaseConnection,
  search_id: i64,
  finding_id: Option<i64>,
  url: &str,
  supports: Option<&str>,
  mark: Option<&str>,
) -> Result<(), Refusal> {
  run(
    db,
    r#"INSERT INTO "search_sources" ("search_id", "finding_id", "url", "title", "supports", "mark") VALUES ($1, $2, $3, NULL, $4, $5)"#,
    vec![
      search_id.into(),
      finding_id.into(),
      url.into(),
      supports.map(str::to_owned).into(),
      mark.map(str::to_owned).into(),
    ],
  )
  .await?;

  Ok(())
}

// One claim and the pages cited under it, placed after whatever the search already holds, so a deeper run
// carries the document on rather than starting it again.
async fn add_finding(
  db: &DatabaseConnection,
  search_id: i64,
  run_id: Option<i64>,
  finding: &Finding,
) -> Result<(), Refusal> {
  let last: Option<i64> = one(
    db,
    r#"SELECT MAX("place") AS "place" FROM "search_findings" WHERE "search_id" = $1"#,
    vec![search_id.into()],
  )
  .await?
  .map(|row| row.try_get::<Option<i64>>("", "place"))
  .transpose()?
  .flatten();
  let id: i64 = one(
    db,
    r#"INSERT INTO "search_findings" ("search_id", "run_id", "branch", "place", "text") VALUES ($1, $2, $3, $4, $5) RETURNING "id""#,
    vec![
      search_id.into(),
      run_id.into(),
      finding.branch.clone().into(),
      (last.unwrap_or(0) + 1).into(),
      finding.text.as_str().into(),
    ],
  )
  .await?
  .ok_or_else(|| Refusal::bad("the finding's row was not written"))?
  .try_get("", "id")?;

  for source in &finding.sources {
    write_source(db, search_id, Some(id), &source.url, source.supports.as_deref(), source.mark.as_deref())
      .await?;
  }

  Ok(())
}

async fn add_pages(db: &DatabaseConnection, search_id: i64, pages: &[OpenedPage]) -> Result<(), Refusal> {
  for page in pages {
    write_source(db, search_id, None, &page.url, None, None).await?;
  }

  Ok(())
}

/// The whole of one search as its document is written from, carried by `runs` as bmo-job lists them; none for
/// a search the archive does not hold.
pub async fn read_search(
  db: &DatabaseConnection,
  search_id: i64,
  runs: &[i64],
) -> Result<Option<Record>, Refusal> {
  let Some(row) = one(
    db,
    r#"SELECT "query", "started_depth", "ended_depth", "status", "started_at", "finished_at", "options", "title", "topic", "keywords", "rule"
       FROM "searches" WHERE "id" = $1"#,
    vec![search_id.into()],
  )
  .await?
  else {
    return Ok(None);
  };
  let finding_rows = all(
    db,
    r#"SELECT "id", "run_id", "branch", "text" FROM "search_findings" WHERE "search_id" = $1 ORDER BY "place", "id""#,
    vec![search_id.into()],
  )
  .await?;
  let source_rows = all(
    db,
    r#"SELECT "finding_id", "url", "supports", "mark" FROM "search_sources" WHERE "search_id" = $1 ORDER BY "id""#,
    vec![search_id.into()],
  )
  .await?;
  let mut sources: Vec<(Option<i64>, HeldSource)> = vec![];

  for source in &source_rows {
    sources.push((
      source.try_get("", "finding_id")?,
      HeldSource {
        url: source.try_get("", "url")?,
        supports: source.try_get("", "supports")?,
        mark: source.try_get("", "mark")?,
      },
    ));
  }

  let under = |finding: Option<i64>| -> Vec<HeldSource> {
    sources
      .iter()
      .filter(|(of, _)| *of == finding)
      .map(|(_, source)| source.clone())
      .collect()
  };
  let mut findings = vec![];

  for finding in &finding_rows {
    findings.push(HeldFinding {
      run_id: finding.try_get("", "run_id")?,
      branch: finding.try_get("", "branch")?,
      text: finding.try_get("", "text")?,
      sources: under(Some(finding.try_get("", "id")?)),
    });
  }

  // A row's options are the launch's own, written in the order a launch reads them, and read back in it.
  let options: Vec<(String, String)> = row
    .try_get::<Option<String>>("", "options")?
    .and_then(|text| serde_json::from_str::<Options>(&text).ok())
    .map(|options| {
      options
        .named()
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect()
    })
    .unwrap_or_default();
  let keywords: Vec<String> = row
    .try_get::<Option<String>>("", "keywords")?
    .and_then(|text| serde_json::from_str(&text).ok())
    .unwrap_or_default();

  Ok(Some(Record {
    query: row.try_get("", "query")?,
    started_depth: row.try_get("", "started_depth")?,
    ended_depth: row.try_get("", "ended_depth")?,
    status: row.try_get("", "status")?,
    started_at: row.try_get("", "started_at")?,
    finished_at: row.try_get("", "finished_at")?,
    options,
    head: ResultHead {
      title: row.try_get("", "title")?,
      topic: row.try_get("", "topic")?,
      keywords,
      rule: row.try_get("", "rule")?,
      kind: None,
    },
    runs: runs.to_vec(),
    findings,
    pages: under(None),
  }))
}

/// Every picture one search kept of an address its result named, by that address: where its copy is.
pub async fn kept_pictures(
  db: &DatabaseConnection,
  search_id: i64,
) -> Result<BTreeMap<String, String>, Refusal> {
  let rows = all(
    db,
    r#"SELECT "url", "path" FROM "archive_attachments" WHERE "search_id" = $1 AND "media" = 'image' AND "url" IS NOT NULL ORDER BY "id""#,
    vec![search_id.into()],
  )
  .await?;
  let mut kept = BTreeMap::new();

  for row in rows {
    kept.insert(row.try_get("", "url")?, row.try_get("", "path")?);
  }

  Ok(kept)
}

/// How a search run ended, as its record is written: the search it answered, the run, the runs that have
/// carried the search, what it found in its own words, everything it streamed, and the word the run was closed
/// with.
pub struct Ending<'a> {
  pub search_id: i64,
  pub run_id: Option<i64>,
  /// Every run that has carried the search, this one included, as bmo-job lists them.
  pub runs: &'a [i64],
  pub result: Option<&'a str>,
  pub stream: &'a str,
  pub outcome: &'a str,
  /// Where the searches' documents are written: the file root's `searches`.
  pub artifacts: &'a Path,
  /// The words the document is written in.
  pub wording: &'a Wording,
  /// The time zone the document tells its times in.
  pub zone: Tz,
}

/// The run's ending put away as the search's: its claims and pages into the record, its head, the search
/// closed with the run's word, and its document written where there is material for one; where it was written.
pub async fn record_run(db: &DatabaseConnection, ending: &Ending<'_>) -> Result<Option<PathBuf>, Refusal> {
  let search_id = ending.search_id;

  if let Some(result) = ending.result
    && leaves_no_document(result)
  {
    note_head(db, search_id, &result_head(result)).await?;
    run(db, r#"UPDATE "searches" SET "documented" = 0 WHERE "id" = $1"#, vec![search_id.into()]).await?;
    finish_search(db, search_id, ending.outcome).await?;

    return Ok(None);
  }

  let trace = read_stream(ending.stream, ending.run_id);
  let findings = attribute_branches(result_findings(ending.result.unwrap_or_default()), &trace);

  for finding in &findings {
    add_finding(db, search_id, ending.run_id, finding).await?;
  }

  if let Some(result) = ending.result {
    note_head(db, search_id, &result_head(result)).await?;
  }

  let held: Vec<String> = read_search(db, search_id, ending.runs)
    .await?
    .map(|search| search.pages.into_iter().map(|page| page.url).collect())
    .unwrap_or_default();

  add_pages(db, search_id, &uncited_pages(&trace, &findings, &held)).await?;
  finish_search(db, search_id, ending.outcome).await?;

  let Some(search) = read_search(db, search_id, ending.runs).await? else {
    return Ok(None);
  };

  if search.findings.is_empty() && search.pages.is_empty() {
    return Ok(None);
  }

  let kept = kept_pictures(db, search_id).await?;
  let text = document::render(&search, &trace, &kept, ending.wording, ending.zone);
  let path =
    write_document(ending.artifacts, search_id, &text).map_err(|why| Refusal::bad(why.to_string()))?;

  run(
    db,
    r#"UPDATE "searches" SET "document" = $1, "documented" = 1 WHERE "id" = $2"#,
    vec![path.to_string_lossy().into_owned().into(), search_id.into()],
  )
  .await?;

  Ok(Some(path))
}

// The document written owner-only under the search's own folder, whole or not at all.
fn write_document(artifacts: &Path, search_id: i64, text: &str) -> std::io::Result<PathBuf> {
  use std::io::Write;
  use std::os::unix::fs::OpenOptionsExt;

  let directory = artifacts.join(search_id.to_string());

  std::fs::create_dir_all(&directory)?;

  let path = directory.join("document.md");
  let staged = directory.join("document.md.tmp");
  let mut file = std::fs::OpenOptions::new()
    .write(true)
    .create(true)
    .truncate(true)
    .mode(0o600)
    .open(&staged)?;

  file.write_all(text.as_bytes())?;
  std::fs::rename(&staged, &path)?;

  Ok(path)
}
