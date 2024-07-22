use log::{debug, error};
use sea_orm::DatabaseConnection;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, Notify};

use crate::apis::pasteboard::Pasteboard;
use crate::db::connection::init_db_connection;
use crate::db::crud;
use crate::db::entities::host_clipboard::Model;
use crate::schema::clipboard::PasteboardContent;
use crate::search_engine::indexer::ClipboardIndexer;
use crate::utils::logger;
use crate::{time_it, tokio_time_it, utils};

pub struct ClipboardHelper {
    pasteboard: Arc<Mutex<Pasteboard>>,
    db: Arc<Mutex<Option<DatabaseConnection>>>,
    sender: mpsc::Sender<PasteboardContent>,
    receiver: Arc<Mutex<mpsc::Receiver<PasteboardContent>>>,
    indexer: Arc<Mutex<Option<ClipboardIndexer>>>,
    initialized: Arc<AtomicBool>,
    init_notifier: Arc<Notify>,
}

impl ClipboardHelper {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(100);
        Self {
            pasteboard: Arc::new(Mutex::new(Pasteboard::new())),
            db: Arc::new(Mutex::new(None)),
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
            indexer: Arc::new(Mutex::new(None)),
            initialized: Arc::new(AtomicBool::new(false)),
            init_notifier: Arc::new(Notify::new()),
        }
    }

    pub async fn init(
        &self,
        log_level: Option<i32>,
        sql_level: Option<i32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        logger::init_logger(log_level, sql_level);
        let db = init_db_connection(None).await?;
        *self.db.lock().await = Some(db.clone());

        let indexer = ClipboardIndexer::new(db, Some(3)).await;
        *self.indexer.lock().await = Some(indexer);

        // 启动读取剪贴板的任务
        let pasteboard_clone = self.pasteboard.clone();
        let sender_clone = self.sender.clone();
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
        let db_clone = self.db.clone();
        let receiver_clone = self.receiver.clone();
        tokio::spawn(async move {
            loop {
                let mut receiver = receiver_clone.lock().await;
                if let Some(content) = receiver.recv().await {
                    let db_guard = db_clone.lock().await;
                    let db = db_guard.as_ref().unwrap();
                    if let Err(e) = crud::host_clipboard::add_clipboard_entry(db, content).await {
                        eprintln!("Error adding clipboard entry: {}", e);
                    }
                }
            }
        });

        let indexer_clone = self.indexer.clone();
        tokio::spawn(async move {
            if let Some(indexer) = indexer_clone.lock().await.as_ref() {
                indexer.start_background_update().await;
            }
        });

        self.initialized.store(true, Ordering::SeqCst);
        self.init_notifier.notify_waiters();

        Ok(())
    }

    async fn ensure_initialized(&self) {
        if !self.initialized.load(Ordering::SeqCst) {
            self.init_notifier.notified().await;
        }
    }

    async fn send_content(&self) {
        let content = unsafe { self.pasteboard.lock().await.get_contents() };
        for c in content {
            if let Err(_) = self.sender.send(c).await {
                break;
            }
        }
    }

    async fn get_clipboards(
        &self,
        num: u64,
        type_list: Option<Vec<i32>>,
    ) -> Result<Vec<Model>, Box<dyn std::error::Error>> {
        self.ensure_initialized().await;
        let db_guard = self.db.lock().await;
        let db = db_guard.as_ref().unwrap();
        let ts = utils::time::get_current_timestamp() - 3 * 24 * 60 * 60;
        let all_entries = crud::host_clipboard::get_num_clipboards_by_timestamp_and_type_list(
            &db,
            Some(num),
            ts,
            type_list,
        )
        .await?;
        Ok(all_entries)
    }

    async fn search_clipboards(
        &self,
        query: &str,
        num: u64,
        type_list: Option<Vec<i32>>,
    ) -> Result<Vec<Model>, Box<dyn std::error::Error>> {
        self.ensure_initialized().await;
        // 首先获取锁
        let indexer_guard = self.indexer.lock().await;

        // 检查indexer是否已初始化
        match indexer_guard.as_ref() {
            Some(indexer) => {
                // 如果indexer已初始化，执行搜索
                let results = indexer.search(query, num, type_list).await;
                Ok(results)
            }
            None => {
                // 如果indexer未初始化，返回错误
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Indexer not initialized",
                )))
            }
        }
    }
    async fn reload_config() {
        todo!()
    }
}

#[tauri::command]
pub async fn rs_invoke_get_clipboards(
    state: tauri::State<'_, Arc<ClipboardHelper>>,
    num: u64,
    type_list: Option<Vec<i32>>,
) -> Result<Vec<Model>, String> {
    match state.get_clipboards(num, type_list).await {
        Ok(clipboards) => Ok(clipboards),
        Err(e) => {
            error!("rs_invoke_get_clipboards err: {:?}", e);
            Err(format!("Failed to get clipboards: {}", e))
        }
    }
}

#[tauri::command]
pub async fn rs_invoke_search_clipboards(
    state: tauri::State<'_, Arc<ClipboardHelper>>,
    query: &str,
    num: u64,
    type_list: Option<Vec<i32>>,
) -> Result<Vec<Model>, String> {
    match state.search_clipboards(query, num, type_list).await {
        Ok(clipboards) => Ok(clipboards),
        Err(e) => {
            error!("rs_invoke_search_clipboards err: {:?}", e);
            Err(format!("Failed to search clipboards: {}", e))
        }
    }
}
