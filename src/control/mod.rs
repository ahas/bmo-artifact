//! The research profiles and the research-settings routes carried from the old repository
//! (`docs/research-runs.md`, "Research profiles"; `docs/api.md`, "Research settings (HTTP)").

use serde_json::Value;

pub mod research;
pub mod research_settings;

/// A JSON number read as a whole number: an integer, or a float with no fraction.
pub fn whole(value: &Value) -> Option<i64> {
  value.as_i64().or_else(|| {
    value
      .as_f64()
      .filter(|number| number.fract() == 0.0 && (i64::MIN as f64..=i64::MAX as f64).contains(number))
      .map(|number| number as i64)
  })
}
