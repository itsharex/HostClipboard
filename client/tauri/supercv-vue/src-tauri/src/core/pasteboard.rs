extern crate chrono;
use std::cmp::PartialEq;

use chrono::offset::FixedOffset;
use chrono::DateTime;

use crate::utils::time::get_current_date_time;

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

impl PartialEq for ContentType {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

#[derive(Debug)]
pub struct PasteboardContent {
    pub text_content: String, // 索引内容
    pub r#type: ContentType,  // 类型
    pub hash: String,            // content or text_content hash
    pub path: String,         // 路径
    pub date_time: DateTime<FixedOffset>,
}

impl PasteboardContent {
    // 创建文本类型的 PasteboardContent
    pub fn new(
        text_content: String,
        content_type: ContentType,
        hash: String,
        path: Option<String>,
    ) -> Self {
        PasteboardContent {
            text_content,
            r#type: content_type,
            hash,
            path: path.unwrap_or_default(),
            date_time: get_current_date_time(),
        }
    }
}
