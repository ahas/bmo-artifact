//! The storage places (`docs/api.md`, "Storage places"): the kinds of place a screen offers, and a link's value of
//! the delegation chain's storage setting judged by the kind it names and written on the chain in the caller's
//! name.

use axum::http::{Method, StatusCode};
use axum::response::Response;
use schemars::generate::SchemaGenerator;

use super::readers::Reader;
use crate::Refusal;
use crate::places::{catalogue, judged};
use crate::plumbing::http::{Call, ConsoleReach, Refused, Route, answer};
use crate::plumbing::openapi::{Operation, json_answer, json_body, text_parameter};
use crate::wire::{StorageEntry, StorageInput, StoragePlaces, StorageRule};

pub struct ListStoragePlaces;

impl Route for ListStoragePlaces {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/storage-places";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, _: Call) -> Result<Response, Refused> {
    Ok(answer(
      StatusCode::OK,
      &StoragePlaces {
        places: catalogue(),
      },
    ))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listStoragePlaces",
      tag: "storage",
      summary: "The kinds of place a document's files may be kept at, each with the settings it takes.",
      parameters: Vec::new(),
      body: None,
      answers: vec![(200, json_answer::<StoragePlaces>(generator))],
      codes: &["unauthorized"],
    }
  }
}

crate::route!(ListStoragePlaces);

pub struct WriteStorage;

impl Route for WriteStorage {
  const METHOD: Method = Method::PUT;
  const PATH: &'static str = "/storage/{link}";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let link = call.path_text("link").await?;
    let asked: StorageInput = call.json().await?;
    let owner = Reader::of(&call)
      .owner()
      .map(str::to_owned)
      .ok_or_else(|| Refusal::forbidden("a storage setting is written in the name of an account"))?;
    let value = match &asked.value {
      Some(value) => Some(judged(value).map_err(Refusal::bad)?),
      None => None,
    };
    let entry = call.standing().accounts.write_storage(&link, value, &owner).await?;

    Ok(answer(
      StatusCode::OK,
      &StorageEntry {
        kind: entry.kind,
        link: entry.link,
        value: entry.value,
        delegation: entry.delegation.key().to_owned(),
        rules: entry
          .rules
          .into_iter()
          .map(|rule| StorageRule {
            who: rule.who,
            value: rule.value,
          })
          .collect(),
        set_by: entry.set_by,
        set_at: entry.set_at,
        locked_by: entry.locked_by,
        writable: entry.writable,
      },
    ))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "writeStorageByLink",
      tag: "storage",
      summary: "Judge a link's storage value by the place it names and write it on the delegation chain in the \
                caller's name.",
      parameters: vec![text_parameter("link", "The chain's link: `platform` or `unit:<id>`.")],
      body: json_body::<StorageInput>(generator),
      answers: vec![(200, json_answer::<StorageEntry>(generator))],
      codes: &[
        "unauthorized",
        "bad-request",
        "bad-link",
        "bad-value",
        "forbidden",
        "not-found",
        "locked",
        "upstream-failed",
      ],
    }
  }
}

crate::route!(WriteStorage);
