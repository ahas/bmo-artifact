//! The morning news page drawn out of the writer's brief: the brief read and held to its limits, the figures
//! the page writes, the page whole and the message that hands it to a room (`docs/news-brief.md`, "How one
//! page is made" and "The page").

mod support;

use std::collections::HashMap;

use bmo_artifact::helpers::marks::fill_marks;
use bmo_artifact::verbs::news_brief::brief::read_brief;
use bmo_artifact::verbs::news_brief::page::{Page, index_chart, message, page_title, render};
use bmo_artifact::verbs::news_brief::series::{Day, Series, change_text, close_text};
use serde_json::json;
use support::news_brief::{brief_value, item_value, words};

fn series(key: &str, days: &[(&str, f64, f64, f64)]) -> Series {
  Series {
    key: key.to_owned(),
    days: days
      .iter()
      .map(|(date, close, change, ratio)| Day {
        date: (*date).to_owned(),
        close: *close,
        change: *change,
        ratio: *ratio,
      })
      .collect(),
  }
}

fn kospi() -> Series {
  series("KOSPI", &[("2026-09-22", 7017.91, 10.19, 0.15), ("2026-09-23", 7080.92, 63.01, 0.9)])
}

#[test]
fn is_read_out_of_a_fence_or_a_sentence_around_it() {
  let answer = format!("Here it is:\n```json\n{}\n```", brief_value());
  let brief = read_brief(&answer).expect("a brief");

  assert_eq!(brief.items.len(), 6);
  assert_eq!(brief.korea_market.movers[0].moved, "+2.1%");
  assert!(brief.real_estate.movers.is_empty());
}

#[test]
fn is_refused_saying_where_when_it_is_not_a_brief() {
  let mut short = brief_value();
  let mut unsafe_link = brief_value();

  short["items"] = json!([item_value(1), item_value(2)]);
  unsafe_link["items"][0]["url"] = json!("javascript:alert(1)");

  assert!(read_brief("no news today").expect_err("refused").contains("no JSON object"));
  assert!(read_brief(&short.to_string()).expect_err("refused").contains("items"));
  assert!(read_brief(&unsafe_link.to_string()).expect_err("refused").contains("items.0.url"));
}

#[test]
fn its_words_are_trimmed_and_held_to_their_limits() {
  let mut padded = brief_value();
  let mut long = brief_value();

  padded["summary"] = json!(["  spaced  "]);
  long["items"][1]["tag"] = json!("a tag far too long");

  assert_eq!(read_brief(&padded.to_string()).expect("a brief").summary, ["spaced"]);
  assert!(read_brief(&long.to_string()).expect_err("refused").contains("items.1.tag"));
}

#[test]
fn figures_are_grouped_and_signed_as_the_page_writes_them() {
  let day = Day {
    date: "2026-09-23".into(),
    close: 7080.92,
    change: 63.01,
    ratio: 0.9,
  };

  assert_eq!(close_text(1_234_567.891), "1,234,567.89");
  assert_eq!(close_text(42.0), "42.00");
  assert_eq!(change_text(&day), "+63.01 (+0.90%)");
  assert_eq!(
    change_text(&Day {
      change: -1234.5,
      ratio: -1.2,
      ..day
    }),
    "-1,234.50 (-1.20%)"
  );
}

#[test]
fn is_titled_for_its_day_and_its_weekday() {
  let words = words();
  let monday = fill_marks(
    &words.day,
    &[("year", "2026"), ("month", "9"), ("date", "28"), ("weekday", &words.weekdays.monday)],
  );

  assert_eq!(page_title(&words, "2026-09-28"), fill_marks(&words.title, &[("day", &monday)]));
  assert_eq!(page_title(&words, "2026-09-28"), "Today's news · Mon 9/28/2026");
}

#[test]
fn carries_every_part_in_order_its_words_escaped() {
  let words = words();
  let mut value = brief_value();

  value["summary"] = json!(["<b>bold</b> not"]);

  let brief = read_brief(&value.to_string()).expect("a brief");
  let thumbnails =
    HashMap::from([("https://news.example/1".to_owned(), "data:image/jpeg;base64,AAAA".to_owned())]);
  let html = render(
    &words,
    &Page {
      day: "2026-09-28",
      brief: &brief,
      thumbnails: &thumbnails,
      korea: &[kospi(), series("KOSDAQ", &[])],
      us: &[],
    },
  );
  let at = |text: &str| html.find(text).unwrap_or_else(|| panic!("the page holds {text}"));

  assert!(html.contains(r#"<html lang="en">"#));
  assert!(html.contains("&lt;b&gt;bold&lt;/b&gt; not"));
  assert!(html.contains(r#"<img class="thumb" src="data:image/jpeg;base64,AAAA" alt="">"#));
  assert!(at(&words.news) < at(&words.korea_market));
  assert!(at(&words.us_market) < at(&words.real_estate));
  assert!(html.contains(r#"<span class="idx-close">7,080.92</span>"#));
  assert!(html.contains(r#"<b class="up">▲</b> +63.01 (+0.90%)"#));
  assert!(html.contains(&words.no_close));
  assert!(html.contains(&format!(r#"<span class="idx-name">{}</span>"#, words.indices["KOSDAQ"])));
  assert!(html.contains("Samsung <b>+2.1%</b>"));
  assert!(html.contains(r#"<meta name="description" content="&lt;b&gt;bold&lt;/b&gt; not">"#));
}

#[test]
fn draws_a_chart_only_where_there_are_days_to_draw() {
  let words = words();
  let chart = index_chart(&words, &kospi());
  let mut single = kospi();

  single.days.truncate(1);

  assert!(chart.contains(r#"<polyline class="line" points="4.0,92.0 316.0,4.0"/>"#));
  assert!(chart.contains(r#"<polygon class="area" points="4,92 4.0,92.0 316.0,4.0 316.0,92"/>"#));
  assert!(chart.contains("<span>09/22</span><span>09/23</span>"));
  assert_eq!(index_chart(&words, &single), "");
}

#[test]
fn is_sent_as_its_link_first_then_its_title_then_its_summary_behind_bullets() {
  let words = words();
  let brief = read_brief(&brief_value().to_string()).expect("a brief");

  assert_eq!(
    message(&words, "2026-09-28", &brief, "https://ai.example/share/t"),
    format!(
      "https://ai.example/share/t\n\n{}\n\n• First summary\n• Second summary",
      page_title(&words, "2026-09-28")
    )
  );
}
