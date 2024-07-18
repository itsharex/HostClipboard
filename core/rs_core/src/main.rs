use std::path::Path;
use log::{debug, info};
use sea_orm::Database;

use db::{connection::establish_connection, crud};

use crate::apis::pasteboard::Pasteboard;
use crate::search_engine::indexer::ClipboardIndexer;
use crate::utils::config::CONFIG;
use crate::utils::logger;

mod apis;
mod db;
mod schema;
mod search_engine;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logger::init_logger();

    let db_path = CONFIG.db_path.join("db.sqlite").expand_tilde()?;
    let db_url = format!("sqlite:{}", db_path.display());
    info!("{}", db_url);
    // 1. 判断数据库文件是否存在，不存在则创建
    if !db_path.exists() {
        debug!("文件不存在，开始创建数据库文件");
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::File::create(db_path).await?;
        Database::connect(&db_url).await?;
        debug!("Database file created.");
    }

    let mut pasteboard = Pasteboard::new();
    let db = establish_connection(&db_url).await?;
    let mut indexer = ClipboardIndexer::new(db.clone(), None).await;
    // let search = indexer.search("sqlx", 10, None).await;
    let search_results = tokio_time_it!(|| async { indexer.search("sqlx", 10, None).await.len() }).await;
    debug!("{:?}", search_results);
    loop {
        unsafe {
            let content = pasteboard.get_contents();
            std::thread::sleep(std::time::Duration::from_millis(500));
            for c in content {
                let _new_entry = crud::host_clipboard::add_clipboard_entry(&db, c).await?;
            }
        }
    }

    trait ExpandTilde {
        fn expand_tilde(&self) -> Result<std::path::PathBuf, std::io::Error>;
    }

    impl ExpandTilde for Path {
        fn expand_tilde(&self) -> Result<std::path::PathBuf, std::io::Error> {
            if !self.starts_with("~") {
                return Ok(self.to_path_buf());
            }

            if self == Path::new("~") {
                return dirs::home_dir().ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, "Home directory not found")
                });
            }

            dirs::home_dir()
                .map(|mut h| {
                    if h == Path::new("/") {
                        self.strip_prefix("~").unwrap().to_path_buf()
                    } else {
                        h.push(self.strip_prefix("~/").unwrap());
                        h
                    }
                })
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, "Home directory not found")
                })
        }
    }
}
