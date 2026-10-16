//! A kept HTML page as it goes out to be opened (`docs/archive.md`, "An HTML page" and "Pictures open whole"):
//! the file as kept, with only a few lines put ahead of its own — the archive's sheet for the reader's scheme,
//! for a page with a dark scheme of its own the bridge that answers its scheme queries, and the clipboard,
//! picture and link bridges a page in a sandbox of its own origin needs. The file on the disk stays as it was
//! handed over; only what is served carries the lines, and a download is the file itself.

use std::cell::Cell;

use lol_html::html_content::ContentType;
use lol_html::{RewriteStrSettings, element, rewrite_str};
use scraper::{Html, Node, Selector};
use serde::Serialize;

use crate::design::{DocumentScheme, PageDesign, default_scheme, page_sheet, scheme_cookie, viewer};

/// The sandbox a page runs in: one list of permissions, which the policy carries and which the console's
/// own frame gives the page it stands; neither grants the same origin.
pub const HOSTED_PAGE_SANDBOX: &str =
  "allow-scripts allow-popups allow-popups-to-escape-sandbox allow-modals allow-downloads";

/// What a hosted page is let do: run its own scripts and draw what it carries or loads over https, open no
/// connection, submit no form, and be framed only by the console.
pub fn hosted_page_policy() -> String {
  [
    format!("sandbox {HOSTED_PAGE_SANDBOX}"),
    "default-src 'none'".to_owned(),
    "script-src 'unsafe-inline' https:".to_owned(),
    "style-src 'unsafe-inline' data: https:".to_owned(),
    "img-src data: blob: https:".to_owned(),
    "media-src data: blob: https:".to_owned(),
    "font-src data: https:".to_owned(),
    "connect-src 'none'".to_owned(),
    "form-action 'none'".to_owned(),
    "base-uri 'none'".to_owned(),
    "frame-ancestors 'self'".to_owned(),
  ]
  .join("; ")
}

/// What every other kept file is let be: a sound, a video, a picture or a file to keep, and never code.
pub const KEPT_FILE_POLICY: &str = "default-src 'none'; script-src 'none'; img-src 'self' data: blob:; media-src 'self' data: blob:; style-src 'self' 'unsafe-inline' data:; font-src 'self' data:; form-action 'none'; base-uri 'none'; frame-ancestors 'self'";

/// A copy through a selection, which needs no permission an opaque origin lacks, put ahead of the page's own
/// `writeText`, the Clipboard API kept as its fallback.
pub const CLIPBOARD_BRIDGE: &str = r#"<script>(()=>{const c=navigator.clipboard;if(!c||!c.writeText)return;const w=c.writeText.bind(c);c.writeText=(t)=>{const a=document.createElement("textarea");a.value=String(t);a.setAttribute("readonly","");a.style.cssText="position:fixed;top:0;left:0;opacity:0";document.body.append(a);a.select();let ok=false;try{ok=document.execCommand("copy")}catch{}a.remove();return ok?Promise.resolve():w(t)}})()</script>"#;

/// A link the page opens in its own window, opened in a tab of its own instead while the page stands in the
/// console's frame; a link within the page still moves the page, and one the page aimed elsewhere keeps its
/// aim.
pub const LINK_BRIDGE: &str = r##"<script>(()=>{if(window.parent===window)return;addEventListener("click",(e)=>{const l=e.target instanceof Element?e.target.closest("a[href]"):null;if(e.defaultPrevented||!l||(l.target!==""&&l.target!=="_self"))return;if((l.getAttribute("href")??"").startsWith("#")||l.protocol==="javascript:")return;l.target="_blank"})})()</script>"##;

/// A page's pictures opened in the console's picture viewer while it stands in the console's frame: the
/// pictures the shared rule lets open take the pointer that says so, and a plain press on one the page has
/// not taken for itself is handed to the frame as every such picture the page draws, in order, with the
/// pressed one among them.
pub fn picture_bridge() -> String {
  let viewer = viewer();
  let selector = serde_json::to_string(&viewer.pictures).unwrap_or_default();
  let message = serde_json::to_string(&viewer.message).unwrap_or_default();

  format!(
    r#"<script>(()=>{{if(window.parent===window)return;const q={selector};const a=/{}/{};const n={};const s=document.createElement("style");s.textContent=q+"{{cursor:zoom-in}}";document.head.append(s);const v=(i)=>{{const src=i.currentSrc||i.src;if(!a.test(src)||i.getClientRects().length===0)return null;const f=i.closest("figure")?.querySelector("figcaption");return{{src,caption:i.alt.trim()||(f?.textContent??"").trim()}}}};addEventListener("click",(e)=>{{const t=e.target;if(e.defaultPrevented||e.button!==0||e.metaKey||e.ctrlKey||e.shiftKey||e.altKey||!(t instanceof HTMLImageElement)||!t.matches(q))return;const all=[];let at=-1;for(const i of document.querySelectorAll(q)){{const p=v(i);if(p===null)continue;if(i===t)at=all.length;all.push(p)}}if(at<0)return;const from=Math.max(0,Math.min(at-n/2,all.length-n));setTimeout(()=>{{if(!e.defaultPrevented)parent.postMessage({{type:{message},at:at-from,pictures:all.slice(from,from+n)}},"*")}})}})}})()</script>"#,
    viewer.address, viewer.address_flags, viewer.most_pictures,
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

/// One preview tag a page writes itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum DeclaredMeta {
  Named { name: String, content: String },
  Property { property: String, content: String },
}

/// The tags of the head a link previews by that a page writes itself: its title, its description and robots,
/// its OpenGraph and Twitter tags and its canonical address. Each stands over the link's own tag of the same
/// name.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct DeclaredHead {
  pub title: Option<String>,
  pub meta: Vec<DeclaredMeta>,
  pub canonical: Option<String>,
}

fn declared(name: &str) -> bool {
  matches!(name, "description" | "robots")
    || name.strip_prefix("og:").is_some_and(|rest| !rest.is_empty())
    || name.strip_prefix("twitter:").is_some_and(|rest| !rest.is_empty())
}

/// What a page declares of the head a link previews by.
pub fn page_head(html: &str) -> DeclaredHead {
  let page = Html::parse_document(html);
  let mut head = DeclaredHead {
    title: page.select(&selector("title")).next().map(|title| {
      title
        .children()
        .filter_map(|child| match child.value() {
          Node::Text(text) => Some(text.to_string()),
          _ => None,
        })
        .collect::<String>()
        .trim()
        .to_owned()
    }),
    ..DeclaredHead::default()
  };

  for meta in page.select(&selector("meta")) {
    let attributes = meta.value();
    let Some(content) = attributes.attr("content") else { continue };
    let named = attributes.attr("name").unwrap_or("").to_lowercase();
    let property = attributes.attr("property").unwrap_or("").to_lowercase();

    if declared(&named) {
      head.meta.push(DeclaredMeta::Named {
        name: named,
        content: content.to_owned(),
      });
    } else if declared(&property) {
      head.meta.push(DeclaredMeta::Property {
        property,
        content: content.to_owned(),
      });
    }
  }

  head.canonical = page
    .select(&selector("link"))
    .find(|link| {
      let rel = link.value().attr("rel").unwrap_or("").to_lowercase();

      rel.split_whitespace().any(|one| one == "canonical")
        && link.value().attr("href").is_some_and(|href| !href.is_empty())
    })
    .and_then(|link| link.value().attr("href").map(str::to_owned));

  head
}

/// The scheme the reader reads in: the one the console's frame names in the address, else the screen mode
/// the reader's cookie keeps, else the default.
pub fn reader_scheme(named: Option<&str>, cookies: Option<&str>) -> DocumentScheme {
  if let Some(scheme) = named.and_then(DocumentScheme::named) {
    return scheme;
  }

  cookies
    .into_iter()
    .flat_map(cookie::Cookie::split_parse)
    .filter_map(Result::ok)
    .find(|kept| kept.name() == scheme_cookie())
    .and_then(|kept| DocumentScheme::named(kept.value()))
    .unwrap_or_else(default_scheme)
}
