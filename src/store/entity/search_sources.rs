//! One source of a document, and the finding it supports where it supports one.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "search_sources")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub id: i64,
  pub search_id: i64,
  pub finding_id: Option<i64>,
  pub url: String,
  pub title: Option<String>,
  pub supports: Option<String>,
  pub mark: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
