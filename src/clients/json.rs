//! One unit reached with JSON both ways (`docs/api.md`, "What this repository calls"): a `2xx` answer read
//! as its shape, a read of one thing answering nothing on `404`, and any other answer a refusal in the
//! unit's own sentence under `error`.

use reqwest::{Method, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

#[derive(serde::Deserialize)]
struct Refused {
  error: String,
}

/// A unit at the address the settings name under `[units]`, where they name one.
#[derive(Clone, Debug)]
pub struct Unit {
  http: reqwest::Client,
  address: Option<String>,
  /// The unit's name, as a sentence about it says it.
  name: &'static str,
  /// The key its address is set under.
  key: &'static str,
}

impl Unit {
  pub fn new(name: &'static str, key: &'static str, address: Option<String>) -> Self {
    Self {
      http: reqwest::Client::new(),
      address: address.map(|address| address.trim_end_matches('/').to_owned()),
      name,
      key,
    }
  }

  // The answer's status and body, or why there was none.
  async fn send(
    &self,
    method: Method,
    path: &str,
    query: &[(&str, String)],
    body: Option<Value>,
  ) -> Result<(StatusCode, Vec<u8>), String> {
    let Some(address) = &self.address else {
      return Err(format!(
        "the {} address is not set: set {} under [units] in the settings",
        self.name, self.key
      ));
    };
    let url = reqwest::Url::parse_with_params(&format!("{address}{path}"), query)
      .map_err(|why| format!("the {} address cannot be read: {why}", self.name))?;
    let mut request = self.http.request(method, url);

    if let Some(body) = body {
      request = request.json(&body);
    }

    let answer = request.send().await.map_err(|why| why.to_string())?;
    let status = answer.status();
    let bytes = answer.bytes().await.map_err(|why| why.to_string())?;

    Ok((status, bytes.to_vec()))
  }

  // A refusal in the unit's own sentence, or its status where it gave none.
  fn refused(&self, status: StatusCode, body: &[u8]) -> String {
    serde_json::from_slice::<Refused>(body)
      .map(|refused| refused.error)
      .unwrap_or_else(|_| format!("{} answered {status}", self.name))
  }

  fn shaped<T: DeserializeOwned>(&self, status: StatusCode, body: &[u8]) -> Result<T, String> {
    serde_json::from_slice(body)
      .map_err(|why| format!("{} answered {status} in a shape it was not asked for: {why}", self.name))
  }

  /// One thing read by its path: nothing where the unit answers `404`.
  pub async fn found<T: DeserializeOwned>(&self, path: &str) -> Result<Option<T>, String> {
    let (status, body) = self.send(Method::GET, path, &[], None).await?;

    match status {
      StatusCode::NOT_FOUND => Ok(None),
      status if status.is_success() => self.shaped(status, &body).map(Some),
      status => Err(self.refused(status, &body)),
    }
  }

  /// A read with the query given.
  pub async fn read<T: DeserializeOwned>(&self, path: &str, query: &[(&str, String)]) -> Result<T, String> {
    let (status, body) = self.send(Method::GET, path, query, None).await?;

    if !status.is_success() {
      return Err(self.refused(status, &body));
    }

    self.shaped(status, &body)
  }

  /// A body posted, its answer read.
  pub async fn post<T: DeserializeOwned>(&self, path: &str, body: &impl Serialize) -> Result<T, String> {
    let (status, answer) = self.send(Method::POST, path, &[], Some(json(body)?)).await?;

    if !status.is_success() {
      return Err(self.refused(status, &answer));
    }

    self.shaped(status, &answer)
  }

  /// A body posted to a path answering with no body: whether it was done, `false` where the unit answers
  /// the status `declined` names.
  pub async fn act(
    &self,
    path: &str,
    body: &impl Serialize,
    declined: Option<StatusCode>,
  ) -> Result<bool, String> {
    let (status, answer) = self.send(Method::POST, path, &[], Some(json(body)?)).await?;

    match status {
      status if status.is_success() => Ok(true),
      status if Some(status) == declined => Ok(false),
      status => Err(self.refused(status, &answer)),
    }
  }
}

fn json(body: &impl Serialize) -> Result<Value, String> {
  serde_json::to_value(body).map_err(|why| format!("the body could not be written: {why}"))
}
