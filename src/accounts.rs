//! The account service, reached through bmo-web's published client (`docs/api.md`, "What this service calls"):
//! the units a reader stands in or under, the company a user or a unit belongs to, a company's tree, a user's
//! login, and the storage setting of the delegation chain, declared, resolved and written. Nothing it answers is
//! kept, so a move, a new unit or a changed setting counts from the next call on.

use axum::http::StatusCode;
use bmo_web_client::{AccountClient, ChainEntry, ChainResolveAsk, ChainValueAsk, ClientFailure};
use serde_json::Value;

use crate::Refusal;
use crate::plumbing::http::Refused;
use crate::settings;
use crate::wire::CompanyUnit;

/// The account service the settings name: none where they name no address, or why the address does not read.
#[derive(Clone, Debug)]
pub struct Accounts {
  client: Result<Option<AccountClient>, String>,
}

/// The status the account service answers a user or a unit that does not stand with.
const NOT_FOUND: u16 = 404;

/// The code the account service answers a kind nobody declared with.
const NO_SUCH_KIND: &str = "no-such-kind";

/// The unit name this service declares its settings and writes their values under.
pub const DECLARED_BY: &str = "artifact";

/// What the storage setting resolved to for an owner: the value naming the place, none where nothing says one,
/// and the company the owner stands in, none where they stand in none.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Storage {
  pub value: Option<Value>,
  pub company: Option<i64>,
}

fn failed(why: ClientFailure) -> Refusal {
  Refusal::upstream(why.to_string())
}

impl Accounts {
  /// The service the settings name.
  pub fn of(settings: &settings::Accounts) -> Self {
    let address = settings.address.trim();
    let client = match address {
      "" => Ok(None),
      address => AccountClient::new(address, settings.token.clone()).map(Some),
    };

    Self { client }
  }

  // The service named, refused where none is or its address does not read.
  fn client(&self) -> Result<&AccountClient, Refusal> {
    match &self.client {
      Ok(Some(client)) => Ok(client),
      Ok(None) => Err(Refusal::upstream("this service names no account service")),
      Err(why) => Err(Refusal::upstream(format!("the account service's address does not read: {why}"))),
    }
  }

  /// The units `user` stands in or under: its own unit and every unit above it up to its company. None where no
  /// account service is named, where the call names no user, or where the account service knows no such user.
  pub async fn units_of(&self, user: &str) -> Result<Vec<i64>, Refusal> {
    if user.is_empty() || matches!(self.client, Ok(None)) {
      return Ok(Vec::new());
    }

    match self.client()?.user_unit(user).await {
      Ok(standing) => Ok(standing.path.iter().map(|unit| unit.id).collect()),
      Err(ClientFailure::Answered {
        status: NOT_FOUND, ..
      }) => Ok(Vec::new()),
      Err(why) => Err(failed(why)),
    }
  }

  /// The company `user` belongs to; none where the account service knows no such user. Refused where no account
  /// service is named.
  pub async fn company_of_user(&self, user: &str) -> Result<Option<i64>, Refusal> {
    match self.client()?.user_unit(user).await {
      Ok(standing) => Ok(Some(standing.company.id)),
      Err(ClientFailure::Answered {
        status: NOT_FOUND, ..
      }) => Ok(None),
      Err(why) => Err(failed(why)),
    }
  }

  /// The company `unit` belongs to, itself for a company; none where the account service knows no such unit.
  /// Refused where no account service is named.
  pub async fn company_of_unit(&self, unit: i64) -> Result<Option<i64>, Refusal> {
    match self.client()?.unit(unit).await {
      Ok(standing) => Ok(Some(standing.company.id)),
      Err(ClientFailure::Answered {
        status: NOT_FOUND, ..
      }) => Ok(None),
      Err(why) => Err(failed(why)),
    }
  }

  /// The login of `user`, none where the account service knows no such user. Refused where no account service is
  /// named.
  pub async fn login_of(&self, user: &str) -> Result<Option<String>, Refusal> {
    match self.client()?.user(user).await {
      Ok(view) => Ok(Some(view.name)),
      Err(ClientFailure::Answered {
        status: NOT_FOUND, ..
      }) => Ok(None),
      Err(why) => Err(failed(why)),
    }
  }

  /// The storage setting declared on the delegation chain (`docs/artifact.md`, "The setting"); nothing is asked
  /// where no account service is named.
  pub async fn declare_storage(&self) -> Result<(), Refusal> {
    if matches!(self.client, Ok(None)) {
      return Ok(());
    }

    self
      .client()?
      .declare_chain_kind(&crate::places::setting())
      .await
      .map(|_| ())
      .map_err(failed)
  }

  /// The storage setting resolved for `user`, the setting declared again and asked once more where it stood
  /// undeclared. Nothing is said where no account service is named, where `user` does not stand, and where the
  /// setting stays undeclared.
  pub async fn storage_of(&self, user: &str) -> Result<Storage, Refusal> {
    if matches!(self.client, Ok(None)) {
      return Ok(Storage::default());
    }

    let client = self.client()?;
    let ask = ChainResolveAsk {
      person: Some(user.to_owned()),
      ..ChainResolveAsk::default()
    };
    let key = crate::places::SETTING;
    let resolved = match client.resolve_chain(key, &ask).await {
      Err(ClientFailure::Answered {
        status: NOT_FOUND,
        answer: Some(answer),
      }) if answer.code == NO_SUCH_KIND => {
        self.declare_storage().await?;
        client.resolve_chain(key, &ask).await
      }
      other => other,
    };

    match resolved {
      Ok(resolved) => Ok(Storage {
        value: resolved.value,
        company: resolved.path.iter().find_map(|link| link.strip_prefix("unit:")?.parse().ok()),
      }),
      Err(ClientFailure::Answered {
        status: NOT_FOUND, ..
      }) => Ok(Storage::default()),
      Err(why) => Err(failed(why)),
    }
  }

  /// A link's value of the storage setting written in the name of `owner`, as the unit that declared it, which
  /// judged the value; the link's entry as the account service answers it. A refusal of the chain's is answered
  /// with its own status and code.
  pub async fn write_storage(
    &self,
    link: &str,
    value: Option<Value>,
    owner: &str,
  ) -> Result<ChainEntry, Refused> {
    let ask = ChainValueAsk { value };

    match self
      .client()?
      .put_chain_value(crate::places::SETTING, link, &ask, owner, DECLARED_BY)
      .await
    {
      Ok(entry) => Ok(entry),
      Err(ClientFailure::Answered {
        status,
        answer: Some(answer),
      }) if (400..500).contains(&status) => Err(Refused::new(
        StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_REQUEST),
        answer.code,
        answer.error,
      )),
      Err(why) => Err(failed(why).into()),
    }
  }

  /// Every unit of `company`, itself first and each unit after its parent, walked down one read a unit; none where
  /// the account service knows no such unit. Refused where no account service is named.
  pub async fn tree_of(&self, company: i64) -> Result<Vec<CompanyUnit>, Refusal> {
    let client = self.client()?;
    let mut units = Vec::new();
    // The units still to read, the next on top, so each unit's children follow it in their own order.
    let mut waiting = vec![company];

    while let Some(id) = waiting.pop() {
      let unit = match client.unit(id).await {
        Ok(unit) => unit,
        Err(ClientFailure::Answered {
          status: NOT_FOUND, ..
        }) => continue,
        Err(why) => return Err(failed(why)),
      };

      waiting.extend(unit.children.iter().rev().map(|child| child.id));
      units.push(CompanyUnit {
        id: unit.id,
        name: unit.name,
        parent: unit.parent,
      });
    }

    Ok(units)
  }
}
