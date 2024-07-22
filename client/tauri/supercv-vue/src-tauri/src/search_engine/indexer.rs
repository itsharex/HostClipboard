use crate::db::crud;
use crate::db::entities::host_clipboard::Model;
use crate::schema::clipboard::ContentType;
use crate::search_engine::index_core::Trie;
use crate::time_it;
use crate::utils::time;
use crate::{std_time_it, tokio_time_it};
use log::debug;
use sea_orm::{DatabaseConnection, DbErr};
use std::mem;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task;
use tokio::time::{interval, Duration};

pub struct ClipboardIndexer {
    index_manager: Arc<Mutex<IndexManager>>,
    db: DatabaseConnection,
}

impl ClipboardIndexer {
    pub async fn new(db: DatabaseConnection, down_days: Option<i64>) -> Self {
        let down_days = down_days.unwrap_or(3);
        let index_manager = IndexManager::new(down_days);
        let index_manager = Arc::new(Mutex::new(index_manager));
        {
            let mut index_manager = index_manager.lock().await;
            index_manager.load_recent_entries(&db).await.unwrap();
        }
        debug!("new ClipboardIndexer: {:?} {}", db, down_days);
        ClipboardIndexer { index_manager, db }
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

    pub async fn search(&self, query: &str, num: u64, type_list: Option<Vec<i32>>) -> Vec<Model> {
        debug!(
            "query: {:?}, n: {:?}, type_list: {:?}",
            &query, &num, &type_list
        );
        let index_manager = self.index_manager.lock().await;
        let trie = index_manager.trie.lock().await;
        let results_id = trie.search(query, num, type_list);

        debug!("results_id: {:?}", results_id);
        crud::host_clipboard::get_clipboard_entries_by_id_list(&self.db, Some(results_id))
            .await
            .unwrap_or_default()
    }
}

struct IndexManager {
    pub trie: Arc<Mutex<Trie>>,
    last_update: i64,
    down_days: i64,
}

impl IndexManager {
    fn new(down_days: i64) -> Self {
        IndexManager {
            trie: Arc::new(Mutex::new(Trie::new())),
            last_update: time::get_current_timestamp(),
            down_days,
        }
    }

    async fn load_recent_entries(&mut self, db: &DatabaseConnection) -> Result<(), DbErr> {
        let now_ts = time::get_current_timestamp();
        let recent_ts = now_ts - self.down_days * 24 * 60 * 60;
        let recent_entries =
            crud::host_clipboard::get_clipboard_entries_by_gt_timestamp(db, recent_ts).await?;

        let mut handles = vec![];

        for entry in recent_entries {
            let trie = Arc::clone(&self.trie);
            let handle = task::spawn(async move {
                debug!("insert: start {:?}", entry.id);
                let mut trie = trie.lock().await;
                trie.insert(&entry);
                debug!("insert: end {:?}", entry.id);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.expect("Task panicked");
        }
        debug!("插入完成");
        self.last_update = now_ts;

        Ok(())
    }

    pub async fn start_background_update(&mut self, db: &DatabaseConnection) {
        let mut interval = interval(Duration::from_secs(300));

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

        for entry in new_entries {
            debug!(
                "insert: {}，trie size {:?} ",
                entry.id,
                mem::size_of_val(&self.trie)
            );
            let trie = Arc::clone(&self.trie);
            let mut trie = trie.lock().await;
            trie.insert(&entry);
            self.last_update = time::get_current_timestamp();
        }

        self.remove_expired_entries(db).await;

        Ok(())
    }

    async fn remove_expired_entries(&mut self, db: &DatabaseConnection) {
        let expired_timestamp = time::get_current_timestamp() - (self.down_days * 24 * 60 * 60);
        let expired_ids = {
            let trie = self.trie.lock().await;
            Some(trie.td_lt_ids(expired_timestamp).into_iter().collect())
        };
        let expired_doc = crud::host_clipboard::get_clipboard_entries_by_id_list(db, expired_ids)
            .await
            .unwrap_or_default();
        for doc in expired_doc {
            let mut trie = self.trie.lock().await;
            trie.delete(&doc);
        }
    }
}
