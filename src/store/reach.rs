//! Which documents an account reads, as conditions on a document's row and as what one document holds for it
//! (`docs/artifact.md`, "Owners"; `docs/api.md`, "Readers"): the documents it owns, those with no owner for the
//! administrator, and those shared with its account or with a unit it stands in or under.

use sea_orm::ConnectionTrait;

use super::documents::LEFT_A_DOCUMENT;
use super::member_shares::{permission_column, permission_of, shared_with};
use super::{all, one};
use crate::plumbing::refusal::Refusal;
use crate::wire::Permission;

/// An account as the document tables are read for it.
#[derive(Clone, Copy, Debug)]
pub struct Account<'a> {
  /// Its account key, empty where the call names none.
  pub key: &'a str,
  /// Whether it is the platform administrator, who keeps the documents with no owner.
  pub administrator: bool,
  /// The units it stands in or under, as the account service answered them for this call.
  pub units: &'a [i64],
}

impl Account<'_> {
  /// The condition a document's row, `d`, meets where this account reaches it wholly: it owns it, or it is the
  /// administrator and the document has no owner. `at` is the placeholder binding the account key.
  fn whole(&self, at: &str) -> String {
    let unowned = if self.administrator { r#" OR d."owner" IS NULL"# } else { "" };

    format!(r#"(d."owner" = {at}{unowned})"#)
  }

  /// The condition a document's row, `d`, meets where it is shared with a unit this account stands in or under.
  /// The units are whole numbers the account service answered, so they are written into the statement.
  fn through_unit(&self) -> String {
    if self.units.is_empty() {
      return "FALSE".to_owned();
    }

    let units = self.units.iter().map(i64::to_string).collect::<Vec<_>>().join(", ");

    format!(
      r#"EXISTS (SELECT 1 FROM "artifact_unit_shares" u WHERE u."search_id" = d."id" AND u."unit_id" IN ({units}))"#
    )
  }

  /// The condition a document's row, `d`, meets where this account reads it.
  pub fn reads(&self, at: &str) -> String {
    format!("({} OR {} OR {})", self.whole(at), shared_with(at), self.through_unit())
  }

  /// The permission this account reads a document's row, `d`, under, as a column read beside it: null where it
  /// reaches the document wholly, `read` where it reads it through a unit alone.
  pub fn permission(&self, at: &str) -> String {
    format!(
      "CASE WHEN {} THEN NULL ELSE COALESCE({}, CASE WHEN {} THEN 'read' END) END",
      self.whole(at),
      permission_column(at),
      self.through_unit()
    )
  }
}

/// What one document holds that decides who reads it: its owner, the permission one account is shared it under,
/// and the units it is shared with.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Holding {
  /// The account that owns it, none for a document with no owner.
  pub owner: Option<String>,
  /// The permission the account asked for holds through a member share, none where it holds none.
  pub member: Option<Permission>,
  /// The units it is shared with.
  pub units: Vec<i64>,
}

/// What one document holds for the account `key`; none where the document does not stand or its row left no
/// document.
pub async fn holding(db: &impl ConnectionTrait, id: i64, key: &str) -> Result<Option<Holding>, Refusal> {
  let Some(row) = one(
    db,
    &format!(
      r#"SELECT d."owner", {} AS "permission" FROM "searches" d WHERE d."id" = $1 AND {LEFT_A_DOCUMENT}"#,
      permission_column("$2")
    ),
    vec![id.into(), key.into()],
  )
  .await?
  else {
    return Ok(None);
  };
  let units = all(
    db,
    r#"SELECT "unit_id" FROM "artifact_unit_shares" WHERE "search_id" = $1 ORDER BY "id""#,
    vec![id.into()],
  )
  .await?
  .iter()
  .map(|unit| unit.try_get::<i64>("", "unit_id"))
  .collect::<Result<Vec<_>, _>>()?;

  Ok(Some(Holding {
    owner: row.try_get("", "owner")?,
    member: permission_of(row.try_get("", "permission")?),
    units,
  }))
}
