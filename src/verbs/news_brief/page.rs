//! The one HTML page drawn out of the writer's brief (`docs/news-brief.md`, "The page"), and the message that
//! hands it to a room. The figures and the charts of the two market sections are never the writer's: they are
//! the indices' own readings, drawn here. The pictures come in as data addresses already made, so the page
//! carries everything it draws and needs nothing to still be online to open. Every word a person reads comes
//! from the wording file's `news_page` lines.

use std::collections::HashMap;

use chrono::{Datelike, NaiveDate};

use super::brief::{Brief, Item, Section};
use super::series::{Series, change_text, close_text};
use crate::helpers::js::fixed;
use crate::helpers::marks::fill_marks;
use crate::wording::news_page::NewsPage;

/// The day a page is for, as its title and its message say it: the year, the month, the date and the weekday
/// of a calendar date written `YYYY-MM-DD`; the text as it stands where it is none.
pub fn spoken_day(words: &NewsPage, day: &str) -> String {
  let Ok(date) = NaiveDate::parse_from_str(day, "%Y-%m-%d") else {
    return day.to_owned();
  };

  fill_marks(
    &words.day,
    &[
      ("year", &date.year().to_string()),
      ("month", &date.month().to_string()),
      ("date", &date.day().to_string()),
      ("weekday", words.weekdays.from_sunday(date.weekday().num_days_from_sunday())),
    ],
  )
}

/// The page's title.
pub fn page_title(words: &NewsPage, day: &str) -> String {
  fill_marks(&words.title, &[("day", &spoken_day(words, day))])
}

/// What the room is sent: the link on the first line, the title under it, then the few lines at the top of
/// the page, each behind a bullet. The bullet and the blank lines are plain characters, since a room with no
/// formatting sends the text as it stands and a markdown mark would reach it as a stray character.
pub fn message(words: &NewsPage, day: &str, brief: &Brief, link: &str) -> String {
  let mut lines = vec![link.to_owned(), String::new(), page_title(words, day), String::new()];

  lines.extend(brief.summary.iter().map(|line| format!("• {line}")));
  lines.join("\n")
}

/// The size a picture is carried at: the card's frame at twice its size, for a sharp screen.
pub const THUMBNAIL: (u32, u32) = (192, 144);

/// The JPEG quality a card's picture is written at: small enough that ten of them keep the page light.
pub const THUMBNAIL_QUALITY: u8 = 62;

/// Text made safe to stand in the page's markup and its attributes.
pub fn escape(text: &str) -> String {
  text
    .replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('"', "&quot;")
    .replace('\'', "&#39;")
}

fn paragraphs(lines: &[String]) -> String {
  lines.iter().map(|line| format!("<p>{}</p>", escape(line))).collect()
}

// The outlet an address belongs to, as a link names it.
fn host_of(address: &str) -> String {
  url::Url::parse(address)
    .ok()
    .and_then(|url| url.host_str().map(|host| host.strip_prefix("www.").unwrap_or(host).to_owned()))
    .unwrap_or_else(|| address.to_owned())
}

fn source_links(sources: &[String]) -> String {
  if sources.is_empty() {
    return String::new();
  }

  let links: String = sources
    .iter()
    .map(|source| {
      format!(
        r#"<a href="{}" target="_blank" rel="noopener">{}</a>"#,
        escape(source),
        escape(&host_of(source))
      )
    })
    .collect();

  format!(r#"<div class="src">{links}</div>"#)
}

// The chart's box, in its own units.
const CHART: (f64, f64, f64) = (320.0, 96.0, 4.0);

// The name an index is read out as.
fn index_name<'a>(words: &'a NewsPage, series: &'a Series) -> &'a str {
  words.indices.get(&series.key).map_or(series.key.as_str(), String::as_str)
}

// A day as an axis or a card names it: `09/22`.
fn month_day(date: &str) -> String {
  date.get(5..).unwrap_or(date).replacen('-', "/", 1)
}

// A coordinate as the chart writes it, one digit after the point.
fn place(value: f64) -> String {
  fixed(value, 1)
}

/// One index's closes as a line over its days. The figures stand beside it as text, so the picture carries the
/// shape and nothing a reader has to read off it; fewer than two closes draw nothing.
pub fn index_chart(words: &NewsPage, series: &Series) -> String {
  let closes: Vec<f64> = series.days.iter().map(|day| day.close).collect();
  let (Some(first), Some(last)) = (series.days.first(), series.days.last()) else {
    return String::new();
  };

  if closes.len() < 2 {
    return String::new();
  }

  let low = closes.iter().copied().fold(f64::INFINITY, f64::min);
  let high = closes.iter().copied().fold(f64::NEG_INFINITY, f64::max);
  let span = if high - low == 0.0 { 1.0 } else { high - low };
  let (width, height, pad) = CHART;
  let steps = (closes.len() - 1) as f64;
  let x = |at: usize| pad + (at as f64 * (width - pad * 2.0)) / steps;
  let y = |close: f64| pad + ((high - close) * (height - pad * 2.0)) / span;
  let points: Vec<String> = closes
    .iter()
    .enumerate()
    .map(|(at, close)| format!("{},{}", place(x(at)), place(y(*close))))
    .collect();
  let label = fill_marks(
    &words.chart_label,
    &[
      ("name", index_name(words, series)),
      ("first", &first.date),
      ("last", &last.date),
      ("count", &closes.len().to_string()),
      ("low", &close_text(low)),
      ("high", &close_text(high)),
    ],
  );
  let label = escape(&label);
  let floor = height - pad;

  [
    format!(
      r#"<svg class="chart" viewBox="0 0 {width} {height}" role="img" aria-label="{label}" preserveAspectRatio="none">"#
    ),
    format!("<title>{label}</title>"),
    format!(
      r#"<polygon class="area" points="{pad},{floor} {} {},{floor}"/>"#,
      points.join(" "),
      place(x(closes.len() - 1))
    ),
    format!(r#"<polyline class="line" points="{}"/>"#, points.join(" ")),
    "</svg>".to_owned(),
    format!(
      r#"<div class="axis"><span>{}</span><span>{}</span></div>"#,
      escape(&month_day(&first.date)),
      escape(&month_day(&last.date))
    ),
  ]
  .concat()
}

fn index_card(words: &NewsPage, series: &Series) -> String {
  let name = escape(index_name(words, series));
  let Some(today) = series.days.last() else {
    return format!(
      r#"<div class="idx"><div class="idx-head"><span class="idx-name">{name}</span><span class="idx-miss">{}</span></div></div>"#,
      escape(&words.no_close)
    );
  };
  let (direction, mark) = if today.change > 0.0 {
    ("up", "▲")
  } else if today.change < 0.0 {
    ("down", "▼")
  } else {
    ("flat", "–")
  };
  let close_of = fill_marks(&words.close_of, &[("date", &month_day(&today.date))]);

  [
    r#"<div class="idx">"#.to_owned(),
    format!(
      r#"<div class="idx-head"><span class="idx-name">{name}</span><span class="idx-date">{}</span></div>"#,
      escape(&close_of)
    ),
    format!(
      r#"<div class="idx-fig"><span class="idx-close">{}</span><span class="idx-move"><b class="{direction}">{mark}</b> {}</span></div>"#,
      close_text(today.close),
      escape(&change_text(today))
    ),
    index_chart(words, series),
    "</div>".to_owned(),
  ]
  .concat()
}

fn market_section(words: &NewsPage, title: &str, section: &Section, series: &[Series]) -> String {
  let movers = if section.movers.is_empty() {
    String::new()
  } else {
    let movers: String = section
      .movers
      .iter()
      .map(|mover| {
        format!(r#"<span class="mover">{} <b>{}</b></span>"#, escape(&mover.name), escape(&mover.moved))
      })
      .collect();

    format!(r#"<div class="movers">{movers}</div>"#)
  };

  [
    format!(r#"<section class="block"><h2>{}</h2>"#, escape(title)),
    format!(
      r#"<div class="idxs">{}</div>"#,
      series.iter().map(|one| index_card(words, one)).collect::<String>()
    ),
    format!(r#"<div class="body">{}</div>"#, paragraphs(&section.lines)),
    movers,
    source_links(&section.sources),
    "</section>".to_owned(),
  ]
  .concat()
}

fn text_section(title: &str, section: &Section) -> String {
  format!(
    r#"<section class="block"><h2>{}</h2><div class="body">{}</div>{}</section>"#,
    escape(title),
    paragraphs(&section.lines),
    source_links(&section.sources)
  )
}

fn news_card(words: &NewsPage, item: &Item, at: usize, thumbnails: &HashMap<String, String>) -> String {
  let picture = thumbnails.get(&item.url).map_or_else(
    || r#"<div class="thumb none" aria-hidden="true"></div>"#.to_owned(),
    |picture| format!(r#"<img class="thumb" src="{picture}" alt="">"#),
  );
  let original = fill_marks(&words.original, &[("host", &host_of(&item.url))]);

  [
    r#"<article class="card">"#.to_owned(),
    picture,
    r#"<div class="card-top">"#.to_owned(),
    format!(
      r#"<div class="card-head"><span class="tag">{}</span><span class="num">{}</span></div>"#,
      escape(&item.tag),
      at + 1
    ),
    format!("<h3>{}</h3>", escape(&item.title)),
    "</div>".to_owned(),
    r#"<div class="card-body">"#.to_owned(),
    paragraphs(&item.lines),
    format!(
      r#"<a class="more" href="{}" target="_blank" rel="noopener">{}</a>"#,
      escape(&item.url),
      escape(&original)
    ),
    "</div></article>".to_owned(),
  ]
  .concat()
}

// Light and fixed: the page is opened in whatever a room's app opens a link in, and every surface carries its
// own ink so nothing of that app's theme reaches inside. The word before a section's sources is filled in.
const STYLE: &str = r#"
:root{--ink:#1d1f23;--ink2:#3d434d;--muted:#5f6670;--line:#e3e6ea;--bg:#f4f5f7;--card:#fff;--brand:#1f7a6d;--brand-soft:#e3f2ef;--up:#d0312d;--down:#1f5fcc}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--ink);font-family:"Pretendard Variable",Pretendard,-apple-system,"Apple SD Gothic Neo","Noto Sans KR",sans-serif;font-size:16px;line-height:1.5;-webkit-text-size-adjust:100%}
main{max-width:800px;margin:0 auto;padding:16px 16px 32px}
header{padding:4px 0 12px}
header h1{font-size:24px;margin:0;line-height:1.3}
.sum{background:var(--brand);color:#fff;border-radius:16px;padding:16px;margin-bottom:24px}
.sum p{margin:0 0 4px;font-size:16px}.sum p:last-child{margin:0}
h2{font-size:20px;margin:0 0 12px;line-height:1.3}
.news{margin-bottom:24px}
.card{display:grid;grid-template-columns:96px minmax(0,1fr);gap:8px 12px;align-items:center;background:var(--card);border-radius:16px;padding:12px;margin-bottom:8px}
.thumb{width:96px;height:72px;object-fit:cover;border-radius:12px;background:var(--line)}
.card-top{min-width:0}
.card-body{grid-column:1/-1;min-width:0}
@media (min-width:560px){.card{grid-template-columns:112px minmax(0,1fr);align-items:start}.thumb{grid-row:1/3;width:112px;height:84px}.card-body{grid-column:2}}
.card-head{display:flex;align-items:center;gap:8px;margin-bottom:4px}
.tag{font-size:14px;font-weight:600;color:var(--brand);background:var(--brand-soft);border-radius:8px;padding:0 8px;line-height:24px}
.num{font-size:14px;color:var(--muted);font-variant-numeric:tabular-nums}
h3{font-size:16px;margin:0 0 4px;line-height:1.4}
.card p,.body p{margin:0;font-size:14px;color:var(--ink2);line-height:1.55}
a{color:var(--brand);text-decoration:none;border-radius:4px}
a:hover{color:#155a50;background:var(--brand-soft)}
a:active{color:#0f4139;background:#cde8e2}
.more{display:inline-block;margin-top:4px;font-size:14px}
.block{background:var(--card);border-radius:16px;padding:16px;margin-bottom:16px}
.idxs{display:grid;grid-template-columns:repeat(auto-fit,minmax(220px,1fr));gap:16px;margin-bottom:12px}
.idx-head{display:flex;justify-content:space-between;align-items:center;font-size:14px}
.idx-name{font-weight:600}
.idx-date,.idx-miss{color:var(--muted)}
.idx-fig{display:flex;align-items:baseline;gap:8px;margin:4px 0;font-variant-numeric:tabular-nums}
.idx-close{font-size:20px;font-weight:700}
.idx-move{font-size:14px;color:var(--ink2)}
.up{color:var(--up)}.down{color:var(--down)}.flat{color:var(--muted)}
.chart{display:block;width:100%;height:72px}
.chart .line{fill:none;stroke:var(--brand);stroke-width:2;vector-effect:non-scaling-stroke;stroke-linejoin:round}
.chart .area{fill:var(--brand);opacity:.12}
.axis{display:flex;justify-content:space-between;font-size:14px;color:var(--muted);font-variant-numeric:tabular-nums}
.body p+p{margin-top:4px}
.movers{display:flex;flex-wrap:wrap;gap:8px;margin-top:12px}
.mover{font-size:14px;background:var(--bg);border-radius:8px;padding:4px 8px;font-variant-numeric:tabular-nums}
.src{display:flex;flex-wrap:wrap;gap:8px;margin-top:12px;font-size:14px}
.src::before{content:"{sources}";color:var(--muted)}
footer{color:var(--muted);font-size:14px;text-align:center;margin-top:16px}
"#;

// The font every surface is set in, the one thing the page asks the web for.
const FONT: &str = "https://cdn.jsdelivr.net/gh/orioncactus/pretendard@v1.3.9/dist/web/variable/pretendardvariable-dynamic-subset.min.css";

/// What one page is drawn out of: the day it is for, the brief, each article's picture as a data address by
/// the article's own address, and the two markets' indices.
pub struct Page<'a> {
  pub day: &'a str,
  pub brief: &'a Brief,
  pub thumbnails: &'a HashMap<String, String>,
  pub korea: &'a [Series],
  pub us: &'a [Series],
}

/// The page, whole, in the words given.
pub fn render(words: &NewsPage, page: &Page<'_>) -> String {
  let title = escape(&page_title(words, page.day));
  let description = escape(&page.brief.summary.join(" "));
  let cards: String = page
    .brief
    .items
    .iter()
    .enumerate()
    .map(|(at, item)| news_card(words, item, at, page.thumbnails))
    .collect();
  let style = STYLE.replace("{sources}", &escape(&words.sources));

  [
    format!(
      r#"<!doctype html><html lang="{}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">"#,
      escape(&words.language)
    ),
    format!(r#"<title>{title}</title><meta name="description" content="{description}">"#),
    format!(r#"<link rel="stylesheet" href="{FONT}">"#),
    format!("<style>{style}</style></head><body><main>"),
    format!("<header><h1>{title}</h1></header>"),
    format!(r#"<section class="sum">{}</section>"#, paragraphs(&page.brief.summary)),
    format!(r#"<section class="news"><h2>{}</h2>{cards}</section>"#, escape(&words.news)),
    market_section(words, &words.korea_market, &page.brief.korea_market, page.korea),
    market_section(words, &words.us_market, &page.brief.us_market, page.us),
    text_section(&words.real_estate, &page.brief.real_estate),
    format!("<footer>{}</footer>", escape(&words.footer)),
    "</main></body></html>".to_owned(),
  ]
  .join("\n")
}
