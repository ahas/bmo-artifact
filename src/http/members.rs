//! The members a document is shared with (`docs/api.md`, "Member shares"): listed, shared with, changed and
//! taken off by whoever keeps the document, each member of its owner's company, to read the document and nothing
//! more.

use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse, Response};
use schemars::generate::SchemaGenerator;

use super::documents::document;
use super::readers::kept;
use crate::Refusal;
use crate::now_ms;
use crate::plumbing::http::{Call, ConsoleReach, Refused, Route, Standing, answer};
use crate::plumbing::openapi::{Operation, id_parameter, json_answer, json_body, query, text_parameter};
use crate::store::{member_shares, page_limit};
use crate::wire::{
  MemberShare, MemberShareChange, MemberShareInput, MemberSharePage, PageQuery, Permission,
};

/// The most members one call shares a document with.
const MOST_USERS: usize = 200;

/// The longest account key a member is named by.
const KEY_LENGTH: usize = 200;

// A member is given reading alone: the document screens write nothing, so no share gives more.
fn readable(permission: Permission) -> Result<(), Refused> {
  match permission {
    Permission::Read => Ok(()),
    other => Err(
      Refusal::read_only(format!(
        "a document is shared with a member to read, never with the permission {}",
        other.word()
      ))
      .into(),
    ),
  }
}

fn user_parameter() -> serde_json::Value {
  text_parameter("user", "The member's account key.")
}

/// What a member or a unit outside the owner's company is refused with.
pub fn other_company(error: impl Into<String>) -> Refused {
  Refused::new(StatusCode::UNPROCESSABLE_ENTITY, "other-company", error)
}

/// The company the owner of one document stands in, as the account service answers it: none for a document with
/// no owner, and a refusal where its owner stands in none.
pub async fn owners_company(standing: &Standing, id: i64) -> Result<Option<i64>, Refused> {
  let Some(owner) = document(standing, id).await?.document.owner else {
    return Ok(None);
  };

  match standing.accounts.company_of_user(&owner).await? {
    Some(company) => Ok(Some(company)),
    None => Err(Refusal::conflict(format!("the owner of document {id} stands in no company")).into()),
  }
}

// Every member asked of the owner's company, where the document has an owner; one of any other is refused before
// anything is stored.
async fn of_owners_company(standing: &Standing, id: i64, users: &[String]) -> Result<(), Refused> {
  let Some(company) = owners_company(standing, id).await? else {
    return Ok(());
  };

  for user in users {
    if standing.accounts.company_of_user(user).await? != Some(company) {
      return Err(other_company(format!(
        "{user:?} is no member of the company the owner of document {id} stands in"
      )));
    }
  }

  Ok(())
}

fn no_member(id: i64, user: &str) -> Refused {
  Refusal::absent(format!("document {id} is not shared with {user:?}")).into()
}

pub struct ListMembers;

impl Route for ListMembers {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/documents/{documentId}/members";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = kept(&mut call).await?;
    let asked: PageQuery = call.query()?;
    let limit = page_limit(asked.limit).ok_or_else(|| Refused::bad("limit is a whole number from 1"))?;
    let (items, more) = member_shares::list(call.standing().db().await?, id, asked.after, limit).await?;

    Ok(answer(StatusCode::OK, &MemberSharePage { items, more }))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listDocumentsMembersByDocumentId",
      tag: "members",
      summary: "The members a document is shared with, in the order they were shared.",
      parameters: [vec![id_parameter("documentId")], query::<PageQuery>(generator)].concat(),
      body: None,
      answers: vec![(200, json_answer::<MemberSharePage>(generator))],
      codes: &["unauthorized", "bad-request", "not-found", "upstream-failed"],
    }
  }
}

crate::route!(ListMembers);

pub struct ShareWithMembers;

impl Route for ShareWithMembers {
  const METHOD: Method = Method::POST;
  const PATH: &'static str = "/documents/{documentId}/members";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = kept(&mut call).await?;
    let asked: MemberShareInput = call.json().await?;

    if asked.users.is_empty() || asked.users.len() > MOST_USERS {
      return Err(Refused::bad(format!("users names 1 to {MOST_USERS} members")));
    }

    if asked.users.iter().any(|user| user.is_empty() || user.chars().count() > KEY_LENGTH) {
      return Err(Refused::bad(format!("each of users is an account key of 1 to {KEY_LENGTH} characters")));
    }

    readable(asked.permission)?;
    of_owners_company(call.standing(), id, &asked.users).await?;

    let items =
      member_shares::add(call.standing().db().await?, id, &asked.users, asked.permission, now_ms()).await?;

    Ok(answer(StatusCode::OK, &MemberSharePage { items, more: false }))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "createDocumentsMembersByDocumentId",
      tag: "members",
      summary: "Share a document with members to read; a member already sharing it keeps its share.",
      parameters: vec![id_parameter("documentId")],
      body: json_body::<MemberShareInput>(generator),
      answers: vec![(200, json_answer::<MemberSharePage>(generator))],
      codes: &[
        "unauthorized",
        "bad-request",
        "not-found",
        "conflict",
        "read-only",
        "other-company",
        "upstream-failed",
      ],
    }
  }
}

crate::route!(ShareWithMembers);

pub struct ChangeMember;

impl Route for ChangeMember {
  const METHOD: Method = Method::PATCH;
  const PATH: &'static str = "/documents/{documentId}/members/{user}";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = kept(&mut call).await?;
    let user = call.path_text("user").await?;
    let asked: MemberShareChange = call.json().await?;

    readable(asked.permission)?;

    let changed: MemberShare = member_shares::set(call.standing().db().await?, id, &user, asked.permission)
      .await?
      .ok_or_else(|| no_member(id, &user))?;

    Ok(answer(StatusCode::OK, &changed))
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "changeDocumentsMembersByDocumentIdUser",
      tag: "members",
      summary: "Set the permission one member reads a document under.",
      parameters: vec![id_parameter("documentId"), user_parameter()],
      body: json_body::<MemberShareChange>(generator),
      answers: vec![(200, json_answer::<MemberShare>(generator))],
      codes: &["unauthorized", "bad-request", "not-found", "read-only", "upstream-failed"],
    }
  }
}

crate::route!(ChangeMember);

pub struct RemoveMember;

impl Route for RemoveMember {
  const METHOD: Method = Method::DELETE;
  const PATH: &'static str = "/documents/{documentId}/members/{user}";
  const CONSOLE: ConsoleReach = ConsoleReach::Screen;

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let id = kept(&mut call).await?;
    let user = call.path_text("user").await?;

    if !member_shares::remove(call.standing().db().await?, id, &user).await? {
      return Err(no_member(id, &user));
    }

    Ok(StatusCode::NO_CONTENT.into_response())
  }

  fn operation(&self, _generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "deleteDocumentsMembersByDocumentIdUser",
      tag: "members",
      summary: "Take a member off a document; their reading stops at once.",
      parameters: vec![id_parameter("documentId"), user_parameter()],
      body: None,
      answers: vec![(204, None)],
      codes: &["unauthorized", "bad-request", "not-found", "upstream-failed"],
    }
  }
}

crate::route!(RemoveMember);
