//! A document's one share link: its token, who made it and when, and when it ends where it does.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "web_search_shares")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub share_token: String,
  #[sea_orm(unique)]
  pub search_id: i64,
  pub created_by: String,
  pub created_at: i64,
  pub expires_at: Option<i64>,
  pub user_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
