//! The kinds of document (`docs/archive.md`, "Kinds"): the catalogue, what a finding is called, and how each
//! kind outlines its body. Carried from archive-kinds.test.ts, archive-kinds/search.test.ts and
//! bot/bot/archive-kinds/kinds.test.ts, the part names read from the English wording.

mod support;

use bmo_artifact::kinds::kind::{folded_parts, piece_headings};
use bmo_artifact::kinds::{
  HeadProperty, KINDS, Outline, OutlineEntry, OutlineFinding, OutlineLevel, finding_contents,
  finding_title, kind,
};
use bmo_artifact::wording::Wording;
use bmo_artifact::wording::search::Search;
use support::{ROOT, rust_files};

fn words() -> Search {
  Wording::english().expect("the English wording").search
}

fn piece(place: i64, lines: &[&str]) -> OutlineFinding {
  OutlineFinding {
    place,
    text: lines.join("\n"),
  }
}

fn entry(slug: &str, text: &str, depth: usize) -> OutlineEntry {
  OutlineEntry {
    slug: slug.to_owned(),
    text: text.to_owned(),
    depth,
    level: None,
  }
}

fn outline_of(id: &str, findings: &[OutlineFinding]) -> Outline {
  kind(Some(id)).outline(findings, &words())
}

fn shape(outline: &Outline) -> Vec<(String, usize, Option<OutlineLevel>)> {
  outline.entries.iter().map(|one| (one.text.clone(), one.depth, one.level)).collect()
}

fn shaped(rows: &[(&str, usize, Option<OutlineLevel>)]) -> Vec<(String, usize, Option<OutlineLevel>)> {
  rows.iter().map(|(text, depth, level)| ((*text).to_owned(), *depth, *level)).collect()
}

// ---------------------------------------------------------------- the catalogue

#[test]
fn names_every_kind_module_in_its_folder_once() {
  let folder = std::path::Path::new(ROOT).join("src/kinds");
  let mut modules: Vec<String> = rust_files(&folder)
    .iter()
    .filter_map(|path| path.file_stem().map(|stem| stem.to_string_lossy().into_owned()))
    .filter(|name| name != "mod" && name != "kind")
    .collect();
  let mut catalogued: Vec<String> = KINDS.iter().map(|kind| kind.id().to_owned()).collect();

  assert_eq!(catalogued, ["search", "import", "learning", "record", "page"]);

  modules.sort();
  catalogued.sort();
  assert_eq!(modules, catalogued);
}

#[test]
fn every_kind_has_a_label_key_and_a_head_of_its_own() {
  for kind in KINDS {
    assert_eq!(kind.label_key(), format!("archive.kind.{}", kind.id()));
    assert!(!kind.head().is_empty(), "{}", kind.id());
  }
}

#[test]
fn a_word_it_does_not_know_or_none_reads_as_a_search() {
  assert_eq!(kind(Some("learning")).id(), "learning");
  assert_eq!(kind(Some("diary")).id(), "search");
  assert_eq!(kind(None).id(), "search");
}

// ---------------------------------------------------------------- what a finding is called

#[test]
fn a_finding_is_called_by_its_title_or_name_line_else_its_first_words() {
  let words = words();
  let title = |place: i64, text: &str| {
    finding_title(
      &OutlineFinding {
        place,
        text: text.to_owned(),
      },
      &words,
    )
  };

  assert_eq!(title(1, "TITLE: mini PC\nbody"), "mini PC");
  assert_eq!(title(2, "\nNAME: GEM12\n..."), "GEM12");
  assert_eq!(
    title(3, "SECTION: The leading techniques of each kind\n..."),
    "The leading techniques of each kind"
  );
  assert_eq!(title(3, "SUMMARY: summary\njust the first sentence"), "just the first sentence");
  assert_eq!(title(4, ""), "Part 4");
  assert_eq!(title(5, &"a".repeat(70)), format!("{}…", "a".repeat(60)));
  // A body opening with its summary label is called by that part, and marks never show.
  assert_eq!(title(6, "SUMMARY:\n1. value[1]"), "Summary");
  assert_eq!(title(6, "VERDICT:\n1. Buy it."), "Short answer");
  assert_eq!(title(7, "It costs 30,000 won[2†]"), "It costs 30,000 won");
  // Written emphasis is the text's, not the title's: a line standing as a title shows its words alone.
  assert_eq!(
    title(8, "See the lecture's **“one picture”**, *short formulas*, and `code`"),
    "See the lecture's “one picture”, short formulas, and code"
  );
}

#[test]
fn a_finding_folds_what_follows_its_fold_line() {
  assert_eq!(folded_parts("short\nMORE:\nlong"), ("short".to_owned(), Some("long".to_owned())));
  assert_eq!(folded_parts("short only"), ("short only".to_owned(), None));
  assert_eq!(folded_parts("short\nMORE:\n  "), ("short".to_owned(), None));
}

// ---------------------------------------------------------------- the contents of one finding

#[test]
fn opens_with_the_findings_own_heading_and_puts_its_parts_under_it() {
  let (entries, slugs) = finding_contents(
    &piece(
      1,
      &[
        "SUMMARY:",
        "1. Buy the Rinkle.",
        "BODY:",
        "| candidate | value |",
        "| --- | --- |",
        "| Rinkle | 129,000 won |",
      ],
    ),
    &words(),
  );

  assert_eq!(entries, [entry("1-summary", "Summary", 1), entry("1-body", "Body", 2)]);
  // Every mark carries the finding it is in, since every finding writes a summary of its own.
  assert_eq!(slugs, ["1-summary", "1-body"]);
}

#[test]
fn puts_each_section_under_the_name_the_search_gave_it() {
  let (entries, _) = finding_contents(
    &piece(
      2,
      &[
        "SUMMARY:",
        "1. The background first.",
        "SECTION: The dispute so far",
        "It began in spring.",
        "SECTION: The maker's position",
        "It explained.",
      ],
    ),
    &words(),
  );

  assert_eq!(
    entries.iter().map(|one| one.text.as_str()).collect::<Vec<_>>(),
    ["Summary", "The dispute so far", "The maker's position"]
  );
}

#[test]
fn names_an_entry_of_a_set_by_its_name_line_and_its_parts_under_that() {
  let (entries, _) = finding_contents(
    &piece(
      3,
      &[
        "NAME: Rinkle Prime S",
        "STOCK: 3 left :: 2026-09-21 :: https://shop.example/1",
        "UNCONFIRMED:",
        "1. none",
      ],
    ),
    &words(),
  );

  assert_eq!(
    entries,
    [
      entry("3-rinkle-prime-s", "Rinkle Prime S", 1),
      entry("3-not-confirmed", "Not confirmed", 2)
    ]
  );
}

// What is folded under the fold has no entry: pressing one would arrive at a collapsed section.
#[test]
fn leaves_what_the_fold_hides_out_of_the_contents() {
  let (entries, _) =
    finding_contents(&piece(2, &["NAME: Rinkle", "short.", "MORE:", "BODY:", "long."]), &words());

  assert_eq!(entries.iter().map(|one| one.text.as_str()).collect::<Vec<_>>(), ["Rinkle"]);
}

// A finding whose text opens with something other than a heading keeps a line of its own, so what stands
// above the first heading is still reachable.
#[test]
fn stands_a_line_for_a_finding_that_opens_with_no_heading() {
  let (entries, _) = finding_contents(&piece(4, &["Words standing first.", "BODY:", "body."]), &words());

  assert_eq!(entries, [entry("4", "Words standing first.", 1), entry("4-body", "Body", 2)]);
}

#[test]
fn the_search_kind_opens_with_a_summary_and_outlines_every_finding_in_turn_each_under_its_own_marks() {
  let outline = outline_of(
    "search",
    &[
      piece(1, &["SUMMARY:", "1. Buy.", "BODY:", "body."]),
      piece(2, &["NAME: Rinkle", "UNCONFIRMED:", "1. none"]),
    ],
  );

  assert!(kind(Some("search")).summary());
  assert_eq!(
    outline.entries.iter().map(|one| (one.text.as_str(), one.depth)).collect::<Vec<_>>(),
    [("Summary", 1), ("Body", 2), ("Rinkle", 1), ("Not confirmed", 2)]
  );
  assert_eq!(outline.marks[&1], ["1-summary", "1-body"]);
  assert_eq!(outline.marks[&2], ["2-rinkle", "2-not-confirmed"]);
}

// ---------------------------------------------------------------- how each other kind outlines its body

// A body with every heading depth from one to four, as one piece of a document.
fn deep() -> Vec<OutlineFinding> {
  vec![piece(
    1,
    &[
      "# Part 1 Basics",
      "## Chapter 1 Numbers",
      "### Section 1 Naturals",
      "#### a. Counting",
      "## Chapter 2 Terms",
      "### Section 1 Monomials",
    ],
  )]
}

// A book with no parts: two ranks of heading, chapters and the sections under them.
fn chapters() -> Vec<OutlineFinding> {
  vec![piece(1, &["## Chapter 1 Numbers", "### Section 1 Naturals", "## Chapter 2 Terms"])]
}

#[test]
fn a_page_imported_whole_is_outlined_by_its_own_headings_every_level_kept_and_none_named() {
  assert!(!kind(Some("import")).summary());
  assert_eq!(
    shape(&outline_of("import", &deep())),
    shaped(&[
      ("Part 1 Basics", 1, None),
      ("Chapter 1 Numbers", 2, None),
      ("Section 1 Naturals", 3, None),
      ("a. Counting", 4, None),
      ("Chapter 2 Terms", 2, None),
      ("Section 1 Monomials", 3, None),
    ])
  );
}

// The importer writes a page's top headings as structure lines and the ones under them as markdown; both
// stand in the contents, the sections above what is under them, and the marks are the ones the console
// stamps on the text.
#[test]
fn an_import_keeps_the_sections_it_wrote_as_structure_lines_above_the_markdown_headings_under_them() {
  let page = [piece(
    1,
    &[
      "SECTION: Overview",
      "body",
      "## Details",
      "### More details",
      "SECTION: Second",
      "## Second details",
    ],
  )];
  let outline = outline_of("import", &page);

  assert_eq!(
    shape(&outline),
    shaped(&[
      ("Overview", 1, None),
      ("Details", 2, None),
      ("More details", 3, None),
      ("Second", 1, None),
      ("Second details", 2, None)
    ])
  );
  assert_eq!(outline.marks[&1], piece_headings(&page[0], &words()).2);
}

#[test]
fn a_page_that_starts_at_a_lower_heading_is_not_indented_for_nothing() {
  assert_eq!(
    shape(&outline_of("import", &chapters())).iter().map(|row| row.1).collect::<Vec<_>>(),
    [1, 2, 1]
  );
}

#[test]
fn study_material_is_a_book_to_three_levels_and_nothing_deeper() {
  let (part, chapter, section) =
    (Some(OutlineLevel::Part), Some(OutlineLevel::Chapter), Some(OutlineLevel::Section));

  assert!(!kind(Some("learning")).summary());
  assert_eq!(
    shape(&outline_of("learning", &deep())),
    shaped(&[
      ("Part 1 Basics", 1, part),
      ("Chapter 1 Numbers", 2, chapter),
      ("Section 1 Naturals", 3, section),
      ("Chapter 2 Terms", 2, chapter),
      ("Section 1 Monomials", 3, section),
    ])
  );
  assert_eq!(
    shape(&outline_of("learning", &chapters())),
    shaped(&[
      ("Chapter 1 Numbers", 1, chapter),
      ("Section 1 Naturals", 2, section),
      ("Chapter 2 Terms", 1, chapter)
    ])
  );
}

// The console stamps every heading it draws, so the marks cover the headings the contents leave out too.
#[test]
fn study_material_marks_every_heading_of_the_text_including_the_ones_its_contents_leave_out() {
  let marks = &outline_of("learning", &deep()).marks[&1];

  assert_eq!(
    marks,
    &[
      "1-part-1-basics",
      "1-chapter-1-numbers",
      "1-section-1-naturals",
      "1-a-counting",
      "1-chapter-2-terms",
      "1-section-1-monomials",
    ]
  );
  assert_eq!(marks, &piece_headings(&deep()[0], &words()).2);
}

#[test]
fn a_record_is_outlined_by_its_headings_to_two_levels_and_one_with_none_has_nothing_to_mark() {
  let (chapter, section) = (Some(OutlineLevel::Chapter), Some(OutlineLevel::Section));

  assert!(!kind(Some("record")).summary());
  assert_eq!(
    shape(&outline_of("record", &deep())),
    shaped(&[
      ("Part 1 Basics", 1, chapter),
      ("Chapter 1 Numbers", 2, section),
      ("Chapter 2 Terms", 2, section)
    ])
  );

  let plain = outline_of("record", &[piece(1, &["Just words written down."])]);

  assert!(plain.entries.is_empty());
  assert_eq!(plain.marks[&1], Vec::<String>::new());
}

#[test]
fn a_page_is_the_page_it_keeps_with_no_summary_and_no_contents_and_shows_its_description() {
  let page = kind(Some("page"));

  assert!(page.hosted());
  assert!(!page.summary());
  assert_eq!(page.head(), [HeadProperty::Description]);
  assert_eq!(page.outline(&deep(), &words()), Outline::default());
  assert_eq!(
    KINDS.iter().filter(|kind| kind.hosted()).map(|kind| kind.id()).collect::<Vec<_>>(),
    ["page"]
  );
}
