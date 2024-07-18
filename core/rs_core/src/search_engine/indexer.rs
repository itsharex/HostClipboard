use std::mem;
use std::sync::Arc;
use log::debug;
use sea_orm::{DatabaseConnection, DbErr};
use tokio::sync::Mutex;
use tokio::time::{Duration, interval};
use crate::time_it;
use crate::std_time_it;
use crate::db::crud;
use crate::db::entities::host_clipboard::Model;
use crate::schema::clipboard::ContentType;
use crate::search_engine::index_core::Trie;
use crate::utils::time;

pub struct ClipboardIndexer {
    index_manager: Arc<Mutex<IndexManager>>,
    db: DatabaseConnection,
}

impl ClipboardIndexer {
    pub async fn new(db: DatabaseConnection, down_days: Option<i64>) -> Self {
        let down_days = down_days.unwrap_or(3);
        let mut index_manager = IndexManager::new(down_days);
        index_manager.load_recent_entries(&db).await.unwrap();
        ClipboardIndexer {
            index_manager: Arc::new(Mutex::new(index_manager)),
            db,
        }
    }

    pub async fn start_background_update(&self) {
        let db = self.db.clone();
        let index_manager = self.index_manager.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(300));
            loop {
                interval.tick().await;
                let mut index_manager = index_manager.lock().await;
                if let Err(e) = index_manager.update_index(&db).await {
                    eprintln!("Error updating index: {:?}", e);
                }
            }
        });
    }

    pub async fn search(
        &self,
        query: &str,
        n: usize,
        doc_type: Option<i32>,
    ) -> Vec<Model> {
        let doc_type = doc_type;
        let index_manager = self.index_manager.lock().await;
        let results_id = index_manager.trie.search(query, n, doc_type);
        debug!("query: {:?}, n: {:?},doc_type: {:?}", query, n, doc_type);
        debug!("results_id: {:?}", results_id);
        crud::host_clipboard::get_clipboard_entries_by_id_list(&self.db, Some(results_id))
            .await
            .unwrap_or_default()
    }
}

struct IndexManager {
    pub trie: Trie,
    last_update: i64, // 最新的索引的 时间戳
    down_days: i64,   // 最多保存多少天的索引 默认3天
}

impl IndexManager {
    fn new(down_days: i64) -> Self {
        let down_days = down_days;

        IndexManager {
            trie: Trie::new(),
            last_update: time::get_current_timestamp(),
            down_days,
        }
    }

    // 加载最近的记录
    async fn load_recent_entries(&mut self, db: &DatabaseConnection) -> Result<(), DbErr> {
        let now_ts = time::get_current_timestamp();
        let recent_ts = now_ts - self.down_days * 24 * 60 * 60;
        let recent_entries =
            crud::host_clipboard::get_clipboard_entries_by_gt_timestamp(db, recent_ts).await?;

        for entry in recent_entries {
            debug!("insert: {:?}", entry.id);
            self.trie.insert(entry);
        }

        self.last_update = now_ts;

        Ok(())
    }

    // 启动后台更新
    pub async fn start_background_update(&mut self, db: &DatabaseConnection) {
        let mut interval = interval(Duration::from_secs(300)); // 每5分钟更新一次

        loop {
            interval.tick().await;
            if let Err(e) = self.update_index(db).await {
                eprintln!("Error updating index: {:?}", e);
            }
        }
    }

    async fn update_index(&mut self, db: &DatabaseConnection) -> Result<(), DbErr> {
        let new_entries =
            crud::host_clipboard::get_clipboard_entries_by_gt_timestamp(db, self.last_update)
                .await?;
        // debug!("new_entries: {:?}", new_entries);
        for entry in new_entries {
            debug!("inset: {}，trie size {:?} ", entry.id, mem::size_of_val(&self.trie));
            std_time_it!(|| {self.trie.insert(entry)});
            self.last_update = time::get_current_timestamp();
        }

        // self.remove_expired_entries();

        Ok(())
    }

    // fn remove_expired_entries(&mut self) {
    //     let expired_timestamp = time::get_current_timestamp() - (self.down_days * 24 * 60 * 60);
    //     let mut expired_docs = Vec::new();
    //
    //     for (doc_id, doc) in self.trie.documents.iter() {
    //         if doc.timestamp < expired_timestamp {
    //             expired_docs.push(*doc_id);
    //         }
    //     }
    //
    //     for doc_id in expired_docs {
    //         self.trie.remove_document(doc_id);
    //     }
    // }
}
