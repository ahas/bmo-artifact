//! A verb's answer and its errors, the shell it runs on, and how the CLI ends with them.

use crate::settings::Settings;

/// What a verb answers: the lines it prints, and the code it ends with where that is not success.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Answer {
  pub lines: Vec<String>,
  pub code: u8,
}

impl From<Vec<String>> for Answer {
  fn from(lines: Vec<String>) -> Self {
    Self { lines, code: 0 }
  }
}

/// Why a verb did not do what it was asked, in the line it prints before it ends with a failure.
#[derive(Debug, thiserror::Error)]
pub enum VerbError {
  /// The caller asked for something the verb refuses, said in the verb's own sentence.
  #[error("{0}")]
  Refused(String),
  /// The verb ended with a code of its own, said in its own sentence.
  #[error("{said}")]
  Ended { code: u8, said: String },
  /// The verb did part of what it was asked: the lines it prints of that, and the refusal of the rest.
  #[error("{refusal}")]
  Partly { lines: Vec<String>, refusal: String },
}

/// What a verb run from the CLI stands on.
#[derive(Clone, Debug)]
pub struct Shell {
  pub settings: Settings,
}

impl Shell {
  pub fn new(settings: Settings) -> Self {
    Self { settings }
  }
}

/// How the CLI ends a verb: what it prints on standard output, what on standard error, and its code.
#[derive(Debug, PartialEq, Eq)]
pub struct Finished {
  pub out: Vec<String>,
  pub err: Option<String>,
  pub code: u8,
}

/// The code a failure ends with, where the verb names none of its own.
pub const FAILURE: u8 = 1;

/// How a verb's answer or error ends the CLI (`docs/api.md`, "CLI answers").
pub fn finished(answered: Result<Answer, VerbError>) -> Finished {
  match answered {
    Ok(answer) => Finished {
      out: answer.lines,
      err: None,
      code: answer.code,
    },
    Err(VerbError::Partly { lines, refusal }) => Finished {
      out: lines,
      err: Some(refusal).filter(|refusal| !refusal.is_empty()),
      code: FAILURE,
    },
    Err(VerbError::Ended { code, said }) => Finished {
      out: Vec::new(),
      err: Some(said),
      code,
    },
    Err(VerbError::Refused(said)) => Finished {
      out: Vec::new(),
      err: Some(said),
      code: FAILURE,
    },
  }
}
