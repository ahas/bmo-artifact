//! The service's own events (`docs/api.md`, "Events"): each change kept in the store with the scope of the
//! member it is about, and every reader of the stream woken to read it.

use std::sync::Arc;

use sea_orm::ConnectionTrait;
use serde::Serialize;
use tokio::sync::watch;

use crate::Refusal;
use crate::store::events;

/// A resource and its change, as the stream names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
  DocumentCreated,
  DocumentChanged,
  DocumentDeleted,
  AttachmentCreated,
  ShareCreated,
  ShareDeleted,
  ImportCreated,
  ImportChanged,
}

impl EventKind {
  /// Every kind the service publishes.
  pub const ALL: [Self; 8] = [
    Self::DocumentCreated,
    Self::DocumentChanged,
    Self::DocumentDeleted,
    Self::AttachmentCreated,
    Self::ShareCreated,
    Self::ShareDeleted,
    Self::ImportCreated,
    Self::ImportChanged,
  ];

  /// The kind as the stream names it: `document.created`.
  pub fn name(self) -> &'static str {
    match self {
      Self::DocumentCreated => "document.created",
      Self::DocumentChanged => "document.changed",
      Self::DocumentDeleted => "document.deleted",
      Self::AttachmentCreated => "attachment.created",
      Self::ShareCreated => "share.created",
      Self::ShareDeleted => "share.deleted",
      Self::ImportCreated => "import.created",
      Self::ImportChanged => "import.changed",
    }
  }
}

/// Where events are published: the store keeps each, and the newest number wakes every reader.
#[derive(Clone, Debug)]
pub struct Publisher {
  newest: Arc<watch::Sender<i64>>,
}

impl Default for Publisher {
  fn default() -> Self {
    Self {
      newest: Arc::new(watch::channel(0).0),
    }
  }
}

impl Publisher {
  /// One event kept and told: its kind, the scope of the member it is about, and the member.
  pub async fn publish<C: ConnectionTrait>(
    &self,
    db: &C,
    kind: EventKind,
    scope: Option<&str>,
    data: &impl Serialize,
  ) -> Result<(), Refusal> {
    let data = serde_json::to_value(data).map_err(|why| Refusal::failed(why.to_string()))?;
    let kept = events::keep(db, kind.name(), scope, &data).await?;

    self.newest.send_replace(kept.id);

    Ok(())
  }

  /// A watch on the newest event's number, which changes whenever one is published.
  pub fn watch(&self) -> watch::Receiver<i64> {
    self.newest.subscribe()
  }
}
