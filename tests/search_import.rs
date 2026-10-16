//! A page brought in whole as a result (`docs/archive.md`, "A page imported whole").

use bmo_artifact::helpers::marks::fill_marks;
use bmo_artifact::verbs::search::document::{result_findings, result_head};
use bmo_artifact::verbs::search::import::{figure_address, imported_result, title_of};
use bmo_artifact::verbs::search::result::{is_answered, is_search_result};
use bmo_artifact::verbs::search::web::{Figure, Page, Reference};
use bmo_artifact::wording::Wording;

fn figure(n: i64) -> Figure {
  Figure {
    n,
    caption: format!("figure {n}"),
    png: None,
    svg: Some(format!("<svg id=\"f{n}\"/>")),
  }
}

fn lesson() -> Page {
  Page {
    url: "https://pages.example/lesson".to_owned(),
    title: "Course book: maths".to_owned(),
    keywords: vec!["probability".to_owned(), "softmax".to_owned()],
    body: "SECTION: contents\n\n1. data\n\nSECTION: 1 data\n\nIMAGE: https://pages.example/lesson#figure-1 :: figure 1\n\n> **maths 1**".to_owned(),
    references: vec![
      Reference {
        text: "the GRPO paper".to_owned(),
        url: "https://papers.example/grpo".to_owned(),
      },
      Reference {
        text: "the same paper".to_owned(),
        url: "https://papers.example/grpo".to_owned(),
      },
    ],
    figures: vec![figure(1), figure(2)],
  }
}

#[test]
fn is_a_result_in_the_structure_every_search_result_is_written_in_with_nothing_of_its_own() {
  let words = Wording::english().unwrap().search.import;
  let result = imported_result("https://pages.example/lesson", &lesson(), &[], &words);
  let head = result_head(&result);

  assert!(is_search_result(&result));
  assert!(is_answered(&result));
  assert_eq!(head.title.as_deref(), Some("Course book: maths"));
  assert_eq!(head.topic, Some(fill_marks(&words.topic, &[("host", "pages.example")])));
  assert_eq!(head.keywords, vec!["probability".to_owned(), "softmax".to_owned()]);
  assert!(!result.contains("SUMMARY:"));
  assert!(!result.contains("UNCONFIRMED:"));
}

#[test]
fn stands_on_the_page_it_came_from_and_on_what_the_page_itself_cites_each_once() {
  let words = Wording::english().unwrap().search.import;
  let findings = result_findings(&imported_result("https://short.example/x", &lesson(), &[], &words));
  let finding = &findings[0];

  assert!(finding.text.contains("SECTION: 1 data"));
  assert_eq!(
    finding.sources.iter().map(|source| source.url.as_str()).collect::<Vec<_>>(),
    ["https://pages.example/lesson", "https://short.example/x", "https://papers.example/grpo"]
  );
  assert_eq!(finding.sources[1].supports.as_deref(), Some(words.moved.as_str()));
}

#[test]
fn says_which_figures_it_could_not_capture() {
  let words = Wording::english().unwrap().search.import;
  let missing = figure(2);
  let result = imported_result("https://pages.example/lesson", &lesson(), &[&missing], &words);
  let said = fill_marks(&words.missing, &[("n", "2"), ("caption", "figure 2")]);

  assert!(result.contains(&format!("UNCONFIRMED:\n1. {said}")));
}

#[test]
fn names_a_figure_by_the_pages_own_address() {
  let page = Page {
    url: "https://pages.example/lesson#top".to_owned(),
    ..lesson()
  };

  assert_eq!(figure_address(&page, &figure(2)), "https://pages.example/lesson#figure-2");
  assert_eq!(
    title_of(&Page {
      title: "  ".to_owned(),
      ..lesson()
    }),
    "pages.example"
  );
}
