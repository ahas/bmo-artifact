//! What this repository holds for a room, as bmo-receiver reads it before it removes one (`docs/api.md`,
//! "Room holding (HTTP)"), asked through the router the service serves.

mod support;

use std::collections::HashMap;
use std::path::Path;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use bmo_artifact::plumbing::http::{Standing, router};
use bmo_artifact::settings::Settings;
use serde_json::{Value, json};
use support::ROOT;
use support::scratch::Scratch;
use tower::ServiceExt;

const WHY: &str = "a search is a record that stands, with its document, after the room it was asked in";

async fn holding(scratch: &Scratch, room: &str) -> (StatusCode, Value) {
  let environment = HashMap::from([("BMO_ARTIFACT_DATABASE__URL".to_owned(), scratch.address())]);
  let settings = Settings::read(Path::new(ROOT), Some(environment)).expect("the settings read");
  let router = router(Standing::new(settings).expect("the server stands")).expect("the routes stand");
  let request = Request::get(format!("/rooms/{room}/holding")).body(Body::empty()).expect("a request");
  let response = router.oneshot(request).await.expect("the router answers");
  let status = response.status();
  let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("the body reads");

  (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

async fn search_in(scratch: &Scratch, chat_id: i64) {
  scratch
    .run(
      "INSERT INTO searches (chat_id, query, started_depth, status, started_at) VALUES ($1, 'a question', 'basic', 'done', 1)",
      vec![chat_id.into()],
    )
    .await;
}

#[tokio::test]
async fn names_each_table_keeping_the_rooms_rows_with_how_many_and_why() {
  let scratch = Scratch::new().await;

  search_in(&scratch, 7).await;
  search_in(&scratch, 7).await;
  search_in(&scratch, -100_200).await;

  assert_eq!(
    holding(&scratch, "7").await,
    (StatusCode::OK, json!({ "kept": [{ "table": "searches", "rows": 2, "why": WHY }] }))
  );
  assert_eq!(
    holding(&scratch, "-100200").await,
    (StatusCode::OK, json!({ "kept": [{ "table": "searches", "rows": 1, "why": WHY }] }))
  );
  assert_eq!(
    holding(&scratch, "8").await,
    (StatusCode::OK, json!({ "kept": [{ "table": "searches", "rows": 0, "why": WHY }] }))
  );
}

#[tokio::test]
async fn a_room_that_is_not_a_whole_number_is_refused() {
  let scratch = Scratch::new().await;
  let (status, body) = holding(&scratch, "abc").await;

  assert_eq!(status, StatusCode::BAD_REQUEST);
  assert_eq!(body["code"], "bad_room");
}
