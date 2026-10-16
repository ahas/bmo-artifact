//! What a document's text is read as before it is drawn (`docs/archive.md`, "Kinds"): a markdown text's
//! headings and sections, and a finding's opening words and pictures' captions. Carried from
//! markdown-structure.test.ts and finding-markdown.test.ts.

use bmo_artifact::markup::finding_markdown::{finding_excerpt, picture_caption};
use bmo_artifact::markup::markdown_structure::{
  MarkdownHeading, heading_slug, markdown_headings, markdown_sections, plain_heading,
};

fn heading(depth: usize, text: &str, slug: &str, line: usize) -> MarkdownHeading {
  MarkdownHeading {
    depth,
    text: text.to_owned(),
    slug: slug.to_owned(),
    line,
  }
}

// ---------------------------------------------------------------- the headings of a text

#[test]
fn are_read_in_order_with_their_depth_words_and_source_line() {
  assert_eq!(
    markdown_headings("# Document\n\nBody\n\n## First part ##\n### Deeper `code`\n"),
    [
      heading(1, "Document", "document", 1),
      heading(2, "First part", "first-part", 5),
      heading(3, "Deeper code", "deeper-code", 6),
    ]
  );
}

#[test]
fn skip_every_line_of_a_fenced_code_block_whichever_fence_opened_it() {
  let source = [
    "# One",
    "```sh",
    "# comment",
    "~~~",
    "# still code",
    "```",
    "~~~~",
    "## not",
    "~~~~",
    "## Two",
  ]
  .join("\n");

  assert_eq!(
    markdown_headings(&source).iter().map(|one| one.text.as_str()).collect::<Vec<_>>(),
    ["One", "Two"]
  );
}

#[test]
fn give_a_repeated_heading_a_mark_of_its_own() {
  let slugs: Vec<String> = markdown_headings("## Example\n## Example\n## Example-1\n## Example")
    .into_iter()
    .map(|one| one.slug)
    .collect();

  assert_eq!(slugs, ["example", "example-1", "example-1-1", "example-2"]);
}

#[test]
fn are_not_a_hash_without_words_a_hash_without_a_space_or_an_indented_block() {
  assert_eq!(
    markdown_headings("#\n#nospace\n    # indented\n#### Four"),
    [heading(4, "Four", "four", 4)]
  );
}

#[test]
fn read_a_text_written_with_carriage_returns_on_the_same_lines() {
  assert_eq!(
    markdown_headings("# a\r\n\r\n## b").iter().map(|one| one.line).collect::<Vec<_>>(),
    [1, 3]
  );
}

#[test]
fn a_headings_words_lose_their_markup() {
  assert_eq!(
    plain_heading("**bold** [link](https://example.org) `code` ~~struck~~ _slanted_"),
    "bold link code struck slanted"
  );
  assert_eq!(plain_heading("snake_case_name"), "snake_case_name");
}

#[test]
fn a_headings_words_make_a_mark_of_letters_and_digits_in_any_script() {
  assert_eq!(heading_slug("Gate runs (runGateModel)!"), "gate-runs-rungatemodel");
  assert_eq!(heading_slug("Rest state — two kinds"), "rest-state-two-kinds");
  assert_eq!(heading_slug("Ärger über Straße"), "ärger-über-straße");
  assert_eq!(heading_slug("!!!"), "section");
}

#[test]
fn a_texts_sections_start_at_each_heading_keep_the_lead_before_the_first_and_join_back_into_the_text() {
  let source = "Lead\n\n# One\nBody\n```\n# code\n```\n## Two\nEnd";
  let sections = markdown_sections(source);

  assert_eq!(sections, ["Lead\n\n", "# One\nBody\n```\n# code\n```\n", "## Two\nEnd"]);
  assert_eq!(sections.concat(), source);
  assert_eq!(markdown_sections("# From the start\nText"), ["# From the start\nText"]);
}

// ---------------------------------------------------------------- a document's opening words

#[test]
fn are_the_key_answers_and_the_summarys_words_without_list_marks_footnote_marks_or_inline_markup() {
  let summary = [
    "VERDICT: Slow Table[2].",
    "SUMMARY:",
    "1. The cheapest place is **Slow Table**[2].",
    "2. [Eight Masters](https://example.com) is `8,000 won`[3†].",
  ]
  .join("\n");

  assert_eq!(
    finding_excerpt(&[&summary], 200),
    "Slow Table. The cheapest place is Slow Table. Eight Masters is 8,000 won."
  );
}

#[test]
fn leave_out_what_is_drawn_rather_than_said_and_what_is_folded_under_the_fold() {
  let entry = [
    "NAME: Slow Table",
    "# Heading",
    "IMAGE: https://example.com/p.jpg :: photo",
    "| menu | price |",
    "---",
    "ADDRESS: Riverside",
    "MENU:",
    "- noodles :: 6,000 won",
    "SUMMARY: A noodle house.",
    "> It has a self-serve bar.",
    "MORE:",
    "a folded story",
  ]
  .join("\n");

  assert_eq!(
    finding_excerpt(&[&entry, "next finding"], 200),
    "A noodle house. It has a self-serve bar. next finding"
  );
}

#[test]
fn are_cut_at_the_last_space_before_the_limit_and_closed_with_an_ellipsis() {
  assert_eq!(finding_excerpt(&["aa b c d ee", "ff"], 12), "aa b c d…");
  assert_eq!(finding_excerpt(&["NAME: only a name"], 12), "");
}

#[test]
fn a_pictures_shown_caption_is_its_caption_with_the_shot_and_the_source_after_it() {
  assert_eq!(
    picture_caption("garden group :: Studio A :: outdoor :: official"),
    "garden group · outdoor · official"
  );
  assert_eq!(picture_caption("soup 9,000"), "soup 9,000");
}
