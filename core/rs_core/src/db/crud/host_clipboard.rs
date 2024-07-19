use sea_orm::ActiveValue::Set;
use sea_orm::*;

use crate::db::entities::host_clipboard::{self, Entity as ClipboardEntries};
use crate::schema::clipboard::PasteboardContent;

pub async fn add_clipboard_entry(
    db: &DatabaseConnection,
    item: PasteboardContent,
) -> Result<host_clipboard::Model, DbErr> {
    let new_entry = host_clipboard::ActiveModel {
        r#type: Set(item.content_type.to_i32()),
        path: Set(item.path),
        content: Set(item.text_content),
        timestamp: Set(item.date_time.timestamp()),
        uuid: Set(item.uuid.clone()),
        ..Default::default()
    };

    let res = ClipboardEntries::insert(new_entry).exec(db).await?;
    ClipboardEntries::find_by_id(res.last_insert_id)
        .one(db)
        .await?
        .ok_or(DbErr::Custom(
            "Failed to retrieve inserted entry".to_string(),
        ))
}

// pub async fn get_clipboard_entries(
//     db: &DatabaseConnection,
// ) -> Result<Vec<host_clipboard::Model>, DbErr> {
//     ClipboardEntries::find().all(db).await
// }
//
// pub async fn get_clipboard_entries_by_num(
//     db: &DatabaseConnection,
//     num: Option<u64>,
// ) -> Result<Vec<host_clipboard::Model>, DbErr> {
//     ClipboardEntries::find()
//         .order_by_desc(host_clipboard::Column::Id)
//         .limit(num)
//         .all(db)
//         .await
// }

pub async fn get_num_clipboards_by_timestamp_and_type(
    db: &DatabaseConnection,
    num: Option<u64>,
    type_int: Option<i32>,
) -> Result<Vec<host_clipboard::Model>, DbErr> {
    let query = host_clipboard::Entity::find()
        .order_by_desc(host_clipboard::Column::Timestamp)
        .limit(num);
    if let Some(type_int) = type_int {
        query
            .filter(host_clipboard::Column::Type.eq(type_int))
            .all(db)
            .await
    } else {
        query.all(db).await
    }
}

pub async fn get_num_clipboards_by_timestamp_and_type_list(
    db: &DatabaseConnection,
    num: Option<u64>,
    type_list: Option<Vec<i32>>,
) -> Result<Vec<host_clipboard::Model>, DbErr> {
    let query = host_clipboard::Entity::find()
        .order_by_desc(host_clipboard::Column::Timestamp)
        .limit(num);
    if let Some(type_list) = type_list {
        query
            .filter(host_clipboard::Column::Type.is_in(type_list))
            .all(db)
            .await
    } else {
        query.all(db).await
    }
}

pub async fn get_clipboard_entries_by_gt_timestamp(
    db: &DatabaseConnection,
    timestamp: i64,
) -> Result<Vec<host_clipboard::Model>, DbErr> {
    let query = host_clipboard::Entity::find()
        .filter(host_clipboard::Column::Timestamp.gt(timestamp))
        .order_by_asc(host_clipboard::Column::Timestamp);

    query.all(db).await
}

pub async fn get_clipboard_entries_by_id_list(
    db: &DatabaseConnection,
    id_list: Option<Vec<i32>>,
) -> Result<Vec<host_clipboard::Model>, DbErr> {
    match id_list {
        Some(ids) if !ids.is_empty() => {
            host_clipboard::Entity::find()
                .filter(host_clipboard::Column::Id.is_in(ids))
                .order_by_desc(host_clipboard::Column::Timestamp)
                .all(db)
                .await
        }
        _ => Ok(vec![]),
    }
}

// pub async fn update_clipboard_entry(
//     db: &DatabaseConnection,
//     id: i32,
//     content: String,
// ) -> Result<host_clipboard::Model, DbErr> {
//     let entry = ClipboardEntries::find_by_id(id).one(db).await?;
//     if let Some(entry) = entry {
//         let mut entry: clipboard::ActiveModel = entry.into();
//         entry.content = Set(content);
//         entry.update(db).await
//     } else {
//         Err(DbErr::Custom("Entry not found".to_string()))
//     }
// }

pub async fn delete_clipboard_entry(
    db: &DatabaseConnection,
    id: i32,
) -> Result<DeleteResult, DbErr> {
    ClipboardEntries::delete_by_id(id).exec(db).await
}
