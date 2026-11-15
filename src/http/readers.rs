//! Who a call reads for, and which documents that reader reaches (`docs/api.md`, "Readers"; `docs/artifact.md`,
//! "Owners"): a reader of every document, or an account reader, which reaches wholly the documents it owns, and
//! the administrator those with no owner, and reads alone the documents shared with its account or with a unit
//! it stands in or under, and the administrator every document another account owns.

use bmo_console_face::{OWNER_KEY_HEADER, OWNER_ROLE_HEADER};

use super::documents::{self, document};
use crate::Refusal;
use crate::plumbing::http::{Call, Credential, Refused, Standing};
use crate::store::documents as rows;
use crate::store::reach::{self, Account, Holding};
use crate::wire::Permission;

/// The role marking the platform administrator, who keeps the documents with no owner and reads every other.
const ADMINISTRATOR: &str = "administrator";

/// Who a call reads for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reader {
  /// A caller holding the service's token that names no account.
  Every,
  /// The account a call names in its owner key: the console relaying for any reader, or a caller holding the
  /// service's token acting for one.
  Account(AccountReader),
}

/// An account a call reads for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountReader {
  /// Its account key, empty where the console's call names none.
  pub key: String,
  /// Whether the call names it the platform administrator.
  pub administrator: bool,
}

/// How far a reader reaches one document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reach {
  /// Wholly: it reads, changes and shares it.
  Whole,
  /// To read alone, under a permission: through a share, or as the administrator reading a document another account
  /// owns.
  Shared(Permission),
}

impl Reach {
  /// The permission a document is answered with for a reader of this reach: none where it reaches it wholly.
  pub fn permission(self) -> Option<Permission> {
    match self {
      Self::Whole => None,
      Self::Shared(permission) => Some(permission),
    }
  }
}

/// The units an account reads documents through, as the account service answers them for one call.
pub struct Listing {
  key: String,
  administrator: bool,
  units: Vec<i64>,
}

impl Listing {
  /// The account as the document tables are read for it.
  pub fn account(&self) -> Account<'_> {
    Account {
      key: &self.key,
      administrator: self.administrator,
      units: &self.units,
    }
  }
}

impl Reader {
  /// The reader a call reads for. A caller holding the service's token reads every document unless it names an
  /// account; the console always reads for the account its owner key names.
  pub fn of(call: &Call) -> Self {
    let key = call.header(OWNER_KEY_HEADER).filter(|key| !key.is_empty());
    let console = call.request.extensions().get::<Credential>() == Some(&Credential::Console);

    if !console && key.is_none() {
      return Self::Every;
    }

    Self::Account(AccountReader {
      key: key.unwrap_or_default(),
      administrator: call.header(OWNER_ROLE_HEADER).as_deref() == Some(ADMINISTRATOR),
    })
  }

  /// The account a document this reader files is owned by; none for a reader of every document.
  pub fn owner(&self) -> Option<&str> {
    match self {
      Self::Every => None,
      Self::Account(account) => Some(account.key.as_str()).filter(|key| !key.is_empty()),
    }
  }

  /// Refused where the reader is an account, on a route that reaches every document.
  pub fn every(&self) -> Result<(), Refused> {
    match self {
      Self::Every => Ok(()),
      Self::Account(_) => {
        Err(Refusal::forbidden("only a reader of every document reaches this route").into())
      }
    }
  }

  /// What a list is read for: none for a reader of every document, and for an account the units the account
  /// service answers it stands in or under; the administrator, who reads every document, is asked none.
  pub async fn listing(&self, standing: &Standing) -> Result<Option<Listing>, Refused> {
    let Self::Account(account) = self else {
      return Ok(None);
    };
    let units = if account.administrator {
      Vec::new()
    } else {
      standing.accounts.units_of(&account.key).await?
    };

    Ok(Some(Listing {
      key: account.key.clone(),
      administrator: account.administrator,
      units,
    }))
  }

  /// How far this reader reaches one document; a document it does not read is refused as one that does not
  /// stand. The account service is asked the reader's units only where a unit share could decide it, never for
  /// the administrator, who reads every document.
  pub async fn reach(&self, standing: &Standing, id: i64) -> Result<Reach, Refused> {
    let key = match self {
      Self::Every => "",
      Self::Account(account) => account.key.as_str(),
    };
    let Holding {
      owner,
      member,
      units,
    } = reach::holding(standing.db().await?, id, key)
      .await?
      .ok_or_else(|| documents::absent(id))?;
    let Self::Account(account) = self else {
      return Ok(Reach::Whole);
    };

    match owner {
      Some(owner) if !key.is_empty() && owner == key => return Ok(Reach::Whole),
      None if account.administrator => return Ok(Reach::Whole),
      _ => {}
    }

    if let Some(permission) = member {
      return Ok(Reach::Shared(permission));
    }

    if account.administrator {
      return Ok(Reach::Shared(Permission::Read));
    }

    if !units.is_empty() {
      let standing_in = standing.accounts.units_of(key).await?;

      if units.iter().any(|unit| standing_in.contains(unit)) {
        return Ok(Reach::Shared(Permission::Read));
      }
    }

    Err(documents::absent(id))
  }

  /// Refused where this reader does not reach one document wholly: one it reads alone is forbidden, one it does
  /// not read refused as one that does not stand.
  pub async fn keeps(&self, standing: &Standing, id: i64) -> Result<(), Refused> {
    match self.reach(standing, id).await? {
      Reach::Whole => Ok(()),
      Reach::Shared(_) => Err(
        Refusal::forbidden(format!(
          "this reader reads document {id} alone; only its owner changes or shares it"
        ))
        .into(),
      ),
    }
  }

  /// The document as this reader reads it, with the permission it reads it under where it reads it alone; a
  /// document it does not read reads to it as one that does not stand.
  pub async fn document(&self, standing: &Standing, id: i64) -> Result<rows::Row, Refused> {
    let reach = self.reach(standing, id).await?;
    let mut row = document(standing, id).await?;

    row.document.permission = reach.permission();

    Ok(row)
  }
}

/// The document the path's `documentId` names, as the call's reader reaches it.
pub async fn reached(call: &mut Call) -> Result<i64, Refused> {
  let id = call.path_id("documentId").await?;

  Reader::of(call).reach(call.standing(), id).await?;

  Ok(id)
}

/// The document the path's `documentId` names, which the call's reader reaches wholly.
pub async fn kept(call: &mut Call) -> Result<i64, Refused> {
  let id = call.path_id("documentId").await?;

  Reader::of(call).keeps(call.standing(), id).await?;

  Ok(id)
}
