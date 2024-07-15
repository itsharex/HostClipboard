use napi_derive::napi;
use sea_orm::Database;
use std::path::Path;
use std::sync::Arc;

use crate::apis::pasteboard::Pasteboard;
use crate::schema::clipboard::PasteboardContent;
use db::{connection::establish_connection, crud};
use tokio::sync::{mpsc, Mutex};

mod apis;
mod db;
mod schema;
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
    db: Arc<Mutex<Option<sea_orm::DatabaseConnection>>>,
    db_path: String,
    sender: mpsc::Sender<PasteboardContent>,
    receiver: Arc<Mutex<mpsc::Receiver<PasteboardContent>>>,
}

impl ClipboardHelper {
    async fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let db_path = Path::new(db_path);
        let db_url = format!("sqlite:{}", db_path.display());
        let pasteboard = Arc::new(Mutex::new(Pasteboard::new()));

        if !db_path.exists() {
            if let Some(parent) = db_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::File::create(db_path).await?;
            Database::connect(&db_url).await?;
        }

        let (sender, receiver) = mpsc::channel(100); // 创建一个容量为100的通道

        let helper = Self {
            pasteboard,
            db: Arc::new(Mutex::new(None)),
            db_path: db_url,
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
        };

        // 启动读取剪贴板的任务
        let pasteboard_clone = helper.pasteboard.clone();
        let sender_clone = helper.sender.clone();
        tokio::spawn(async move {
            loop {
                unsafe {
                    let content = {
                        let mut pasteboard = pasteboard_clone.lock().await;
                        pasteboard.get_contents()
                    };
                    for c in content {
                        if let Err(_) = sender_clone.send(c).await {
                            break;
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        });

        // 启动写入数据库的任务
        let db_clone = helper.db.clone();
        let receiver_clone = helper.receiver.clone();
        let db_path_clone = helper.db_path.clone();
        tokio::spawn(async move {
            loop {
                let mut receiver = receiver_clone.lock().await;
                if let Some(content) = receiver.recv().await {
                    let db = {
                        let mut db_guard = db_clone.lock().await;
                        if db_guard.is_none() {
                            *db_guard = Some(establish_connection(&db_path_clone).await.unwrap());
                        }
                        db_guard.as_ref().unwrap().clone()
                    };
                    if let Err(e) = crud::host_clipboard::add_clipboard_entry(&db, content).await {
                        eprintln!("Error adding clipboard entry: {}", e);
                    }
                }
            }
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

    async fn get_db(&self) -> Result<sea_orm::DatabaseConnection, Box<dyn std::error::Error>> {
        let mut db = self.db.lock().await;
        if db.is_none() {
            *db = Some(establish_connection(&self.db_path).await?);
        }
        Ok(db.as_ref().unwrap().clone())
    }

    async fn get_all_clipboard_entries(
        &self,
    ) -> Result<Vec<ClipboardEntry>, Box<dyn std::error::Error>> {
        let db = self.get_db().await?;
        let all_entries = crud::host_clipboard::get_clipboard_entries(&db).await?;

        Ok(all_entries
            .into_iter()
            .map(|entry| ClipboardEntry {
                id: entry.id,
                r#type: entry.r#type,
                path: entry.path,
                content: entry.content,
                timestamp: entry.timestamp,
                uuid: entry.uuid,
            })
            .collect())
    }

    async fn get_num_clipboard_entries(
        &self,
        num: i64,
    ) -> Result<Vec<ClipboardEntry>, Box<dyn std::error::Error>> {
        let db = self.get_db().await?;
        let opt_num = if num > 0 { Some(num as u64) } else { None };
        let all_entries = crud::host_clipboard::get_clipboard_entries_by_num(&db, opt_num).await?;

        Ok(all_entries
            .into_iter()
            .map(|entry| ClipboardEntry {
                id: entry.id,
                r#type: entry.r#type,
                path: entry.path,
                content: entry.content,
                timestamp: entry.timestamp,
                uuid: entry.uuid,
            })
            .collect())
    }
}

#[napi]
pub struct JsClipboardHelper(Arc<ClipboardHelper>);

#[napi]
impl JsClipboardHelper {
    #[napi(factory)]
    pub async fn new(db_path: String) -> Self {
        let helper = ClipboardHelper::new(&db_path)
            .await
            .expect("Failed to create ClipboardHelper");
        Self(Arc::new(helper))
    }

    #[napi]
    pub async fn get_all_clipboard_entries(&self) -> napi::Result<ClipboardList> {
        let entries = self
            .0
            .get_all_clipboard_entries()
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(ClipboardList { entries })
    }

    #[napi]
    pub async fn get_num_clipboard_entries(&self, num: i64) -> napi::Result<ClipboardList> {
        let entries = self
            .0
            .get_num_clipboard_entries(num)
            .await
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(ClipboardList { entries })
    }

    #[napi]
    pub async fn now_get_content(&self) -> napi::Result<()> {
        self.0.send_content().await;
        Ok(())
    }
}
