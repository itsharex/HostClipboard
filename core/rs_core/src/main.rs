use std::path::Path;

use sea_orm::Database;

use db::{connection::establish_connection, crud};

use crate::apis::pasteboard::Pasteboard;

mod apis;
mod db;
mod schema;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path =
        Path::new("/Users/zeke/work/workspace/github_work/HostClipboard/core/rs_core/test.db")
            .expand_tilde()?;
    let db_url = format!("sqlite:{}", db_path.display());
    println!("{}", db_url);
    // 1. 判断数据库文件是否存在，不存在则创建
    if !db_path.exists() {
        println!("文件不存在，开始创建数据库文件");
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::File::create(db_path).await?;
        Database::connect(&db_url).await?;
        println!("Database file created.");
    }

    let mut pasteboard = Pasteboard::new();
    let db = establish_connection(&db_url).await?;
    loop {
        unsafe {
            let content = pasteboard.get_contents();
            std::thread::sleep(std::time::Duration::from_millis(500));

            for c in content {
                let new_entry = crud::host_clipboard::add_clipboard_entry(&db, c).await?;
            }
        }
    }
    // let all_entries = crud::host_clipboard::get_clipboard_entries(&db).await?;
    // println!("All entries: {:?}", all_entries);

    //
    // let db = establish_connection(&db_url).await?;
    // Migrator::up(&db, None).await?;
    //
    //
    // // 2. 插入一条数据
    // let cont_1 = PasteboardContent::new(
    //     "Test content".to_string(), ContentType::Text, None, None);
    // let _ = crud::host_clipboard::add_clipboard_entry(&db, "Test content".to_string()).await?;
    // let new_entry = crud::host_clipboard::add_clipboard_entry(&db, "Test content2".to_string()).await?;
    // println!("Inserted entry: {:?}", new_entry);
    //
    // // 查询所有数据
    // let all_entries = crud::host_clipboard::get_clipboard_entries(&db).await?;
    // println!("All entries: {:?}", all_entries);
    //
    // // // 更新数据
    // // let updated_entry =
    // //     crud::host_clipboard::update_clipboard_entry(&db, new_entry.id, "Updated content".to_string())
    // //         .await?;
    // // println!("Updated entry: {:?}", updated_entry);
    //
    // // 删除数据
    // crud::host_clipboard::delete_clipboard_entry(&db, new_entry.id).await?;
    // println!("Entry deleted.");
    //
    // // 再次查询所有数据
    // let remaining_entries = crud::host_clipboard::get_clipboard_entries(&db).await?;
    // println!("Remaining entries: {:?}", remaining_entries);

    Ok(())
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
