use std::path::PathBuf;
use std::sync::Arc;

use once_cell::sync::Lazy;

pub static CONFIG: Lazy<Arc<Config>> = Lazy::new(|| Arc::new(Config::new()));

pub struct Config {
    pub db_path: PathBuf,
    pub files_path: PathBuf,
    pub logs_path: PathBuf,
}

impl Config {
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("读取用户家目录失败");
        let db_path = home.join(".cache/super-cv/db");
        let files_path = home.join(".cache/super-cv/files");
        let logs_path = home.join(".cache/super-cv/logs");
        for p in [&db_path, &files_path, &logs_path].iter() {
            if !p.exists() {
                std::fs::create_dir_all(p)
                    .expect(format!("创建目录 {} 失败", p.display()).as_str());
            }
        }
        Self {
            db_path,
            files_path,
            logs_path,
        }
    }
}
