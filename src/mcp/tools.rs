//! The harness tools (`docs/api.md`, "The harness plugin"), each a thin call of one method of the service's
//! own client, registered beside its call.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::tool::{ArtifactTool, ToolFailure};
use crate::wire::{
  Attachment, AttachmentInput, ContentQuery, Document, DocumentChange, DocumentInput, DocumentPage,
  DocumentsQuery, Import, ImportInput, Share, ShareInput, SharePage, SharesQuery,
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
