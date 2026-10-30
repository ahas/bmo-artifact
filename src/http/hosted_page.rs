//! A kept HTML page as it goes out to be opened (`docs/archive.md`, "An HTML page" and "Pictures open whole"):
//! the file as kept, with only a few lines put ahead of its own — the archive's sheet for the reader's scheme,
//! the bridge that answers the page's scheme queries with the scheme it is drawn in, the clipboard, picture
//! and link bridges a page in a sandbox of its own origin needs, and for a page that writes a sound the
//! player's script and the bridge that stands each sound in it. The file on the disk stays as it was handed
//! over; only what is served carries the lines, and a download is the file itself. The sandbox, the policy
//! and the bridges are read from `client/design/`, which the console plugin bundle carries and reads too.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::LazyLock;

use lol_html::html_content::ContentType;
use lol_html::{RewriteStrSettings, element, rewrite_str};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

use crate::design::{DocumentScheme, PageDesign, default_scheme, page_sheet, viewer};

#[derive(Deserialize)]
struct Hosting {
  sandbox: String,
  policy: Vec<String>,
}

static HOSTING: LazyLock<Hosting> = LazyLock::new(|| {
  serde_json::from_str(include_str!("../../client/design/hosted-page.json"))
    .expect("the hosted page file reads")
});

/// What a hosted page is let do: run its own scripts and draw what it carries or loads over https, open no
/// connection, submit no form, and be framed only by the console. It opens on the sandbox the page runs in,
/// the one list of permissions the console's own frame gives the page it stands too; neither grants the
/// same origin.
pub fn hosted_page_policy() -> String {
  std::iter::once(format!("sandbox {}", HOSTING.sandbox))
    .chain(HOSTING.policy.iter().cloned())
    .collect::<Vec<_>>()
    .join("; ")
}

/// What every other kept file is let be: a sound, a video, a picture or a file to keep, and never code.
pub const KEPT_FILE_POLICY: &str = "default-src 'none'; script-src 'none'; img-src 'self' data: blob:; media-src 'self' data: blob:; style-src 'self' 'unsafe-inline' data:; font-src 'self' data:; form-action 'none'; base-uri 'none'; frame-ancestors 'self'";

/// A copy through a selection, which needs no permission an opaque origin lacks, put ahead of the page's own
/// `writeText`, the Clipboard API kept as its fallback.
pub const CLIPBOARD_BRIDGE: &str = concat!(
  "<script data-archive-prelude>",
  include_str!("../../client/design/bridges/clipboard.js"),
  "</script>"
);

/// A link the page opens in its own window, opened in a tab of its own instead while the page stands in the
/// console's frame; a link within the page still moves the page, and one the page aimed elsewhere keeps its
/// aim.
pub const LINK_BRIDGE: &str = concat!(
  "<script data-archive-prelude>",
  include_str!("../../client/design/bridges/link.js"),
  "</script>"
);

/// A page's pictures opened in the console's picture viewer while it stands in the console's frame: the
/// pictures the shared rule lets open take the pointer that says so, and a plain press on one the page has
/// not taken for itself is handed to the frame as every such picture the page draws, in order, with the
/// pressed one among them. The bridge is the design's script called with the design's viewer.
pub fn picture_bridge() -> String {
  let rule = serde_json::to_string(viewer()).unwrap_or_default().replace('<', "\\u003c");

  format!(
    "<script data-archive-prelude>{}({rule})</script>",
    include_str!("../../client/design/bridges/picture.js").trim_end()
  )
}

fn selector(css: &str) -> Selector {
  Selector::parse(css).expect("a selector")
}

/// What marks a line of the prelude: the sheet by its own mark, and every other line by the prelude's. A page
/// filed carrying an earlier serving's lines goes out without them.
pub const PRELUDE_LINES: &str = "[data-archive-sheet], [data-archive-prelude]";

/// How a page is to be drawn, read off what it brings: in the archive's design where it brings no stylesheet
/// of its own — a style element marked `data-archive` is an addition to the design rather than one, and an
/// earlier serving's sheet is none of the page's — and in its own layout otherwise, declaring a dark scheme of
/// its own where a `color-scheme` meta names dark or one of its own style elements asks
/// `prefers-color-scheme`.
pub fn page_design(html: &str) -> PageDesign {
  let page = Html::parse_document(html);
  let prelude = selector(PRELUDE_LINES);
  let mut own_sheet = false;
  let mut own_dark = false;

  for style in page.select(&selector("style")) {
    if style.value().attr("data-archive").is_none() && !prelude.matches(&style) {
      own_sheet = true;
      own_dark |= style.text().any(|text| text.contains("prefers-color-scheme"));
    }
  }

  for link in page.select(&selector("link")) {
    let rel = link.value().attr("rel").unwrap_or("").to_lowercase();

    own_sheet |= rel.split_whitespace().any(|one| one == "stylesheet");
  }

  for meta in page.select(&selector("meta")) {
    let named = meta.value().attr("name").unwrap_or("").eq_ignore_ascii_case("color-scheme");
    let content = meta.value().attr("content").unwrap_or("").to_lowercase();

    own_dark |= named
      && content
        .split(|one: char| !one.is_ascii_alphanumeric() && one != '_')
        .any(|word| word == "dark");
  }

  match (own_sheet, own_dark) {
    (false, _) => PageDesign::Archive,
    (true, false) => PageDesign::Own,
    (true, true) => PageDesign::OwnDark,
  }
}

/// The scheme a page's questions about the scheme are answered with: the reader's on a page in the design or
/// with a dark scheme of its own, light on a page drawn inverted, which is drawn light and turned, and none
/// where the reader's scheme is the system's and the page is not inverted, the system answering then.
pub fn answered_scheme(design: PageDesign, scheme: DocumentScheme) -> Option<DocumentScheme> {
  match (design, scheme) {
    (PageDesign::Own, _) => Some(DocumentScheme::Light),
    (_, DocumentScheme::System) => None,
    (_, chosen) => Some(chosen),
  }
}

/// The bridge that answers a page's scheme queries with the scheme it is drawn in: every rule of the page's
/// own the browser has read that asks `prefers-color-scheme` has the question replaced by one that is always
/// true for the chosen scheme and never true for the other, and a script's own question to `matchMedia` is
/// answered the same way. The archive's sheet keeps its own questions, which ask the reader's system.
pub fn scheme_bridge(scheme: DocumentScheme) -> String {
  format!(
    "<script data-archive-prelude>{}({})</script>",
    include_str!("../../client/design/bridges/scheme.js").trim_end(),
    script_text(&scheme.word())
  )
}

/// Where one sound a page keeps is played from and taken to be saved, as addresses the page loads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SoundFile {
  pub play: String,
  pub download: String,
}

/// What a page that writes a sound is handed with it (`docs/archive.md`, "An HTML page"): the player's
/// script, where the bundle carries one, and each file the document keeps by its name, at addresses the
/// sandboxed page loads; the words the player says and their language.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PageSounds {
  #[serde(skip)]
  pub player: Option<String>,
  pub files: BTreeMap<String, SoundFile>,
  pub words: serde_json::Value,
  pub locale: String,
}

/// Whether a page writes an `audio` element.
pub fn writes_sound(html: &str) -> bool {
  Html::parse_document(html).select(&selector("audio")).next().is_some()
}

// A text written into a script, so nothing in it closes the script.
fn script_text(value: &impl Serialize) -> String {
  serde_json::to_string(value).unwrap_or_default().replace('<', "\\u003c")
}

/// The lines that stand each of a page's sounds in the design's player: the player's script, deferred so
/// the page is drawn first, and the design's sound bridge called with the page's sounds and the scheme the
/// player is drawn in.
pub fn sound_bridge(sounds: &PageSounds, scheme: Option<DocumentScheme>) -> String {
  let script = sounds.player.as_ref().map_or_else(String::new, |player| {
    let address = player.replace('&', "&amp;").replace('"', "&quot;");

    format!(r#"<script data-archive-prelude defer src="{address}"></script>"#)
  });
  let mut handed = serde_json::to_value(sounds).unwrap_or_default();

  handed["scheme"] = scheme.map(DocumentScheme::word).into();

  format!(
    "{script}<script data-archive-prelude>{}({})</script>",
    include_str!("../../client/design/bridges/sound.js").trim_end(),
    script_text(&handed)
  )
}

/// What goes ahead of the page's own lines: the archive's sheet for the reader's scheme, first so the first
/// paint is already in it, then the scheme bridge where the page's questions are answered, then the
/// clipboard, picture and link bridges, and the sound lines where the page writes a sound and is handed its
/// sounds.
pub fn page_prelude(html: &str, scheme: DocumentScheme, sounds: Option<&PageSounds>) -> String {
  let design = page_design(html);
  let answered = answered_scheme(design, scheme);
  let questions = answered.map_or_else(String::new, scheme_bridge);
  let heard = match sounds {
    Some(sounds) if writes_sound(html) => sound_bridge(sounds, answered),
    _ => String::new(),
  };

  format!(
    "<style data-archive-sheet>{}</style>{questions}{CLIPBOARD_BRIDGE}{}{LINK_BRIDGE}{heard}",
    page_sheet(scheme, design),
    picture_bridge()
  )
}

/// The page with its prelude standing ahead of its first element, after its doctype, so the doctype still
/// opens the document and the page is not drawn in quirks mode, and with the lines of an earlier serving it
/// was filed carrying left out. A page with no element at all takes it first.
pub fn hosted_page(html: &str, scheme: DocumentScheme, sounds: Option<&PageSounds>) -> String {
  let prelude = page_prelude(html, scheme, sounds);
  let placed = Cell::new(false);
  let rewritten = rewrite_str(
    html,
    RewriteStrSettings::new()
      .append_element_content_handler(element!("*", |element| {
        if !placed.replace(true) {
          element.before(&prelude, ContentType::Html);
        }

        Ok(())
      }))
      .append_element_content_handler(element!(PRELUDE_LINES, |element| {
        element.remove();

        Ok(())
      })),
  );

  match rewritten {
    Ok(page) if placed.get() => page,
    _ => format!("{prelude}{html}"),
  }
}

/// The scheme the reader reads in: the one the request names, else the design's default.
pub fn reader_scheme(named: Option<&str>) -> DocumentScheme {
  named.and_then(DocumentScheme::named).unwrap_or_else(default_scheme)
}
