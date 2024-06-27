extern crate chrono;

use std::cmp::PartialEq;
use std::io;
use std::ops::Deref;
use chrono::Local;
use cocoa::appkit::{
    NSPasteboard, NSPasteboardTypeMultipleTextSelection, NSPasteboardTypePNG,
    NSPasteboardTypeString, NSPasteboardTypeTIFF,
};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSArray, NSString};
use cocoa::foundation::{NSData, NSInteger};
use crate::core::pasteboard_content::{ContentType, PasteboardContent};

#[link(name = "AppKit", kind = "framework")]
extern "C" {
    pub static NSPasteboardTypeFileURL: id;
}


pub struct Pasteboard {
    pub change_count: NSInteger,
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

    pub unsafe fn set_contents(&mut self, item: id) -> Result<(), io::Error> {
        let pasteboard: id = NSPasteboard::generalPasteboard(nil);
        pasteboard.clearContents();

        let item_list = NSArray::arrayWithObject(nil, item);

        let success = pasteboard.writeObjects(item_list) == 1;

        if success {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to write to the pasteboard",
            ))
        }
    }
}

impl Pasteboard {
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


#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;
    use cocoa::appkit::{NSApp, NSApplication, NSImage, NSPasteboard};
    use cocoa::base::{id, nil};
    use cocoa::foundation::{NSArray, NSAutoreleasePool, NSData, NSString, NSURL};
    use objc::runtime::{Object, Sel};
    use objc::declare::ClassDecl;
    use objc::{msg_send, sel, sel_impl};
    use std::ptr;
    use crate::core::pasteboard::{ContentType, Pasteboard};

    #[test]
    fn test_build() {
        println!("build pass!")
    }

    #[test]
    fn new_type() {
        let c_type = ContentType::File;
        println!("{:?}", c_type.to_string());
    }

    #[test]
    fn set_contents_text() {
        // 获取上上个剪切板内容
        unsafe {
            // 获取剪切板
            let pasteboard: id = NSPasteboard::generalPasteboard(nil);

            // 清除剪切板内容
            pasteboard.clearContents();

            // 写入文本
            let string = NSString::alloc(nil).init_str("123Hello, Rust and Cocoa!");
            let objects = NSArray::arrayWithObject(nil, string);
            let _: bool = msg_send![pasteboard, writeObjects: objects];
        }
    }

       #[test]
    fn set_contents_file() {
        // 获取上上个剪切板内容
        unsafe {
            // 获取剪切板
            let pasteboard: id = NSPasteboard::generalPasteboard(nil);

            // 清除剪切板内容
            pasteboard.clearContents();

            // let file_path = NSString::alloc(nil).init_str("/Users/zeke/Downloads/表格问答功能测试反馈-0626.xlsx");
            let file_path = NSString::alloc(nil).init_str("/Users/zeke/Downloads/iShot2024-06-26 16.48.00.png");
            let file_url = NSURL::fileURLWithPath_(nil, file_path);
            let objects = NSArray::arrayWithObject(nil, file_url);
            let _: bool = msg_send![pasteboard, writeObjects: objects];

        }
    }

       #[test]
    fn set_contents_png() {
        // 获取上上个剪切板内容
        unsafe {
            // 获取剪切板
            let pasteboard: id = NSPasteboard::generalPasteboard(nil);

            // 清除剪切板内容
            pasteboard.clearContents();

            let image_path = NSString::alloc(nil).init_str("/Users/zeke/Downloads/iShot2024-06-26 16.48.00.png");
            let image_url = NSURL::fileURLWithPath_(nil, image_path);
            let image_data = NSData::dataWithContentsOfURL_(nil, image_url);
            let image = NSImage::initWithData_(NSImage::alloc(nil), image_data);
            let objects = NSArray::arrayWithObject(nil, image);
            let _: bool = msg_send![pasteboard, writeObjects: objects];
        }
    }
}