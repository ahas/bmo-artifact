//! A kept HTML page as it goes out to be opened (`docs/archive.md`, "An HTML page" and "Pictures open whole"):
//! the file as kept, with only a few lines put ahead of its own — the archive's sheet for the reader's scheme,
//! for a page with a dark scheme of its own the bridge that answers its scheme queries, and the clipboard,
//! picture and link bridges a page in a sandbox of its own origin needs. The file on the disk stays as it was
//! handed over; only what is served carries the lines, and a download is the file itself. The sandbox, the
//! policy and the bridges are read from `client/design/`, which the console plugin bundle carries and reads
//! too.

use std::cell::Cell;
use std::sync::LazyLock;

use lol_html::html_content::ContentType;
use lol_html::{RewriteStrSettings, element, rewrite_str};
use scraper::{Html, Selector};
use serde::Deserialize;

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
pub const CLIPBOARD_BRIDGE: &str =
  concat!("<script>", include_str!("../../client/design/bridges/clipboard.js"), "</script>");

/// A link the page opens in its own window, opened in a tab of its own instead while the page stands in the
/// console's frame; a link within the page still moves the page, and one the page aimed elsewhere keeps its
/// aim.
pub const LINK_BRIDGE: &str =
  concat!("<script>", include_str!("../../client/design/bridges/link.js"), "</script>");

/// A page's pictures opened in the console's picture viewer while it stands in the console's frame: the
/// pictures the shared rule lets open take the pointer that says so, and a plain press on one the page has
/// not taken for itself is handed to the frame as every such picture the page draws, in order, with the
/// pressed one among them. The bridge is the design's script called with the design's viewer.
pub fn picture_bridge() -> String {
  let rule = serde_json::to_string(viewer()).unwrap_or_default().replace('<', "\\u003c");

  format!(
    "<script>{}({rule})</script>",
    include_str!("../../client/design/bridges/picture.js").trim_end()
  )
}

fn selector(css: &str) -> Selector {
  Selector::parse(css).expect("a selector")
}

/// How a page is to be drawn, read off what it brings: in the archive's design where it brings no stylesheet
/// of its own — a style element marked `data-archive` is an addition to the design rather than one — and in
/// its own layout otherwise, declaring a dark scheme of its own where a `color-scheme` meta names dark or one
/// of its own style elements asks `prefers-color-scheme`.
pub fn page_design(html: &str) -> PageDesign {
  let page = Html::parse_document(html);
  let mut own_sheet = false;
  let mut own_dark = false;

  for style in page.select(&selector("style")) {
    if style.value().attr("data-archive").is_none() {
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

/// The bridge that answers a self-dark page's scheme queries with the reader's choice: every rule the browser
/// has read that asks `prefers-color-scheme` has the question replaced by one that is always true for the
/// chosen scheme and never true for the other, and a script's own question is answered the same way.
pub fn scheme_bridge(scheme: DocumentScheme) -> String {
  let chosen = serde_json::to_string(scheme.word()).unwrap_or_default();

  format!(
    r#"<script>(()=>{{const w={chosen};const t=(q)=>String(q).replace(/\(\s*prefers-color-scheme\s*:\s*(dark|light)\s*\)/gi,(_,v)=>v.toLowerCase()===w?"(min-width: 0px)":"(max-width: -1px)");const s=(rules)=>{{for(const r of rules){{if(r.media&&/prefers-color-scheme/i.test(r.media.mediaText))r.media.mediaText=t(r.media.mediaText);if(r.cssRules)s(r.cssRules)}}}};const a=()=>{{for(const sheet of document.styleSheets){{try{{s(sheet.cssRules)}}catch{{}}}}}};new MutationObserver(a).observe(document.documentElement,{{childList:true,subtree:true}});document.addEventListener("DOMContentLoaded",a);addEventListener("load",a);const m=matchMedia.bind(window);window.matchMedia=(q)=>m(t(q))}})()</script>"#
  )
}

/// What goes ahead of the page's own lines: the archive's sheet for the reader's scheme, first so the first
/// paint is already in it, then the scheme bridge where it is wanted, then the clipboard, picture and link
/// bridges.
pub fn page_prelude(html: &str, scheme: DocumentScheme) -> String {
  let design = page_design(html);
  let answered = if design == PageDesign::OwnDark && scheme != DocumentScheme::System {
    scheme_bridge(scheme)
  } else {
    String::new()
  };

  format!(
    "<style data-archive-sheet>{}</style>{answered}{CLIPBOARD_BRIDGE}{}{LINK_BRIDGE}",
    page_sheet(scheme, design),
    picture_bridge()
  )
}

/// The page with its prelude standing ahead of its first element, after its doctype, so the doctype still
/// opens the document and the page is not drawn in quirks mode. A page with no element at all takes it
/// first.
pub fn hosted_page(html: &str, scheme: DocumentScheme) -> String {
  let prelude = page_prelude(html, scheme);
  let placed = Cell::new(false);
  let rewritten = rewrite_str(
    html,
    RewriteStrSettings::new().append_element_content_handler(element!("*", |element| {
      if !placed.replace(true) {
        element.before(&prelude, ContentType::Html);
      }

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
