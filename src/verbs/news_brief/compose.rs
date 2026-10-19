//! The page put together out of the writer's brief (`docs/news-brief.md`, "How one page is made"): each
//! article's picture fetched through bmo-explorer and cut to its card, the indices' closes read from the
//! `stock_index` site tool, and the page drawn. A picture or an index that could not be had leaves its place
//! plain rather than stopping the page, and is said among the composing's notes.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use scraper::{Html, Selector};

use super::brief::{Brief, Item};
use super::page::{Page, THUMBNAIL, THUMBNAIL_QUALITY, render};
use super::series::{CHART_DAYS, Day, KOREA, Series, US};
use crate::clients::tool_host::{PageReader, ToolHost};
use crate::documents::ground::thumbnail;
use crate::wording::news_page::NewsPage;

// How many pictures are fetched at once: enough that ten articles do not take ten turns, few enough that one
// outlet is not asked for four pages in the same second.
const PICTURE_LANES: usize = 3;

// How long each read is waited for (`docs/api.md`, "Read a page and fetch a picture").
const PAGE_HEAD_WAIT: Duration = Duration::from_secs(10);
const PICTURE_WAIT: Duration = Duration::from_secs(60);
const CLOSES_WAIT: Duration = Duration::from_secs(60);

/// The pacing name one morning's reading goes under: the day and the room it is for.
pub fn pacing(day: &str, chat_id: i64) -> String {
  format!("news-brief:{day}:{chat_id}")
}

/// The page drawn, and what the composing said of what it could not have.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Composed {
  pub html: String,
  pub said: Vec<String>,
}

/// The picture a page names of itself for sharing, read from its tags, resolved against the page and kept
/// only where it is a web address.
pub fn page_picture(html: &str, base: &url::Url) -> Option<String> {
  let head = Html::parse_document(html);
  let meta = Selector::parse("meta").expect("the selector");
  let mut named: HashMap<String, String> = HashMap::new();

  for tag in head.select(&meta) {
    let key = tag.attr("property").or_else(|| tag.attr("name")).unwrap_or_default().to_lowercase();

    if let Some(content) = tag.attr("content")
      && !key.is_empty()
    {
      named.entry(key).or_insert_with(|| content.to_owned());
    }
  }

  let picture = ["og:image:secure_url", "og:image", "twitter:image"]
    .iter()
    .find_map(|key| named.get(*key))?
    .trim();

  base
    .join(picture)
    .ok()
    .filter(|url| matches!(url.scheme(), "http" | "https"))
    .map(String::from)
}

async fn waited<T>(wait: Duration, read: impl Future<Output = Result<T, String>>) -> Result<T, String> {
  tokio::time::timeout(wait, read)
    .await
    .unwrap_or_else(|_| Err(format!("no answer within {} seconds", wait.as_secs())))
}

// The picture an article's page names of itself; none where the page is no markup or names none.
async fn own_picture(host: &ToolHost, address: &str) -> Result<Option<String>, String> {
  let page = waited(PAGE_HEAD_WAIT, host.fetch_page(address)).await?;
  let Some(html) = page.html else {
    return Ok(None);
  };
  let base =
    url::Url::parse(&page.url).map_err(|why| format!("the page's address does not read: {why}"))?;

  Ok(page_picture(&html, &base))
}

// One article's picture as a data address: the one the writer named, or the article's own.
async fn picture_of(host: &ToolHost, item: &Item) -> Result<Option<String>, String> {
  let address = match &item.image {
    Some(named) => named.clone(),
    None => match own_picture(host, &item.url).await? {
      Some(own) => own,
      None => return Ok(None),
    },
  };
  let raster = waited(PICTURE_WAIT, host.picture(&address)).await?;
  let cut = thumbnail(&raster.bytes, THUMBNAIL.0, THUMBNAIL.1, THUMBNAIL_QUALITY)?;

  Ok(Some(format!("data:image/jpeg;base64,{}", STANDARD.encode(cut))))
}

/// Each article's picture as a data address, by the article's address, and what was said of those not had.
/// An article whose picture could not be had is simply not among them; its card stands without one.
pub async fn article_pictures(host: &ToolHost, items: &[Item]) -> (HashMap<String, String>, Vec<String>) {
  let queue = Mutex::new(items.iter().enumerate().collect::<VecDeque<_>>());
  let pictures = Mutex::new(HashMap::new());
  let said = Mutex::new(Vec::new());
  let next = || queue.lock().unwrap_or_else(std::sync::PoisonError::into_inner).pop_front();
  let say = |at: usize, line: String| {
    said.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push((at, line))
  };
  let lane = async || {
    while let Some((at, item)) = next() {
      match picture_of(host, item).await {
        Ok(Some(picture)) => {
          pictures
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(item.url.clone(), picture);
        }
        Ok(None) => say(at, format!("no picture for {}", item.url)),
        Err(why) => say(at, format!("the picture of {} could not be had: {why}", item.url)),
      }
    }
  };

  futures_util::future::join_all((0..PICTURE_LANES).map(|_| lane())).await;

  // The lanes finish in the order the outlets answer; the notes are kept in the articles' own.
  let mut said = said.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner);

  said.sort_by_key(|(at, _)| *at);

  (
    pictures.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner),
    said.into_iter().map(|(_, line)| line).collect(),
  )
}

/// Every index the page draws, in each section's order, with the closes the site tool read; an index it
/// could not read stands with no days, and is said.
pub async fn index_series(words: &NewsPage, host: &ToolHost) -> (Vec<Series>, Vec<Series>, Vec<String>) {
  let keys: Vec<&str> = KOREA.iter().chain(&US).copied().collect();
  let mut said = Vec::new();
  let read = match waited(CLOSES_WAIT, host.index_closes(&keys, CHART_DAYS)).await {
    Ok(read) => read,
    Err(why) => {
      said.push(format!("the index closes could not be read: {why}"));
      vec![]
    }
  };
  let mut series_of = |key: &str| {
    let days: Vec<Day> = read
      .iter()
      .find(|series| series.key == key)
      .map(|series| {
        let mut days: Vec<Day> = series
          .days
          .iter()
          .map(|day| Day {
            date: day.date.clone(),
            close: day.close,
            change: day.change,
            ratio: day.ratio,
          })
          .collect();

        days.drain(..days.len().saturating_sub(CHART_DAYS));
        days
      })
      .unwrap_or_default();

    if days.is_empty() {
      let name = words.indices.get(key).map_or(key, String::as_str);

      said.push(format!("{name} could not be read"));
    }

    Series {
      key: key.to_owned(),
      days,
    }
  };
  let korea: Vec<Series> = KOREA.iter().map(|key| series_of(key)).collect();
  let us: Vec<Series> = US.iter().map(|key| series_of(key)).collect();

  (korea, us, said)
}

/// The page for one day, whole, read through the tool host under the run's pacing.
pub async fn compose(words: &NewsPage, day: &str, brief: &Brief, host: &ToolHost) -> Composed {
  let ((thumbnails, mut said), (korea, us, unread)) =
    tokio::join!(article_pictures(host, &brief.items), index_series(words, host));
  let read = korea.iter().chain(&us).filter(|series| !series.days.is_empty()).count();

  said.extend(unread);
  said.push(format!(
    "{} of {} pictures; {read} of {} indices",
    thumbnails.len(),
    brief.items.len(),
    KOREA.len() + US.len()
  ));

  Composed {
    html: render(
      words,
      &Page {
        day,
        brief,
        thumbnails: &thumbnails,
        korea: &korea,
        us: &us,
      },
    ),
    said,
  }
}
