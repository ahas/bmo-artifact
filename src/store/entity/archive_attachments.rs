//! One file a document keeps beside it, once for each path in its folder.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "archive_attachments")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub id: i64,
  pub search_id: i64,
  pub media: String,
  pub url: Option<String>,
  pub path: String,
  pub r#type: String,
  pub bytes: i64,
  pub name: String,
  pub place: Option<i64>,
  pub kept_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
