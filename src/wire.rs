//! The members, bodies and queries the routes answer and take (`docs/api.md`), in the common wire format:
//! camelCase keys, times in epoch milliseconds, ids as signed 64-bit integers. The routes, the client and the
//! OpenAPI document all read these types, so a field is written once.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The caller's own metadata: an object of strings.
pub type Labels = BTreeMap<String, String>;

/// A failure: its kebab-case code and its English sentence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(rename = "Error")]
pub struct WireError {
  pub code: String,
  pub error: String,
}

/// Readiness and version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Health {
  pub ready: bool,
  /// Why the service is not ready; null when it is.
  pub reason: Option<String>,
  pub version: String,
}

/// One event: its number, its kind, when it was kept and the member it is about.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Event {
  pub id: i64,
  /// The resource and its change: `document.created`.
  pub kind: String,
  /// Epoch milliseconds.
  pub at: i64,
  /// The member as a read answers it, or its key alone for a removal.
  pub data: Value,
}

/// A page of events, oldest first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EventPage {
  pub items: Vec<Event>,
  pub more: bool,
}

/// What `GET /events` is narrowed and resumed by.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EventsQuery {
  /// The kinds to keep, comma-separated.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub kinds: Option<String>,
  /// The event to resume after, by its number.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub after: Option<i64>,
  /// At most this many events in a page: 100 where none is given, 1000 at most.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub limit: Option<i64>,
  /// Only the events of members held under this scope.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub scope: Option<String>,
}

/// One document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Document {
  pub id: i64,
  /// One of the kinds `GET /document-kinds` lists.
  pub kind: String,
  pub title: String,
  /// What it is held under; null where none was given.
  pub scope: Option<String>,
  /// Who filed it.
  pub actor_name: String,
  /// The account key of the account that owns it, as `Owner-Key` named it when it was filed; null for a document
  /// with no owner (`docs/artifact.md`, "Owners").
  pub owner: Option<String>,
  /// The caller's own metadata.
  pub labels: Labels,
  /// What it reads as before it is opened: a page's declared description, else the opening prose of its
  /// body, cut at 160 characters; null where it has none.
  pub description: Option<String>,
  pub attachment_count: i64,
  /// Whether it keeps a picture, so its cover picture has one to send.
  pub has_picture: bool,
  /// Epoch milliseconds.
  pub created_at: i64,
  /// Epoch milliseconds.
  pub changed_at: i64,
  /// For an account reader that reads the document alone, through a share or as the administrator reading one
  /// another account owns, the permission it reads it under; absent where the reader reaches it wholly, and in
  /// every event (`docs/api.md`, "Readers").
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub permission: Option<Permission>,
}

/// What a member a document is shared with may do with it (`docs/api.md`, "Member shares"): `read` is given,
/// and `edit` is named so that asking for it is refused rather than misread.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
  Read,
  Edit,
}

impl Permission {
  /// The word it is written and kept as.
  pub fn word(self) -> &'static str {
    match self {
      Self::Read => "read",
      Self::Edit => "edit",
    }
  }

  /// The permission a kept word names, none for a word no permission is written as.
  pub fn of_word(word: &str) -> Option<Self> {
    [Self::Read, Self::Edit].into_iter().find(|permission| permission.word() == word)
  }
}

/// One member a document is shared with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemberShare {
  pub id: i64,
  pub document_id: i64,
  /// The member's account key, as the console writes `Owner-Key`.
  pub user: String,
  pub permission: Permission,
  /// Epoch milliseconds.
  pub shared_at: i64,
}

/// A page of member shares, in the order they were shared.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemberSharePage {
  pub items: Vec<MemberShare>,
  pub more: bool,
}

/// The members to share a document with, and what they may do with it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemberShareInput {
  /// The members' account keys: 1 to 200, each of 1 to 200 characters.
  pub users: Vec<String>,
  /// `read`; `edit` is refused.
  pub permission: Permission,
}

/// A member share's permission set anew.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemberShareChange {
  /// `read`; `edit` is refused.
  pub permission: Permission,
}

/// One organisation unit a document is shared with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UnitShare {
  pub id: i64,
  pub document_id: i64,
  /// The unit's number, as the account service numbers it.
  pub unit: i64,
  /// Epoch milliseconds.
  pub shared_at: i64,
}

/// A page of unit shares, in the order they were shared.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UnitSharePage {
  pub items: Vec<UnitShare>,
  pub more: bool,
}

/// One unit of a company's tree, as the account service names it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompanyUnit {
  /// The unit's number, as the account service numbers it.
  pub id: i64,
  pub name: String,
  /// The unit it stands directly under; null for the company.
  pub parent: Option<i64>,
}

/// The units of a document owner's company, which the document may be shared with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompanyUnits {
  /// The company's number, the unit at the top of its tree.
  pub company: i64,
  /// Every unit of the company: the company first, each unit after its parent, siblings in the account service's
  /// order.
  pub units: Vec<CompanyUnit>,
}

/// The organisation units to share a document with, each to read it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UnitShareInput {
  /// The units' numbers: 1 to 200 of them.
  pub units: Vec<i64>,
}

/// A page of documents, newest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPage {
  pub items: Vec<Document>,
  pub more: bool,
}

/// What `GET /documents` is filtered and paged by.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DocumentsQuery {
  /// One kind.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub kind: Option<String>,
  /// One scope.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub scope: Option<String>,
  /// `key:value` for documents whose label `key` is `value`, or `key` alone for those carrying it.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub label: Option<String>,
  /// A phrase matched whole and without regard to case in the title, the labels and the body.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub q: Option<String>,
  /// Only documents older than this one, by its id.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub before: Option<i64>,
  /// Only documents newer than this one, by its id.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub after: Option<i64>,
  /// At most this many: 30 where none is given, 200 at most.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub limit: Option<i64>,
}

/// A document to file.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DocumentInput {
  /// One of the kinds `GET /document-kinds` lists.
  pub kind: String,
  /// Its title; a page with a `title` element of its own may leave it empty.
  pub title: String,
  /// The markdown body, for every kind but a page.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub body: Option<String>,
  /// The HTML, for a page.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub html: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub scope: Option<String>,
  pub actor_name: String,
  #[serde(default)]
  pub labels: Labels,
  /// What it reads as before it is opened, where the filer says it; else it is read out of the body or the page.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
}

/// A document rewritten under its number: each field given replaces its own, and the labels replace the
/// labels whole.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DocumentChange {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub title: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub body: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub html: Option<String>,
  pub labels: Labels,
}

/// How a file is sent.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContentQuery {
  /// The scheme a hosted page's prelude draws in: `light`, `dark` or `system`, the design's default where
  /// none is named.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub scheme: Option<String>,
  /// The language a hosted page's player speaks, a locale of the console bundle's words; English where none
  /// is named or the words hold none of it.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub language: Option<String>,
  /// Send the file itself to keep, under its name.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub download: Option<bool>,
}

/// One kind a document may be filed as.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DocumentKind {
  pub name: String,
  pub description: String,
  /// Whether a document of this kind is the HTML page it keeps, filed as `html` and opened as that page.
  pub hosted: bool,
  /// How many levels of its headings its table of contents lists, 0 for none.
  pub contents_depth: u8,
}

/// A page of kinds, in the catalogue's order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DocumentKindPage {
  pub items: Vec<DocumentKind>,
  pub more: bool,
}

/// What `GET /label-values` is asked: the label whose values it answers, and how many at most.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LabelValuesQuery {
  /// The label's key.
  pub key: String,
  /// At most this many: 30 where none is given, 200 at most.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub limit: Option<i64>,
}

/// One value a label holds, and how many documents carry it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LabelValue {
  pub value: String,
  pub count: i64,
}

/// A page of a label's values, the most carried first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LabelValuePage {
  pub items: Vec<LabelValue>,
  pub more: bool,
}

/// A page asked by `after` and `limit`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PageQuery {
  /// Only the members after this one, by its id.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub after: Option<i64>,
  /// At most this many: 30 where none is given, 200 at most.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub limit: Option<i64>,
}

/// One file a document keeps.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
  pub id: i64,
  pub document_id: i64,
  /// The name it is shown and named by in the body; a last part such as `-ko` before the extension is the
  /// language it is in.
  pub name: String,
  /// Its type as kept.
  pub media_type: String,
  /// What it is read as: `image`, `audio`, `video`, `text`, `page` or `file`.
  pub kind: String,
  pub byte_count: i64,
  /// The caller's word for its part in the document; `head` marks a file a reader is shown at a markdown
  /// document's head, a sound in the listen band. A page stands its sounds where it writes them.
  pub role: Option<String>,
  /// Epoch milliseconds.
  pub created_at: i64,
}

/// A page of attachments, in the order they were kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentPage {
  pub items: Vec<Attachment>,
  pub more: bool,
}

/// A file to attach.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentInput {
  pub name: String,
  pub media_type: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub role: Option<String>,
  /// The file's bytes in base64.
  pub content: String,
}

/// A page a document refers to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
  pub id: i64,
  pub document_id: i64,
  pub url: String,
  /// The page's title as the document or its preview gives it.
  pub title: Option<String>,
  /// The description the page declares for a preview.
  pub preview: Option<String>,
}

/// A page of references, in the document's order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReferencePage {
  pub items: Vec<Reference>,
  pub more: bool,
}

/// A share link.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Share {
  pub token: String,
  pub document_id: i64,
  /// The share address, a `/` and the token.
  pub url: String,
  /// Epoch milliseconds.
  pub created_at: i64,
  /// Epoch milliseconds; null for a link with no end.
  pub expires_at: Option<i64>,
}

/// What a live link reads as before it is opened (`docs/api.md`, "Share links"): the values a console writes
/// into the head of the page it serves at the link's address.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShareHead {
  /// The document's title, as the page shows it.
  pub title: String,
  /// The document's description, led by its kind's word where the page names the kind over the title; absent
  /// where the document has none.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  /// The share read of the cover picture, carrying the token; absent where the document keeps none.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub image: Option<String>,
  /// The link's address.
  pub url: String,
  /// The locale of the language the words are in, as Open Graph writes one: `ko_KR`.
  pub locale: String,
  /// The name the card stands under.
  pub site_name: String,
  /// The Open Graph type: `article`.
  pub r#type: String,
  /// The word the page names the document's kind by; absent where the page names none.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub kind: Option<String>,
}

/// The language a share head is asked in.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HeadQuery {
  /// A locale of the console bundle's words; English where none is named or the words hold none of it.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub language: Option<String>,
}

/// A page of live links, oldest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SharePage {
  pub items: Vec<Share>,
  pub more: bool,
}

/// What `GET /shares` is filtered and paged by.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SharesQuery {
  /// One document's link.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub document_id: Option<i64>,
  /// Only the links made after this time, in epoch milliseconds: the last item's `createdAt`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub after: Option<i64>,
  /// At most this many: 30 where none is given, 200 at most.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub limit: Option<i64>,
}

/// A link to make.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShareInput {
  pub document_id: i64,
  /// When it stops opening, in epoch milliseconds; none for a link with no end.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub expires_at: Option<i64>,
}

/// A document filed and a link to it opened in one call: what a document is handed to somebody as.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Publication {
  pub document: Document,
  /// The link that opens it, with no end: it names the document, so it serves the body the document stands
  /// with whenever it is opened.
  pub share: Share,
}

/// Where an import stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ImportState {
  Fetching,
  Filed,
  Failed,
}

impl ImportState {
  /// The word the store keeps.
  pub fn word(self) -> &'static str {
    match self {
      Self::Fetching => "fetching",
      Self::Filed => "filed",
      Self::Failed => "failed",
    }
  }

  /// The state a stored word names; none for a word that names none.
  pub fn of_word(word: &str) -> Option<Self> {
    [Self::Fetching, Self::Filed, Self::Failed]
      .into_iter()
      .find(|state| state.word() == word)
  }
}

/// One import.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Import {
  pub id: i64,
  /// The address asked for.
  pub url: String,
  pub state: ImportState,
  /// The document it filed; null until it is filed.
  pub document_id: Option<i64>,
  /// Why it failed; null otherwise.
  pub error: Option<String>,
  /// Epoch milliseconds.
  pub created_at: i64,
}

/// A page to import.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImportInput {
  pub url: String,
  /// The document's title; the page's own where none is given.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub title: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub scope: Option<String>,
  pub actor_name: String,
}

/// How many members of each resource a scope holds.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScopeCounts {
  pub documents: i64,
  pub attachments: i64,
  pub shares: i64,
  pub imports: i64,
}

/// What the service holds under one scope, counted by resource.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScopeHolding {
  pub scope: String,
  pub counts: ScopeCounts,
  /// Null for a read, `released` once let go of, `preview` for a dry run.
  pub state: Option<String>,
}

/// Whether a removal is only previewed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DryRunQuery {
  /// Answer what would go, and let go of nothing.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub dry_run: Option<bool>,
}
