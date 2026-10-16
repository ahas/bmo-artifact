//! One picture a document keeps, once for each address it was fetched from.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "search_images")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub id: i64,
  pub search_id: i64,
  pub url: String,
  pub path: String,
  pub r#type: String,
  pub bytes: i64,
  pub kept_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
