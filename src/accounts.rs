//! The account service, reached through bmo-web's published client (`docs/api.md`, "What this service calls"):
//! the units a reader stands in or under, the company a user or a unit belongs to, and a company's tree. Nothing it
//! answers is kept, so a move or a new unit counts from the next call on.

use bmo_web_client::{AccountClient, ClientFailure};

use crate::Refusal;
use crate::settings;
use crate::wire::CompanyUnit;

/// The account service the settings name: none where they name no address, or why the address does not read.
#[derive(Clone, Debug)]
pub struct Accounts {
  client: Result<Option<AccountClient>, String>,
}

/// The status the account service answers a user or a unit that does not stand with.
const NOT_FOUND: u16 = 404;

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
