//! A wording line's `{name}` marks filled in, as every line of the document wording file is.

/// A line with each `{name}` mark it holds replaced by the value named so. A mark nothing is named for, and
/// a brace that opens no mark, stays as written; a value is put in as it stands, never read for marks.
pub fn fill_marks(line: &str, values: &[(&str, &str)]) -> String {
  let mut out = String::new();
  let mut rest = line;

  while let Some(open) = rest.find('{') {
    out.push_str(&rest[..open]);

    let after = &rest[open + 1..];
    let name_end = after
      .find(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
      .unwrap_or(after.len());
    let value = (name_end > 0 && after[name_end..].starts_with('}'))
      .then(|| values.iter().find(|(name, _)| *name == &after[..name_end]))
      .flatten();

    match value {
      Some((_, value)) => {
        out.push_str(value);
        rest = &after[name_end + 1..];
      }
      None => {
        out.push('{');
        rest = after;
      }
    }
  }

  out + rest
}
