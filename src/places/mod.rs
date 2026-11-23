//! Where a document's files are kept (`docs/artifact.md`, "Places"): the one interface every kind of place
//! answers, the catalogue of the kinds, a file's path naming its place and its location, and the place a new
//! document is filed at, chosen by the delegation chain's storage setting.

pub mod local;
pub mod sharepoint;

use std::ops::Bound;
use std::sync::Arc;

use axum::body::Bytes;
use axum::http::HeaderValue;
use bmo_web_client::{ChainKind, ChainLevel, ChainMode, ChainPersonWriter, ChainShape};
use futures_util::StreamExt as _;
use futures_util::future::BoxFuture;
use futures_util::stream::BoxStream;
use headers::{Header, Range};
use serde_json::{Map, Value};

use crate::Refusal;
use crate::accounts::{Accounts, DECLARED_BY};
use crate::settings::Settings;
use crate::wire::{StorageField, StorageFieldShape, StoragePlace};

/// The delegation chain's setting that chooses a new document's place (`docs/artifact.md`, "The setting").
pub const SETTING: &str = "artifact.storage";

/// The field of the setting's value that names the kind of place.
pub const KIND_FIELD: &str = "place";

/// What a place's call comes to.
pub type Pending<'a, T> = BoxFuture<'a, Result<T, Refusal>>;

/// A file read at its place: the whole file's size, the stretch read where a range was asked and answered, both
/// ends counted in, and the bytes as they come.
pub struct Reading {
  pub total: u64,
  pub span: Option<(u64, u64)>,
  pub body: BoxStream<'static, Result<Bytes, std::io::Error>>,
}

impl Reading {
  /// The bytes read, gathered whole.
  pub async fn whole(self) -> Result<Vec<u8>, Refusal> {
    let mut bytes = Vec::with_capacity(usize::try_from(self.total).unwrap_or_default());
    let mut body = self.body;

    while let Some(part) = body.next().await {
      bytes.extend_from_slice(
        &part.map_err(|why| Refusal::upstream(format!("the file could not be read: {why}")))?,
      );
    }

    Ok(bytes)
  }
}

/// What a place says of a file that stands: its size, and a version that changes whenever it is written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kept {
  pub bytes: u64,
  pub version: String,
}

/// One place a document's files are kept at, opened over the settings it was chosen with. A location is the
/// place's own: what a file's path carries after the place's kind.
pub trait Place: Send + Sync {
  /// The name of its kind, which every path at it begins with.
  fn kind(&self) -> &'static str;
  /// The folder a new document's files stand in, for its number and its owner.
  fn folder<'a>(&'a self, document: i64, owner: Option<&'a str>) -> Pending<'a, String>;
  /// A file written whole at a location, replacing what stood there.
  fn write<'a>(&'a self, at: &'a str, bytes: Vec<u8>) -> Pending<'a, ()>;
  /// A file read, in the one stretch `range` asks for where it asks for one the file holds, else whole.
  fn read<'a>(&'a self, at: &'a str, range: Option<&'a str>) -> Pending<'a, Reading>;
  /// What stands at a location, none where no file does.
  fn exists<'a>(&'a self, at: &'a str) -> Pending<'a, Option<Kept>>;
  /// A location let go of with everything under it; one where nothing stands is gone already.
  fn delete<'a>(&'a self, at: &'a str) -> Pending<'a, ()>;
}

/// What a place opens over: the service's settings, the account service and the HTTP client it calls out with.
#[derive(Clone)]
pub struct Grounds {
  pub settings: Arc<Settings>,
  pub accounts: Accounts,
  pub http: reqwest::Client,
}

/// One kind of place (`docs/artifact.md`, "Places"): what it is, the settings it takes, how it judges them and how
/// it opens over them.
pub trait PlaceKind: Sync {
  fn name(&self) -> &'static str;
  /// What it is, in one English sentence.
  fn description(&self) -> &'static str;
  /// The settings it takes beside its name.
  fn fields(&self) -> Vec<StorageField>;
  /// The settings a value names judged, trimmed as they are kept, or why they are refused.
  fn judge(&self, settings: &Map<String, Value>) -> Result<Map<String, Value>, String>;
  /// The place opened over settings it judged, for a document of `owner`.
  fn open(
    &self,
    settings: &Map<String, Value>,
    grounds: &Grounds,
    owner: Option<&str>,
  ) -> Result<Arc<dyn Place>, Refusal>;
}

/// Every kind of place, in the order `GET /storage-places` answers them. Adding a kind is adding its module and its
/// line here.
pub static PLACES: [&dyn PlaceKind; 2] = [&local::Local, &sharepoint::SharePoint];

/// The kind of place named `name`.
pub fn kind_named(name: &str) -> Option<&'static dyn PlaceKind> {
  PLACES.iter().copied().find(|kind| kind.name() == name)
}

/// Every kind of place as the screens are told of it.
pub fn catalogue() -> Vec<StoragePlace> {
  PLACES
    .iter()
    .map(|kind| StoragePlace {
      name: kind.name().to_owned(),
      description: kind.description().to_owned(),
      fields: kind.fields(),
    })
    .collect()
}

/// The setting as this service declares it on the account service's delegation chain.
pub fn setting() -> ChainKind {
  ChainKind {
    key: SETTING.to_owned(),
    declared_by: DECLARED_BY.to_owned(),
    description:
      "Where the documents of the people under a link are kept: the place and its settings, judged by \
                  bmo-artifact."
        .to_owned(),
    shape: ChainShape::Json,
    choices: Vec::new(),
    mode: ChainMode::Nearest,
    default: None,
    rules: false,
    levels: vec![ChainLevel::Platform, ChainLevel::Unit],
    unit_permission: "manage".to_owned(),
    person_writer: ChainPersonWriter::Heads,
  }
}

/// A value of the setting judged by the kind it names: the value as it is written, or why it is refused.
pub fn judged(value: &Value) -> Result<Value, String> {
  let fields = value.as_object().ok_or("a storage value is an object naming its place")?;
  let name = fields
    .get(KIND_FIELD)
    .and_then(Value::as_str)
    .ok_or("a storage value names its kind of place in `place`")?;
  let kind = kind_named(name.trim()).ok_or_else(|| {
    let names: Vec<&str> = PLACES.iter().map(|kind| kind.name()).collect();

    format!("no place is named {name:?}; the places are {}", names.join(", "))
  })?;
  let mut settings = fields.clone();

  settings.remove(KIND_FIELD);

  let mut judged = kind.judge(&settings)?;

  judged.insert(KIND_FIELD.to_owned(), Value::from(kind.name()));

  Ok(Value::Object(judged))
}

/// A field of the settings that is none of `fields`: refused, naming it.
pub fn unknown_field(settings: &Map<String, Value>, fields: &[StorageField]) -> Result<(), String> {
  match settings.keys().find(|key| !fields.iter().any(|field| field.key == **key)) {
    Some(key) => Err(format!("a storage place takes no field {key:?}")),
    None => Ok(()),
  }
}

/// A field of the settings as a trimmed string, none where it is absent or empty; refused where it is no string.
pub fn text(settings: &Map<String, Value>, key: &str) -> Result<Option<String>, String> {
  match settings.get(key) {
    None | Some(Value::Null) => Ok(None),
    Some(Value::String(text)) => Ok(Some(text.trim().to_owned()).filter(|text| !text.is_empty())),
    Some(_) => Err(format!("{key} is a string")),
  }
}

/// A field a kind takes, as its own list states it.
pub fn field(key: &str, shape: StorageFieldShape, required: bool) -> StorageField {
  StorageField {
    key: key.to_owned(),
    shape,
    required,
  }
}

/// The stretch of a file the Range header asks for, both ends counted in, none where it asks for none or for
/// none a file of `bytes` holds, the whole file being the answer then.
pub fn range_of(asked: Option<&str>, bytes: u64) -> Option<(u64, u64)> {
  let value = HeaderValue::from_str(asked?.trim()).ok()?;
  let range = Range::decode(&mut std::iter::once(&value)).ok()?;
  let (start, end) = range.satisfiable_ranges(bytes).next()?;
  let from = match start {
    Bound::Included(from) => from,
    Bound::Excluded(from) => from + 1,
    Bound::Unbounded => 0,
  };
  let to = match end {
    Bound::Included(to) => to.min(bytes.checked_sub(1)?),
    Bound::Excluded(to) => to.checked_sub(1)?.min(bytes.checked_sub(1)?),
    Bound::Unbounded => bytes.checked_sub(1)?,
  };

  (from <= to && from < bytes).then_some((from, to))
}

/// A file at its place: the place and the location there.
#[derive(Clone)]
pub struct Located {
  pub place: Arc<dyn Place>,
  pub at: String,
}

impl std::fmt::Debug for Located {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    formatter.write_str(&self.path())
  }
}

impl Located {
  /// The path a row keeps it by: its place's kind and its location.
  pub fn path(&self) -> String {
    format!("{}:{}", self.place.kind(), self.at)
  }

  /// The file read, as `range` asks.
  pub async fn read(&self, range: Option<&str>) -> Result<Reading, Refusal> {
    self.place.read(&self.at, range).await
  }

  /// The file read whole.
  pub async fn whole(&self) -> Result<Vec<u8>, Refusal> {
    self.read(None).await?.whole().await
  }

  /// What stands there.
  pub async fn exists(&self) -> Result<Option<Kept>, Refusal> {
    self.place.exists(&self.at).await
  }
}

/// The place a new document is filed at, and what it keeps of the choice.
pub struct Chosen {
  pub place: Arc<dyn Place>,
  /// The value that chose it, none for the local place no value named.
  pub value: Option<Value>,
  /// The company the owner stands in.
  pub company: Option<i64>,
}

/// Every place a document's files may be kept at, opened over the service's grounds.
#[derive(Clone)]
pub struct Places {
  grounds: Grounds,
  local: Arc<dyn Place>,
}

impl Places {
  pub fn new(settings: &Settings, accounts: &Accounts) -> Self {
    let grounds = Grounds {
      settings: Arc::new(settings.clone()),
      accounts: accounts.clone(),
      http: reqwest::Client::new(),
    };
    let local = Arc::new(local::LocalPlace::new(&settings.files.root));

    Self { grounds, local }
  }

  /// The place on this machine, which a document no value placed elsewhere stands at.
  pub fn local(&self) -> Arc<dyn Place> {
    Arc::clone(&self.local)
  }

  /// The place a document filed under `value` keeps its files at, for its `owner`: the local place where no value
  /// names one.
  pub fn open(&self, value: Option<&Value>, owner: Option<&str>) -> Result<Arc<dyn Place>, Refusal> {
    let Some(fields) = value.and_then(Value::as_object) else {
      return Ok(self.local());
    };
    let name = fields.get(KIND_FIELD).and_then(Value::as_str).unwrap_or_default();
    let kind = kind_named(name).ok_or_else(|| {
      Refusal::failed(format!("the document was filed at a place no kind is named {name:?}"))
    })?;

    if kind.name() == local::NAME {
      return Ok(self.local());
    }

    let mut settings = fields.clone();

    settings.remove(KIND_FIELD);
    kind.open(&settings, &self.grounds, owner)
  }

  /// The file a row's `path` names, at the place the document was filed under: a path that names no kind of place
  /// is the local place's location as it stands.
  pub fn locate(&self, value: Option<&Value>, owner: Option<&str>, path: &str) -> Result<Located, Refusal> {
    let named = path.split_once(':').filter(|(kind, _)| kind_named(kind).is_some());
    let Some((kind, at)) = named else {
      return Ok(Located {
        place: self.local(),
        at: path.to_owned(),
      });
    };
    let place = if kind == local::NAME { self.local() } else { self.open(value, owner)? };

    if place.kind() != kind {
      return Err(Refusal::failed(format!("the file {path} is at a place its document was not filed at")));
    }

    Ok(Located {
      place,
      at: at.to_owned(),
    })
  }

  /// The place a new document of `owner` is filed at, as the delegation chain's storage setting resolves for them:
  /// the local place for a document with no owner and wherever the chain says nothing.
  pub async fn choose(&self, owner: Option<&str>) -> Result<Chosen, Refusal> {
    let Some(owner) = owner else {
      return Ok(Chosen {
        place: self.local(),
        value: None,
        company: None,
      });
    };
    let storage = self.grounds.accounts.storage_of(owner).await?;
    let value = match storage.value {
      Some(value) => Some(judged(&value).map_err(|why| {
        Refusal::upstream(format!("the storage setting standing for account {owner} does not read: {why}"))
      })?),
      None => None,
    };
    let place = self.open(value.as_ref(), Some(owner))?;

    Ok(Chosen {
      place,
      value,
      company: storage.company,
    })
  }
}

/// A stream of one block of bytes, for a place that reads a file whole.
pub fn whole_stream(bytes: Bytes) -> BoxStream<'static, Result<Bytes, std::io::Error>> {
  futures_util::stream::once(async move { Ok(bytes) }).boxed()
}
