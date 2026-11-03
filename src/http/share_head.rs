//! The share head (`docs/api.md`, "Share links"): what a live link reads as before it is opened, as data a
//! console writes into the head of the page it serves at the link's address (bmo-ui's `docs/console-bundles.md`,
//! "Share pages"; `docs/artifact.md`, "Where a person reads it"). The token is the whole of what it is held to.

use axum::http::{Method, StatusCode};
use axum::response::Response;
use schemars::generate::SchemaGenerator;
use serde_json::Value;

use super::documents::document;
use super::shares::{shared_link, token_parameter};
use crate::Refusal;
use crate::kinds;
use crate::plumbing::http::{Call, ConsoleReach, Refused, Route, answer};
use crate::plumbing::openapi::{Operation, json_answer, query};
use crate::wire::{HeadQuery, ShareHead};

/// The path the manifest names under `shareHead`.
pub const SHARE_HEAD: &str = "/shares/{token}/head";

/// The Open Graph type every document is told as.
const PAGE_TYPE: &str = "article";

// One word of the bundle's words, by its path under the namespace; none where the words hold no text there.
fn word(words: &Value, path: &[&str]) -> Option<String> {
  path
    .iter()
    .fold(words, |inner, step| &inner[*step])
    .as_str()
    .filter(|said| !said.is_empty())
    .map(str::to_owned)
}

// The pack's sentence joining a kind's word and a description, each put in its place once, so neither is read as
// the sentence's own.
fn described(sentence: &str, kind: &str, description: &str) -> String {
  sentence
    .split("{description}")
    .map(|part| part.replace("{kind}", kind))
    .collect::<Vec<_>>()
    .join(description)
}

pub struct ReadShareHead;

impl Route for ReadShareHead {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = SHARE_HEAD;
  const CONSOLE: ConsoleReach = ConsoleReach::Open;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let token = call.path_text("token").await?;
    let asked: HeadQuery = call.query()?;
    let standing = call.standing();
    let link = shared_link(standing, &token).await?;
    let words = standing
      .console
      .words_in(asked.language.as_deref())
      .ok_or_else(|| Refusal::absent("the console bundle is not built"))?;
    let shown = document(standing, link.document_id).await?.document;
    // The kind's word leads the description where the page names the kind over the title, as it does for every
    // document but a page drawn whole.
    let kind = (!kinds::kind(Some(&shown.kind)).hosted())
      .then(|| word(&words, &["kind", &shown.kind]).unwrap_or_else(|| shown.kind.clone()));
    let sentence = word(&words, &["preview", "described"]);
    let description =
      shown.description.filter(|said| !said.is_empty()).map(|said| match (&kind, &sentence) {
        (Some(kind), Some(sentence)) => described(sentence, kind, &said),
        _ => said,
      });

    Ok(answer(
      StatusCode::OK,
      &ShareHead {
        title: shown.title,
        description,
        image: shown.has_picture.then(|| format!("/shares/{token}/picture")),
        url: link.url,
        locale: word(&words, &["preview", "locale"]).unwrap_or_default(),
        site_name: word(&words, &["preview", "site_name"]).unwrap_or_default(),
        r#type: PAGE_TYPE.into(),
        kind,
      },
    ))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listSharesHeadByToken",
      tag: "shares",
      summary: "What a live link reads as before it is opened, as a console writes it into the page's head.",
      parameters: [vec![token_parameter()], query::<HeadQuery>(generator)].concat(),
      body: None,
      answers: vec![(200, json_answer::<ShareHead>(generator))],
      codes: &["unauthorized", "bad-request", "not-found", "gone"],
    }
  }
}

crate::route!(ReadShareHead);
