//! A kept HTML page as it goes out (`docs/archive.md`, "An HTML page"): where its prelude stands, the design
//! it is drawn in, its sheet in each scheme, the head it declares, and the scheme its reader reads in.
//! Carried from hosted-page.test.ts.

use bmo_artifact::design::{DocumentScheme, PageDesign, page_sheet};
use bmo_artifact::http::hosted_page::{
  CLIPBOARD_BRIDGE, LINK_BRIDGE, hosted_page, page_design, page_prelude, picture_bridge, reader_scheme,
  scheme_bridge,
};
use bmo_artifact::markup::page_head::{DeclaredHead, DeclaredMeta, page_head};

use DocumentScheme::{Dark, Light, System};

// ---------------------------------------------------------------- where the prelude stands

#[test]
fn carries_its_prelude_after_its_doctype_ahead_of_its_first_element() {
  assert_eq!(
    hosted_page("<!doctype html><p>x</p>", Dark),
    format!("<!doctype html>{}<p>x</p>", page_prelude("<p>x</p>", Dark))
  );
  // The prelude stands ahead of the first element, so a line break written after the doctype, and a comment
  // before it, stay where the page wrote them.
  assert_eq!(
    hosted_page("\u{feff}<!-- a -->\n<!DOCTYPE html>\n<html>", Dark),
    format!("\u{feff}<!-- a -->\n<!DOCTYPE html>\n{}<html>", page_prelude("<html>", Dark))
  );
}

#[test]
fn carries_it_first_where_the_page_has_no_doctype_or_no_element() {
  assert_eq!(
    hosted_page("<html><p>x</p>", Dark),
    format!("{}<html><p>x</p>", page_prelude("<html>", Dark))
  );
  assert_eq!(hosted_page("just words", Dark), format!("{}just words", page_prelude("", Dark)));
}

#[test]
fn opens_on_the_sheet_of_the_readers_scheme_the_clipboard_picture_and_link_bridges_after_it() {
  assert_eq!(
    page_prelude("<p>x</p>", Light),
    format!(
      "<style data-archive-sheet>{}</style>{CLIPBOARD_BRIDGE}{}{LINK_BRIDGE}",
      page_sheet(Light, PageDesign::Archive),
      picture_bridge()
    )
  );
}

#[test]
fn the_picture_bridge_reads_the_one_rule_the_console_reads() {
  let bridge = picture_bridge();

  assert!(bridge.starts_with("<script>((viewer) => {"), "{bridge}");
  assert!(
    bridge.contains(r#"})({"pictures":"img:not(a[href] img, button img, [role=\"button\"] img"#),
    "{bridge}"
  );
  assert!(
    bridge.contains(r#""address":"^(?:https?:|data:image\\/)","address_flags":"i","message":"bmo-pictures","most_pictures":500"#),
    "{bridge}"
  );
  assert!(bridge.ends_with("})</script>"), "{bridge}");
}

// ---------------------------------------------------------------- the design a page is drawn in

#[test]
fn is_the_archives_where_the_page_brings_no_stylesheet_its_own_marked_additions_aside() {
  assert_eq!(page_design("<h1>x</h1><p style='color:red'>y</p>"), PageDesign::Archive);
  assert_eq!(
    page_design("<style data-archive>.x{color:var(--doc-accent)}</style><h1>x</h1>"),
    PageDesign::Archive
  );
}

#[test]
fn is_its_own_where_it_brings_a_style_element_or_a_linked_sheet() {
  assert_eq!(page_design("<style>body{color:#222}</style>"), PageDesign::Own);
  assert_eq!(
    page_design(r#"<link rel="preload stylesheet" href="https://example.com/a.css">"#),
    PageDesign::Own
  );
}

#[test]
fn is_its_own_in_both_schemes_where_it_answers_a_dark_system_or_names_dark() {
  assert_eq!(
    page_design("<style>@media (prefers-color-scheme:dark){body{background:#000}}</style>"),
    PageDesign::OwnDark
  );
  assert_eq!(
    page_design(r#"<meta name="color-scheme" content="light dark"><style>body{color:#222}</style>"#),
    PageDesign::OwnDark
  );
  assert_eq!(
    page_design("<style data-archive>@media (prefers-color-scheme:dark){}</style><style>b{}</style>"),
    PageDesign::Own
  );
}

// ---------------------------------------------------------------- a page's sheet

#[test]
fn draws_a_page_with_no_design_of_its_own_in_the_palette_of_the_scheme() {
  assert!(page_sheet(Dark, PageDesign::Archive).contains("--doc-paper:#141416"));
  assert!(page_sheet(Light, PageDesign::Archive).contains("--doc-paper:#f6f5f1"));
  assert!(
    page_sheet(System, PageDesign::Archive)
      .contains("@media (prefers-color-scheme:dark){:root{--doc-paper:#141416")
  );
}

#[test]
fn puts_the_archives_type_over_a_designed_pages_bare_elements_in_the_light_palette_inverted_in_dark() {
  let dark = page_sheet(Dark, PageDesign::Own);

  assert!(dark.contains("--doc-paper:#f6f5f1"));
  assert!(dark.contains("html h1{"));
  assert!(!dark.contains("max-width:calc(48rem"));
  assert!(dark.contains("filter:invert(1) hue-rotate(180deg)"));
  assert!(!page_sheet(Light, PageDesign::Own).contains("filter:invert"));
  assert!(
    page_sheet(System, PageDesign::Own).contains("@media (prefers-color-scheme:dark){html{filter:invert")
  );
}

#[test]
fn leaves_a_page_with_its_own_dark_scheme_to_draw_itself() {
  assert_eq!(page_sheet(Dark, PageDesign::OwnDark), ":root{color-scheme:dark}");
  assert_eq!(page_sheet(System, PageDesign::OwnDark), ":root{color-scheme:light dark}");
}

#[test]
fn answers_a_self_dark_pages_scheme_queries_with_the_readers_choice_and_leaves_a_system_readers_alone() {
  let page = "<style>@media (prefers-color-scheme:dark){body{background:#000}}</style>";

  assert!(page_prelude(page, Light).contains(&scheme_bridge(Light)));
  assert!(!page_prelude(page, System).contains("<script>(()=>{const w="));
  assert!(!page_prelude("<style>b{}</style>", Dark).contains("<script>(()=>{const w="));
}

// ---------------------------------------------------------------- the head a page declares

#[test]
fn is_its_title_the_preview_tags_it_writes_in_lower_case_and_its_canonical_address() {
  let page = concat!(
    r#"<html><head><meta charset=utf-8><title> My &amp; Title </title><meta name=Description content="My description"><meta content="x" property='OG:Image'>"#,
    r#"<meta name="viewport" content="width=device-width"><meta name="twitter:card" content="summary"><link rel=canonical href="/x"></head><title>Second</title>"#
  );

  assert_eq!(
    page_head(page),
    DeclaredHead {
      title: Some("My & Title".to_owned()),
      meta: vec![
        DeclaredMeta::Named {
          name: "description".to_owned(),
          content: "My description".to_owned()
        },
        DeclaredMeta::Property {
          property: "og:image".to_owned(),
          content: "x".to_owned()
        },
        DeclaredMeta::Named {
          name: "twitter:card".to_owned(),
          content: "summary".to_owned()
        },
      ],
      canonical: Some("/x".to_owned()),
    }
  );
}

#[test]
fn is_nothing_where_the_page_writes_none_of_it() {
  assert_eq!(page_head("<p>x</p>"), DeclaredHead::default());
}

// ---------------------------------------------------------------- the scheme the reader reads in

#[test]
fn is_the_one_the_frame_names_else_the_one_the_cookie_keeps_else_dark() {
  assert_eq!(reader_scheme(Some("light"), Some("nuxt-color-mode=dark")), Light);
  assert_eq!(reader_scheme(Some("sepia"), Some("a=1; nuxt-color-mode=system")), System);
  assert_eq!(reader_scheme(None, Some("nuxt-color-mode=sepia")), Dark);
  assert_eq!(reader_scheme(None, None), Dark);
}
