use std::thread;
use std::time::Duration;

use fltk::{app, browser::Browser, frame::Frame, prelude::*, window::Window};

use crate::core::pasteboard::{Pasteboard, PasteboardContent};

mod core;

fn main() {
    let my_app = app::App::default();
    let mut wind = Window::new(100, 100, 400, 300, "Clipboard Content");

    let mut pasteboard = Pasteboard::new();
    let mut contents : Vec<PasteboardContent> = vec![];

    let mut frame = Frame::new(10, 10, 380, 280, "");
    let mut browser = Browser::new(10, 10, 380, 280, "");

    // 每1秒更新一次
    let mut counter = 0;
    let _ = thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(1));
        counter += 1;
        unsafe {
            let new_contents = pasteboard.get_contents();
            for content in new_contents {
                // Add each new content to the browser
                let content_str = format!("{:?}", content);
                browser.add(&content_str);
            }
        }
    });

    // Show the window
    wind.end();
    wind.show();
    my_app.run().unwrap();
}

