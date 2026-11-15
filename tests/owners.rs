//! Owners and unit shares (`docs/artifact.md`, "Owners" and "Unit shares"; `docs/api.md`, "Readers" and "Unit
//! shares"): a document filed for an account read by that account alone, shared with a unit and read by everyone
//! under it, a share kept inside the owner's company, a move and a withdrawal counted from the next call, a
//! document with no owner kept the administrator's, every document read by the administrator, the owner's company's
//! tree, and the units asked of a stand-in account service.

use crate::support;

use std::sync::Arc;

use axum::http::{Method, StatusCode};
use bmo_artifact::store::unit_shares;
use serde_json::{Value, json};
use support::accounts::Directory;
use support::service::{CONSOLE_TOKEN, Service};

/// The company the owner stands in.
const COMPANY: i64 = 1;
/// A unit of the company.
const TEAM: i64 = 2;
/// A unit under the team.
const PART: i64 = 3;
/// Another unit of the company, beside the team.
const BESIDE: i64 = 4;
/// Another company.
const ELSEWHERE: i64 = 9;

/// The owner, standing in the part.
const OWNER: &str = "11";
/// A colleague in the same part.
const COLLEAGUE: &str = "12";
/// A person standing in the team, above the part.
const ABOVE: &str = "13";
/// A person standing in the unit beside the team.
const ASIDE: &str = "14";
/// A person of another company.
const OUTSIDER: &str = "15";
/// The platform administrator.
const ADMINISTRATOR: &str = "1";

// The company's tree, another company, and the people placed in them.
async fn organisation() -> (Arc<Directory>, String) {
  let directory = Arc::new(Directory::default());

  directory.unit(COMPANY, "Company", None);
  directory.unit(TEAM, "Team", Some(COMPANY));
  directory.unit(PART, "Part", Some(TEAM));
  directory.unit(BESIDE, "Beside", Some(COMPANY));
  directory.unit(ELSEWHERE, "Elsewhere", None);

  for (user, unit) in
    [(OWNER, PART), (COLLEAGUE, PART), (ABOVE, TEAM), (ASIDE, BESIDE), (OUTSIDER, ELSEWHERE)]
  {
    directory.place(user, unit);
  }

  let address = directory.listening().await;

  (directory, address)
}

// A document filed by a holder of the service's token, for `owner` where one is named.
async fn filed(service: &Service, title: &str, owner: Option<&str>) -> Value {
  let headers: Vec<(&str, &str)> = owner.map(|owner| ("owner-key", owner)).into_iter().collect();
  let made = service
    .call_with(
      Method::POST,
      "/documents",
      Some(
        json!({ "kind": "record", "title": title, "body": "Words.", "actorName": "tester", "labels": {} }),
      ),
      &headers,
    )
    .await;

  assert_eq!(made.status, StatusCode::CREATED, "{}", made.json());

  made.json()
}

fn id_of(document: &Value) -> i64 {
  document["id"].as_i64().expect("an id")
}

// A call the console relays for a signed-in reader of the role and account given.
async fn as_reader(
  service: &Service,
  role: &str,
  user: &str,
  method: Method,
  path: &str,
  body: Option<Value>,
) -> (StatusCode, Value) {
  let answered = service
    .call_by(Some(CONSOLE_TOKEN), method, path, body, &[("owner-role", role), ("owner-key", user)])
    .await;
  let json = if answered.status == StatusCode::NO_CONTENT { Value::Null } else { answered.json() };

  (answered.status, json)
}

// A call the console relays for the platform administrator.
async fn administers(
  service: &Service,
  method: Method,
  path: &str,
  body: Option<Value>,
) -> (StatusCode, Value) {
  as_reader(service, "administrator", ADMINISTRATOR, method, path, body).await
}

async fn reads(service: &Service, user: &str, path: &str) -> (StatusCode, Value) {
  as_reader(service, "member", user, Method::GET, path, None).await
}

fn ids(page: &Value) -> Vec<i64> {
  page["items"]
    .as_array()
    .expect("items")
    .iter()
    .filter_map(|item| item["id"].as_i64())
    .collect()
}

#[tokio::test]
async fn a_new_document_is_read_by_its_owner_alone() {
  let (_directory, accounts) = organisation().await;
  let service = Service::with_accounts(&accounts).await;
  let document = filed(&service, "Private", Some(OWNER)).await;
  let id = id_of(&document);
  let path = format!("/documents/{id}");

  assert_eq!(document["owner"], OWNER);

  let (status, read) = reads(&service, OWNER, &path).await;

  assert_eq!(status, StatusCode::OK);
  assert!(read.get("permission").is_none(), "the owner reads it wholly: {read}");

  // No other account but the administrator reads it before it is shared: not a colleague, not the head above.
  for user in [COLLEAGUE, ABOVE, ASIDE] {
    let (status, absent) = reads(&service, user, &path).await;

    assert_eq!(status, StatusCode::NOT_FOUND, "{user}");
    assert_eq!(absent["error"], format!("there is no document {id}"));
    assert!(!ids(&reads(&service, user, "/documents").await.1).contains(&id), "{user}");
    assert!(!ids(&reads(&service, user, &format!("/documents?q=%23{id}")).await.1).contains(&id));
  }

  let (status, read) = as_reader(&service, "administrator", ADMINISTRATOR, Method::GET, &path, None).await;

  assert_eq!(status, StatusCode::OK, "{read}");
  assert_eq!(read["permission"], "read", "the administrator reads it alone");

  // A supervisor acting for another account reads no more than that account; the service's own token reads it.
  let other = service.call_with(Method::GET, &path, None, &[("owner-key", COLLEAGUE)]).await;

  assert_eq!(other.status, StatusCode::NOT_FOUND);
  assert_eq!(service.call(Method::GET, &path, None).await.status, StatusCode::OK);

  let (_, owned) = reads(&service, OWNER, "/documents").await;

  assert_eq!(ids(&owned), [id]);
}

#[tokio::test]
async fn a_unit_share_opens_the_document_to_everyone_under_the_unit() {
  let (directory, accounts) = organisation().await;
  let service = Service::with_accounts(&accounts).await;
  let id = id_of(&filed(&service, "For the team", Some(OWNER)).await);
  let path = format!("/documents/{id}");
  let units = format!("/documents/{id}/units");
  let (status, page) =
    as_reader(&service, "member", OWNER, Method::POST, &units, Some(json!({ "units": [TEAM] }))).await;

  assert_eq!(status, StatusCode::OK, "{page}");
  assert_eq!(page["items"][0]["unit"], TEAM);
  assert_eq!(page["items"][0]["documentId"], id);

  for user in [COLLEAGUE, ABOVE] {
    let (status, read) = reads(&service, user, &path).await;

    assert_eq!(status, StatusCode::OK, "{user}");
    assert_eq!(read["permission"], "read");

    let (_, listed) = reads(&service, user, "/documents").await;

    assert_eq!(ids(&listed), [id]);
    assert_eq!(listed["items"][0]["permission"], "read");
  }

  assert_eq!(reads(&service, ASIDE, &path).await.0, StatusCode::NOT_FOUND);
  assert_eq!(reads(&service, OUTSIDER, &path).await.0, StatusCode::NOT_FOUND);

  // One who reads through the unit shares nothing on.
  let (status, refused) =
    as_reader(&service, "member", COLLEAGUE, Method::POST, &units, Some(json!({ "units": [BESIDE] })))
      .await;

  assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
  assert_eq!(
    as_reader(
      &service,
      "member",
      COLLEAGUE,
      Method::POST,
      "/shares",
      Some(json!({ "documentId": id }))
    )
    .await
    .0,
    StatusCode::FORBIDDEN
  );

  // A person moved out of the unit reads by the unit they stand in now.
  directory.place(COLLEAGUE, BESIDE);

  assert_eq!(reads(&service, COLLEAGUE, &path).await.0, StatusCode::NOT_FOUND);

  // The company opens it to everyone of the company, and to nobody outside it.
  as_reader(&service, "member", OWNER, Method::POST, &units, Some(json!({ "units": [COMPANY] }))).await;

  assert_eq!(reads(&service, ASIDE, &path).await.0, StatusCode::OK);
  assert_eq!(reads(&service, OUTSIDER, &path).await.0, StatusCode::NOT_FOUND);

  let (_, listed) = as_reader(&service, "member", OWNER, Method::GET, &units, None).await;
  let shared: Vec<i64> = listed["items"]
    .as_array()
    .expect("items")
    .iter()
    .filter_map(|unit| unit["unit"].as_i64())
    .collect();

  assert_eq!(shared, [TEAM, COMPANY]);

  // Taking a unit off stops whoever read through it alone, at once.
  let (status, _) =
    as_reader(&service, "member", OWNER, Method::DELETE, &format!("{units}/{COMPANY}"), None).await;

  assert_eq!(status, StatusCode::NO_CONTENT);
  assert_eq!(reads(&service, ASIDE, &path).await.0, StatusCode::NOT_FOUND);
  assert_eq!(reads(&service, ABOVE, &path).await.0, StatusCode::OK, "the team's share stands");
  assert_eq!(
    as_reader(&service, "member", OWNER, Method::DELETE, &format!("{units}/{COMPANY}"), None)
      .await
      .0,
    StatusCode::NOT_FOUND
  );
}

#[tokio::test]
async fn every_share_stays_inside_the_owners_company() {
  let (_directory, accounts) = organisation().await;
  let service = Service::with_accounts(&accounts).await;
  let id = id_of(&filed(&service, "Inside", Some(OWNER)).await);
  let units = format!("/documents/{id}/units");
  let members = format!("/documents/{id}/members");

  let (status, refused) =
    as_reader(&service, "member", OWNER, Method::POST, &units, Some(json!({ "units": [ELSEWHERE] }))).await;

  assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
  assert_eq!(refused["code"], "other-company");

  let (status, refused) =
    as_reader(&service, "member", OWNER, Method::POST, &units, Some(json!({ "units": [77] }))).await;

  assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");

  let (status, refused) = as_reader(
    &service,
    "member",
    OWNER,
    Method::POST,
    &members,
    Some(json!({ "users": [COLLEAGUE, OUTSIDER], "permission": "read" })),
  )
  .await;

  assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
  assert_eq!(refused["code"], "other-company");
  assert_eq!(
    reads(&service, COLLEAGUE, &format!("/documents/{id}")).await.0,
    StatusCode::NOT_FOUND,
    "nothing was stored"
  );

  let (status, _) = as_reader(
    &service,
    "member",
    OWNER,
    Method::POST,
    &members,
    Some(json!({ "users": [ASIDE], "permission": "read" })),
  )
  .await;

  assert_eq!(status, StatusCode::OK);
  assert_eq!(reads(&service, ASIDE, &format!("/documents/{id}")).await.1["permission"], "read");
  assert_eq!(
    as_reader(&service, "member", ASIDE, Method::GET, &members, None).await.0,
    StatusCode::FORBIDDEN
  );
}

#[tokio::test]
async fn a_document_with_no_owner_stays_the_administrators() {
  let (_directory, accounts) = organisation().await;
  let service = Service::with_accounts(&accounts).await;
  let document = filed(&service, "Earlier", None).await;
  let id = id_of(&document);
  let path = format!("/documents/{id}");

  assert_eq!(document["owner"], Value::Null);

  let (status, read) = as_reader(&service, "administrator", ADMINISTRATOR, Method::GET, &path, None).await;

  assert_eq!(status, StatusCode::OK);
  assert!(read.get("permission").is_none());
  assert_eq!(reads(&service, OWNER, &path).await.0, StatusCode::NOT_FOUND);

  // The administrator shares it with members as before, and with no unit, since it belongs to no company.
  let (status, _) = as_reader(
    &service,
    "administrator",
    ADMINISTRATOR,
    Method::POST,
    &format!("{path}/members"),
    Some(json!({ "users": [OUTSIDER], "permission": "read" })),
  )
  .await;

  assert_eq!(status, StatusCode::OK);
  assert_eq!(reads(&service, OUTSIDER, &path).await.0, StatusCode::OK);

  let (status, refused) = as_reader(
    &service,
    "administrator",
    ADMINISTRATOR,
    Method::POST,
    &format!("{path}/units"),
    Some(json!({ "units": [COMPANY] })),
  )
  .await;

  assert_eq!(status, StatusCode::CONFLICT, "{refused}");
}

#[tokio::test]
async fn the_administrator_reads_every_document_of_every_company_and_changes_none_it_does_not_own() {
  let (_directory, accounts) = organisation().await;
  let service = Service::with_accounts(&accounts).await;
  let id = id_of(&filed(&service, "Elsewhere alone", Some(OUTSIDER)).await);
  let path = format!("/documents/{id}");

  // Owned by a person of another company and shared with nobody, it is in the administrator's list, search and
  // reads, under `read`.
  let (status, listed) = administers(&service, Method::GET, "/documents", None).await;

  assert_eq!(status, StatusCode::OK);
  assert_eq!(ids(&listed), [id]);
  assert_eq!(listed["items"][0]["permission"], "read");

  let (_, found) = administers(&service, Method::GET, "/documents?q=Elsewhere", None).await;

  assert_eq!(ids(&found), [id]);

  let (status, read) = administers(&service, Method::GET, &path, None).await;

  assert_eq!(status, StatusCode::OK, "{read}");
  assert_eq!(read["owner"], OUTSIDER);
  assert_eq!(read["permission"], "read");
  assert_eq!(
    administers(&service, Method::GET, &format!("{path}/content"), None).await.0,
    StatusCode::OK
  );

  // Sharing, rewriting and letting go of it stay its owner's.
  for (method, at, body) in [
    (
      Method::POST,
      format!("{path}/members"),
      Some(json!({ "users": [ADMINISTRATOR], "permission": "read" })),
    ),
    (Method::GET, format!("{path}/units"), None),
    (Method::GET, format!("{path}/company-units"), None),
    (Method::POST, "/shares".to_owned(), Some(json!({ "documentId": id }))),
    (Method::PATCH, path.clone(), Some(json!({ "title": "Taken", "labels": {} }))),
    (Method::DELETE, path.clone(), None),
  ] {
    let (status, refused) = administers(&service, method.clone(), &at, body).await;

    assert_eq!(status, StatusCode::FORBIDDEN, "{method} {at}: {refused}");
  }

  assert_eq!(reads(&service, OUTSIDER, &path).await.1["title"], "Elsewhere alone");
}

#[tokio::test]
async fn the_owner_picks_units_from_its_companys_tree() {
  let (directory, accounts) = organisation().await;
  let service = Service::with_accounts(&accounts).await;
  let id = id_of(&filed(&service, "To share", Some(OWNER)).await);
  let tree = format!("/documents/{id}/company-units");
  let (status, offered) = as_reader(&service, "member", OWNER, Method::GET, &tree, None).await;

  assert_eq!(status, StatusCode::OK, "{offered}");
  assert_eq!(offered["company"], COMPANY);
  assert_eq!(
    offered["units"],
    json!([
      { "id": COMPANY, "name": "Company", "parent": null },
      { "id": TEAM, "name": "Team", "parent": COMPANY },
      { "id": PART, "name": "Part", "parent": TEAM },
      { "id": BESIDE, "name": "Beside", "parent": COMPANY },
    ])
  );

  // A unit made after counts from the next ask, since no tree is kept.
  directory.unit(5, "New", Some(BESIDE));

  let (_, again) = as_reader(&service, "member", OWNER, Method::GET, &tree, None).await;

  assert_eq!(again["units"][4], json!({ "id": 5, "name": "New", "parent": BESIDE }));

  // Nobody else reads the tree through the document: one it is shared with is refused, one it is not finds nothing.
  as_reader(
    &service,
    "member",
    OWNER,
    Method::POST,
    &format!("/documents/{id}/units"),
    Some(json!({ "units": [PART] })),
  )
  .await;

  assert_eq!(reads(&service, COLLEAGUE, &tree).await.0, StatusCode::FORBIDDEN);
  assert_eq!(reads(&service, ASIDE, &tree).await.0, StatusCode::NOT_FOUND);

  // A document with no owner belongs to no company.
  let earlier = id_of(&filed(&service, "Earlier", None).await);
  let (status, refused) = as_reader(
    &service,
    "administrator",
    ADMINISTRATOR,
    Method::GET,
    &format!("/documents/{earlier}/company-units"),
    None,
  )
  .await;

  assert_eq!(status, StatusCode::CONFLICT, "{refused}");
}

#[tokio::test]
async fn only_the_owner_changes_a_document_and_an_account_reaches_no_whole_store() {
  let (_directory, accounts) = organisation().await;
  let service = Service::with_accounts(&accounts).await;
  let id = id_of(&filed(&service, "Mine", Some(OWNER)).await);
  let path = format!("/documents/{id}");
  let rewrite = json!({ "title": "Changed", "labels": {} });

  as_reader(
    &service,
    "member",
    OWNER,
    Method::POST,
    &format!("{path}/units"),
    Some(json!({ "units": [PART] })),
  )
  .await;

  let shared = service
    .call_with(Method::PATCH, &path, Some(rewrite.clone()), &[("owner-key", COLLEAGUE)])
    .await;

  assert_eq!(shared.status, StatusCode::FORBIDDEN, "{}", shared.json());

  let stranger = service.call_with(Method::DELETE, &path, None, &[("owner-key", ASIDE)]).await;

  assert_eq!(stranger.status, StatusCode::NOT_FOUND);

  let owned = service.call_with(Method::PATCH, &path, Some(rewrite), &[("owner-key", OWNER)]).await;

  assert_eq!(owned.status, StatusCode::OK, "{}", owned.json());
  assert_eq!(owned.json()["title"], "Changed");

  for whole in ["/events?limit=1", "/scopes/anything"] {
    let refused = service.call_with(Method::GET, whole, None, &[("owner-key", OWNER)]).await;

    assert_eq!(refused.status, StatusCode::FORBIDDEN, "{whole}");
  }

  let imported = service
    .call_with(
      Method::POST,
      "/imports",
      Some(json!({ "url": "https://pages.invalid/a", "actorName": "tester" })),
      &[("owner-key", OWNER)],
    )
    .await
    .json();
  let import = format!("/imports/{}", imported["id"]);

  assert_eq!(
    service.call_with(Method::GET, &import, None, &[("owner-key", OWNER)]).await.status,
    StatusCode::OK
  );
  assert_eq!(
    service.call_with(Method::GET, &import, None, &[("owner-key", COLLEAGUE)]).await.status,
    StatusCode::NOT_FOUND
  );
}

#[tokio::test]
async fn a_unit_read_the_account_service_does_not_answer_is_refused() {
  // An account service that answers nothing: the owner's own read never asks it, and a read a unit share decides
  // is refused rather than guessed.
  let service = Service::with_accounts("http://127.0.0.1:9").await;
  let mine = id_of(&filed(&service, "Mine", Some(OWNER)).await);
  let theirs = id_of(&filed(&service, "Theirs", Some(ASIDE)).await);
  let db = service.standing.db().await.expect("the database");

  unit_shares::add(db, theirs, &[COMPANY], 0).await.expect("the unit share is kept");

  assert_eq!(reads(&service, OWNER, &format!("/documents/{mine}")).await.0, StatusCode::OK);

  // The administrator reads every document, so no unit is asked for it.
  let (status, read) = administers(&service, Method::GET, &format!("/documents/{theirs}"), None).await;

  assert_eq!(status, StatusCode::OK, "{read}");
  assert_eq!(ids(&administers(&service, Method::GET, "/documents", None).await.1).len(), 2);

  let (status, failed) = reads(&service, OWNER, &format!("/documents/{theirs}")).await;

  assert_eq!(status, StatusCode::BAD_GATEWAY, "{failed}");
  assert_eq!(failed["code"], "upstream-failed");

  let answered = service
    .call(Method::POST, &format!("/documents/{mine}/units"), Some(json!({ "units": [TEAM] })))
    .await;

  assert_eq!(answered.status, StatusCode::BAD_GATEWAY, "{}", answered.json());
}
