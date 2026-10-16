//! A document: the search, import, record, learning material or page it was filed as, its state and
//! its timing.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "searches")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub id: i64,
  pub chat_id: i64,
  pub query: String,
  pub started_depth: String,
  pub ended_depth: Option<String>,
  pub status: String,
  pub started_at: i64,
  pub finished_at: Option<i64>,
  pub document: Option<String>,
  pub asker_id: Option<i64>,
  pub options: Option<String>,
  pub title: Option<String>,
  pub topic: Option<String>,
  pub keywords: Option<String>,
  pub rule: Option<String>,
  pub documented: Option<i64>,
  pub kind: Option<String>,
  pub author: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
