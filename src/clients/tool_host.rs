//! The tool host, through which bmo-explorer reads a page whole and fetches a raster picture, each address
//! checked and each request paced on its side (`docs/api.md`, "Read a page and fetch a picture").

use std::future::Future;
use std::pin::Pin;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use serde_json::{Value, json};

/// A picture as it was fetched: its type and its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Raster {
  pub kind: String,
  pub bytes: Vec<u8>,
}

/// A reference the page's own reference list names.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct ReadReference {
  pub text: String,
  pub url: String,
}

/// A figure the page draws, as the page says it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct DrawnFigure {
  pub n: i64,
  #[serde(default)]
  pub caption: String,
  #[serde(default)]
  pub svg: Option<String>,
}

/// The page as it read itself: its address after redirects, its title and keywords, its text, its
/// references and the figures it draws.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct ReadPage {
  pub url: String,
  #[serde(default)]
  pub title: String,
  #[serde(default)]
  pub keywords: Vec<String>,
  #[serde(default)]
  pub body: String,
  #[serde(default)]
  pub references: Vec<ReadReference>,
  #[serde(default)]
  pub figures: Vec<DrawnFigure>,
}

/// One figure captured as a picture: its number and the PNG's bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Captured {
  pub n: i64,
  pub png: Vec<u8>,
}

/// A page read whole, and each of its figures that captured.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reading {
  pub page: ReadPage,
  pub captured: Vec<Captured>,
}

/// What reading comes to: what was read, or why not, in a sentence.
pub type Reached<'a, T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'a>>;

/// The two reads the search line asks of the web.
pub trait PageReader: Send + Sync {
  /// The raster picture behind an address.
  fn picture<'a>(&'a self, address: &'a str) -> Reached<'a, Raster>;
  /// A page drawn and read whole, with its figures captured.
  fn page<'a>(&'a self, address: &'a str) -> Reached<'a, Reading>;
}

/// The tool host at the address the settings name, where they name one, reading under one pacing name.
#[derive(Clone, Debug)]
pub struct ToolHost {
  http: reqwest::Client,
  address: Option<String>,
  pacing: String,
}

#[derive(Deserialize)]
struct Refused {
  error: String,
}

/// A tool result as the tool host answers it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolResult {
  #[serde(default)]
  content: Vec<Value>,
  #[serde(default)]
  structured_content: Option<Value>,
  #[serde(default)]
  is_error: bool,
}

#[derive(Deserialize)]
struct ImportedPage {
  page: ReadPage,
  #[serde(default)]
  captured: Vec<CapturedFigure>,
}

#[derive(Deserialize)]
struct CapturedFigure {
  n: i64,
  png: String,
}

fn bytes_of(base64: &str, what: &str) -> Result<Vec<u8>, String> {
  STANDARD.decode(base64).map_err(|why| format!("the {what} is not base64: {why}"))
}

impl ToolHost {
  /// The tool host at `address`, its requests paced together under `pacing`: one name for each search
  /// run's reading.
  pub fn new(address: Option<String>, pacing: impl Into<String>) -> Self {
    Self {
      http: reqwest::Client::new(),
      address: address.map(|address| address.trim_end_matches('/').to_owned()),
      pacing: pacing.into(),
    }
  }

  // One entry called with its arguments: its tool result, or the sentence refusing it.
  async fn call(&self, tool: &str, arguments: Value) -> Result<ToolResult, String> {
    let Some(address) = &self.address else {
      return Err(
        "the tool host's address is not set: set tool_host under [units] in the settings".to_owned(),
      );
    };
    let answer = self
      .http
      .post(format!("{address}/calls"))
      .json(&json!({ "tool": tool, "arguments": arguments }))
      .send()
      .await
      .map_err(|why| why.to_string())?;
    let status = answer.status();
    let body = answer.bytes().await.map_err(|why| why.to_string())?;

    if !status.is_success() {
      return Err(
        serde_json::from_slice::<Refused>(&body)
          .map(|refused| refused.error)
          .unwrap_or_else(|_| format!("the tool host answered {status}")),
      );
    }

    let result: ToolResult = serde_json::from_slice(&body)
      .map_err(|why| format!("the tool host answered {status} with no tool result: {why}"))?;

    if result.is_error {
      let said: Vec<&str> = result
        .content
        .iter()
        .filter(|block| block["type"] == "text")
        .filter_map(|block| block["text"].as_str())
        .collect();

      return Err(if said.is_empty() { format!("{tool} failed") } else { said.join("\n") });
    }

    Ok(result)
  }
}

impl PageReader for ToolHost {
  fn picture<'a>(&'a self, address: &'a str) -> Reached<'a, Raster> {
    Box::pin(async move {
      let result = self
        .call("explorer/fetch_raster", json!({ "address": address, "pacing": self.pacing }))
        .await?;
      let image = result
        .content
        .iter()
        .find(|block| block["type"] == "image")
        .ok_or_else(|| "the picture's answer carries no image".to_owned())?;

      Ok(Raster {
        kind: image["mimeType"].as_str().unwrap_or_default().to_owned(),
        bytes: bytes_of(image["data"].as_str().unwrap_or_default(), "picture")?,
      })
    })
  }

  fn page<'a>(&'a self, address: &'a str) -> Reached<'a, Reading> {
    Box::pin(async move {
      let result = self.call("explorer/import_page", json!({ "address": address })).await?;
      let imported: ImportedPage = serde_json::from_value(result.structured_content.unwrap_or_default())
        .map_err(|why| format!("the page's answer is not a reading: {why}"))?;
      let mut captured = Vec::new();

      for figure in imported.captured {
        captured.push(Captured {
          n: figure.n,
          png: bytes_of(&figure.png, "captured figure")?,
        });
      }

      Ok(Reading {
        page: imported.page,
        captured,
      })
    })
  }
}
