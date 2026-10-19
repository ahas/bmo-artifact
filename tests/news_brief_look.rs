//! The server's minute loop that starts each room's morning news page (`docs/news-brief.md`, "When it runs"):
//! when a row is due, the words its runner is started on, and a look over a scratch database that marks and
//! starts a row once and tells the owner once of an empty calendar, at moments it is handed and with no
//! waiting.

mod support;

use std::collections::HashMap;
use std::path::Path;

use bmo_artifact::clients::receiver::Receiver;
use bmo_artifact::plumbing::background::{loop_faults, loops};
use bmo_artifact::settings::Settings;
use bmo_artifact::store::news_briefs::{self, Brief, Change, set_holiday};
use bmo_artifact::verbs::news_brief::launch::runner_args;
use bmo_artifact::verbs::news_brief::look::{Ground, LATE_LIMIT_MS, Look, due, start_of};
use support::ROOT;
use support::agency::{self, Book as AgencyBook};
use support::scratch::Scratch;
use support::units::{self, Book};

// 2026-10-07, a Wednesday, 08:00 UTC.
const WEDNESDAY_EIGHT: i64 = 1_791_360_000_000;
const MINUTE: i64 = 60_000;

fn row() -> Brief {
  Brief {
    id: 1,
    chat_id: -100,
    send_at: "08:30".into(),
    lead_minutes: 40,
    share_hours: 24,
    enabled: true,
    last_day: None,
    created_at: 1,
  }
}

#[test]
fn a_row_starts_on_a_working_day_from_its_lead_until_the_late_limit() {
  let utc = chrono_tz::UTC;

  assert_eq!(due(&row(), WEDNESDAY_EIGHT, utc, false).as_deref(), Some("2026-10-07"));
  assert_eq!(due(&row(), WEDNESDAY_EIGHT - 11 * MINUTE, utc, false), None);
  assert_eq!(
    due(&row(), WEDNESDAY_EIGHT + 30 * MINUTE + LATE_LIMIT_MS, utc, false).as_deref(),
    Some("2026-10-07")
  );
  assert_eq!(due(&row(), WEDNESDAY_EIGHT + 30 * MINUTE + LATE_LIMIT_MS + 1, utc, false), None);
  assert_eq!(due(&row(), WEDNESDAY_EIGHT, utc, true), None);
  assert_eq!(due(&row(), WEDNESDAY_EIGHT + 3 * 24 * 60 * MINUTE, utc, false), None);

  let started = Brief {
    last_day: Some("2026-10-07".into()),
    ..row()
  };
  let off = Brief {
    enabled: false,
    ..row()
  };

  assert_eq!(due(&started, WEDNESDAY_EIGHT, utc, false), None);
  assert_eq!(due(&off, WEDNESDAY_EIGHT, utc, false), None);
  // The same moment is the afternoon nine hours east, past the late limit of that day's page.
  assert_eq!(due(&row(), WEDNESDAY_EIGHT, chrono_tz::Asia::Tokyo, false), None);
}

#[test]
fn the_runner_is_given_the_room_the_day_the_links_life_and_the_time() {
  let row = row();

  assert_eq!(
    runner_args(&start_of(&row, "2026-10-07"), 100, "bmo-news-brief-1"),
    [
      "news-brief-runner",
      "--chat",
      "-100",
      "--day",
      "2026-10-07",
      "--share-hours",
      "24",
      "--send-at",
      "08:30",
      "--run",
      "100",
      "--unit",
      "bmo-news-brief-1"
    ]
  );
}

#[tokio::test]
async fn a_news_page_due_is_marked_and_started_once_and_an_empty_calendar_is_told_once() {
  assert!(loops().any(|one| one.name() == "news-brief"));
  assert!(loop_faults().is_empty());

  let scratch = Scratch::new().await;
  let (units, address) = units::serve(Book::default()).await;
  let (stand, agency_address) = agency::serve(AgencyBook::default()).await;
  let environment = HashMap::from([("BMO_ARTIFACT_DATABASE__URL".to_owned(), scratch.address())]);
  let settings = Settings::read(Path::new(ROOT), Some(environment)).expect("the settings");
  let agency = agency::client(&agency_address);
  let receiver = Receiver::new(Some(address), Some(units::RECEIVER_TOKEN.to_owned()));
  let ground = Ground {
    db: scratch.db(),
    agency: &*agency,
    receiver: &receiver,
    zone: settings.time.zone,
  };
  let added = Change {
    chat_id: Some(-100),
    lead_minutes: Some(10),
    enabled: Some(true),
    ..Change::default()
  };

  news_briefs::add(scratch.db(), &added, 1).await.expect("added");

  let look = Look::new();
  let now = WEDNESDAY_EIGHT + 25 * MINUTE;

  assert_eq!(look.look(&ground, now).await.expect("looked"), 1);
  assert_eq!(look.look(&ground, now + MINUTE).await.expect("looked"), 0);

  let launches = stand.book().launches.clone();

  assert_eq!(launches.len(), 1);
  assert_eq!(
    launches[0].1.command.args[..9],
    [
      "news-brief-runner",
      "--chat",
      "-100",
      "--day",
      "2026-10-07",
      "--share-hours",
      "24",
      "--send-at",
      "08:30"
    ]
  );
  assert!(launches[0].1.command.program.ends_with("bmo-artifact-cli"));
  assert_eq!(
    scratch
      .count(r#"SELECT COUNT(*) FROM "news_briefs" WHERE "last_day" = '2026-10-07'"#, vec![])
      .await,
    1
  );

  let handed = units.book().handed.clone();

  assert_eq!(handed.len(), 1);
  assert_eq!((handed[0].chat, handed[0].sort.as_str()), (None, "news_brief"));
  assert!(handed[0].said.order.contains("no day of 2026"), "{}", handed[0].said.order);

  // A holiday is no working day, and a year the calendar holds is told nothing of.
  let next = WEDNESDAY_EIGHT + 24 * 60 * MINUTE + 25 * MINUTE;

  set_holiday(scratch.db(), "2026-10-08", "A day off").await.expect("added");

  assert_eq!(Look::new().look(&ground, next).await.expect("looked"), 0);
  assert_eq!(stand.book().launches.len(), 1);
  assert_eq!(units.book().handed.len(), 1);
}
