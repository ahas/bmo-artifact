//! A stop from outside a verb's process: `SIGTERM` or `SIGINT`, heard from the moment it is listened for.

use tokio::signal::unix::{Signal, SignalKind, signal};

use super::verb::VerbError;

/// The two signals a run apart from its caller is stopped with.
pub struct Stop {
  terminate: Signal,
  interrupt: Signal,
}

impl Stop {
  /// Both signals listened for, refused where the process cannot hear them.
  pub fn listen() -> Result<Self, VerbError> {
    let heard = |kind| {
      signal(kind).map_err(|why| VerbError::Refused(format!("the stop signal cannot be heard: {why}")))
    };

    Ok(Self {
      terminate: heard(SignalKind::terminate())?,
      interrupt: heard(SignalKind::interrupt())?,
    })
  }

  /// The name of the signal that came.
  pub async fn next(&mut self) -> &'static str {
    tokio::select! {
      _ = self.terminate.recv() => "SIGTERM",
      _ = self.interrupt.recv() => "SIGINT",
    }
  }
}
