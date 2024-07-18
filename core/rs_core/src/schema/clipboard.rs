extern crate chrono;
use std::cmp::PartialEq;
use std::fmt;
use std::io;
use std::io::Write;
use std::sync::{Arc, Mutex};

use chrono::Datelike;
use chrono::DateTime;
use chrono::offset::FixedOffset;
use cocoa::appkit::{NSPasteboardTypePNG, NSPasteboardTypeTIFF};
use cocoa::base::nil;
use log::debug;

use crate::apis::safe_objc_ptr::SafeObjcPtr;
use crate::time_it;
use crate::tokio_time_it;
use crate::utils;
use crate::utils::config::CONFIG;

#[derive(Debug, Clone)]
pub enum ContentType {
    Text,
    Image,
    File,
}

impl ContentType {
    pub fn to_i32(&self) -> i32 {
        match self {
            ContentType::Text => 0,
            ContentType::Image => 1,
            ContentType::File => 2,
        }
    }
}
impl ToString for ContentType {
    fn to_string(&self) -> String {
        match self {
            ContentType::Text => "Text".to_string(),
            ContentType::File => "File".to_string(),
            ContentType::Image => "Image".to_string(),
        }
    }
}

pub struct PasteboardContent {
    pub text_content: String,          // 索引内容
    pub content_type: ContentType,     // 类型
    pub content: Option<Vec<u8>>,      // 二进制内容
    pub path: String,                  // 路径
    pub item: Arc<Mutex<SafeObjcPtr>>, // 对应的item对象
    pub uuid: String,
    pub date_time: DateTime<FixedOffset>,
}

impl PartialEq for ContentType {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

fn get_local_path(date_time: &DateTime<FixedOffset>, suffix: &str) -> Result<String, io::Error> {
    let root_file_path = CONFIG
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
impl PasteboardContent {
    // 创建文本类型的 PasteboardContent
    pub fn new(
        text_content: String,
        content_type: ContentType,
        content: Option<Vec<u8>>,
        item: *mut objc::runtime::Object,
    ) -> Arc<Mutex<Self>> {
        let date_time = utils::time::get_current_date_time();

        // 定义text_content
        let (text_content_show, path): (String, String);

        match content_type {
            ContentType::Text => {
                path = match &content {
                    Some(_data) => {
                        let suffix = "txt";
                        get_local_path(&date_time, suffix).unwrap()
                    }
                    _ => "".to_string(),
                };
                text_content_show = text_content
            }
            ContentType::Image => {
                let suffix = text_content;
                let size = match &content {
                    Some(data) => utils::file::format_size(data.len()),
                    _ => "".to_string(),
                };

                path = get_local_path(&date_time, &suffix).unwrap();
                text_content_show = format!("{}: ({})", content_type.to_string(), size)
            }
            ContentType::File => {
                let size = utils::file::get_file_size(&text_content);
                path = text_content;
                text_content_show = format!("{}: {} ({})", content_type.to_string(), path, size)
            }
        };

        let safe_item = SafeObjcPtr::new(item);

        let pasteboard_content = Arc::new(Mutex::new(PasteboardContent {
            text_content: text_content_show,
            content_type,
            content,
            path,
            item: Arc::new(Mutex::new(safe_item)),
            uuid: utils::uuid::get_uuid(),
            date_time,
        }));

        let pasteboard_content_clone = Arc::clone(&pasteboard_content);
        // 启动异步任务来执行save_path
        tokio::spawn(async move {
            if let Err(e) = Self::async_save_path(pasteboard_content_clone).await {
                eprintln!("Error saving path: {}", e);
            }
        });
        pasteboard_content
    }

    async fn async_save_path(pasteboard_content: Arc<Mutex<Self>>) -> Result<(), String> {
        let item = pasteboard_content.lock().unwrap().clone();
        debug!("启动 读取data的tokio,text_content: {}", item.text_content);
        let result = tokio_time_it!(|| item.save_path());
        if let Ok(()) = result {
            Ok(())
        } else {
            Err("Failed to save path".to_string())
        }
    }

    pub fn save_path(self) -> Result<(), io::Error> {
        // TODO: 存入路径
        if self.path.is_empty() {
            return Ok(());
        }
        match self.content {
            Some(data) => {
                let mut file = std::fs::File::create(self.path).unwrap();
                file.write_all(&data).unwrap();
            }
            _ => {}
        }

        Ok(())
    }

    pub fn _fake_new(num: usize) -> Vec<Self> {
        let mut result: Vec<PasteboardContent> = vec![];
        let empty_ptr = SafeObjcPtr::new(nil);
        let empty_item = Arc::new(Mutex::new(empty_ptr));

        for i in 0..num {
            let text = std::iter::repeat(format!("{} hi", i)).take(40).collect();
            result.push(PasteboardContent {
                text_content: text,
                content_type: ContentType::Text,
                content: None,
                path: "".to_string(),
                item: Arc::clone(&empty_item),
                uuid: utils::uuid::get_uuid(),
                date_time: utils::time::get_current_date_time(),
            });
        }
        result
    }
}

impl Clone for PasteboardContent {
    fn clone(&self) -> Self {
        PasteboardContent {
            text_content: self.text_content.clone(),
            content_type: self.content_type.clone(),
            content: self.content.clone(),
            path: self.path.clone(),
            item: Arc::clone(&self.item),
            uuid: self.uuid.clone(),
            date_time: self.date_time.clone(),
        }
    }
}

impl fmt::Debug for PasteboardContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let content_length = match &self.content {
            Some(bytes) => format!("{}", bytes.len()),
            None => "None".to_string(),
        };

        f.debug_struct("PasteboardContent")
            .field("content_type", &self.content_type)
            .field("content", &content_length)
            .field("text_content", &self.text_content)
            .finish()
    }
}
