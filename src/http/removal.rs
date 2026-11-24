//! The platform administrator's deletion of an account or a unit for good (`docs/api.md`, 「Removing an account or
//! a unit」): what the service keeps of it, counted and taken. An account's documents are let go of one by one as
//! `DELETE /documents/{documentId}` lets go of one.

use axum::http::{Method, StatusCode};
use axum::response::Response;
use schemars::generate::SchemaGenerator;

use super::documents::let_go;
use super::readers::Reader;
use crate::plumbing::http::{Call, ConsoleReach, Credential, Refused, Route, answer};
use crate::plumbing::openapi::{Operation, id_parameter, json_answer, query};
use crate::store::removal;
use crate::wire::{AccountHeld, RemovalCounts, Removed, UnitHeld, UnitRemovalQuery};

const CODES: &[&str] = &["unauthorized", "bad-request", "forbidden"];

// The service's token, or the console's naming the administrator; any other reader is refused.
fn administrator(call: &Call) -> Result<(), Refused> {
  let service = call.request.extensions().get::<Credential>() == Some(&Credential::Service);
  match Reader::of(call) {
    _ if service => Ok(()),
    Reader::Account(reader) if reader.administrator => Ok(()),
    _ => Err(Refused::new(
      StatusCode::FORBIDDEN,
      "forbidden",
      "a removal is the platform administrator's alone",
    )),
  }
}

// The account or unit the path names: a positive whole number.
async fn key(call: &mut Call, name: &str) -> Result<i64, Refused> {
  let id = call.path_id(name).await?;
  if id > 0 {
    Ok(id)
  } else {
    Err(Refused::bad(format!("{name} is a positive whole number, not {id}")))
  }
}

pub struct CountAccountRemoval;

impl Route for CountAccountRemoval {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/accounts/{account}/removal";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    administrator(&call)?;
    let account = key(&mut call, "account").await?.to_string();
    let counts = removal::account_held(call.standing().db().await?, &account).await?;
    Ok(answer(StatusCode::OK, &RemovalCounts { counts }))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "getAccountsRemovalByAccount",
      tag: "removal",
      summary: "What removing the account for good would take now; nothing changes.",
      parameters: vec![id_parameter("account")],
      body: None,
      answers: vec![(200, json_answer::<RemovalCounts<AccountHeld>>(generator))],
      codes: CODES,
    }
  }
}

crate::route!(CountAccountRemoval);

pub struct RemoveAccount;

impl Route for RemoveAccount {
  const METHOD: Method = Method::DELETE;
  const PATH: &'static str = "/accounts/{account}";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    administrator(&call)?;
    let account = key(&mut call, "account").await?.to_string();
    let standing = call.standing();
    let db = standing.db().await?;
    let mut removed = AccountHeld::default();

    for (id, scope) in removal::owned(db, &account).await? {
      let_go(standing, id, scope.as_deref()).await?;
      removed.documents += 1;
    }

    (removed.member_shares, removed.imports, removed.reads) = removal::forget_account(db, &account).await?;

    Ok(answer(StatusCode::OK, &Removed { removed }))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "deleteAccountsByAccount",
      tag: "removal",
      summary: "Remove the account for good: its documents let go of, its shares, imports and reads forgotten.",
      parameters: vec![id_parameter("account")],
      body: None,
      answers: vec![(200, json_answer::<Removed<AccountHeld>>(generator))],
      codes: CODES,
    }
  }
}

crate::route!(RemoveAccount);

pub struct CountUnitRemoval;

impl Route for CountUnitRemoval {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/org-units/{unit}/removal";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    administrator(&call)?;
    call.query::<UnitRemovalQuery>()?;
    let unit = key(&mut call, "unit").await?;
    let counts = removal::unit_held(call.standing().db().await?, unit).await?;
    Ok(answer(StatusCode::OK, &RemovalCounts { counts }))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    let mut parameters = vec![id_parameter("unit")];
    parameters.extend(query::<UnitRemovalQuery>(generator));
    Operation {
      id: "getOrgUnitsRemovalByUnit",
      tag: "removal",
      summary: "What removing the unit for good would take now; nothing changes.",
      parameters,
      body: None,
      answers: vec![(200, json_answer::<RemovalCounts<UnitHeld>>(generator))],
      codes: CODES,
    }
  }
}

crate::route!(CountUnitRemoval);

pub struct RemoveUnit;

impl Route for RemoveUnit {
  const METHOD: Method = Method::DELETE;
  const PATH: &'static str = "/org-units/{unit}";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    administrator(&call)?;
    call.query::<UnitRemovalQuery>()?;
    let unit = key(&mut call, "unit").await?;
    let unit_shares = removal::forget_unit(call.standing().db().await?, unit).await?;
    Ok(answer(
      StatusCode::OK,
      &Removed {
        removed: UnitHeld { unit_shares },
      },
    ))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    let mut parameters = vec![id_parameter("unit")];
    parameters.extend(query::<UnitRemovalQuery>(generator));
    Operation {
      id: "deleteOrgUnitsByUnit",
      tag: "removal",
      summary: "Remove the unit for good: it is taken off every document shared with it.",
      parameters,
      body: None,
      answers: vec![(200, json_answer::<Removed<UnitHeld>>(generator))],
      codes: CODES,
    }
  }
}

crate::route!(RemoveUnit);
