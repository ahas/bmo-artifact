//! JavaScript's reading of a text, where a twin of a TypeScript helper must read it the same way: what
//! counts as white space, and a text taken as its characters.

/// JavaScript's white space, as a regular expression's class holds it.
pub const SPACE: &str =
  r"\t\n\x0B\x0C\r \x{a0}\x{1680}\x{2000}-\x{200a}\x{2028}\x{2029}\x{202f}\x{205f}\x{3000}\x{feff}";

/// What JavaScript's `encodeURIComponent` leaves as it is: letters, digits and `-_.!~*'()`.
pub const URI_COMPONENT: &percent_encoding::AsciiSet = &percent_encoding::NON_ALPHANUMERIC
  .remove(b'-')
  .remove(b'_')
  .remove(b'.')
  .remove(b'!')
  .remove(b'~')
  .remove(b'*')
  .remove(b'\'')
  .remove(b'(')
  .remove(b')');

/// A whole number read off the front of a text as `parseInt` reads one: its digits, after white space and a
/// minus, up to the first that is none; none where it opens with no number at all.
pub fn parse_int(text: &str) -> Option<i64> {
  let digits = trim(text);
  let end = digits
    .char_indices()
    .find(|(at, one)| !(one.is_ascii_digit() || (*at == 0 && *one == '-')))
    .map_or(digits.len(), |(at, _)| at);

  digits[..end].parse().ok()
}

/// Whether a character is JavaScript's white space.
pub fn is_space(character: char) -> bool {
  matches!(
    character,
    '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
      ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
  )
}

/// A text with JavaScript's white space taken off both ends.
pub fn trim(text: &str) -> &str {
  text.trim_matches(is_space)
}

/// A text's characters, as positions into it are counted.
pub fn chars(text: &str) -> Vec<char> {
  text.chars().collect()
}

/// Characters put back into a text.
pub fn text(characters: &[char]) -> String {
  characters.iter().collect()
}

/// A number written with `places` digits after the point as `toFixed` writes it: rounded on the number's
/// exact value, a half going up and away from nought, where Rust's own formatter takes a half to the even
/// digit.
pub fn fixed(value: f64, places: usize) -> String {
  // A double's exact decimal expansion never runs past this many digits after the point.
  let exact = format!("{:.1100}", value.abs());
  let (whole, fraction) = exact.split_once('.').unwrap_or((&exact, ""));
  let mut digits: Vec<u8> = format!("{whole}{}", &fraction[..places]).into_bytes();

  if fraction.as_bytes()[places] >= b'5' {
    let mut at = digits.len();

    loop {
      if at == 0 {
        digits.insert(0, b'1');
        break;
      }

      at -= 1;

      if digits[at] == b'9' {
        digits[at] = b'0';
      } else {
        digits[at] += 1;
        break;
      }
    }
  }

  let digits = String::from_utf8(digits).unwrap_or_default();
  let (whole, fraction) = digits.split_at(digits.len() - places);
  let sign = if value < 0.0 { "-" } else { "" };

  if places == 0 { format!("{sign}{whole}") } else { format!("{sign}{whole}.{fraction}") }
}
