extern crate chrono;

use std::cmp::PartialEq;
use std::fmt;
use std::io;
use std::sync::{Arc, Mutex};
use cocoa::base::nil;

use crate::core::safe_objc_ptr::SafeObjcPtr;


#[derive(Debug, Clone)]
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
    pub item: Arc<Mutex<SafeObjcPtr>>,
}

impl ToString for ContentType {
    fn to_string(&self) -> String {
        match self {
            ContentType::Text => "Text".to_string(),
            ContentType::File => "File".to_string(),
            ContentType::FileImage => "FileImage".to_string(),
            ContentType::PBImage => "PBImage".to_string(),
            ContentType::PBOther => "PBOther".to_string(),
        }
    }
}


impl PartialEq for ContentType {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}


impl PasteboardContent {
    // 创建文本类型的 PasteboardContent
    pub fn new(
        content_text: String,
        content_type: ContentType,
        content: Option<Vec<u8>>,
        item: *mut objc::runtime::Object,
    ) -> Self {
        let text_content = if content_type == ContentType::Text {
            content_text
        } else {
            format!("[{}]: {}", content_type.to_string(), content_text)
        };
        let safe_item = SafeObjcPtr::new(item);
        PasteboardContent {
            text_content,
            content_type,
            content,
            item: Arc::new(Mutex::new(safe_item)),
        }
    }

    pub fn get_item(&self) -> *mut objc::runtime::Object {
        self.item.lock().unwrap().get()
    }

    pub fn read_content(&self) -> Result<(), io::Error> {
        // TODO: 读取content 二进制数据 并且存入content字段
        Ok(())
    }


    pub fn fake_new(num: usize) -> Vec<Self> {
        let mut result: Vec<PasteboardContent> = vec![];
        let empty_ptr = SafeObjcPtr::new(nil);
        let empty_item = Arc::new(Mutex::new(empty_ptr));

        for i in 0..num {
            let text = std::iter::repeat(format!("{} hi", i)).take(40
            ).collect();
            result.push(PasteboardContent {
                text_content: text,
                content_type: ContentType::Text,
                content: None,
                item: Arc::clone(&empty_item),
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
            item: Arc::clone(&self.item),
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

// #[cfg(test)]
// mod tests {
//     use crate::core::pasteboard::{ContentType, Pasteboard};
//
//     #[test]
//     fn test_build() {
//         println!("build pass!")
//     }
//
//     #[test]
//     fn new_type() {
//         let c_type = ContentType::File;
//         println!("{:?}", c_type.to_string());
//     }
//
//     fn set_contents() {
//         // 获取上上个剪切板内容
//         let mut pasteboard = Pasteboard::new();
//     }
// }