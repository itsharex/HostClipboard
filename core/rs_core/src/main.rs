use crate::apis::pasteboard::Pasteboard;
use crate::db::connection::init_db_connection;
use crate::search_engine::indexer::ClipboardIndexer;
use crate::utils::config::CONFIG;
use crate::utils::logger;
use db::crud;
use fltk::enums::Cursor::N;
use log::{debug, info};
use sea_orm::Database;
use std::path::Path;
use std::time::Duration;

mod apis;
mod db;
mod schema;
mod search_engine;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = logger::init_logger(None, Some(2));

    let db = init_db_connection(None).await?;
    let mut pasteboard = Pasteboard::new();

    let mut indexer = ClipboardIndexer::new(db.clone(), None).await;

    let search_results =
        tokio_time_it!(|| async { indexer.search("e", 10, None).await.len() }).await;

    tokio::spawn(async move {
        let _ = &indexer.start_background_update().await;
    });

    debug!("{:?}", search_results);
    loop {
        // logger_handle.flush();
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
