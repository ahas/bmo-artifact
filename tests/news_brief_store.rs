//! The morning news page's rooms and the holiday calendar as their store keeps them (`docs/news-brief.md`,
//! "When it runs" and "The holiday calendar").

mod support;

use axum::http::StatusCode;
use bmo_artifact::store::news_briefs::{
  self, Brief, Change, Holiday, Named, add, brief, holiday, holidays_of, mark_started, remove,
  remove_holiday, set, set_holiday,
};
use support::scratch::Scratch;

#[tokio::test]
async fn rooms_are_added_off_changed_field_by_field_and_marked_with_the_day_they_started() {
  let scratch = Scratch::new().await;
  let db = scratch.db();
  let named = Named {
    chat: Some("-123456789".into()),
    ..Named::default()
  };
  let id = add(db, &Change::read(&named).expect("read"), 100).await.expect("added");

  assert_eq!(
    brief(db, id).await.expect("read"),
    Some(Brief {
      id,
      chat_id: -123456789,
      send_at: news_briefs::SEND_AT.into(),
      lead_minutes: news_briefs::LEAD_MINUTES,
      share_hours: news_briefs::SHARE_HOURS,
      enabled: false,
      last_day: None,
      created_at: 100,
    })
  );

  let change = Change {
    send_at: Some("07:50".into()),
    enabled: Some(true),
    ..Change::default()
  };
  let changed = set(db, id, &change).await.expect("set").expect("the row stands");

  assert_eq!((changed.send_at.as_str(), changed.enabled, changed.lead_minutes), ("07:50", true, 40));

  mark_started(db, id, "2026-09-28").await.expect("marked");

  assert_eq!(
    brief(db, id).await.expect("read").and_then(|row| row.last_day),
    Some("2026-09-28".into())
  );
  assert_eq!(set(db, id + 1, &change).await.expect("set"), None);
  assert_eq!(remove(db, id).await.expect("removed").map(|row| row.id), Some(id));
  assert_eq!(brief(db, id).await.expect("read"), None);
  assert_eq!(remove(db, id).await.expect("removed"), None);
}

#[tokio::test]
async fn what_a_row_may_not_hold_is_refused_writing_nothing() {
  let scratch = Scratch::new().await;
  let db = scratch.db();
  let id = add(
    db,
    &Change {
      chat_id: Some(1),
      ..Change::default()
    },
    100,
  )
  .await
  .expect("added");
  let refused = |named: Named| Change::read(&named).expect_err("refused").detail;

  assert_eq!(
    refused(Named {
      at: Some("8:30".into()),
      ..Named::default()
    }),
    "8:30 is not a time; write it HH:MM"
  );
  assert!(
    refused(Named {
      lead: Some("5".into()),
      ..Named::default()
    })
    .contains("lead")
  );
  assert!(
    refused(Named {
      lead: Some("ten".into()),
      ..Named::default()
    })
    .contains("lead")
  );
  assert!(
    refused(Named {
      share_hours: Some("0".into()),
      ..Named::default()
    })
    .contains("hours")
  );
  assert_eq!(
    refused(Named {
      chat: Some("room".into()),
      ..Named::default()
    }),
    "a room is named by its chat id"
  );

  let refusal = set(
    db,
    id,
    &Change {
      lead_minutes: Some(181),
      ..Change::default()
    },
  )
  .await
  .expect_err("refused");

  assert_eq!(refusal.status, StatusCode::BAD_REQUEST);
  assert!(add(db, &Change::default(), 100).await.is_err());
  assert_eq!(brief(db, id).await.expect("read").map(|row| row.lead_minutes), Some(40));
  assert_eq!(scratch.count(r#"SELECT count(*) FROM "news_briefs""#, vec![]).await, 1);
}

#[tokio::test]
async fn a_holiday_is_added_trimmed_renamed_listed_by_year_and_taken_off() {
  let scratch = Scratch::new().await;
  let db = scratch.db();

  assert_eq!(holidays_of(db, 2028).await.expect("read"), []);

  let added = set_holiday(db, "2028-01-01", "  New Year's Day ").await.expect("added");
  let new_year = Holiday {
    day: "2028-01-01".into(),
    name: "New Year's Day".into(),
  };

  assert_eq!(added, new_year);
  set_holiday(db, "2028-03-01", "Independence Movement Day").await.expect("added");
  set_holiday(db, "2027-12-25", "Christmas Day").await.expect("added");
  set_holiday(db, "2028-01-01", "New Year").await.expect("renamed");

  assert_eq!(
    holidays_of(db, 2028)
      .await
      .expect("read")
      .iter()
      .map(|day| day.name.as_str())
      .collect::<Vec<_>>(),
    ["New Year", "Independence Movement Day"]
  );
  assert!(remove_holiday(db, "2028-01-01").await.expect("removed"));
  assert!(!remove_holiday(db, "2028-01-01").await.expect("removed"));
  assert_eq!(holiday(db, "2028-01-01").await.expect("read"), None);
  assert_eq!(
    set_holiday(db, "2028-1-1", "New Year").await.expect_err("refused").detail,
    "2028-1-1 is not a day; write it YYYY-MM-DD"
  );
  assert!(set_holiday(db, "2028-02-30", "New Year").await.is_err());
  assert_eq!(
    set_holiday(db, "2028-01-02", "  ").await.expect_err("refused").detail,
    "a holiday has a name"
  );
  assert_eq!(holidays_of(db, 2028).await.expect("read").len(), 1);
}
