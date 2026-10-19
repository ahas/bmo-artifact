//! The indices the page's two market sections draw, and the figures it writes of them
//! (`docs/news-brief.md`, "How one page is made"). The closes are the `stock_index` site tool's reading,
//! never the writer's: this module holds them and says them, and reads no price list itself.

use crate::helpers::js::fixed;

/// The indices of the home market section, by the key each is read under, in the order the page draws them.
pub const KOREA: [&str; 2] = ["KOSPI", "KOSDAQ"];
/// The indices of the US market section, the same way.
pub const US: [&str; 3] = ["DJI", "SPX", "NASDAQ"];

/// How many closes a chart draws: a month of trading.
pub const CHART_DAYS: usize = 20;

/// One trading day's close, with its change against the day before in points and in percent.
#[derive(Clone, Debug, PartialEq)]
pub struct Day {
  /// The day it closed, `YYYY-MM-DD` in its own market.
  pub date: String,
  pub close: f64,
  pub change: f64,
  pub ratio: f64,
}

/// One index's closes, oldest first, by the key it is read under.
#[derive(Clone, Debug, PartialEq)]
pub struct Series {
  pub key: String,
  pub days: Vec<Day>,
}

// A number with its thousands grouped by commas and two digits after the point, as the page writes figures.
fn grouped(value: f64) -> String {
  let written = fixed(value, 2);
  let (sign, digits) = written.strip_prefix('-').map_or(("", written.as_str()), |rest| ("-", rest));
  let (whole, fraction) = digits.split_once('.').unwrap_or((digits, "00"));
  let mut out = String::new();

  for (at, digit) in whole.chars().enumerate() {
    if at > 0 && (whole.len() - at) % 3 == 0 {
      out.push(',');
    }

    out.push(digit);
  }

  format!("{sign}{out}.{fraction}")
}

fn signed(value: f64) -> String {
  let text = grouped(value);

  if value > 0.0 { format!("+{text}") } else { text }
}

/// A close as the page writes it: `7,080.92`.
pub fn close_text(close: f64) -> String {
  grouped(close)
}

/// A day's change as the page writes it: `+63.01 (+0.90%)`.
pub fn change_text(day: &Day) -> String {
  format!("{} ({}%)", signed(day.change), signed(day.ratio))
}
