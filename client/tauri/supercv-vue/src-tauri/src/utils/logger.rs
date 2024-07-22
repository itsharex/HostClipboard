use flexi_logger::{colored_opt_format, opt_format, Cleanup, Criterion, Duplicate, FileSpec, Logger, Naming, LoggerHandle};
use std::time::Duration;

use crate::utils::config::CONFIG;
pub fn init_logger(log_level: Option<i32>, sql_level: Option<i32>) {
    let logger_str = format!(
        "{}, sqlx={}",
        convert_log(log_level),
        convert_log(sql_level)
    );
    println!("logger setting: {}", logger_str);
    Logger::try_with_str(logger_str)
        .unwrap()
        .log_to_file(FileSpec::default().directory(CONFIG.logs_path.to_str().unwrap()))
        .format_for_files(opt_format)
        .format_for_stderr(colored_opt_format)
        .rotate(
            Criterion::Size(10 * 1024 * 1024), // 按大小切分，10MB
            Naming::Timestamps,
            Cleanup::KeepLogFiles(10), // 保留10个日志文件
        )
        .duplicate_to_stderr(Duplicate::All)
        .start()
        .unwrap_or_else(|e| panic!("Logger init失败 err: {:?}", e));

}

pub fn convert_log(log_int: Option<i32>) -> String {
    match log_int {
        Some(0) => "trace".to_string(),
        Some(1) => "error".to_string(),
        Some(2) => "warn".to_string(),
        Some(3) => "info".to_string(),
        Some(4) => "debug".to_string(),
        Some(5) => "trace".to_string(),
        _ => "debug".to_string(),
    }
}

#[macro_export]
macro_rules! time_it {
    ($start:expr, $func:expr) => {{
        let result = $func(); // 直接执行闭包
        let duration = $start.elapsed();
        let elapsed_micros = duration.as_micros();
        let elapsed_secs = duration.as_secs_f64();
        debug!(
            "file={}:{} elapsed={}µs elapsed_secs={:.6e}",
            file!(),
            line!(),
            elapsed_micros,
            elapsed_secs
        );
        result
    }};
}

//tokio_time_it!(|| item.save_path());
#[macro_export]
macro_rules! tokio_time_it {
    ($func:expr) => {{
        let start = tokio::time::Instant::now();
        time_it!(start, $func)
    }};
}

#[macro_export]
macro_rules! std_time_it {
    ($func:expr) => {{
        let start = std::time::Instant::now();
        time_it!(start, $func)
    }};
}
