//! What the service holds under a caller's scope (`docs/api.md`, "Scopes"): counted, and let go of with a
//! preview first where the caller asks for one.

use axum::http::{Method, StatusCode};
use axum::response::Response;
use schemars::generate::SchemaGenerator;
use serde_json::json;

use crate::Refusal;
use crate::documents::keeper::document_folder;
use crate::events::EventKind;
use crate::plumbing::http::{Call, Refused, Route, Standing, answer};
use crate::plumbing::openapi::{Operation, json_answer, query, text_parameter};
use crate::store::{documents, scopes};
use crate::wire::{DryRunQuery, ScopeCounts, ScopeHolding};

fn scope_parameter() -> serde_json::Value {
  text_parameter("scope", "The caller's scope, as it wrote it.")
}

// What a scope holds, refused where it holds nothing.
async fn held(standing: &Standing, scope: &str) -> Result<ScopeCounts, Refused> {
  let counts = scopes::counts(standing.db().await?, scope).await?;

  if counts == ScopeCounts::default() {
    return Err(Refusal::absent(format!("nothing is held under the scope {scope:?}")).into());
  }

  Ok(counts)
}

pub struct ReadScope;

impl Route for ReadScope {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/scopes/{scope}";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let scope = call.path_text("scope").await?;
    let counts = held(call.standing(), &scope).await?;

    Ok(answer(
      StatusCode::OK,
      &ScopeHolding {
        scope,
        counts,
        state: None,
      },
    ))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "readScopesByScope",
      tag: "scopes",
      summary: "What the service holds under a scope, counted by resource.",
      parameters: vec![scope_parameter()],
      body: None,
      answers: vec![(200, json_answer::<ScopeHolding>(generator))],
      codes: &["unauthorized", "not-found"],
    }
  }
}

crate::route!(ReadScope);

pub struct ReleaseScope;

impl Route for ReleaseScope {
  const METHOD: Method = Method::DELETE;
  const PATH: &'static str = "/scopes/{scope}";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let scope = call.path_text("scope").await?;
    let asked: DryRunQuery = call.query()?;
    let standing = call.standing();
    let counts = held(standing, &scope).await?;

    if asked.dry_run == Some(true) {
      return Ok(answer(
        StatusCode::OK,
        &ScopeHolding {
          scope,
          counts,
          state: Some("preview".to_owned()),
        },
      ));
    }

    let db = standing.db().await?;

    for id in scopes::documents(db, &scope).await? {
      let tokens = documents::delete(db, id).await?;
      let _ = std::fs::remove_dir_all(document_folder(&standing.settings.files.root, id));

      for token in tokens {
        standing
          .events
          .publish(db, EventKind::ShareDeleted, Some(&scope), &json!({ "token": token }))
          .await?;
      }

      standing
        .events
        .publish(db, EventKind::DocumentDeleted, Some(&scope), &json!({ "id": id }))
        .await?;
    }

    scopes::drop_imports(db, &scope).await?;

    Ok(answer(
      StatusCode::OK,
      &ScopeHolding {
        scope,
        counts,
        state: Some("released".to_owned()),
      },
    ))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "deleteScopesByScope",
      tag: "scopes",
      summary: "Let go of everything held under a scope; dryRun=true answers what would go.",
      parameters: [vec![scope_parameter()], query::<DryRunQuery>(generator)].concat(),
      body: None,
      answers: vec![(200, json_answer::<ScopeHolding>(generator))],
      codes: &["unauthorized", "bad-request", "not-found"],
    }
  }
}

crate::route!(ReleaseScope);
