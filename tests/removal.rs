//! The platform administrator's deletion of an account or a unit for good (`docs/api.md`, 「Removing an account or
//! a unit」): the count changes nothing and names what the removal then takes; the account's documents go, its
//! share of another's document goes while that document stays, a unit is taken off what was shared with it, a
//! second removal takes nothing, and a member's console call is refused.

use crate::support;

use axum::http::{Method, StatusCode};
use bmo_artifact::store::unit_shares;
use serde_json::{Value, json};
use support::service::{CONSOLE_TOKEN, Service};

const GOING: &str = "7";
const STAYING: &str = "8";

async fn filed(service: &Service, title: &str) -> i64 {
  let made = service
    .call(
      Method::POST,
      "/documents",
      Some(
        json!({ "kind": "record", "title": title, "body": "Words.", "actorName": "tester", "labels": {} }),
      ),
    )
    .await;
  assert_eq!(made.status, StatusCode::CREATED, "{}", made.json());
  made.json()["id"].as_i64().expect("an id")
}

async fn owned_by(service: &Service, id: i64, owner: &str) {
  service
    .scratch
    .run(r#"UPDATE "searches" SET "owner" = $1 WHERE "id" = $2"#, vec![owner.into(), id.into()])
    .await;
}

async fn as_administrator(service: &Service, method: Method, path: &str) -> (StatusCode, Value) {
  let answered = service
    .call_by(
      Some(CONSOLE_TOKEN),
      method,
      path,
      None,
      &[("owner-role", "administrator"), ("owner-key", "1")],
    )
    .await;
  (answered.status, answered.json())
}

#[tokio::test]
async fn an_accounts_documents_and_shares_go_and_nothing_else_does() {
  let service = Service::new().await;
  let first = filed(&service, "First").await;
  let second = filed(&service, "Second").await;
  let theirs = filed(&service, "Theirs").await;
  owned_by(&service, first, GOING).await;
  owned_by(&service, second, GOING).await;
  owned_by(&service, theirs, STAYING).await;
  service
    .scratch
    .run(
      r#"INSERT INTO "artifact_member_shares" ("search_id", "user_key", "permission", "shared_at") VALUES ($1, $2, 'read', 0)"#,
      vec![theirs.into(), GOING.into()],
    )
    .await;

  let path = format!("/accounts/{GOING}");
  let (status, counted) = as_administrator(&service, Method::GET, &format!("{path}/removal")).await;
  assert_eq!(status, StatusCode::OK, "{counted}");
  assert_eq!(
    counted,
    json!({ "counts": { "documents": 2, "member-shares": 1, "imports": 0, "reads": 0 } })
  );

  let (status, removed) = as_administrator(&service, Method::DELETE, &path).await;
  assert_eq!(status, StatusCode::OK, "{removed}");
  assert_eq!(removed["removed"], counted["counts"]);
  for id in [first, second] {
    assert_eq!(
      service.call(Method::GET, &format!("/documents/{id}"), None).await.status,
      StatusCode::NOT_FOUND
    );
  }
  assert_eq!(
    service.call(Method::GET, &format!("/documents/{theirs}"), None).await.status,
    StatusCode::OK
  );

  let again = service.call(Method::DELETE, &path, None).await;
  assert_eq!(again.status, StatusCode::OK);
  assert_eq!(
    again.json()["removed"],
    json!({ "documents": 0, "member-shares": 0, "imports": 0, "reads": 0 })
  );
}

#[tokio::test]
async fn a_unit_is_taken_off_what_was_shared_with_it() {
  let service = Service::new().await;
  let id = filed(&service, "Shared").await;
  owned_by(&service, id, STAYING).await;
  let db = service.standing.db().await.expect("the database");
  unit_shares::add(db, id, &[13], 0).await.expect("the unit share is kept");

  let (status, counted) = as_administrator(&service, Method::GET, "/org-units/13/removal?company=3").await;
  assert_eq!((status, &counted), (StatusCode::OK, &json!({ "counts": { "unit-shares": 1 } })));
  let (status, removed) = as_administrator(&service, Method::DELETE, "/org-units/13?company=3").await;
  assert_eq!((status, &removed["removed"]), (StatusCode::OK, &counted["counts"]));
  assert_eq!(
    service.call(Method::GET, &format!("/documents/{id}"), None).await.status,
    StatusCode::OK
  );
  let (_, again) = as_administrator(&service, Method::DELETE, "/org-units/13").await;
  assert_eq!(again["removed"], json!({ "unit-shares": 0 }));
}

#[tokio::test]
async fn a_removal_is_the_administrators_alone_and_names_an_id() {
  let service = Service::new().await;
  let refused = service
    .call_by(
      Some(CONSOLE_TOKEN),
      Method::DELETE,
      "/accounts/7",
      None,
      &[("owner-role", "member"), ("owner-key", "7")],
    )
    .await;
  assert_eq!(refused.status, StatusCode::FORBIDDEN);
  assert_eq!(
    service.call(Method::DELETE, "/accounts/seven", None).await.status,
    StatusCode::BAD_REQUEST
  );
  assert_eq!(service.call(Method::DELETE, "/accounts/0", None).await.status, StatusCode::BAD_REQUEST);
}
