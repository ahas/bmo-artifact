//! An HTML file read for what it says, before a document is written out of it (`docs/artifact.md`, "An HTML
//! page"): `bmo-artifact html-read`. It prints the page's title and its words, a block to a line, every picture the
//! page carries inside itself standing as `![](<name>)` where it stood, named after `picture` or the stem asked for
//! where one document takes the pictures of two pages; with `--pictures` those pictures are written into that
//! folder under the same names. The words are cut at the answer ceiling, and `--from` carries on from the line the
//! cut names.

use std::path::PathBuf;
use std::sync::LazyLock;

use clap::Args;
use regex::Regex;

use crate::documents::page_reading::{PICTURE_STEM, read_html};
use crate::helpers::ceiling::{Said, cap_items};
use crate::plumbing::verb::{Answer, Shell, Verb, VerbError};

/// What `bmo-artifact html-read` is given.
#[derive(Args)]
pub struct HtmlReadArgs {
  /// The HTML file.
  file: PathBuf,
  /// The folder the pictures inside the page are written into.
  #[arg(long)]
  pictures: Option<PathBuf>,
  /// What the pictures are named after, where it is not `picture`.
  #[arg(long)]
  stem: Option<String>,
  /// The line to read on from, by its number from 1.
  #[arg(long)]
  from: Option<String>,
}

/// `html-read`.
pub struct HtmlRead;

impl Verb for HtmlRead {
  const NAME: &'static str = "html-read";
  const ABOUT: &'static str = "Reads an HTML file for what it says before a document is written out of it: its \
                               title and its words a block to a line, the pictures inside it named where they \
                               stood and with --pictures written out, cut at the answer ceiling with --from \
                               carrying on.";

  type Args = HtmlReadArgs;

  async fn run(&self, args: HtmlReadArgs, _shell: &Shell) -> Result<Answer, VerbError> {
    html_read(&args).map(Answer::from)
  }
}

crate::verb!(HtmlRead);

/// The file's reading, cut at the ceiling, with its pictures written where they were asked to be.
pub fn html_read(asked: &HtmlReadArgs) -> Result<Vec<String>, VerbError> {
  static STEM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\p{L}\p{N}_-]{1,40}$").expect("a pattern"));

  let file = asked.file.display();
  let refused = |why: String| Err(VerbError::Refused(format!("html-read: {why}")));
  let Ok(bytes) = std::fs::read(&asked.file) else {
    return refused(format!("{file} cannot be read."));
  };
  let from = match asked.from.as_deref().map_or(Ok(1), str::parse::<usize>) {
    Ok(from) if from >= 1 => from,
    _ => return refused("--from names a line by its number, from 1.".to_owned()),
  };
  let stem = asked.stem.as_deref().unwrap_or(PICTURE_STEM);

  if !STEM.is_match(stem) {
    return refused("--stem is a short name of letters, digits, - and _.".to_owned());
  }

  let reading = read_html(&String::from_utf8_lossy(&bytes), stem);

  if let Some(folder) = asked.pictures.as_ref().filter(|_| !reading.pictures.is_empty()) {
    let written = std::fs::create_dir_all(folder).and_then(|()| {
      reading
        .pictures
        .iter()
        .try_for_each(|picture| std::fs::write(folder.join(&picture.name), &picture.bytes))
    });

    if let Err(why) = written {
      return refused(format!("the pictures could not be written into {}: {why}", folder.display()));
    }
  }

  let pictures = match (reading.pictures.len(), &asked.pictures) {
    (0, _) => "pictures: none inside the page".to_owned(),
    (count, None) => format!("pictures: {count} inside the page, not written — pass --pictures <dir>"),
    (count, Some(folder)) => format!("pictures: {count} inside the page, written to {}", folder.display()),
  };
  let head = vec![
    format!("title: {}", reading.title.as_deref().unwrap_or("(none)")),
    format!("lines: {}", reading.lines.len()),
    pictures,
    String::new(),
  ];
  let numbered: Vec<String> = reading
    .lines
    .iter()
    .enumerate()
    .skip(from - 1)
    .map(|(at, line)| format!("{}\t{line}", at + 1))
    .collect();
  let capped = cap_items(
    &numbered,
    |kept| [head.as_slice(), kept].concat().join("\n"),
    Said::new("lines", "Give --from the next line number to read on."),
  );
  let mut lines = [head.as_slice(), capped.kept].concat();

  if let Some(note) = &capped.note {
    lines.push(format!("{note} Next: --from {}", from + capped.kept.len()));
  }

  Ok(lines)
}
