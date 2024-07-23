extern crate chrono;

use std::sync::Arc;

use chrono::Local;
use cocoa::appkit::{
    NSPasteboard, NSPasteboardTypeMultipleTextSelection, NSPasteboardTypePNG,
    NSPasteboardTypeString, NSPasteboardTypeTIFF,
};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSArray, NSData, NSInteger, NSString};
use url::Url;

use crate::schema::clipboard::{ContentType, PasteboardContent};

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

    // pub unsafe fn set_contents(&mut self, item: id) -> Result<(), io::Error> {
    //     let pasteboard: id = NSPasteboard::generalPasteboard(nil);
    //     pasteboard.clearContents();

    //     let item_list = NSArray::arrayWithObject(nil, item);

    //     let success = pasteboard.writeObjects(item_list) == true;

    //     if success {
    //         Ok(())
    //     } else {
    //         Err(io::Error::new(
    //             io::ErrorKind::Other,
    //             "Failed to write to the pasteboard",
    //         ))
    //     }
    // }
}

impl Pasteboard {
    unsafe fn get_item(&self, item: id) -> Option<PasteboardContent> {
        // 优先检查文件 URL 类型
        // 如果文件小于 3M 则转存
        // 如果文件大于 3M 使用原始路径
        // 先不存了，直接使用原始路径
        if let Some(file_url_str) = self.get_file_url(item) {
            let file_end = file_url_str.split('.').last().unwrap_or("");
            let img_extensions = ["png", "jpg", "jpeg", "bmp", "gif"];
            let content_type = if img_extensions.contains(&file_end.to_lowercase().as_str()) {
                ContentType::Image
            } else {
                ContentType::File
            };
            let url = Url::parse(&*file_url_str).expect("Invalid URL");
            let path_str = url.to_file_path().unwrap().to_str().unwrap().to_string();

            let pasteboard_content = PasteboardContent::new(path_str, content_type, None, item);
            let res = Arc::clone(&pasteboard_content).lock().unwrap().clone();
            return Some(res);
        }

        // 检查多文本类型
        // 如果文本小于 1024 则存放在数据库
        // 如果文件大于 1024 则 截断+存原始文本文件
        if let Some(string) = self.get_multi_text_content(item) {
            let (text, bytes) = string_is_large(string);
            let pasteboard_content = PasteboardContent::new(text, ContentType::Text, bytes, item);
            let res = Arc::clone(&pasteboard_content).lock().unwrap().clone();
            return Some(res);
        }

        // 检查文本类型
        if let Some(string) = self.get_text_content(item) {
            let (text, bytes) = string_is_large(string);
            let pasteboard_content = PasteboardContent::new(text, ContentType::Text, bytes, item);
            let res = Arc::clone(&pasteboard_content).lock().unwrap().clone();
            return Some(res);
        }

        // 对于两者都不是的 则要读取 item data， 如果是 NSPasteboardTypeTIFF， NSPasteboardTypePNG则是图片
        // 否则则是文件
        // 读取 item data
        if let Some(rust_bytes) = self.get_data(item, NSPasteboardTypeTIFF) {
            let suffix = "tiff".to_string();
            let pasteboard_content =
                PasteboardContent::new(suffix, ContentType::Image, Some(rust_bytes), item);
            let res = Arc::clone(&pasteboard_content).lock().unwrap().clone();
            return Some(res);
        }

        // 读取 item data
        if let Some(rust_bytes) = self.get_data(item, NSPasteboardTypePNG) {
            let suffix = "png".to_string();
            let pasteboard_content =
                PasteboardContent::new(suffix, ContentType::Image, Some(rust_bytes), item);
            let res = Arc::clone(&pasteboard_content).lock().unwrap().clone();
            return Some(res);
        }

        // 读取 item data
        let text_content = format!("{}.other", self.get_now_string());
        let pasteboard_content =
            PasteboardContent::new(text_content, ContentType::File, None, item);
        let res = Arc::clone(&pasteboard_content).lock().unwrap().clone();
        return Some(res);
    }

    unsafe fn get_file_url(&self, item: id) -> Option<String> {
        let file_url: id = item.stringForType(NSPasteboardTypeFileURL);
        if file_url.is_null() {
            return None;
        }
        let rust_bytes = file_url.UTF8String();
        Some(
            urlencoding::decode(
                std::ffi::CStr::from_ptr(rust_bytes)
                    .to_string_lossy()
                    .as_ref(),
            )
            .expect("UTF-8")
            .into_owned(),
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

#[cfg(test)]
mod tests {
    use cocoa::appkit::{NSImage, NSPasteboard};
    use cocoa::base::{id, nil};
    use cocoa::foundation::{NSArray, NSData, NSString, NSURL};
    use objc::{msg_send, sel, sel_impl};

    use crate::schema::clipboard::ContentType;

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
            let file_path =
                NSString::alloc(nil).init_str("/Users/zeke/Downloads/iShot2024-06-26 16.48.00.png");
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

            let image_path =
                NSString::alloc(nil).init_str("/Users/zeke/Downloads/iShot2024-06-26 16.48.00.png");
            let image_url = NSURL::fileURLWithPath_(nil, image_path);
            let image_data = NSData::dataWithContentsOfURL_(nil, image_url);
            let image = NSImage::initWithData_(NSImage::alloc(nil), image_data);
            let objects = NSArray::arrayWithObject(nil, image);
            let _: bool = msg_send![pasteboard, writeObjects: objects];
        }
    }
}

// 这些是 macOS 和 iOS 开发中使用的 `NSPasteboard`（在 iOS 中称为 `UIPasteboard`）的类型标识符。每个类型标识符代表一种可以在剪贴板中存储的数据格式。以下是每个类型的含义：
//
// 1. **NSPasteboardTypeString**: 表示纯文本字符串。
// 2. **NSPasteboardTypePDF**: 表示 PDF 文档数据。
// 3. **NSPasteboardTypeTIFF**: 表示 TIFF 图像数据。
// 4. **NSPasteboardTypePNG**: 表示 PNG 图像数据。
// 5. **NSPasteboardTypeRTF**: 表示富文本格式（Rich Text Format）数据。
// 6. **NSPasteboardTypeRTFD**: 表示富文本格式文档（Rich Text Format Document）数据。
// 7. **NSPasteboardTypeHTML**: 表示 HTML 格式的数据。
// 8. **NSPasteboardTypeTabularText**: 表示表格文本数据。
// 9. **NSPasteboardTypeFont**: 表示字体数据。
// 10. **NSPasteboardTypeRuler**: 表示标尺数据（通常用于文本编辑器中的缩进和制表符设置）。
// 11. **NSPasteboardTypeColor**: 表示颜色数据。
// 12. **NSPasteboardTypeSound**: 表示声音数据。
// 13. **NSPasteboardTypeMultipleTextSelection**: 表示多重文本选择数据。
// 14. **NSPasteboardTypeFindPanelSearchOptions**: 表示查找面板搜索选项数据。
//
// 这些类型标识符用于在应用程序之间传递和共享数据，确保数据以正确的格式被读取和处理。
