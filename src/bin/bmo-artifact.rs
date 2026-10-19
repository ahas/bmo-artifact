//! The HTTP server: every registered route, on the address the settings name, and every registered loop
//! while it listens.

use std::process::ExitCode;
use std::sync::Arc;

use bmo_artifact::plumbing::background::{loop_faults, start};
use bmo_artifact::plumbing::http::{Standing, router};
use bmo_artifact::settings::Settings;
use tokio::net::TcpListener;
use tokio::signal::unix::{SignalKind, signal};

#[tokio::main]
async fn main() -> ExitCode {
  let settings = match Settings::load() {
    Ok(settings) => settings,
    Err(why) => {
      eprintln!("{why}");
      return ExitCode::FAILURE;
    }
  };
  let listen = settings.server.listen;
  let standing = match Standing::new(settings) {
    Ok(standing) => standing,
    Err(why) => {
      eprintln!("{why}");
      return ExitCode::FAILURE;
    }
  };
  let faults = loop_faults();

  if !faults.is_empty() {
    eprintln!("the loops cannot be run: {}", faults.join("; "));
    return ExitCode::FAILURE;
  }

  let looping = Arc::new(standing.clone());
  let router = match router(standing) {
    Ok(router) => router,
    Err(faults) => {
      eprintln!("the routes cannot be served: {}", faults.join("; "));
      return ExitCode::FAILURE;
    }
  };
  let listener = match TcpListener::bind(listen).await {
    Ok(listener) => listener,
    Err(why) => {
      eprintln!("{listen} cannot be listened on: {why}");
      return ExitCode::FAILURE;
    }
  };

  eprintln!("bmo-artifact {} listens on {listen}", bmo_artifact::VERSION);

  let loops = start(looping);
  let served = axum::serve(listener, router).with_graceful_shutdown(stopped()).await;

  loops.stop();

  match served {
    Ok(()) => ExitCode::SUCCESS,
    Err(why) => {
      eprintln!("the server stopped: {why}");
      ExitCode::FAILURE
    }
  }
}

// Ends once the process is asked to stop, by an interrupt or by its unit.
async fn stopped() {
  let Ok(mut terminate) = signal(SignalKind::terminate()) else {
    let _ = tokio::signal::ctrl_c().await;
    return;
  };

  tokio::select! {
    _ = tokio::signal::ctrl_c() => {}
    _ = terminate.recv() => {}
  }
}
