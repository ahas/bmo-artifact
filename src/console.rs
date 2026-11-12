//! The document screens' console face (`docs/api.md`, "The console"): this service's declaration and the bundle
//! folder the build of `client/` writes, handed to bmo-chat's `bmo-console-face`, which answers the manifest,
//! the bundle's files and their guard; and how a deployment's console probe reaches the service.

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

use bmo_console_face::{BundleFolder, ConsoleFace, ConsoleFaceError};
use serde_json::{Value, json};

use crate::http::share_head::SHARE_HEAD;
use crate::http::share_reads::SHARE_READS;
use crate::settings::{Console, Settings};

// The files the build writes into the bundle folder that the manifest names.
const MODULE_FILE: &str = "bundle.js";
const STYLESHEET_FILE: &str = "bundle.css";
const WORDS_FILE: &str = "words.json";

/// The locale a word is read in where the reader's holds none.
pub const WORDS_LOCALE: &str = "en";

/// Every manifest field this service decides, naming the other services the screens call as `calls` names them.
pub fn declaration(calls: &[String]) -> Value {
  json!({
    "name": "artifact",
    "title": "artifact.title.screen",
    "icon": "i-material-symbols-archive-outline",
    "screenPath": "/artifact",
    "formerScreenPaths": ["/archive"],
    "serviceName": "artifact",
    "role": "member",
    "ownerKey": false,
    "events": false,
    "websocket": false,
    "pushKinds": [],
    "shareReads": SHARE_READS,
    "shareHead": SHARE_HEAD,
    "calls": calls,
  })
}

/// The face the settings' console stands on: none where they name no console token, or the face's refusal of
/// the bundle folder or the declaration.
pub fn face(console: &Console) -> Result<Option<ConsoleFace>, ConsoleFaceError> {
  if console.token.is_empty() {
    return Ok(None);
  }

  let folder = BundleFolder::new(&console.bundle, MODULE_FILE, WORDS_FILE).with_stylesheet(STYLESHEET_FILE);

  ConsoleFace::new(&declaration(&console.calls).to_string(), &folder, &console.token).map(Some)
}

/// A word of the bundle's words by its dotted key, in the locale `language` names, else in English.
pub fn word<'a>(face: &'a ConsoleFace, language: Option<&str>, key: &str) -> Option<&'a str> {
  language
    .and_then(|language| face.word(language, key))
    .or_else(|| face.word(WORDS_LOCALE, key))
}

/// How the testbed's console probe reaches the service the settings name: the address its routes answer at,
/// an unspecified listen address read as the loopback one, the console's token and the service's.
pub fn reach(settings: &Settings) -> Value {
  let mut listen = settings.server.listen;

  if listen.ip().is_unspecified() {
    listen.set_ip(match listen {
      SocketAddr::V4(_) => Ipv4Addr::LOCALHOST.into(),
      SocketAddr::V6(_) => Ipv6Addr::LOCALHOST.into(),
    });
  }

  json!({
    "address": format!("http://{listen}"),
    "consoleToken": settings.console.token,
    "token": settings.server.token,
  })
}
