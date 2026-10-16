//! A result's pictures read as what they show (`docs/archive.md`, "Pictures, reviews and footnotes" and
//! "The pictures a search keeps"): the pictures the run actually looked at, which alone may stand in its result, and the albums the
//! room is sent them in, one to each entry and shot, so a reader compares like with like.
//!
//! A picture's line is `IMAGE: <address> :: <caption>`, and a picture of something chosen between carries
//! three more fields after the caption: the entry it belongs to, its shot and its source (the business's own
//! sample or a customer's photo).

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::helpers::run_log::json::Json;
use crate::helpers::run_log::lines::stream_lines;
use crate::helpers::toolbook::mcp_call_of;

// The picture viewer, and the tool a pooled branch reports through, as the web server names them.
const VIEWER: &str = "view_image";
const BRANCH: &str = "research_branch";

static IMAGE_LINE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"^[ \t]*IMAGE:[ \t]*(\S+)(?:[ \t]*::[ \t]*(.*?))?[ \t]*$").expect("a pattern")
});
static NAME_LINE: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"^[ \t]*NAME:[ \t]*(.*?)[ \t]*$").expect("a pattern"));
static ITEM_LINE: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"^[ \t]*===ITEM===[ \t]*$").expect("a pattern"));
static FIELD_SPLIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[ \t]*::[ \t]*").expect("a pattern"));

/// One picture a result names, with what its line and the entry it stands in say of it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Picture {
  pub address: String,
  pub caption: String,
  /// The entry it belongs to: its own field where it names one, otherwise the `NAME:` it stands under.
  pub entry: Option<String>,
  /// The composition or kind of shot, which the entry's pictures are grouped by.
  pub shot: Option<String>,
  /// Whose picture it is: the business's or maker's own sample, or a customer's photo.
  pub source: Option<String>,
}

fn filled(value: Option<&str>) -> Option<String> {
  value.map(str::trim).filter(|value| !value.is_empty()).map(str::to_owned)
}

/// The pictures a result names, in the order written, each with the entry it stands in.
pub fn pictures(result: &str) -> Vec<Picture> {
  let mut entry: Option<String> = None;
  let mut found = Vec::new();

  for line in result.split('\n') {
    if ITEM_LINE.is_match(line) {
      entry = None;
    } else if let Some(name) = NAME_LINE.captures(line) {
      entry = filled(Some(&name[1]));
    } else if let Some(image) = IMAGE_LINE.captures(line) {
      let rest = image.get(2).map_or("", |rest| rest.as_str());
      let fields: Vec<&str> = FIELD_SPLIT.split(rest).collect();
      let field = |at: usize| filled(fields.get(at).copied());

      found.push(Picture {
        address: image[1].to_owned(),
        caption: field(0).unwrap_or_default(),
        entry: field(1).or_else(|| entry.clone()),
        shot: field(2),
        source: field(3),
      });
    }
  }

  found
}

fn is_web_address(value: &str) -> bool {
  url::Url::parse(value).is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
}

// One address as two writings of it compare: a page's raw HTML hands addresses over with their ampersands
// escaped, and the run may give the tool one writing and its line the other.
fn same_picture(address: &str) -> String {
  address.trim().replace("&amp;", "&")
}

/// The addresses a run looked at, read off its stream: every picture its viewer showed it, its own or an
/// in-process branch's, and every picture a pooled branch's report names, which that branch's launcher let
/// through only where the branch looked at it. None where the stream shows no tool call at all, which says
/// the stream could not be read rather than that nothing was looked at.
pub fn looked_at(stream: &str) -> Option<HashSet<String>> {
  let mut seen = HashSet::new();
  let mut branches: HashSet<String> = HashSet::new();
  let mut called = false;

  for line in stream.split('\n').map(str::trim).filter(|line| line.starts_with('{')) {
    let Some(event) = Json::parse(line) else { continue };

    for logged in stream_lines(&event) {
      match logged.kind {
        "tool" => {
          called = true;

          let tool = logged.name.as_deref().and_then(mcp_call_of).map(|(_, tool)| tool);
          let input = Json::parse(&logged.text);

          match tool.as_deref() {
            Some(VIEWER) => {
              if let Some(image) =
                input.as_ref().and_then(|input| input.get("image")).and_then(Json::as_str)
              {
                seen.insert(same_picture(image));
              }
            }
            Some(BRANCH) => branches.extend(logged.reference.clone()),
            _ => {}
          }
        }
        "tool-result"
          if logged.reference.as_ref().is_some_and(|reference| branches.contains(reference)) =>
        {
          seen.extend(pictures(&logged.text).iter().map(|picture| same_picture(&picture.address)));
        }
        _ => {}
      }
    }
  }

  called.then_some(seen)
}

/// The result with every picture taken out whose web address the run never looked at, and how many went. A
/// line naming a file this machine keeps is no picture fetched from the web and stays.
pub fn only_looked_at(result: &str, looked: &HashSet<String>) -> (String, usize) {
  let mut dropped = 0;
  let kept: Vec<&str> = result
    .split('\n')
    .filter(|line| {
      let unlooked = IMAGE_LINE
        .captures(line)
        .is_some_and(|image| is_web_address(&image[1]) && !looked.contains(&same_picture(&image[1])));

      dropped += usize::from(unlooked);
      !unlooked
    })
    .collect();

  (kept.join("\n"), dropped)
}

/// One picture as the photo tool takes it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Sent {
  pub url: String,
  #[serde(skip_serializing_if = "String::is_empty")]
  pub label: String,
}

/// One album: one photo call, its pictures in order and its caption.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Album {
  pub pictures: Vec<Sent>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub caption: Option<String>,
}

// What an album gathers its pictures by: the entry and the shot, none for the pictures carrying no shot.
type AlbumKey = Option<(Option<String>, String)>;

fn joined(parts: &[Option<&String>]) -> String {
  parts.iter().flatten().map(|part| part.as_str()).collect::<Vec<_>>().join(" · ")
}

/// The albums a result's pictures go to the room in, in the order the pictures stand: one to each entry and
/// shot, captioned with both, every picture labelled with its entry, shot and source; the pictures carrying
/// no shot together as one album where the first of them stands, each labelled with its entry, or with its
/// caption where it belongs to none.
pub fn albums(result: &str) -> Vec<Album> {
  let mut albums: Vec<(AlbumKey, Album)> = Vec::new();

  for picture in pictures(result) {
    let key = picture.shot.clone().map(|shot| (picture.entry.clone(), shot));
    let label = match &picture.shot {
      Some(shot) => joined(&[picture.entry.as_ref(), Some(shot), picture.source.as_ref()]),
      None => picture.entry.clone().unwrap_or_else(|| picture.caption.clone()),
    };
    let sent = Sent {
      url: picture.address.clone(),
      label,
    };

    match albums.iter_mut().find(|(known, _)| *known == key) {
      Some((_, album)) => album.pictures.push(sent),
      None => {
        let caption = key.as_ref().map(|(entry, shot)| joined(&[entry.as_ref(), Some(shot)]));

        albums.push((
          key,
          Album {
            pictures: vec![sent],
            caption,
          },
        ));
      }
    }
  }

  albums.into_iter().map(|(_, album)| album).collect()
}
