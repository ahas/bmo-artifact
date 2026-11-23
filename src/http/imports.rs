//! The imports (`docs/api.md`, "Imports"): one asked for and answered at once, `fetching`, the page then
//! read and filed in a task of its own; and one read back.

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use axum::response::Response;
use schemars::generate::SchemaGenerator;

use super::readers::Reader;
use crate::Refusal;
use crate::documents::import::{Asked, import_page};
use crate::events::EventKind;
use crate::plumbing::http::{Call, Refused, Route, Standing, answer};
use crate::plumbing::openapi::{Operation, id_parameter, json_answer, json_body};
use crate::store::{documents, imports};
use crate::wire::{Import, ImportInput};

/// What an import still `fetching` is failed with once the process that read it is gone.
pub const STOPPED: &str = "the service stopped before the page was read";

/// Every import a stopped process left `fetching` failed, each told as changed.
pub async fn abandon(standing: &Standing) -> Result<(), Refused> {
  let db = standing.db().await?;

  for import in imports::abandon(db, STOPPED).await? {
    standing.events.publish(db, EventKind::ImportChanged, None, &import).await?;
  }

  Ok(())
}

// The page read and filed, the import moved on and every change told.
async fn run(standing: Arc<Standing>, asked: imports::Asked) -> Result<(), Refused> {
  let db = standing.db().await?;
  let scope = asked.scope.as_deref();
  let read = import_page(
    db,
    &standing.places,
    standing.reader.as_ref(),
    &standing.wording.import,
    Asked {
      url: &asked.import.url,
      title: asked.title.as_deref(),
      scope,
      actor_name: &asked.actor_name,
      owner: asked.owner.as_deref(),
    },
  )
  .await;
  let settled = match &read {
    Ok(imported) => {
      if let Some(row) = documents::read(db, imported.document_id).await? {
        standing.events.publish(db, EventKind::DocumentCreated, scope, &row.document).await?;
      }

      for attachment in &imported.attachments {
        standing.events.publish(db, EventKind::AttachmentCreated, scope, attachment).await?;
      }

      imports::settle(db, asked.import.id, Ok(imported.document_id)).await?
    }
    Err(why) => imports::settle(db, asked.import.id, Err(why)).await?,
  };

  if let Some(import) = settled {
    standing.events.publish(db, EventKind::ImportChanged, scope, &import).await?;
  }

  Ok(())
}

pub struct MakeImport;

impl Route for MakeImport {
  const METHOD: Method = Method::POST;
  const PATH: &'static str = "/imports";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let input: ImportInput = call.json().await?;
    let web = url::Url::parse(input.url.trim()).is_ok_and(|url| matches!(url.scheme(), "http" | "https"));

    if !web {
      return Err(Refused::bad(format!("{:?} is not an http or https address", input.url)));
    }

    if input.actor_name.trim().is_empty() {
      return Err(Refused::bad("an import needs an actorName"));
    }

    let reader = Reader::of(&call);
    let standing = call.shared();
    let db = standing.db().await?;
    let asked = imports::insert(db, &input, reader.owner()).await?;
    let made = asked.import.clone();

    standing
      .events
      .publish(db, EventKind::ImportCreated, asked.scope.as_deref(), &made)
      .await?;
    tokio::spawn(async move {
      let id = asked.import.id;

      if let Err(why) = run(Arc::clone(&standing), asked).await {
        // Whatever stopped the reading short still ends the import, so nobody waits on it.
        if let Ok(db) = standing.db().await
          && let Ok(Some(import)) = imports::settle(db, id, Err(&why.error)).await
        {
          let _ = standing.events.publish(db, EventKind::ImportChanged, None, &import).await;
        }
      }
    });

    Ok(answer(StatusCode::CREATED, &made))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "createImports",
      tag: "imports",
      summary: "Import a page: fetched through the explorer client and filed as a document.",
      parameters: Vec::new(),
      body: json_body::<ImportInput>(generator),
      answers: vec![(201, json_answer::<Import>(generator))],
      codes: &["unauthorized", "bad-request"],
    }
  }
}

crate::route!(MakeImport);

pub struct ReadImport;

impl Route for ReadImport {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/imports/{importId}";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = call.path_id("importId").await?;
    let reader = Reader::of(&call);
    // An account reads the imports it asked for alone; any other reads to it as one that does not stand.
    let asked = imports::read(call.standing().db().await?, id)
      .await?
      .filter(|asked| matches!(reader, Reader::Every) || asked.owner.as_deref() == reader.owner())
      .ok_or_else(|| Refusal::absent(format!("there is no import {id}")))?;

    Ok(answer(StatusCode::OK, &asked.import))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "readImportsByImportId",
      tag: "imports",
      summary: "One import: fetching, filed with its document, or why it failed.",
      parameters: vec![id_parameter("importId")],
      body: None,
      answers: vec![(200, json_answer::<Import>(generator))],
      codes: &["unauthorized", "bad-request", "not-found"],
    }
  }
}

crate::route!(ReadImport);
