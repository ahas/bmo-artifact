//! A kept HTML page as it goes out (`docs/archive.md`, "An HTML page"): where its prelude stands, the design
//! it is drawn in, its sheet in each scheme, and the scheme its reader reads in.
//! Carried from hosted-page.test.ts.

use std::collections::BTreeMap;

use bmo_artifact::design::{DocumentScheme, PageDesign, page_sheet};
use bmo_artifact::http::hosted_page::{
  CLIPBOARD_BRIDGE, LINK_BRIDGE, PageSounds, SoundFile, answered_scheme, hosted_page, page_design,
  page_prelude, picture_bridge, reader_scheme, scheme_bridge, sound_bridge, writes_sound,
};

use DocumentScheme::{Dark, Light, System};

// ---------------------------------------------------------------- where the prelude stands

#[test]
fn carries_its_prelude_after_its_doctype_ahead_of_its_first_element() {
  assert_eq!(
    hosted_page("<!doctype html><p>x</p>", Dark, None),
    format!("<!doctype html>{}<p>x</p>", page_prelude("<p>x</p>", Dark, None))
  );
  // The prelude stands ahead of the first element, so a line break written after the doctype, and a comment
  // before it, stay where the page wrote them.
  assert_eq!(
    hosted_page("\u{feff}<!-- a -->\n<!DOCTYPE html>\n<html>", Dark, None),
    format!("\u{feff}<!-- a -->\n<!DOCTYPE html>\n{}<html>", page_prelude("<html>", Dark, None))
  );
}

#[test]
fn carries_it_first_where_the_page_has_no_doctype_or_no_element() {
  assert_eq!(
    hosted_page("<html><p>x</p>", Dark, None),
    format!("{}<html><p>x</p>", page_prelude("<html>", Dark, None))
  );
  assert_eq!(
    hosted_page("just words", Dark, None),
    format!("{}just words", page_prelude("", Dark, None))
  );
}

#[test]
fn opens_on_the_sheet_of_the_readers_scheme_the_scheme_clipboard_picture_and_link_bridges_after_it() {
  assert_eq!(
    page_prelude("<p>x</p>", Light, None),
    format!(
      "<style data-archive-sheet>{}</style>{}{CLIPBOARD_BRIDGE}{}{LINK_BRIDGE}",
      page_sheet(Light, PageDesign::Archive),
      scheme_bridge(Light),
      picture_bridge()
    )
  );
}

#[test]
fn marks_every_line_it_writes_as_the_preludes() {
  let prelude = page_prelude("<audio src=\"attachment:a.ogg\"></audio>", Dark, Some(&sounds()));
  let page = scraper::Html::parse_fragment(&prelude);
  let unmarked: Vec<String> = page
    .select(&scraper::Selector::parse("style, script").expect("a selector"))
    .filter(|line| line.attr("data-archive-sheet").is_none() && line.attr("data-archive-prelude").is_none())
    .map(|line| line.html())
    .collect();

  assert_eq!(unmarked, Vec::<String>::new());
}

#[test]
fn the_picture_bridge_reads_the_one_rule_the_console_reads() {
  let bridge = picture_bridge();

  assert!(bridge.starts_with("<script data-archive-prelude>((viewer) => {"), "{bridge}");
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

// ---------------------------------------------------------------- a page's questions about the scheme

#[test]
fn answers_a_pages_scheme_questions_with_the_scheme_it_is_drawn_in() {
  // In the design, and with a dark scheme of its own, a page is drawn in the reader's scheme; drawn inverted,
  // it is drawn light and turned, whatever the reader's scheme.
  assert_eq!(answered_scheme(PageDesign::Archive, Dark), Some(Dark));
  assert_eq!(answered_scheme(PageDesign::Archive, Light), Some(Light));
  assert_eq!(answered_scheme(PageDesign::OwnDark, Dark), Some(Dark));
  assert_eq!(answered_scheme(PageDesign::Own, Dark), Some(Light));
  assert_eq!(answered_scheme(PageDesign::Own, System), Some(Light));
  // A reader whose scheme is the system's leaves the questions to the system.
  assert_eq!(answered_scheme(PageDesign::Archive, System), None);
  assert_eq!(answered_scheme(PageDesign::OwnDark, System), None);
}

#[test]
fn puts_the_answer_ahead_of_the_pages_own_scripts_a_diagrams_theme_among_them() {
  // A research page in the design, whose diagram library asks `matchMedia` which theme to draw in.
  let page =
    r#"<style data-archive>.switch{}</style><script>matchMedia("(prefers-color-scheme: dark)")</script>"#;

  assert!(page_prelude(page, Dark, None).contains(&scheme_bridge(Dark)));
  assert!(page_prelude(page, Light, None).contains(&scheme_bridge(Light)));
  assert!(!page_prelude(page, System, None).contains("((chosen)"));
  assert!(page_prelude("<style>b{}</style>", Dark, None).contains(&scheme_bridge(Light)));
  assert!(scheme_bridge(Dark).ends_with("})(\"dark\")</script>"), "{}", scheme_bridge(Dark));
}

// ---------------------------------------------------------------- one sheet, the reader's

// A page filed carrying the lines an earlier serving put ahead of it, in the dark scheme.
fn served_once() -> String {
  format!(
    "<!doctype html>{}<html lang=\"en\"><head><style data-archive>.switch{{color:var(--doc-muted)}}</style></head><body><h1>x</h1></body></html>",
    page_prelude("<h1>x</h1>", Dark, None)
  )
}

#[test]
fn reads_an_earlier_servings_sheet_as_none_of_the_pages_own() {
  assert_eq!(page_design(&served_once()), PageDesign::Archive);
}

#[test]
fn sends_exactly_one_palette_the_readers_whatever_lines_the_page_was_filed_carrying() {
  for scheme in [Light, Dark] {
    let page = hosted_page(&served_once(), scheme, None);

    assert_eq!(page.matches("<style data-archive-sheet>").count(), 1, "{page}");
    assert_eq!(page.matches("--doc-paper:").count(), 1, "{page}");
    assert!(page.contains(&format!(
      "<style data-archive-sheet>{}</style>",
      page_sheet(scheme, PageDesign::Archive)
    )));
    assert_eq!(page.matches(&scheme_bridge(scheme)).count(), 1);
    assert!(!page.contains(&scheme_bridge(if scheme == Light { Dark } else { Light })));
    assert_eq!(page.matches(LINK_BRIDGE).count(), 1);
  }

  assert!(hosted_page(&served_once(), Light, None).contains("--doc-paper:#f6f5f1"));
  assert!(hosted_page(&served_once(), Dark, None).contains("--doc-paper:#141416"));
}

// ---------------------------------------------------------------- a page's sounds

fn sounds() -> PageSounds {
  PageSounds {
    player: Some("../../player/bmo-audio-player.0123456789abcdef.js".into()),
    files: BTreeMap::from([(
      "narration-ko.ogg".to_owned(),
      SoundFile {
        play: "../../readings/abc/attachments/7".into(),
        download: "../../readings/abc/attachments/7?download=true".into(),
      },
    )]),
    words: serde_json::json!({ "common": { "action": { "play": "Play" } } }),
    locale: "en".into(),
  }
}

#[test]
fn knows_a_page_that_writes_a_sound() {
  assert!(writes_sound(r#"<section><audio src="attachment:narration-ko.ogg"></audio></section>"#));
  assert!(!writes_sound("<p>audio</p>"));
}

#[test]
fn hands_a_page_that_writes_a_sound_the_player_and_its_sounds_after_the_other_lines() {
  let page = r#"<style data-archive>.a{}</style><audio src="attachment:narration-ko.ogg" data-language="ko"></audio>"#;
  let prelude = page_prelude(page, Dark, Some(&sounds()));

  assert!(prelude.ends_with(&format!("{LINK_BRIDGE}{}", sound_bridge(&sounds(), Some(Dark)))));

  let bridge = sound_bridge(&sounds(), Some(Dark));

  assert!(bridge.starts_with(
    r#"<script data-archive-prelude defer src="../../player/bmo-audio-player.0123456789abcdef.js"></script><script data-archive-prelude>((sounds) => {"#
  ));
  assert!(bridge.contains(r#""files":{"narration-ko.ogg":{"#), "{bridge}");
  assert!(bridge.contains(r#""play":"../../readings/abc/attachments/7""#), "{bridge}");
  assert!(
    bridge.contains(r#""download":"../../readings/abc/attachments/7?download=true""#),
    "{bridge}"
  );
  assert!(bridge.contains(r#""locale":"en""#));
  assert!(bridge.contains(r#""scheme":"dark""#));
  assert!(sound_bridge(&sounds(), None).contains(r#""scheme":null"#));
  // The page itself is left as written: the band and the element stand where the page put them.
  assert!(hosted_page(page, Dark, Some(&sounds())).ends_with(page));
}

#[test]
fn hands_a_page_with_no_sound_no_player() {
  assert!(!page_prelude("<p>x</p>", Dark, Some(&sounds())).contains("sounds"));
  assert!(!page_prelude("<audio src=\"a.ogg\"></audio>", Dark, None).contains("((sounds)"));
}

#[test]
fn writes_nothing_into_a_script_that_closes_it() {
  let mut named = sounds();

  named.files.insert(
    "</script>.ogg".into(),
    SoundFile {
      play: "x".into(),
      download: "y".into(),
    },
  );

  assert!(!sound_bridge(&named, None).contains("</script>.ogg"));
}

// ---------------------------------------------------------------- the scheme the reader reads in

#[test]
fn is_the_one_the_request_names_else_the_designs_default() {
  assert_eq!(reader_scheme(Some("light")), Light);
  assert_eq!(reader_scheme(Some("system")), System);
  assert_eq!(reader_scheme(Some("sepia")), Dark);
  assert_eq!(reader_scheme(None), Dark);
}
