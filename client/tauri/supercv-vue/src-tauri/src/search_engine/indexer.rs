use crate::db::crud;
use crate::db::entities::host_clipboard::Model;
use crate::search_engine::index_core::Trie;
use crate::time_it;
use crate::utils::time;
use log::debug;
use sea_orm::{DatabaseConnection, DbErr};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use deepsize::DeepSizeOf;

pub struct ClipboardIndexer {
    trie: Arc<RwLock<Trie>>,
    db: DatabaseConnection,
    last_update: Arc<RwLock<i64>>,
}

impl ClipboardIndexer {
    pub async fn new(db: DatabaseConnection) -> Self {
        debug!("start");
        let trie = Arc::new(RwLock::new(Trie::new()));
        // {
        //     let trie_s = trie.clone().read().await;
        //     debug!("trie size: {:?}", &trie_s.deep_size_of());     
        // }

        let last_update = Arc::new(RwLock::new(time::get_current_timestamp()));

        let indexer = ClipboardIndexer {
            trie,
            db,
            last_update,
        };

        time_it!(async {
            indexer.load_recent_entries().await.unwrap();
        })
        .await;

        debug!("end");

        indexer
    }

    pub async fn start_background_update(&self) {
        let db = self.db.clone();
        let trie = self.trie.clone();
        let last_update = self.last_update.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(300));
            loop {
                interval.tick().await;
                if let Err(e) = Self::update_index(&db, &trie, &last_update).await {
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
        let trie = self.trie.read().await;
        let results_id = time_it!(sync || {
            debug!("trie size: {:?}", &trie.deep_size_of());   
            trie.search(query, num, type_list)
        })();

        debug!("results_id: {:?}", results_id);
        crud::host_clipboard::get_clipboard_entries_by_id_list(&self.db, Some(results_id))
            .await
            .unwrap_or_default()
    }

    async fn load_recent_entries(&self) -> Result<(), DbErr> {
        let now_ts = time::get_current_timestamp();
        let recent_entries =
            crud::host_clipboard::get_clipboards_by_type_list(&self.db, None, None).await?;
        debug!("load entries num: {:?}", recent_entries.len());
        let trie = self.trie.write().await;
        trie.insert_list(&recent_entries);

        let mut last_update = self.last_update.write().await;
        *last_update = now_ts;

        Ok(())
    }

    async fn update_index(
        db: &DatabaseConnection,
        trie: &Arc<RwLock<Trie>>,
        last_update: &Arc<RwLock<i64>>,
    ) -> Result<(), DbErr> {
        let current_last_update = *last_update.read().await;
        let new_entries =
            crud::host_clipboard::get_clipboard_entries_by_gt_timestamp(db, current_last_update)
                .await?;

        if !new_entries.is_empty() {
            let mut trie = trie.write().await;
            trie.insert_list(&new_entries);

            let mut last_update = last_update.write().await;
            *last_update = time::get_current_timestamp();
        }

        // Self::remove_expired_entries(db, trie, down_days).await;

        Ok(())
    }

    // async fn remove_expired_entries(db: &DatabaseConnection, trie: &Arc<RwLock<Trie>>) {
    //     let expired_timestamp = time::get_current_timestamp() - (down_days * 24 * 60 * 60);
    //     let expired_ids = {
    //         let trie = trie.read().await;
    //         trie.td_lt_ids(expired_timestamp).into_iter().collect()
    //     };
    //     let expired_docs =
    //         crud::host_clipboard::get_clipboard_entries_by_id_list(db, Some(expired_ids))
    //             .await
    //             .unwrap_or_default();
    //
    //     let mut trie = trie.write().await;
    //     for doc in expired_docs {
    //         trie.delete(&doc);
    //     }
    // }
}
