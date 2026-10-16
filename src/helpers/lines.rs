//! What a research run's launch is composed out of: the trees its documents are read from, a document it
//! cannot go out without, and a document's `{{name}}` marks filled in.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use axum::http::StatusCode;
use regex::{Captures, Regex};

use crate::Refusal;

static MARK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{(\w+)\}\}").expect("the mark"));

/// A launch that could not be composed, with why.
pub fn fault(detail: impl Into<String>) -> Refusal {
  Refusal::new(StatusCode::INTERNAL_SERVER_ERROR, detail)
}

/// The trees a launch is composed out of.
#[derive(Clone, Debug)]
pub struct Trees {
  /// Where the agent documents and the rulebooks are read: the tree the running code ships with.
  pub code: PathBuf,
  /// The agents root: where a composed system prompt is written, and where the configurations a launch
  /// is handed that no checkout holds are kept.
  pub agents: PathBuf,
  /// The checkout a run that is no worker's own works in.
  pub source: PathBuf,
  pub data: PathBuf,
  pub home: PathBuf,
}

impl Trees {
  /// The folder of one agent's own documents.
  pub fn agent_dir(&self, agent: &str) -> PathBuf {
    self.code.join("prompts").join("agents").join(agent)
  }

  /// A rulebook under the prompts.
  pub fn rulebook_path(&self, name: &str) -> PathBuf {
    self.code.join("prompts").join("rules").join(format!("{name}.md"))
  }
}

/// A document a launch cannot go out without, read whole and trimmed, as the composition reads every
/// agent document.
pub fn read_required(path: &Path) -> Result<String, Refusal> {
  Ok(
    std::fs::read_to_string(path)
      .map_err(|why| fault(format!("{}: {why}", path.display())))?
      .trim()
      .to_owned(),
  )
}

/// A document's `{{name}}` marks filled with what is named, any other left as it stands.
pub fn render(template: &str, values: &[(&str, &str)]) -> String {
  MARK
    .replace_all(template, |found: &Captures| {
      values
        .iter()
        .find(|(name, _)| *name == &found[1])
        .map_or_else(|| found[0].to_owned(), |(_, value)| (*value).to_owned())
    })
    .into_owned()
}
