//! One finding of a document, in its place, with the run and the branch that wrote it.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "search_findings")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub id: i64,
  pub search_id: i64,
  pub run_id: Option<i64>,
  pub branch: Option<String>,
  pub place: i64,
  pub text: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
