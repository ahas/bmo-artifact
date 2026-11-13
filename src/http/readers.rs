//! Who a call reads for, and which documents that reader reaches (`docs/api.md`, "Readers"): a reader of every
//! document, or a member reader held to the documents shared with their own account, to read them alone.

use bmo_console_face::{OWNER_KEY_HEADER, OWNER_ROLE_HEADER};

use super::documents::{self, document};
use crate::Refusal;
use crate::plumbing::http::{Call, Credential, Refused, Standing};
use crate::store::{documents as rows, member_shares};
use crate::wire::Permission;

/// The role whose reader the console relays for reads every document.
const ADMINISTRATOR: &str = "administrator";

/// Who a call reads for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reader {
  /// A caller holding the service's token, or the console relaying for the administrator.
  Every,
  /// The console relaying for any other reader: the account its owner key names, empty where it names none.
  Member(String),
}

impl Reader {
  /// The reader a call reads for. The console's headers are its word alone, so they are read only on a call
  /// carrying its token.
  pub fn of(call: &Call) -> Self {
    if call.request.extensions().get::<Credential>() != Some(&Credential::Console)
      || call.header(OWNER_ROLE_HEADER).as_deref() == Some(ADMINISTRATOR)
    {
      return Self::Every;
    }

    Self::Member(call.header(OWNER_KEY_HEADER).unwrap_or_default())
  }

  /// The account a member reader reads for; none for a reader of every document.
  pub fn member(&self) -> Option<&str> {
    match self {
      Self::Every => None,
      Self::Member(user) => Some(user),
    }
  }

  /// Refused where the reader is a member, who reads and never changes how a document is shared.
  pub fn shares(&self) -> Result<(), Refused> {
    match self {
      Self::Every => Ok(()),
      Self::Member(_) => {
        Err(Refusal::forbidden("a document is shared only by a reader of every document").into())
      }
    }
  }

  /// The document as this reader reads it, with the permission a member reads it under; a document not shared
  /// with a member reads to them as one that does not stand.
  pub async fn document(&self, standing: &Standing, id: i64) -> Result<rows::Row, Refused> {
    let permission = self.reaches(standing, id).await?;
    let mut row = document(standing, id).await?;

    row.document.permission = permission;

    Ok(row)
  }

  /// Whether this reader reaches one document: the permission a member reads it under, none for a reader of
  /// every document; a document not shared with a member is refused as one that does not stand.
  pub async fn reaches(&self, standing: &Standing, id: i64) -> Result<Option<Permission>, Refused> {
    let Some(user) = self.member() else {
      return Ok(None);
    };
    let held = match user {
      "" => None,
      user => member_shares::permission(standing.db().await?, id, user).await?,
    };

    held.map(Some).ok_or_else(|| documents::absent(id))
  }
}

/// The document the path's `documentId` names, as the call's reader reaches it.
pub async fn reached(call: &mut Call) -> Result<i64, Refused> {
  let id = call.path_id("documentId").await?;

  Reader::of(call).reaches(call.standing(), id).await?;

  Ok(id)
}
