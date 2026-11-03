//! The share links (`docs/api.md`, "Share links"): listed, made, read and taken away, the links whose time
//! has passed swept on every read and every making, and what each link opened kept where it has ended.

use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse, Response};
use schemars::generate::SchemaGenerator;
use sea_orm::DatabaseConnection;
use serde_json::json;

use super::documents::document;
use crate::Refusal;
use crate::events::EventKind;
use crate::now_ms;
use crate::plumbing::http::{Call, ConsoleReach, Refused, Route, Standing, answer};
use crate::plumbing::openapi::{Operation, json_answer, json_body, query, text_parameter};
use crate::store::{documents, page_limit, shares};
use crate::wire::{Share, ShareInput, SharePage, SharesQuery};

/// The links whose time has passed swept, each told as taken away.
pub async fn sweep(standing: &Standing) -> Result<(), Refused> {
  let db = standing.db().await?;

  for token in shares::sweep(db, now_ms() / 1000).await? {
    standing
      .events
      .publish(db, EventKind::ShareDeleted, None, &json!({ "token": token }))
      .await?;
  }

  Ok(())
}

/// The scope of the document a link opens, for the events about it.
async fn scope_of(standing: &Standing, document_id: i64) -> Result<Option<String>, Refused> {
  Ok(
    documents::read(standing.db().await?, document_id)
      .await?
      .and_then(|row| row.document.scope),
  )
}

/// The path parameter a share link's token is named by.
pub fn token_parameter() -> serde_json::Value {
  text_parameter("token", "The share link's token.")
}

/// What a token that names no live link answers: what that link opened where it has ended, and nothing at all
/// where it never named one (`docs/api.md`, "Share links").
async fn no_live_link(db: &DatabaseConnection, token: &str) -> Result<Refused, Refusal> {
  let Some(note) = shares::note_by_token(db, token).await? else {
    return Ok(Refusal::absent("no live share link has this token").into());
  };

  let opened = if note.title.trim().is_empty() {
    format!("document {}", note.document_id)
  } else {
    format!("\"{}\" (document {})", note.title.trim(), note.document_id)
  };

  Ok(Refusal::gone(format!("This share link has ended: it opened {opened}.")).into())
}

/// The document a live link opens, swept first; a token that names no live link answers what it opened where it
/// has ended, and nothing at all where it never named one.
pub async fn shared_document(standing: &Standing, token: &str) -> Result<i64, Refused> {
  sweep(standing).await?;

  let db = standing.db().await?;

  if let Some(link) = shares::by_token(db, token).await? {
    return Ok(link.document_id);
  }

  Err(no_live_link(db, token).await?)
}

/// The live link a token names, as the API answers it, swept first; a token that names no live link answers as
/// `shared_document` says.
pub async fn shared_link(standing: &Standing, token: &str) -> Result<Share, Refused> {
  sweep(standing).await?;

  let db = standing.db().await?;
  let Some(link) = shares::by_token(db, token).await? else {
    return Err(no_live_link(db, token).await?);
  };

  Ok(link.wire(&standing.settings.share.address))
}

pub struct ListShares;

impl Route for ListShares {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/shares";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let asked: SharesQuery = call.query()?;
    let limit = page_limit(asked.limit).ok_or_else(|| Refused::bad("limit is a whole number from 1"))?;
    let standing = call.standing();

    sweep(standing).await?;

    // A cursor in milliseconds: the links made in the second it names were on the page it came from.
    let after = asked.after.map(|after| after.div_euclid(1000));
    let (links, more) = shares::list(standing.db().await?, asked.document_id, after, limit).await?;
    let address = &standing.settings.share.address;

    Ok(answer(
      StatusCode::OK,
      &SharePage {
        items: links.iter().map(|link| link.wire(address)).collect(),
        more,
      },
    ))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listShares",
      tag: "shares",
      summary: "Share links, by document.",
      parameters: query::<SharesQuery>(generator),
      body: None,
      answers: vec![(200, json_answer::<SharePage>(generator))],
      codes: &["unauthorized", "bad-request"],
    }
  }
}

crate::route!(ListShares);

pub struct MakeShare;

impl Route for MakeShare {
  const METHOD: Method = Method::POST;
  const PATH: &'static str = "/shares";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let asked: ShareInput = call.json().await?;
    let standing = call.standing();
    let now = now_ms();

    if asked.expires_at.is_some_and(|at| at <= now) {
      return Err(Refused::bad("expiresAt is a time still to come"));
    }

    let scope = document(standing, asked.document_id).await?.document.scope;

    sweep(standing).await?;

    let db = standing.db().await?;
    let address = &standing.settings.share.address;
    // A link ends at the second its time falls in, never before it.
    let expires_at = asked.expires_at.map(|at| (at + 999).div_euclid(1000));

    match shares::create(db, asked.document_id, "", now / 1000, expires_at).await? {
      Some(made) => {
        let share = made.wire(address);

        standing.events.publish(db, EventKind::ShareCreated, scope.as_deref(), &share).await?;

        Ok(answer(StatusCode::CREATED, &share))
      }
      None => {
        let standing_link = shares::of_document(db, asked.document_id)
          .await?
          .ok_or_else(|| Refusal::failed("the share link was not written"))?;

        Ok(answer(StatusCode::OK, &standing_link.wire(address)))
      }
    }
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "createShares",
      tag: "shares",
      summary: "Make a share link that ends at a time, or never; the standing link is answered when one is open.",
      parameters: Vec::new(),
      body: json_body::<ShareInput>(generator),
      answers: vec![(201, json_answer::<Share>(generator)), (200, json_answer::<Share>(generator))],
      codes: &["unauthorized", "bad-request", "not-found"],
    }
  }
}

crate::route!(MakeShare);

pub struct ReadShare;

impl Route for ReadShare {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/shares/{token}";
  // A share read: the token is the whole of what it is held to (bmo-ui's `docs/console-bundles.md`, "Share pages").
  const CONSOLE: ConsoleReach = ConsoleReach::Open;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let token = call.path_text("token").await?;

    Ok(answer(StatusCode::OK, &shared_link(call.standing(), &token).await?))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "readSharesByToken",
      tag: "shares",
      summary: "A share link and the document it opens.",
      parameters: vec![token_parameter()],
      body: None,
      answers: vec![(200, json_answer::<Share>(generator))],
      codes: &["unauthorized", "not-found", "gone"],
    }
  }
}

crate::route!(ReadShare);

pub struct EndShare;

impl Route for EndShare {
  const METHOD: Method = Method::DELETE;
  const PATH: &'static str = "/shares/{token}";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let token = call.path_text("token").await?;
    let standing = call.standing();
    let db = standing.db().await?;
    let link = shares::by_token(db, &token)
      .await?
      .ok_or_else(|| Refusal::absent("no share link has this token"))?;

    // What the link opened is kept where it ends, so the token answers it long after it stops opening.
    shares::note_of_document(db, link.document_id, now_ms() / 1000, shares::ENDED).await?;

    if shares::remove(db, &token).await? {
      let scope = scope_of(standing, link.document_id).await?;

      standing
        .events
        .publish(db, EventKind::ShareDeleted, scope.as_deref(), &json!({ "token": token }))
        .await?;
    }

    Ok(StatusCode::NO_CONTENT.into_response())
  }

  fn operation(&self, _generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "deleteSharesByToken",
      tag: "shares",
      summary: "End a share link.",
      parameters: vec![token_parameter()],
      body: None,
      answers: vec![(204, None)],
      codes: &["unauthorized", "not-found"],
    }
  }
}

crate::route!(EndShare);
