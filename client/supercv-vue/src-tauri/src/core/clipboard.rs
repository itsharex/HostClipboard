use crate::core::pasteboard::{ContentType, PasteboardContent};
use crate::db::crud::host_clipboard::add_clipboard_entry;
use crate::utils::config::CONFIG;
use crate::utils::file::{format_size, get_file_size};
use crate::utils::hash::hash_vec;
use crate::utils::time::get_current_date_time;
use crate::{time_it, utils};
use chrono::Datelike;
use clipboard_rs::common::RustImage;
use clipboard_rs::{Clipboard, ClipboardContext, ClipboardHandler, RustImageData};
use log::{debug, error};
use sea_orm::DatabaseConnection;
use std::io;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use url::Url;

pub struct ClipboardHandle {
    db: Arc<Mutex<DatabaseConnection>>,
    ctx: ClipboardContext,
    last_hash: String,
    sender: Sender<PasteboardContent>,
    receiver_handle: JoinHandle<()>,
    runtime: Arc<Runtime>,
}

impl ClipboardHandle {
    pub fn new(db: Arc<Mutex<DatabaseConnection>>) -> Self {
        let ctx = ClipboardContext::new().unwrap();
        let (sender, receiver) = mpsc::channel();
        let runtime = Arc::new(Runtime::new().unwrap());

        let db_clone = db.clone();
        let runtime_clone = runtime.clone();
        let receiver_handle = std::thread::spawn(move || {
            Self::process_receiver(receiver, db_clone, runtime_clone);
        });

        ClipboardHandle {
            ctx,
            db,
            sender,
            last_hash: "".to_string(),
            receiver_handle,
            runtime,
        }
    }
    fn process_receiver(
        receiver: Receiver<PasteboardContent>,
        db: Arc<Mutex<DatabaseConnection>>,
        runtime: Arc<Runtime>,
    ) {
        while let Ok(content) = receiver.recv() {
            // debug!("Received clipboard content: {:?}", content);
            runtime.block_on(async {
                Self::add_clipboard_entry(&db, content).await;
            });
        }
    }

    async fn add_clipboard_entry(db: &Arc<Mutex<DatabaseConnection>>, content: PasteboardContent) {
        let db_guard = db.lock().await;
        time_it!(async add_clipboard_entry(&db_guard, content))
            .await
            .unwrap();
    }

    fn new_text_content(&mut self, text_content: String) -> Option<PasteboardContent> {
        if string_is_large(&text_content) || text_content.trim().is_empty() {
            return None;
        }

        let hash = utils::hash::hash_str(&text_content);
        if self.check_hash(&hash) {
            return None;
        }
        self.last_hash = hash.clone();
        return Some(PasteboardContent::new(
            text_content,
            ContentType::Text,
            hash,
            None,
        ));
    }

    fn new_file_content(&mut self, file_url: String) -> Option<PasteboardContent> {
        const IMG_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "bmp", "gif"];

        let file_end = file_url.rsplit('.').next().unwrap_or("");
        let is_image = IMG_EXTENSIONS
            .iter()
            .any(|&ext| ext == file_end.to_lowercase());

        let url = Url::parse(&*file_url).expect("Invalid URL");
        let path_str = url.to_file_path().unwrap().to_str().unwrap().to_string();
        let hash = utils::hash::hash_str(&path_str);

        if self.check_hash(&hash) {
            return None;
        }
        self.last_hash = hash.clone();

        return if is_image {
            let text_content = format!("Image: {} ({})", path_str, get_file_size(&path_str));
            Some(PasteboardContent::new(
                text_content,
                ContentType::Image,
                hash,
                Some(path_str),
            ))
        } else {
            let text_content = format!("File: {} ({})", path_str, get_file_size(&path_str));
            Some(PasteboardContent::new(
                text_content,
                ContentType::File,
                hash,
                Some(path_str),
            ))
        };
    }

    fn new_img_content(&mut self, img: &RustImageData) -> Option<PasteboardContent> {
        let (w, h) = img.get_size();
        let text_content = format!(
            "Image: {}x{} ({})",
            w,
            h,
            format_size(img.get_bytes().len())
        );
        let hash = hash_vec(img.get_bytes());
        let path = get_local_path("png").unwrap();
        img.save_to_path(&path).unwrap();
        if self.check_hash(&hash) {
            return None;
        }
        self.last_hash = hash.clone();
        Some(PasteboardContent::new(
            text_content,
            ContentType::Image,
            hash,
            Some(path),
        ))
    }
    fn check_hash(&self, hash: &str) -> bool {
        return if self.last_hash == *hash {
            debug!("check_hash true");
            true
        } else {
            false
        };
    }
}

impl ClipboardHandler for ClipboardHandle {
    fn on_clipboard_change(&mut self) {
        let mut content = None;

        match self.ctx.get_files() {
            Ok(file_urls) if !file_urls.is_empty() => {
                for f_url in file_urls {
                    content = self.new_file_content(f_url);
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
        if content.is_none() {
            if let Ok(img) = self.ctx.get_image() {
                // 假设可以直接从 img 对象获取尺寸
                content = self.new_img_content(&img);
            } else if let Ok(text) = self.ctx.get_text() {
                content = self.new_text_content(text);
            }
        }
        // 将content push
        if let Some(content) = content {
            let _ = self.sender.send(content);
        }
    }
}

fn string_is_large(input: &String) -> bool {
    const LARGE_SIZE: usize = 250000;
    let input_len = input.len();
    debug!("get_sting_length: {}", input_len);
    input_len > LARGE_SIZE
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
    if !Path::new(&root_file_path).exists() {
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
