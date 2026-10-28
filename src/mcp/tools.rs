//! The harness tools (`docs/api.md`, "The harness plugin"), each a thin call of one method of the service's
//! own client, registered beside its call.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::tool::{ArtifactTool, ToolFailure};
use crate::kinds::Kind;
use crate::plumbing::http::BODY_LIMIT;
use crate::wire::{
  Attachment, AttachmentInput, ContentQuery, Document, DocumentChange, DocumentInput, DocumentPage,
  DocumentsQuery, Import, ImportInput, Labels, Publication, Share, ShareInput, SharePage, SharesQuery,
};

/// One document, by its number.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocumentKey {
  pub document_id: i64,
}

/// A document to file.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileInput {
  pub body: DocumentInput,
}

/// A document to publish: what it is called, what it reads as, and its words, given or read off this machine.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublishInput {
  /// The document's title.
  pub title: String,
  /// What it reads as before it is opened, in a line or two.
  #[serde(default)]
  pub description: Option<String>,
  /// Its words as markdown, for a record or a learning book; the page's HTML for a page.
  #[serde(default)]
  pub body: Option<String>,
  /// A file on this machine whose whole text is the body, or the HTML of a page, given instead of `body`.
  #[serde(default)]
  pub path: Option<String>,
  /// The kind it is filed as: `record` where none is named, `learning` or `page` beside it.
  #[serde(default)]
  pub kind: Option<String>,
  /// What it is held under, where the caller keeps one.
  #[serde(default)]
  pub scope: Option<String>,
  /// Who files it.
  pub actor_name: String,
}

/// A document rewritten under its number.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RewriteInput {
  pub document_id: i64,
  #[serde(flatten)]
  pub change: DocumentChange,
}

/// A document's file, in base64.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContentOutput {
  /// The file's bytes in base64.
  pub content: String,
  pub media_type: String,
}

/// A file attached to a document.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttachInput {
  pub document_id: i64,
  #[serde(flatten)]
  pub attachment: AttachmentInput,
}

/// One share link, by its token.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TokenInput {
  pub token: String,
}

/// Nothing: the route answered 204.
#[derive(Debug, Serialize, JsonSchema)]
pub struct Nothing {}

/// Documents searched by their words.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchInput {
  /// The phrase, matched whole and without regard to case in the title, the labels and the body.
  pub q: String,
  #[serde(default)]
  pub kind: Option<String>,
  #[serde(default)]
  pub scope: Option<String>,
}

fn documents() -> ArtifactTool {
  ArtifactTool::new(
    "artifact_documents",
    "Documents by kind, scope, label and time; q searches their words.",
    "GET /documents",
    |client, query: DocumentsQuery| async move {
      Ok::<DocumentPage, ToolFailure>(client.documents(&query).await?)
    },
  )
}

crate::tool!(documents);

fn file() -> ArtifactTool {
  ArtifactTool::new(
    "artifact_file",
    "File a document: a record, a learning book, an import or a page.",
    "POST /documents",
    |client, input: FileInput| async move { Ok::<Document, ToolFailure>(client.file(&input.body).await?) },
  )
}

crate::tool!(file);

// The whole of a file this machine holds, as the text a document is filed with: refused where it cannot be
// read, is not text, or is over what one call may carry.
fn read_text(path: &str) -> Result<String, ToolFailure> {
  let bytes = std::fs::read(path)
    .map_err(|why| ToolFailure::Arguments(format!("the file {path} could not be read: {why}")))?;

  if bytes.len() > BODY_LIMIT {
    return Err(ToolFailure::Arguments(format!(
      "the file {path} weighs {} bytes, over the {BODY_LIMIT} one call may carry",
      bytes.len()
    )));
  }

  String::from_utf8(bytes)
    .map_err(|_| ToolFailure::Arguments(format!("the file {path} is not text this service can keep")))
}

// The document a publish call files: its words as given, else the whole of the file it names, laid in the body
// or the HTML its kind is filed with.
fn publish_input(input: PublishInput) -> Result<DocumentInput, ToolFailure> {
  let words = match (input.body, input.path) {
    (Some(body), None) => body,
    (None, Some(path)) => read_text(&path)?,
    (Some(_), Some(_)) => {
      return Err(ToolFailure::Arguments(
        "give the document's words as `body` or as the `path` of a file, not both".to_owned(),
      ));
    }
    (None, None) => {
      return Err(ToolFailure::Arguments(
        "give the document's words as `body` or as the `path` of a file".to_owned(),
      ));
    }
  };

  let kind = input.kind.unwrap_or_else(|| crate::kinds::RecordKind.name().to_owned());
  let (body, html) = if crate::kinds::kind(Some(&kind)).hosted() {
    (None, Some(words))
  } else {
    (Some(words), None)
  };

  Ok(DocumentInput {
    kind,
    title: input.title,
    body,
    html,
    scope: input.scope,
    actor_name: input.actor_name,
    labels: Labels::default(),
    description: input.description,
  })
}

fn publish() -> ArtifactTool {
  ArtifactTool::new(
    "artifact_publish",
    "Publish a document as an artifact: file it and answer with a permanent share link to hand to a reader. \
     Takes its title, a short description, and its words as markdown `body` or as the `path` of a file on this \
     machine. The link has no end and opens with no console login; it keeps opening across restarts and \
     redeployments, and it names the document rather than a body, so a rewrite keeps the same link serving the \
     newest body.",
    "POST /publications",
    |client, input: PublishInput| async move {
      Ok::<Publication, ToolFailure>(client.publish(&publish_input(input)?).await?)
    },
  )
}

crate::tool!(publish);

fn document() -> ArtifactTool {
  ArtifactTool::new(
    "artifact_document",
    "One document and its head.",
    "GET /documents/{documentId}",
    |client, key: DocumentKey| async move {
      Ok::<Document, ToolFailure>(client.document(key.document_id).await?)
    },
  )
}

crate::tool!(document);

fn rewrite() -> ArtifactTool {
  ArtifactTool::new(
    "artifact_rewrite",
    "Rewrite a document under its number; its share links stay.",
    "PATCH /documents/{documentId}",
    |client, input: RewriteInput| async move {
      Ok::<Document, ToolFailure>(client.rewrite(input.document_id, &input.change).await?)
    },
  )
}

crate::tool!(rewrite);

fn content() -> ArtifactTool {
  ArtifactTool::new(
    "artifact_content",
    "The document's body as filed.",
    "GET /documents/{documentId}/content",
    |client, key: DocumentKey| async move {
      let asked = ContentQuery {
        scheme: None,
        download: Some(true),
      };
      let content = client.content(key.document_id, &asked).await?;

      Ok::<ContentOutput, ToolFailure>(ContentOutput {
        content: STANDARD.encode(content.bytes),
        media_type: content.media_type,
      })
    },
  )
}

crate::tool!(content);

fn attach() -> ArtifactTool {
  ArtifactTool::new(
    "artifact_attach",
    "Attach a file to a document, a sound to be heard at its head among them.",
    "POST /documents/{documentId}/attachments",
    |client, input: AttachInput| async move {
      Ok::<Attachment, ToolFailure>(client.attach(input.document_id, &input.attachment).await?)
    },
  )
}

crate::tool!(attach);

fn shares() -> ArtifactTool {
  ArtifactTool::new(
    "artifact_shares",
    "Share links, by document.",
    "GET /shares",
    |client, query: SharesQuery| async move { Ok::<SharePage, ToolFailure>(client.shares(&query).await?) },
  )
}

crate::tool!(shares);

fn share() -> ArtifactTool {
  ArtifactTool::new(
    "artifact_share",
    "Make a share link that ends at a time, or never; the standing link is answered when one is open.",
    "POST /shares",
    |client, input: ShareInput| async move { Ok::<Share, ToolFailure>(client.share(&input).await?) },
  )
}

crate::tool!(share);

fn unshare() -> ArtifactTool {
  ArtifactTool::new(
    "artifact_unshare",
    "End a share link.",
    "DELETE /shares/{token}",
    |client, input: TokenInput| async move {
      client.unshare(&input.token).await?;

      Ok::<Nothing, ToolFailure>(Nothing {})
    },
  )
}

crate::tool!(unshare);

fn import() -> ArtifactTool {
  ArtifactTool::new(
    "artifact_import",
    "Import a page: fetched through the explorer client and filed as a document.",
    "POST /imports",
    |client, input: ImportInput| async move { Ok::<Import, ToolFailure>(client.import(&input).await?) },
  )
}

crate::tool!(import);

fn search() -> ArtifactTool {
  ArtifactTool::new(
    "artifact_search",
    "Search documents by words, newest first.",
    "GET /documents",
    |client, input: SearchInput| async move {
      let query = DocumentsQuery {
        q: Some(input.q),
        kind: input.kind,
        scope: input.scope,
        ..DocumentsQuery::default()
      };

      Ok::<DocumentPage, ToolFailure>(client.documents(&query).await?)
    },
  )
}

crate::tool!(search);
