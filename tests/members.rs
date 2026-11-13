//! Member shares and the readers they open documents to (`docs/api.md`, "Readers" and "Member shares"): a
//! document shared with a member read by that member and refused to a stranger, the share ended and the read
//! refused again, a member who reads and shares nothing on, reading as the one permission given, and the link
//! sharing standing as it does beside them.

use crate::support;

use axum::http::{Method, StatusCode};
use bmo_artifact::store::member_shares;
use serde_json::{Value, json};
use support::service::{CONSOLE_TOKEN, Service};

const MEMBER: &str = "101";
const STRANGER: &str = "202";

async fn filed(service: &Service, title: &str, category: &str) -> i64 {
  service
    .call(
      Method::POST,
      "/documents",
      Some(json!({
        "kind": "record", "title": title, "body": "Words.", "actorName": "tester", "labels": { "category": category }
      })),
    )
    .await
    .json()["id"]
    .as_i64()
    .expect("an id")
}

async fn shared(service: &Service, id: i64, user: &str) -> Value {
  let made = service
    .call(
      Method::POST,
      &format!("/documents/{id}/members"),
      Some(json!({ "users": [user], "permission": "read" })),
    )
    .await;

  assert_eq!(made.status, StatusCode::OK, "{}", made.json());

  made.json()
}

// A call the console relays for a signed-in reader of the role and account given.
async fn read_as(
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

async fn member_reads(service: &Service, user: &str, path: &str) -> (StatusCode, Value) {
  read_as(service, "member", user, Method::GET, path, None).await
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
async fn a_document_shared_with_a_member_reads_to_them_and_to_no_stranger() {
  let service = Service::new().await;
  let handed = filed(&service, "Handed", "brief").await;
  let kept = filed(&service, "Kept", "report").await;
  let page = shared(&service, handed, MEMBER).await;

  assert_eq!(page["items"][0]["user"], MEMBER);
  assert_eq!(page["items"][0]["permission"], "read");

  let (status, list) = member_reads(&service, MEMBER, "/documents").await;

  assert_eq!((status, ids(&list)), (StatusCode::OK, vec![handed]));
  assert_eq!(list["items"][0]["permission"], "read");

  for path in [
    format!("/documents/{handed}"),
    format!("/documents/{handed}/content"),
    format!("/documents/{handed}/attachments"),
    format!("/documents/{handed}/references"),
  ] {
    assert_eq!(member_reads(&service, MEMBER, &path).await.0, StatusCode::OK, "{path}");
  }

  assert_eq!(
    member_reads(&service, MEMBER, &format!("/documents/{handed}")).await.1["permission"],
    "read"
  );

  let (status, values) = member_reads(&service, MEMBER, "/label-values?key=category").await;

  assert_eq!((status, values["items"].as_array().map(Vec::len)), (StatusCode::OK, Some(1)));
  assert_eq!(values["items"][0]["value"], "brief");

  // A document not shared with the reader reads as one that does not stand, to the member and the stranger alike.
  for (user, id) in [(MEMBER, kept), (STRANGER, handed), (STRANGER, kept), ("", handed)] {
    for path in [
      format!("/documents/{id}"),
      format!("/documents/{id}/content"),
      format!("/documents/{id}/attachments"),
    ] {
      let (status, refused) = member_reads(&service, user, &path).await;

      assert_eq!(
        (status, refused["code"].as_str()),
        (StatusCode::NOT_FOUND, Some("not-found")),
        "{user} {path}"
      );
      assert_eq!(refused["error"], format!("there is no document {id}"), "{user} {path}");
    }
  }

  assert_eq!(ids(&member_reads(&service, STRANGER, "/documents").await.1), Vec::<i64>::new());

  // The administrator and the service's token read every document, under no permission.
  let (status, every) = read_as(&service, "administrator", STRANGER, Method::GET, "/documents", None).await;

  assert_eq!((status, ids(&every)), (StatusCode::OK, vec![kept, handed]));
  assert!(
    every["items"]
      .as_array()
      .expect("items")
      .iter()
      .all(|item| item.get("permission").is_none())
  );

  let read = service.call(Method::GET, &format!("/documents/{kept}"), None).await;

  assert_eq!(read.status, StatusCode::OK);
  assert!(read.json().get("permission").is_none());
}

#[tokio::test]
async fn ending_a_share_refuses_the_read_again() {
  let service = Service::new().await;
  let id = filed(&service, "Handed", "brief").await;

  shared(&service, id, MEMBER).await;

  assert_eq!(member_reads(&service, MEMBER, &format!("/documents/{id}")).await.0, StatusCode::OK);

  let ended = service.call(Method::DELETE, &format!("/documents/{id}/members/{MEMBER}"), None).await;

  assert_eq!(ended.status, StatusCode::NO_CONTENT);

  let (status, refused) = member_reads(&service, MEMBER, &format!("/documents/{id}")).await;

  assert_eq!((status, refused["code"].as_str()), (StatusCode::NOT_FOUND, Some("not-found")));
  assert_eq!(ids(&member_reads(&service, MEMBER, "/documents").await.1), Vec::<i64>::new());

  let listed = service.call(Method::GET, &format!("/documents/{id}/members"), None).await;

  assert_eq!(listed.json()["items"], json!([]));

  let again = service.call(Method::DELETE, &format!("/documents/{id}/members/{MEMBER}"), None).await;

  assert_eq!(
    (again.status, again.json()["code"].as_str()),
    (StatusCode::NOT_FOUND, Some("not-found"))
  );
}

#[tokio::test]
async fn the_administrator_shares_through_the_console_and_a_member_shares_nothing_on() {
  let service = Service::new().await;
  let id = filed(&service, "Handed", "brief").await;
  let members = format!("/documents/{id}/members");
  let body = json!({ "users": [MEMBER], "permission": "read" });
  let (status, _) =
    read_as(&service, "administrator", STRANGER, Method::POST, &members, Some(body.clone())).await;

  assert_eq!(status, StatusCode::OK);

  let link = service.call(Method::POST, "/shares", Some(json!({ "documentId": id }))).await.json();
  let token = link["token"].as_str().expect("a token").to_owned();

  for (method, path, body) in [
    (Method::GET, members.clone(), None),
    (Method::POST, members.clone(), Some(body)),
    (Method::PATCH, format!("{members}/{MEMBER}"), Some(json!({ "permission": "read" }))),
    (Method::DELETE, format!("{members}/{MEMBER}"), None),
    (Method::POST, "/shares".to_owned(), Some(json!({ "documentId": id }))),
    (Method::DELETE, format!("/shares/{token}"), None),
  ] {
    let (status, refused) = read_as(&service, "member", MEMBER, method.clone(), &path, body).await;

    assert_eq!(
      (status, refused["code"].as_str()),
      (StatusCode::FORBIDDEN, Some("forbidden")),
      "{method} {path}"
    );
  }

  // Nothing the member asked for changed: the share and the link stand as they were.
  assert_eq!(member_reads(&service, MEMBER, &format!("/documents/{id}")).await.0, StatusCode::OK);
  assert_eq!(
    service.call(Method::GET, &format!("/shares/{token}"), None).await.status,
    StatusCode::OK
  );
}

#[tokio::test]
async fn a_member_is_given_reading_alone() {
  let service = Service::new().await;
  let id = filed(&service, "Handed", "brief").await;
  let members = format!("/documents/{id}/members");
  let refused = service
    .call(Method::POST, &members, Some(json!({ "users": [MEMBER], "permission": "edit" })))
    .await;

  assert_eq!(
    (refused.status, refused.json()["code"].as_str()),
    (StatusCode::UNPROCESSABLE_ENTITY, Some("read-only"))
  );
  assert_eq!(service.call(Method::GET, &members, None).await.json()["items"], json!([]));

  shared(&service, id, MEMBER).await;

  let refused = service
    .call(Method::PATCH, &format!("{members}/{MEMBER}"), Some(json!({ "permission": "edit" })))
    .await;

  assert_eq!(
    (refused.status, refused.json()["code"].as_str()),
    (StatusCode::UNPROCESSABLE_ENTITY, Some("read-only"))
  );

  let kept = service
    .call(Method::PATCH, &format!("{members}/{MEMBER}"), Some(json!({ "permission": "read" })))
    .await;

  assert_eq!((kept.status, kept.json()["permission"].as_str()), (StatusCode::OK, Some("read")));

  for users in [json!([]), json!([""])] {
    let refused = service
      .call(Method::POST, &members, Some(json!({ "users": users, "permission": "read" })))
      .await;

    assert_eq!(refused.status, StatusCode::BAD_REQUEST, "{users}");
  }

  // Sharing with a member again keeps the one share standing.
  let again = shared(&service, id, MEMBER).await;

  assert_eq!(again["items"].as_array().map(Vec::len), Some(1));
  assert_eq!(again["items"][0]["id"], kept.json()["id"]);

  let absent = service.call(Method::GET, "/documents/999999/members", None).await;

  assert_eq!(absent.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_link_opens_its_document_whoever_it_is_shared_with() {
  let service = Service::new().await;
  let handed = filed(&service, "Handed", "brief").await;
  let kept = filed(&service, "Kept", "report").await;
  let mut tokens = Vec::new();

  for id in [handed, kept] {
    let made = service.call(Method::POST, "/shares", Some(json!({ "documentId": id }))).await;

    assert_eq!(made.status, StatusCode::CREATED);
    tokens.push(made.json()["token"].as_str().expect("a token").to_owned());
  }

  shared(&service, handed, MEMBER).await;

  // A member lists the links of the documents shared with them alone.
  let (status, links) = member_reads(&service, MEMBER, "/shares").await;
  let opened: Vec<i64> = links["items"]
    .as_array()
    .expect("items")
    .iter()
    .filter_map(|link| link["documentId"].as_i64())
    .collect();

  assert_eq!((status, opened), (StatusCode::OK, vec![handed]));

  // A link's token opens its document to whoever holds it, a document shared with no member among them, and ending a
  // member share touches no link.
  service
    .call(Method::DELETE, &format!("/documents/{handed}/members/{MEMBER}"), None)
    .await;

  for (token, id) in tokens.iter().zip([handed, kept]) {
    let read = service
      .call_by(Some(CONSOLE_TOKEN), Method::GET, &format!("/shares/{token}/document"), None, &[])
      .await;

    assert_eq!((read.status, read.json()["id"].as_i64()), (StatusCode::OK, Some(id)));
  }
}

#[tokio::test]
async fn a_document_let_go_of_takes_its_member_shares_with_it() {
  let service = Service::new().await;
  let id = filed(&service, "Handed", "brief").await;

  shared(&service, id, MEMBER).await;
  assert_eq!(
    service.call(Method::DELETE, &format!("/documents/{id}"), None).await.status,
    StatusCode::NO_CONTENT
  );

  let db = service.standing.db().await.expect("the database");
  let (left, more) = member_shares::list(db, id, None, 10).await.expect("the members read");

  assert_eq!((left, more), (Vec::new(), false));
}
