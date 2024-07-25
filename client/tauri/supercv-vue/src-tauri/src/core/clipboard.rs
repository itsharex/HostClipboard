use crate::core::pasteboard::{ContentType, PasteboardContent};
use crate::utils;
use crate::utils::config::CONFIG;
use crate::utils::file::get_file_size;
use crate::utils::time::get_current_date_time;
use chrono::{DateTime, Datelike, FixedOffset};
use clipboard_rs::common::RustImage;
use clipboard_rs::{
    Clipboard, ClipboardContext, ClipboardHandler, ClipboardWatcher, ClipboardWatcherContext,
    RustImageData,
};
use image::DynamicImage;
use log::error;
use sea_orm::DatabaseConnection;
use std::io;
use tokio::sync::mpsc::UnboundedSender;

struct ClipboardManager {
    ctx: ClipboardContext,
    db: DatabaseConnection,
    sender: UnboundedSender<PasteboardContent>,
    last_hash: u64,
}

pub struct ImageData {
    inner: RustImageData,
}

impl ImageData {
    pub fn new(inner: RustImageData) -> Self {
        ImageData { inner }
    }

    pub fn get_hash(&self) -> Option<&DynamicImage> {
        let mut buffer = Vec::new();
        self.inner.data.write_to(&mut buffer, format).unwrap();
    }

    fn save_to_path(&self, path: &str) -> clipboard_rs::Result<()> {
        self.inner.save_to_path(path)
    }
}

impl ClipboardManager {
    pub fn new(db: DatabaseConnection) -> Self {
        let ctx = ClipboardContext::new().unwrap();
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

        let db_clone = db.clone();

        tokio::spawn(async move {
            while let Some(content) = receiver.recv().await {
                Self::add_clipboard_entry(&db_clone, content).await;
            }
        });

        ClipboardManager {
            ctx,
            db,
            sender,
            last_hash: 0,
        }
    }

    async fn add_clipboard_entry(db: &DatabaseConnection, content: PasteboardContent) {
        todo!();
    }

    fn new_text_content(text_content: String) -> Some(PasteboardContent) {
        const LARGE_SIZE: usize = 1024;

        if text_content.len() > LARGE_SIZE {
            None
        } else {
            let hash = utils::hash::hash_str(&text_content);
            return PasteboardContent::new(text_content, ContentType::Text, hash, None, None);
        }
    }

    fn new_file_content(file_url: &String) -> Some(PasteboardContent) {
        const IMG_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "bmp", "gif"];

        let file_end = file_url.rsplit('.').next().unwrap_or("");
        let is_image = IMG_EXTENSIONS
            .iter()
            .any(|&ext| ext == file_end.to_lowercase());
        let path_str = file_url
            .to_file_path()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let hash = utils::hash::hash_str(&path_str);
        let text_content = format!("{} ({})", path_str, get_file_size(&path_str));
        return if is_image {
            Some(PasteboardContent::new(
                text_content,
                ContentType::Image,
                hash,
                None,
                path_str,
            ))
        } else {
            Some(PasteboardContent::new(
                text_content,
                ContentType::File,
                hash,
                None,
                path_str,
            ))
        };
    }

    fn new_img_content(img: ImageData) -> Some(PasteboardContent) {
        let path = get_local_path("tiff");
    }
}

impl ClipboardHandler for ClipboardManager {
    fn on_clipboard_change(&mut self) {
        let mut content = None;

        match self.ctx.get_files() {
            Ok(file_urls) if !file_urls.is_empty() => {
                for f_url in file_urls {
                    content = Self::new_file_content(&f_url);
                }
            }
            Ok(_) => {}
            Err(e) => {
                #[cfg(target_os = "windows")]
                {}

                #[cfg(any(target_os = "macos", target_os = "linux"))]
                {
                    error!("Error getting files from clipboard: {}", e);
                }
            }
        };

        if let Ok(img) = self.ctx.get_image() {
            // 假设可以直接从 img 对象获取尺寸
            content = Self::new_img_content(ImageData::new(img));
        } else if let Ok(text) = self.ctx.get_text() {
            content = Self::new_text_content(text);
        }
    }
}

fn string_is_large(input: String) -> (String, Option<Vec<u8>>) {
    const LARGE_SIZE: usize = 1024;
    let input_len = input.len();

    if input_len > LARGE_SIZE {
        // Truncate to the first 500 characters for simplicity.
        let truncated_input = &input[..input
            .char_indices()
            .nth(LARGE_SIZE as usize)
            .map_or(input_len, |i| i.0)];
        // Return the truncated string and the length of the original string in bytes as an Option.
        return (
            truncated_input.to_string(),
            Some(input.clone().into_bytes()),
        );
    } else {
        // Return the original string and None since it's under the size limit.
        (input, None)
    }
}

fn get_local_path(suffix: &str) -> Result<String, io::Error> {
    let date_time = get_current_date_time();
    let root_file_path = CONFIG
        .read()
        .unwrap()
        .files_path
        .join(format!(
            "{}{}{}",
            date_time.year(),
            date_time.month(),
            date_time.day()
        ))
        .to_str()
        .unwrap()
        .to_string();
    // 判断root_file_path 是否存在 不存在则递归创建
    if !std::path::Path::new(&root_file_path).exists() {
        std::fs::create_dir_all(&root_file_path).unwrap_or_else(|e| {
            panic!("Failed to create directories: {}", e);
        });
    }
    Ok(format!(
        "{}/{}.{}",
        root_file_path,
        date_time.timestamp(),
        suffix
    ))
}
