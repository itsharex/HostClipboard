extern crate chrono;
use std::fmt;
use std::io;

use chrono::Local;
use cocoa::appkit::{
    NSPasteboard, NSPasteboardTypeMultipleTextSelection, NSPasteboardTypePNG,
    NSPasteboardTypeString, NSPasteboardTypeTIFF,
};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSArray, NSString};
use cocoa::foundation::{NSData, NSInteger};

#[link(name = "AppKit", kind = "framework")]
extern "C" {
    pub static NSPasteboardTypeFileURL: id;
}
pub struct Pasteboard {
    pub change_count: NSInteger,
}

#[derive(Debug)]
pub enum ContentType {
    Text,
    File,
    FileImage,
    PBImage,
    PBOther,
}

pub struct PasteboardContent {
    pub text_content: String,
    pub content_type: ContentType,
    pub content: Option<Vec<u8>>,
    item: id,
}

impl PasteboardContent {
    // 创建文本类型的 PasteboardContent
    pub fn new(
        text_content: String,
        content_type: ContentType,
        content: Option<Vec<u8>>,
        item: id,
    ) -> Self {
        PasteboardContent {
            text_content,
            content_type,
            content,
            item,
        }
    }

    pub fn read_content(&self) -> Result<(), io::Error> {
        // TODO: 读取content 二进制数据 并且存入content字段
        Ok(())
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
impl Pasteboard {
    pub fn new() -> Self {
        Pasteboard {
            change_count: unsafe { NSPasteboard::generalPasteboard(nil).changeCount() },
        }
    }
    pub unsafe fn get_contents(&mut self) -> Vec<PasteboardContent> {
        let mut contents = vec![];
        let (new_change_count, items) = unsafe {
            let pasteboard: id = NSPasteboard::generalPasteboard(nil);
            (pasteboard.changeCount(), pasteboard.pasteboardItems())
        };

        if self.change_count == new_change_count {
            return contents;
        }
        self.change_count = new_change_count;
        let item_count = unsafe { items.count() };

        for i in 0..item_count {
            let item: id = items.objectAtIndex(i);
            if let Some(content) = self.get_item(item) {
                contents.push(content);
            }
        }

        contents
    }

    unsafe fn get_item(&self, item: id) -> Option<PasteboardContent> {
        // 优先检查文件 URL 类型
        if let Some(file_url_str) = self.get_file_url(item) {
            let file_end = file_url_str.split('.').last().unwrap_or("");
            let img_extensions = ["png", "jpg", "jpeg", "bmp", "gif"];
            let content_type = if img_extensions.contains(&file_end.to_lowercase().as_str()) {
                ContentType::FileImage
            } else {
                ContentType::File
            };
            return Some(PasteboardContent::new(
                file_url_str,
                content_type,
                None,
                item,
            ));
        }

        // 检查多文本类型
        if let Some(string) = self.get_multi_text_content(item) {
            return Some(PasteboardContent::new(
                string,
                ContentType::Text,
                None,
                item,
            ));
        }

        // 检查文本类型
        if let Some(string) = self.get_text_content(item) {
            return Some(PasteboardContent::new(
                string,
                ContentType::Text,
                None,
                item,
            ));
        }

        // 对于两者都不是的 则要读取 item data， 如果是 NSPasteboardTypeTIFF， NSPasteboardTypePNG则是图片
        // 否则则是文件
        // 读取 item data
        if let Some(rust_bytes) = self.get_data(item, NSPasteboardTypeTIFF) {
            let text_content = self.get_now_string();
            return Some(PasteboardContent::new(
                text_content,
                ContentType::PBImage,
                Some(rust_bytes),
                item,
            ));
        }

        // 读取 item data
        if let Some(rust_bytes) = self.get_data(item, NSPasteboardTypePNG) {
            let text_content = self.get_now_string();
            return Some(PasteboardContent::new(
                text_content,
                ContentType::PBImage,
                Some(rust_bytes),
                item,
            ));
        }

        // 读取 item data
        let text_content = format!("{}-other", self.get_now_string());
        Some(PasteboardContent::new(
            text_content,
            ContentType::PBOther,
            None,
            item,
        ))
    }

    unsafe fn get_file_url(&self, item: id) -> Option<String> {
        let file_url: id = item.stringForType(NSPasteboardTypeFileURL);
        if file_url.is_null() {
            return None;
        }
        let rust_bytes = file_url.UTF8String();
        Some(
            std::ffi::CStr::from_ptr(rust_bytes)
                .to_string_lossy()
                .to_string(),
        )
    }

    unsafe fn get_text_content(&self, item: id) -> Option<String> {
        let content: id = item.stringForType(NSPasteboardTypeString);
        if content.is_null() {
            return None;
        }
        let rust_bytes = content.UTF8String();
        Some(
            std::ffi::CStr::from_ptr(rust_bytes)
                .to_string_lossy()
                .to_string(),
        )
    }

    unsafe fn get_multi_text_content(&self, item: id) -> Option<String> {
        let content: id = item.stringForType(NSPasteboardTypeMultipleTextSelection);
        if content.is_null() {
            return None;
        }
        let rust_bytes = content.UTF8String();
        Some(
            std::ffi::CStr::from_ptr(rust_bytes)
                .to_string_lossy()
                .to_string(),
        )
    }

    unsafe fn get_data(&self, item: id, data_type: id) -> Option<Vec<u8>> {
        let data: id = item.dataForType(data_type);
        if data.is_null() {
            return None;
        }
        let length = data.length() as usize;
        let bytes_ptr = data.bytes() as *const u8;
        Some(std::slice::from_raw_parts(bytes_ptr, length).to_vec())
    }

    fn get_now_string(&self) -> String {
        let now = Local::now();
        now.format("%Y%m%d-%H:%M:%S").to_string()
    }
}

