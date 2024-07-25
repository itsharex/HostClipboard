use log::error;
use sea_orm::DatabaseConnection;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, Notify};

use crate::core::pasteboard::Pasteboard;
use crate::db::connection::init_db_connection;
use crate::db::crud;
use crate::db::entities::host_clipboard::Model;
use crate::core::pasteboard::PasteboardContent;
use crate::utils::config::{UserConfig, CONFIG};
use crate::utils::{config, logger};
use crate::{time_it};
use log::debug;

pub struct ClipboardHelper {
    pasteboard: Arc<Mutex<Pasteboard>>,
    db: Arc<Mutex<Option<DatabaseConnection>>>,
    sender: mpsc::Sender<PasteboardContent>,
    receiver: Arc<Mutex<mpsc::Receiver<PasteboardContent>>>,
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


        self.initialized.store(true, Ordering::SeqCst);
        self.init_notifier.notify_waiters();

        Ok(())
    }

    async fn ensure_initialized(&self) {
        if !self.initialized.load(Ordering::SeqCst) {
            self.init_notifier.notified().await;
        }
    }
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
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
        let all_entries = time_it!(async {
            crud::host_clipboard::get_clipboards_by_type_list(&db, None, Some(num), type_list)
        })
        .await
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
        let db_guard = self.db.lock().await;
        let db = db_guard.as_ref().unwrap();
        let all_entries = time_it!(async {
            crud::host_clipboard::get_clipboards_by_type_list(
                &db,
                Some(query),
                Some(num),
                type_list,
            )
        })
        .await
        .await?;
        Ok(all_entries)
    }
    async fn get_user_config() -> UserConfig {
        CONFIG.read().unwrap().user_config.clone()
    }

    async fn set_user_config(user_config: UserConfig) -> io::Result<()> {
        config::update(user_config).await
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

#[tauri::command]
pub async fn rs_invoke_is_initialized(
    state: tauri::State<'_, Arc<ClipboardHelper>>,
) -> Result<bool, String> {
    match state.is_initialized() {
        true => Ok(true),
        false => Ok(false),
    }
}

#[tauri::command]
pub async fn rs_invoke_get_user_config(
    _: tauri::State<'_, Arc<ClipboardHelper>>,
) -> Result<UserConfig, String> {
    match ClipboardHelper::get_user_config().await {
        config => Ok(config),
    }
}

#[tauri::command]
pub async fn rs_invoke_set_user_config(
    _: tauri::State<'_, Arc<ClipboardHelper>>,
    user_config: UserConfig,
) -> Result<bool, String> {
    match ClipboardHelper::set_user_config(user_config).await {
        Ok(()) => Ok(true),
        Err(e) => {
            error!("rs_invoke_set_config err: {:?}", e);
            Err(format!("Failed to set config: {}", e))
        }
    }
}
