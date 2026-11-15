//! Publishing (`docs/api.md`, "Publications"): a document filed and a link to it opened in one call, so a
//! caller hands somebody one address that keeps opening what was filed, the newest body of it alike.

use axum::http::{Method, StatusCode};
use axum::response::Response;
use schemars::generate::SchemaGenerator;

use super::documents::document;
use super::readers::Reader;
use super::shares::sweep;
use crate::Refusal;
use crate::documents::filing;
use crate::events::EventKind;
use crate::now_ms;
use crate::plumbing::http::{Call, Refused, Route, answer};
use crate::plumbing::openapi::{Operation, json_answer, json_body};
use crate::store::shares;
use crate::wire::{DocumentInput, Publication};

pub struct PublishDocument;

impl Route for PublishDocument {
  const METHOD: Method = Method::POST;
  const PATH: &'static str = "/publications";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let reader = Reader::of(&call);
    let input: DocumentInput = call.json().await?;
    let standing = call.standing();
    let db = standing.db().await?;
    let id = filing::file(db, &standing.settings.files.root, &input, reader.owner()).await?;
    let filed = document(standing, id).await?.document;

    standing
      .events
      .publish(db, EventKind::DocumentCreated, filed.scope.as_deref(), &filed)
      .await?;

    sweep(standing).await?;

    // A link with no end: it names the document, so whoever holds it reads the body the document stands with,
    // whenever they open it and however often the document is rewritten.
    let link = match shares::create(db, id, "", now_ms() / 1000, None).await? {
      Some(made) => made,
      None => shares::of_document(db, id)
        .await?
        .ok_or_else(|| Refusal::failed("the share link was not written"))?,
    };
    let share = link.wire(&standing.settings.share.address);

    standing
      .events
      .publish(db, EventKind::ShareCreated, filed.scope.as_deref(), &share)
      .await?;

    Ok(answer(
      StatusCode::CREATED,
      &Publication {
        document: filed,
        share,
      },
    ))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "createPublications",
      tag: "publications",
      summary: "File a document and open a link to it with no end.",
      parameters: Vec::new(),
      body: json_body::<DocumentInput>(generator),
      answers: vec![(201, json_answer::<Publication>(generator))],
      codes: &["unauthorized", "bad-request"],
    }
  }
}

crate::route!(PublishDocument);
