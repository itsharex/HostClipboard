use log::debug;
use napi_derive::napi;
use sea_orm::Database;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use crate::apis::pasteboard::Pasteboard;
use crate::db::connection::init_db_connection;
use crate::schema::clipboard::PasteboardContent;
use crate::search_engine::indexer::ClipboardIndexer;
use crate::utils::logger;
use db::{crud, entities};

mod apis;
mod db;
mod schema;
mod search_engine;
mod utils;

#[napi(object)]
pub struct ClipboardEntry {
    pub id: i32,
    pub r#type: i32,
    pub path: String,
    pub content: String,
    pub timestamp: i64,
    pub uuid: String,
}

#[napi(object)]
pub struct ClipboardList {
    pub entries: Vec<ClipboardEntry>,
}

struct ClipboardHelper {
    pasteboard: Arc<Mutex<Pasteboard>>,
    db: Arc<Mutex<sea_orm::DatabaseConnection>>,
    sender: mpsc::Sender<PasteboardContent>,
    receiver: Arc<Mutex<mpsc::Receiver<PasteboardContent>>>,
    indexer: Arc<ClipboardIndexer>,
}

fn convert_to_clipboard_entry(entry: entities::host_clipboard::Model) -> ClipboardEntry {
    ClipboardEntry {
        id: entry.id,
        r#type: entry.r#type,
        path: entry.path,
        content: entry.content,
        timestamp: entry.timestamp,
        uuid: entry.uuid,
    }
}
impl ClipboardHelper {
    async fn new(
        log_level: Option<i32>,
        sql_level: Option<i32>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        logger::init_logger(log_level, sql_level);
        let db = init_db_connection(None).await?;
        let pasteboard = Arc::new(Mutex::new(Pasteboard::new()));
        let indexer = Arc::new(ClipboardIndexer::new(db.clone(), Some(3)).await);

        let (sender, receiver) = mpsc::channel(100);

        let helper = Self {
            pasteboard,
            db: Arc::new(Mutex::new(db)),
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
            indexer,
        };

        // 启动读取剪贴板的任务
        let pasteboard_clone = helper.pasteboard.clone();
        let sender_clone = helper.sender.clone();
        tokio::spawn(async move {
            loop {
                let content = {
                    let mut pasteboard = pasteboard_clone.lock().await;
                    unsafe { pasteboard.get_contents() }
                };
                for c in content {
                    if sender_clone.send(c).await.is_err() {
                        break;
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        });

        // 启动写入数据库的任务
        let db_clone = helper.db.clone();
        let receiver_clone = helper.receiver.clone();
        tokio::spawn(async move {
            loop {
                let mut receiver = receiver_clone.lock().await;
                if let Some(content) = receiver.recv().await {
                    let db = db_clone.lock().await;
                    if let Err(e) = crud::host_clipboard::add_clipboard_entry(&*db, content).await {
                        eprintln!("Error adding clipboard entry: {}", e);
                    }
                }
            }
        });

        let indexer_clone = helper.indexer.clone();
        tokio::spawn(async move {
            indexer_clone.start_background_update().await;
        });

        Ok(helper)
    }

    async fn send_content(&self) {
        let content = unsafe { self.pasteboard.lock().await.get_contents() };
        for c in content {
            if let Err(_) = self.sender.send(c).await {
                break;
            }
        }
    }

    async fn get_num_clipboard_by_type(
        &self,
        num: i64,
        type_int: Option<i32>,
    ) -> Result<Vec<ClipboardEntry>, Box<dyn std::error::Error>> {
        let db = self.db.lock().await;
        let opt_num = if num > 0 { Some(num as u64) } else { None };
        let all_entries =
            crud::host_clipboard::get_num_clipboards_by_timestamp_and_type(&db, opt_num, type_int)
                .await?;

        Ok(all_entries
            .into_iter()
            .map(convert_to_clipboard_entry)
            .collect())
    }

    async fn get_num_clipboard_by_type_list(
        &self,
        num: i64,
        type_list: Option<Vec<i32>>,
    ) -> Result<Vec<ClipboardEntry>, Box<dyn std::error::Error>> {
        let db = self.db.lock().await;
        let opt_num = if num > 0 { Some(num as u64) } else { None };
        let all_entries = crud::host_clipboard::get_num_clipboards_by_timestamp_and_type_list(
            &db, opt_num, type_list,
        )
        .await?;

        Ok(all_entries
            .into_iter()
            .map(convert_to_clipboard_entry)
            .collect())
    }

    async fn search_num_clipboard_by_type(
        &self,
        query: &str,
        num: usize,
        type_int: Option<i32>,
    ) -> Result<Vec<ClipboardEntry>, Box<dyn std::error::Error>> {
        let results =
            tokio_time_it!(|| async { self.indexer.search(query, num, type_int).await }).await;
        Ok(results
            .into_iter()
            .map(convert_to_clipboard_entry)
            .collect())
    }

    async fn search_num_clipboard_by_type_list(
        &self,
        query: &str,
        num: usize,
        type_list: Option<Vec<i32>>,
    ) -> Result<Vec<ClipboardEntry>, Box<dyn std::error::Error>> {
        let results = tokio_time_it!(|| async {
            self.indexer
                .search_by_type_list(query, num, type_list)
                .await
        })
        .await;
        Ok(results
            .into_iter()
            .map(convert_to_clipboard_entry)
            .collect())
    }

    async fn set_config() {
        todo!()
    }
    async fn get_config() {
        todo!()
    }

    async fn delete_by_id() {
        !todo!()
    }
}

impl ClipboardHelper {}
#[napi]
pub struct JsClipboardHelper(Arc<ClipboardHelper>);

#[napi]
impl JsClipboardHelper {
    #[napi(factory)]
    pub async fn new(
        #[napi(ts_arg_type = "number")] log_level: Option<i32>,
        #[napi(ts_arg_type = "number")] sql_level: Option<i32>,
    ) -> napi::Result<Self> {
        let helper = ClipboardHelper::new(log_level, sql_level)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(Self(Arc::new(helper)))
    }

    #[napi]
    pub async fn refresh_clipboard(&self) -> napi::Result<()> {
        self.0.send_content().await;
        Ok(())
    }

    #[napi]
    pub async fn get_clipboard_entries(
        &self,
        #[napi(ts_arg_type = "number")] num: i64,
        #[napi(ts_arg_type = "number | undefined")] type_int: Option<i32>,
    ) -> napi::Result<ClipboardList> {
        let entries = self
            .0
            .get_num_clipboard_by_type(num, type_int)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(ClipboardList { entries })
    }

    #[napi]
    pub async fn get_clipboard_entries_by_type_list(
        &self,
        #[napi(ts_arg_type = "number")] num: i64,
        #[napi(ts_arg_type = "number[] | undefined")] type_list: Option<Vec<i32>>,
    ) -> napi::Result<ClipboardList> {
        let entries = self
            .0
            .get_num_clipboard_by_type_list(num, type_list)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(ClipboardList { entries })
    }

    #[napi]
    pub async fn search_clipboard_entries(
        &self,
        query: String,
        #[napi(ts_arg_type = "number")] num: i64,
        #[napi(ts_arg_type = "number | undefined")] type_int: Option<i32>,
    ) -> napi::Result<ClipboardList> {
        let entries = self
            .0
            .search_num_clipboard_by_type(&query, num as usize, type_int)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(ClipboardList { entries })
    }

    #[napi]
    pub async fn search_clipboard_entries_by_type_list(
        &self,
        query: String,
        #[napi(ts_arg_type = "number")] num: i64,
        #[napi(ts_arg_type = "number[] | undefined")] type_list: Option<Vec<i32>>,
    ) -> napi::Result<ClipboardList> {
        let entries = self
            .0
            .search_num_clipboard_by_type_list(&query, num as usize, type_list)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(ClipboardList { entries })
    }


    #[napi]
    pub async fn update_config(&self) -> napi::Result<()> {
        // TODO: Implement get_config functionality
        Err(napi::Error::from_reason("Not implemented"))
    }

}
