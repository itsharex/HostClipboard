use clipboard_rs::{ClipboardWatcher, ClipboardWatcherContext, WatcherShutdown};
use log::{debug, error};
use sea_orm::DatabaseConnection;
use std::io;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::core::clipboard::ClipboardHandle;
use crate::db::connection::init_db_connection;
use crate::db::crud;
use crate::db::entities::host_clipboard::Model;
use crate::time_it;
use crate::utils::config::{UserConfig, CONFIG};
use crate::utils::{config, logger};

pub struct ClipboardHelper {
    db: Arc<Mutex<DatabaseConnection>>,
    watcher_shutdown: WatcherShutdown,
}

impl ClipboardHelper {
    pub async fn new(log_level: Option<i32>, sql_level: Option<i32>) -> Self {
        logger::init_logger(log_level, sql_level);
        // 初始化数据库连接
        let db_connection = init_db_connection(None)
            .await
            .expect("Failed to connect to database");
        let db = Arc::new(Mutex::new(db_connection));

        // 创建 ClipboardHandle
        let clipboard_manager = ClipboardHandle::new(db.clone());

        let mut watcher = ClipboardWatcherContext::new().unwrap();
        let watcher_shutdown = watcher
            .add_handler(clipboard_manager)
            .get_shutdown_channel();
        // 在新的任务中启动 watcher
        let _ = tokio::spawn(async move {
            watcher.start_watch();
        });

        Self {
            db,
            watcher_shutdown,
            // watcher_handle,
        }
    }


    async fn get_clipboards(
        &self,
        num: u64,
        type_list: Option<Vec<i32>>,
    ) -> Result<Vec<Model>, Box<dyn std::error::Error>> {
        let db_guard = self.db.lock().await;
        let all_entries = time_it!(async {
            crud::host_clipboard::get_clipboards_by_type_list(&db_guard, None, Some(num), type_list)
        })
        .await?;
        Ok(all_entries)
    }

    async fn search_clipboards(
        &self,
        query: &str,
        num: u64,
        type_list: Option<Vec<i32>>,
    ) -> Result<Vec<Model>, Box<dyn std::error::Error>> {
        let db_guard = self.db.lock().await;
        let all_entries = time_it!(async {
            crud::host_clipboard::get_clipboards_by_type_list(
                &db_guard,
                Some(query),
                Some(num),
                type_list,
            )
        })
        .await?;
        Ok(all_entries)
    }

    pub async fn get_user_config() -> UserConfig {
        CONFIG.read().unwrap().user_config.clone()
    }

    pub async fn set_user_config(user_config: UserConfig) -> io::Result<()> {
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
