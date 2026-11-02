//! `GET /events` (`docs/api.md`, "Events"): the service's one stream, as server-sent events, or a page of
//! the same events for a reader that asks for JSON; narrowed by kind and scope, resumed after an event, and
//! refused `gone` where the resume is from before the window kept.

use std::collections::VecDeque;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::http::{Method, StatusCode, header};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use futures_util::{StreamExt, stream};
use schemars::generate::SchemaGenerator;
use serde_json::json;
use tokio::sync::watch;

use crate::Refusal;
use crate::plumbing::http::{Call, Refused, Route, Standing, answer};
use crate::plumbing::openapi::{Operation, header as header_parameter, json_answer, query};
use crate::store::events;
use crate::wire::{Event, EventPage, EventsQuery};

/// How often a quiet stream says it is still open.
pub const KEEP_ALIVE: Duration = Duration::from_secs(15);
/// How long a stream waits for a new event before it looks again.
pub const LOOK_AGAIN: Duration = Duration::from_secs(1);
/// How many events a page holds where none is asked, and at most.
pub const PAGE: i64 = 100;
pub const MOST: i64 = 1000;

// What a stream carries from one turn to the next.
struct Reading {
  standing: Arc<Standing>,
  newest: watch::Receiver<i64>,
  after: i64,
  kinds: Vec<String>,
  scope: Option<String>,
  ready: VecDeque<Event>,
}

async fn next(mut reading: Reading) -> Option<(Result<SseEvent, Infallible>, Reading)> {
  loop {
    if let Some(event) = reading.ready.pop_front() {
      reading.after = event.id;

      let data = serde_json::to_string(
        &json!({ "id": event.id, "kind": event.kind, "at": event.at, "data": event.data }),
      )
      .unwrap_or_default();

      return Some((
        Ok(SseEvent::default().id(event.id.to_string()).event(event.kind).data(data)),
        reading,
      ));
    }

    let db = reading.standing.db().await.ok()?;

    if let Ok((found, _)) =
      events::after(db, reading.after, &reading.kinds, reading.scope.as_deref(), PAGE).await
      && !found.is_empty()
    {
      reading.ready.extend(found);
      continue;
    }

    let _ = tokio::time::timeout(LOOK_AGAIN, reading.newest.changed()).await;
  }
}

pub struct ListEvents;

impl Route for ListEvents {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/events";

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let asked: EventsQuery = call.query()?;
    let standing = call.shared();
    let db = standing.db().await?;
    let resumed = match call.header("last-event-id") {
      Some(id) => Some(
        id.trim()
          .parse::<i64>()
          .map_err(|_| Refused::bad("Last-Event-ID is an event's number"))?,
      ),
      None => asked.after,
    };
    let kinds: Vec<String> = asked
      .kinds
      .as_deref()
      .unwrap_or_default()
      .split(',')
      .map(str::trim)
      .filter(|kind| !kind.is_empty())
      .map(str::to_owned)
      .collect();

    events::sweep(db).await?;

    if let (Some(after), Some(oldest)) = (resumed, events::oldest(db).await?)
      && after + 1 < oldest
    {
      return Err(Refusal::new(StatusCode::GONE, "the events after this one are no longer kept").into());
    }

    let wants_page = call
      .header(header::ACCEPT.as_str())
      .is_some_and(|accept| accept.contains("application/json"));

    if wants_page {
      // A page read with no cursor starts at the oldest event kept.
      let after = match resumed {
        Some(after) => after,
        None => events::oldest(db).await?.map_or(0, |oldest| oldest - 1),
      };
      let limit = match asked.limit {
        None => PAGE,
        Some(limit) if (1..=MOST).contains(&limit) => limit,
        Some(_) => return Err(Refused::bad(format!("limit is a whole number from 1 to {MOST}"))),
      };
      let (items, more) = events::after(db, after, &kinds, asked.scope.as_deref(), limit).await?;

      return Ok(answer(StatusCode::OK, &EventPage { items, more }));
    }

    // A stream opened with no cursor starts at what is kept from now on: the reader has read the collections.
    let after = match resumed {
      Some(after) => after,
      None => events::newest(db).await?.unwrap_or(0),
    };
    // The stream ends with the server's stop.
    let stopped = standing.stopped();
    let reading = Reading {
      newest: standing.events.watch(),
      standing: Arc::clone(&standing),
      after,
      kinds,
      scope: asked.scope,
      ready: VecDeque::new(),
    };

    Ok(
      Sse::new(stream::unfold(reading, next).take_until(stopped))
        .keep_alive(KeepAlive::new().interval(KEEP_ALIVE))
        .into_response(),
    )
  }

  fn operation(&self, generator: &mut SchemaGenerator) -> Operation {
    Operation {
      id: "listEvents",
      tag: "events",
      summary: "The service's one event stream (SSE), or a page of it with Accept: application/json.",
      parameters: [
        query::<EventsQuery>(generator),
        vec![header_parameter("Last-Event-ID", "The event to resume after.")],
      ]
      .concat(),
      body: None,
      answers: vec![(
        200,
        Some(json!({
          "text/event-stream": { "schema": { "type": "string" } },
          "application/json": json_answer::<EventPage>(generator).and_then(|content| content.get("application/json").cloned()),
        })),
      )],
      codes: &["unauthorized", "bad-request", "gone"],
    }
  }
}

crate::route!(ListEvents);
