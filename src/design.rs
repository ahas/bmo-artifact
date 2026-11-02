//! The archive's one document design (`docs/artifact.md`, "How a document is drawn"), read from
//! `client/design/document.json`, which the console plugin bundle carries and reads too: the palette of each
//! scheme, the sheet a page document goes out with, and which pictures open the picture viewer.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

/// What a reader asked to read in: the console's screen mode, where `system` is whatever the reader's own
/// system asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentScheme {
  Light,
  Dark,
  System,
}

impl DocumentScheme {
  /// The scheme a word names, none for any other word.
  pub fn named(word: &str) -> Option<Self> {
    match word {
      "light" => Some(Self::Light),
      "dark" => Some(Self::Dark),
      "system" => Some(Self::System),
      _ => None,
    }
  }

  /// The word a scheme is named by.
  pub fn word(self) -> &'static str {
    match self {
      Self::Light => "light",
      Self::Dark => "dark",
      Self::System => "system",
    }
  }
}

/// How a page is drawn, read off what it brings (`docs/artifact.md`, "An HTML page").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageDesign {
  /// It brings no stylesheet of its own, and is drawn wholly in the archive's design.
  Archive,
  /// It brings its own, and keeps its layout under the archive's type.
  Own,
  /// It brings its own with a dark scheme of its own, and draws itself.
  OwnDark,
}

#[derive(Deserialize)]
struct Palettes {
  light: Vec<(String, String)>,
  dark: Vec<(String, String)>,
}

/// Which pictures open the viewer, and the message a page in the console's frame asks for it by.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Viewer {
  /// The selector a picture that opens the viewer matches.
  pub pictures: String,
  /// The pattern an address the viewer may show matches, as a script writes it, with its flags.
  pub address: String,
  pub address_flags: String,
  /// The type of the message a page posts its frame.
  pub message: String,
  pub most_pictures: usize,
  pub most_caption_chars: usize,
}

#[derive(Deserialize)]
struct Design {
  palettes: Palettes,
  inverted: String,
  element_rules: Vec<(String, String)>,
  narrow_element_rules: Vec<(String, String)>,
  page_rules: Vec<String>,
  viewer: Viewer,
  scheme_cookie: String,
  default_scheme: String,
}

static DESIGN: LazyLock<Design> = LazyLock::new(|| {
  serde_json::from_str(include_str!("../client/design/document.json")).expect("the design file reads")
});

/// Which pictures open the viewer.
pub fn viewer() -> &'static Viewer {
  &DESIGN.viewer
}

/// The cookie the console keeps the screen mode in.
pub fn scheme_cookie() -> &'static str {
  &DESIGN.scheme_cookie
}

/// The scheme a reader who never chose one reads in.
pub fn default_scheme() -> DocumentScheme {
  DocumentScheme::named(&DESIGN.default_scheme).unwrap_or(DocumentScheme::Dark)
}

// Each colour as the variable a sheet names it by.
fn variables(palette: &[(String, String)]) -> String {
  palette
    .iter()
    .map(|(name, colour)| format!("--doc-{name}:{colour}"))
    .collect::<Vec<_>>()
    .join(";")
}

/// The palette for the console, which marks its dark mode with the `dark` class on the root.
pub fn document_palette() -> String {
  format!(
    ":root{{{}}}:root.dark{{{}}}",
    variables(&DESIGN.palettes.light),
    variables(&DESIGN.palettes.dark)
  )
}

// The palette of one scheme, written on the root with the browser's own colour scheme to match.
fn scheme_palette(scheme: DocumentScheme) -> String {
  let (light, dark) = (variables(&DESIGN.palettes.light), variables(&DESIGN.palettes.dark));

  match scheme {
    DocumentScheme::Light => format!(":root{{{light};color-scheme:light}}"),
    DocumentScheme::Dark => format!(":root{{{dark};color-scheme:dark}}"),
    DocumentScheme::System => {
      format!(
        ":root{{{light};color-scheme:light dark}}@media (prefers-color-scheme:dark){{:root{{{dark}}}}}"
      )
    }
  }
}

// The rules as a sheet, each selector written under `scope` where one is given; the root's own element is
// never scoped under itself.
fn sheet(rules: &[(String, String)], scope: &str) -> String {
  rules
    .iter()
    .map(|(selector, declarations)| {
      let scoped: Vec<String> = selector
        .split(',')
        .map(|one| if one == "html" { one.to_owned() } else { format!("{scope}{one}") })
        .collect();

      format!("{}{{{declarations}}}", scoped.join(","))
    })
    .collect()
}

fn element_sheet(scope: &str) -> String {
  format!(
    "{}@media (max-width:640px){{{}}}",
    sheet(&DESIGN.element_rules, scope),
    sheet(&DESIGN.narrow_element_rules, scope)
  )
}

/// The sheet a page goes out with, for the scheme its reader reads in (`docs/artifact.md`, "An HTML page").
pub fn page_sheet(scheme: DocumentScheme, design: PageDesign) -> String {
  match design {
    PageDesign::Archive => {
      format!("{}{}{}", scheme_palette(scheme), element_sheet(""), DESIGN.page_rules.concat())
    }
    PageDesign::OwnDark => format!(
      ":root{{color-scheme:{}}}",
      if scheme == DocumentScheme::System { "light dark" } else { scheme.word() }
    ),
    PageDesign::Own => {
      let light = format!("{}{}", scheme_palette(DocumentScheme::Light), element_sheet("html "));

      match scheme {
        DocumentScheme::Light => light,
        DocumentScheme::Dark => format!("{light}{}", DESIGN.inverted),
        DocumentScheme::System => {
          format!("{light}@media (prefers-color-scheme:dark){{{}}}", DESIGN.inverted)
        }
      }
    }
  }
}
