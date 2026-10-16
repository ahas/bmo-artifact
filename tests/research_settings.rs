//! The agents screen's research settings (`docs/api.md`, "Research settings (HTTP)"), asked through the
//! router the service serves, with bmo-agency and the tool host stood in for: what `GET` reads of the pool,
//! the profiles and the engines, each change `POST` makes, and every refusal by its status and code.

mod support;

use std::collections::HashMap;
use std::path::Path;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use bmo_artifact::plumbing::http::{Standing, router};
use bmo_artifact::settings::Settings;
use serde_json::{Value, json};
use support::ROOT;
use support::agency::{self, Book, Stand};
use support::scratch::Scratch;
use support::units::{self, Book as UnitsBook};
use tower::ServiceExt;

struct Screen {
  scratch: Scratch,
  agency: Stand,
  settings: HashMap<String, String>,
}

impl Screen {
  async fn new(book: Book, units: UnitsBook) -> Self {
    let scratch = Scratch::new().await;
    let (agency, address) = agency::serve(book).await;
    let (_, tool_host) = units::serve(units).await;
    let settings = HashMap::from([
      ("BMO_ARTIFACT_DATABASE__URL".to_owned(), scratch.address()),
      ("BMO_ARTIFACT_UNITS__AGENCY".to_owned(), address),
      ("BMO_ARTIFACT_UNITS__TOOL_HOST".to_owned(), tool_host),
    ]);

    Self {
      scratch,
      agency,
      settings,
    }
  }

  async fn ask(&self, request: Request<Body>) -> (StatusCode, Value) {
    let settings = Settings::read(Path::new(ROOT), Some(self.settings.clone())).expect("the settings read");
    let router = router(Standing::new(settings).expect("the server stands")).expect("the routes stand");
    let response = router.oneshot(request).await.expect("the router answers");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("the body reads");

    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
  }

  async fn read(&self) -> (StatusCode, Value) {
    self.ask(Request::get("/research-settings").body(Body::empty()).unwrap()).await
  }

  async fn change(&self, body: &str) -> (StatusCode, Value) {
    let request = Request::post("/research-settings")
      .header("content-type", "application/json")
      .body(Body::from(body.to_owned()))
      .unwrap();

    self.ask(request).await
  }
}

fn engines() -> UnitsBook {
  UnitsBook {
    engines: Some(json!({
      "engines": [
        { "name": "engine-a", "category": "web", "korean_native": false, "state": "ok", "elapsed_ms": 300, "result_count": 9 },
        { "name": "engine-b", "category": "web", "korean_native": true, "state": "suspended", "reason": "captcha", "suspended_until": 4_102_444_800_000_i64 },
        { "name": "engine-c", "category": "news", "state": "suspended", "suspended_until": 1 },
      ],
      "cooldowns": [{ "reason": "captcha", "ms": 600_000 }],
    })),
    ..UnitsBook::default()
  }
}

fn profile<'a>(body: &'a Value, key: &str) -> &'a Value {
  body["profiles"]
    .as_array()
    .unwrap()
    .iter()
    .find(|one| one["key"] == key)
    .expect("the key's profile")
}

#[tokio::test]
async fn reads_the_pool_the_profiles_and_the_engines_as_the_screen_shows_them() {
  let screen = Screen::new(Book::lending(), engines()).await;
  let (status, body) = screen.read().await;

  assert_eq!(status, StatusCode::OK, "{body}");
  assert!(body["readAt"].as_i64().is_some());

  let pool = &body["pool"];

  assert_eq!(pool["coworkerTiers"], json!(["small"]));
  assert_eq!(pool["defaults"], json!({ "small": 4 }));
  assert_eq!(pool["capacityLimit"].as_i64(), Some(128));
  assert_eq!(
    pool["roles"]
      .as_array()
      .unwrap()
      .iter()
      .map(|role| (role["name"].clone(), role["tier"].clone(), role["usedBy"].clone()))
      .collect::<Vec<_>>(),
    [
      (json!("scout"), json!("small"), json!(["search:thorough", "search:exhaustive"])),
      (json!("helper"), json!("small"), json!([])),
      (json!("keeper"), Value::Null, json!([])),
    ]
  );
  assert_eq!(pool["tiers"].as_array().unwrap().len(), 1);

  let tier = &pool["tiers"][0];

  assert_eq!(
    (
      &tier["provider"],
      &tier["label"],
      &tier["tier"],
      &tier["model"],
      &tier["effort"],
      &tier["capacity"]
    ),
    (
      &json!("prov-a"),
      &json!("PROV-A"),
      &json!("small"),
      &json!("model-a"),
      &json!("low"),
      &json!(2)
    )
  );
  // The control role is never lent, so it serves no tier.
  assert_eq!(tier["roles"], json!(["scout", "helper"]));

  assert_eq!(
    body["profiles"]
      .as_array()
      .unwrap()
      .iter()
      .map(|one| one["key"].clone())
      .collect::<Vec<_>>(),
    ["search:basic", "search:thorough", "search:exhaustive", "browse"]
  );

  let basic = profile(&body, "search:basic");
  let thorough = profile(&body, "search:thorough");

  assert_eq!(
    (&basic["setAt"], &basic["maxChildren"], &basic["room"]),
    (&json!(0), &json!(0), &Value::Null)
  );
  assert_eq!(basic["seed"]["strategy"], "direct-then-browser");
  assert_eq!(basic["strategies"], json!(["direct-then-browser", "browser-first", "direct-only"]));
  assert_eq!(
    (&basic["tier"], &basic["provider"], &basic["model"], &basic["effort"], &basic["error"]),
    (&json!("large"), &json!("prov-a"), &json!("model-b"), &json!("high"), &Value::Null)
  );
  assert_eq!(
    thorough["room"],
    json!({ "state": "free", "provider": "prov-a", "tier": "small", "model": "model-a", "capacity": 2,
      "active": 0, "waiting": 0, "free": 2, "error": null })
  );
  assert_eq!(body["limits"]["maxChildren"], json!({ "max": 16 }));
  assert_eq!(body["limits"]["researchMs"], json!({ "min": 10000 }));
  assert_eq!(
    body["engines"]
      .as_array()
      .unwrap()
      .iter()
      .map(|one| (one["engine"].clone(), one["resting"].clone()))
      .collect::<Vec<_>>(),
    [
      (json!("engine-a"), json!(false)),
      (json!("engine-b"), json!(true)),
      (json!("engine-c"), json!(false))
    ]
  );
  assert_eq!(body["engines"][1]["korean"], true);
  assert_eq!(body["cooldowns"], json!([{ "reason": "captcha", "ms": 600_000 }]));
  assert!(body.get("enginesError").is_none());
}

#[tokio::test]
async fn reads_the_screen_without_the_engines_where_bmo_explorer_cannot_be_read() {
  let screen = Screen::new(Book::lending(), UnitsBook::default()).await;
  let (status, body) = screen.read().await;

  assert_eq!(status, StatusCode::OK, "{body}");
  assert_eq!((&body["engines"], &body["cooldowns"]), (&json!([]), &json!([])));
  assert_eq!(body["enginesError"], "nothing is served there");
}

#[tokio::test]
async fn a_profile_whose_route_or_effort_does_not_hold_says_why_it_cannot_launch() {
  let book = Book {
    refused_efforts: vec!["high".to_owned()],
    ..Book::lending()
  };
  let screen = Screen::new(book, engines()).await;
  let (_, body) = screen.read().await;
  let basic = profile(&body, "search:basic");

  assert_eq!((&basic["provider"], &basic["model"]), (&json!("prov-a"), &Value::Null));
  assert_eq!(basic["error"], "model-b does not take the high effort");
}

#[tokio::test]
async fn sets_and_resets_a_pool_limit_through_bmo_agency() {
  let screen = Screen::new(Book::lending(), engines()).await;

  assert_eq!(
    screen
      .change(r#"{"action":"set-pool-limit","provider":"prov-a","tier":"small","capacity":3}"#)
      .await,
    (StatusCode::OK, json!({ "ok": true }))
  );
  assert_eq!(
    screen.change(r#"{"action":"reset-pool-limits"}"#).await,
    (StatusCode::OK, json!({ "ok": true }))
  );
  assert_eq!(screen.agency.book().limits, [("prov-a".to_owned(), "small".to_owned(), 3)]);
  assert_eq!(screen.agency.book().limits_reset, 1);
}

#[tokio::test]
async fn saves_a_profile_reads_it_back_and_resets_it_to_its_seed() {
  let screen = Screen::new(Book::lending(), engines()).await;
  let values = json!({ "capMs": null, "synthesisReserveMs": null, "childRole": "scout", "maxChildren": 2,
    "waitLimitMs": 1000, "strategy": "browser-first", "hostIntervalMs": 500, "engineIntervalMs": 700 });
  let saved = screen
    .change(&json!({ "action": "set-profile", "key": "search:thorough", "values": values }).to_string())
    .await;

  assert_eq!(saved, (StatusCode::OK, json!({ "ok": true })));

  let (_, body) = screen.read().await;
  let thorough = profile(&body, "search:thorough");

  assert_eq!((&thorough["maxChildren"], &thorough["strategy"]), (&json!(2), &json!("browser-first")));
  assert!(thorough["setAt"].as_i64().unwrap() > 0);

  let reset = screen.change(r#"{"action":"reset-profile","key":"search:thorough"}"#).await;
  let (_, body) = screen.read().await;

  assert_eq!(reset, (StatusCode::OK, json!({ "ok": true })));
  assert_eq!(profile(&body, "search:thorough")["maxChildren"], 4);
  assert_eq!(screen.scratch.count("SELECT COUNT(*) FROM research_profiles", vec![]).await, 1);
}

#[tokio::test]
async fn a_profile_that_fails_its_checks_is_refused_with_each_field_named_and_nothing_saved() {
  let screen = Screen::new(Book::lending(), engines()).await;
  let values = json!({ "capMs": 60000, "synthesisReserveMs": null, "childRole": "scout", "maxChildren": 2,
    "waitLimitMs": 1000, "strategy": "visible", "hostIntervalMs": 500, "engineIntervalMs": 700 });
  let (status, body) = screen
    .change(&json!({ "action": "set-profile", "key": "search:basic", "values": values }).to_string())
    .await;

  assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
  assert_eq!(body["code"], "invalid_profile");
  assert_eq!(
    body["faults"]
      .as_array()
      .unwrap()
      .iter()
      .map(|one| (one["field"].clone(), one["code"].clone()))
      .collect::<Vec<_>>(),
    [
      (json!("capMs"), json!("no_clock")),
      (json!("maxChildren"), json!("branchless")),
      (json!("strategy"), json!("bad_strategy")),
    ]
  );
  assert_eq!(
    body["error"],
    "This run holds no total time. This run opens no branches. This run does not accept that fetch strategy."
  );
  assert_eq!(screen.scratch.count("SELECT COUNT(*) FROM research_profiles", vec![]).await, 0);
}

#[tokio::test]
async fn refuses_each_change_it_cannot_take_by_its_status_and_code() {
  let book = Book {
    limit_refusal: Some((
      422,
      "limit_refused".to_owned(),
      "prov-a holds at most 2 small places".to_owned(),
    )),
    ..Book::lending()
  };
  let screen = Screen::new(book, engines()).await;
  let cases = [
    ("not json", StatusCode::BAD_REQUEST, "not_json"),
    (r#"["set-profile"]"#, StatusCode::BAD_REQUEST, "bad_action"),
    (r#"{"action":"set-mapping"}"#, StatusCode::BAD_REQUEST, "bad_action"),
    (r#"{}"#, StatusCode::BAD_REQUEST, "bad_action"),
    (
      r#"{"action":"set-pool-limit","provider":"prov-b","tier":"small","capacity":1}"#,
      StatusCode::BAD_REQUEST,
      "bad_pool_limit",
    ),
    (
      r#"{"action":"set-pool-limit","provider":"prov-a","tier":"large","capacity":1}"#,
      StatusCode::BAD_REQUEST,
      "bad_pool_limit",
    ),
    (
      r#"{"action":"set-pool-limit","provider":"prov-a","tier":"small","capacity":999}"#,
      StatusCode::BAD_REQUEST,
      "bad_capacity",
    ),
    (
      r#"{"action":"set-pool-limit","provider":"prov-a","tier":"small","capacity":1.5}"#,
      StatusCode::BAD_REQUEST,
      "bad_capacity",
    ),
    (
      r#"{"action":"set-pool-limit","provider":"prov-a","tier":"small","capacity":2}"#,
      StatusCode::UNPROCESSABLE_ENTITY,
      "limit_refused",
    ),
    (
      r#"{"action":"set-profile","key":"nowhere","values":{}}"#,
      StatusCode::BAD_REQUEST,
      "bad_key",
    ),
    (r#"{"action":"reset-profile"}"#, StatusCode::BAD_REQUEST, "bad_key"),
    (r#"{"action":"set-profile","key":"browse"}"#, StatusCode::BAD_REQUEST, "no_values"),
  ];

  for (body, status, code) in cases {
    let (answered, refused) = screen.change(body).await;

    assert_eq!((answered, refused["code"].as_str()), (status, Some(code)), "{body}: {refused}");
    assert!(refused["error"].as_str().is_some_and(|error| !error.is_empty()), "{body}: {refused}");
  }

  // bmo-agency's own sentence is handed on as it stands.
  let (_, refused) = screen
    .change(r#"{"action":"set-pool-limit","provider":"prov-a","tier":"small","capacity":2}"#)
    .await;

  assert_eq!(refused["error"], "prov-a holds at most 2 small places");
  assert!(screen.agency.book().limits.is_empty());
}

#[tokio::test]
async fn answers_502_where_bmo_agency_cannot_be_reached() {
  let mut screen = Screen::new(Book::lending(), engines()).await;
  let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
  let address = format!("http://{}", closed.local_addr().unwrap());

  drop(closed);
  screen.settings.insert("BMO_ARTIFACT_UNITS__AGENCY".to_owned(), address);

  let (status, body) = screen.read().await;

  assert_eq!((status, body["code"].as_str()), (StatusCode::BAD_GATEWAY, Some("agency_failed")));
  assert!(body["error"].as_str().unwrap().starts_with("bmo-agency could not "), "{body}");
}
