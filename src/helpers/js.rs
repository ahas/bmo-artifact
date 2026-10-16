//! JavaScript's reading of a text, where a twin of a TypeScript helper must read it the same way: what
//! counts as white space, and a text taken as its characters.

/// JavaScript's white space, as a regular expression's class holds it.
pub const SPACE: &str =
  r"\t\n\x0B\x0C\r \x{a0}\x{1680}\x{2000}-\x{200a}\x{2028}\x{2029}\x{202f}\x{205f}\x{3000}\x{feff}";

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
