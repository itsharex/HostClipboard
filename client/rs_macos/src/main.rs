use std::thread;
use std::time::Duration;
use fltk::{
    app,
    browser::HoldBrowser,
    frame::Frame,
    input::Input,
    prelude::*,
    window::Window,
    enums::{Event, Key},
};
use unicode_segmentation::UnicodeSegmentation;

mod core;
use crate::core::pasteboard::Pasteboard;
use crate::core::pasteboard_content::PasteboardContent;

fn main() {
    let app = app::App::default().load_system_fonts();
    let mut wind = Window::new(100, 100, 800, 600, "剪贴板内容");

    // 顶部的文本输入框
    let mut input = Input::new(10, 10, 780, 30, "");

    let mut browser = HoldBrowser::new(10, 50, 380, 540, "");
    let browser_max_chars = browser.width() as usize / 8;
    // 设置字体大小
    browser.set_text_size(16);

    // 右侧展示区域
    let mut display_frame = Frame::new(400, 50, 390, 540, "");
    display_frame.set_frame(fltk::enums::FrameType::BorderBox);

    wind.end();
    wind.show();

    let mut pasteboard = Pasteboard::new();

    // 更新线程
    let (sender, receiver) = app::channel::<Vec<PasteboardContent>>();

    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(1));
        let new_contents = unsafe { pasteboard.get_contents() };
        sender.send(new_contents);
    });

    wind.end();
    wind.show();

    // Initial browser selection index
    let mut selected_index = 0;

    input.handle({
        let mut browser = browser.clone();
        move |_, ev| match ev {
            Event::KeyDown => {
                match app::event_key() {
                    Key::Up => {
                        if selected_index > 1 {
                            selected_index -= 1;
                            browser.select(selected_index);
                            browser.redraw();
                        }
                        true
                    }
                    Key::Down => {
                        if selected_index < browser.size() {
                            selected_index += 1;
                            browser.select(selected_index);
                            browser.redraw();
                        }
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    });

    while app.wait() {
        if let Some(contents) = receiver.recv() {
            for content in contents {
                let mut show_text = content.text_content.trim().replace("\n", " ");

                show_text = if show_text.graphemes(true).count() > browser_max_chars {
                    show_text = show_text.graphemes(true).take(browser_max_chars).collect::<String>();
                    show_text.push_str("...");
                    show_text
                } else {
                    show_text
                };

                browser.insert(1, show_text.as_str());
                browser.redraw();
            }
        }
    }
}
