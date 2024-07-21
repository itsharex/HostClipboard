//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.15

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "host_clipboard")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub r#type: i32,
    pub path: String,
    pub content: String,
    pub timestamp: i64,
    pub uuid: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
