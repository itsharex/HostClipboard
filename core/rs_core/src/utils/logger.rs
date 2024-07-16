use crate::utils::config::CONFIG;
use flexi_logger::{
    colored_opt_format, opt_format, Cleanup, Criterion, Duplicate, FileSpec, Logger, Naming,
};

pub fn init_logger() {
    Logger::try_with_str("debug")
        .unwrap()
        .log_to_file(FileSpec::default().directory(CONFIG.logs_path.to_str().unwrap()))
        .format_for_files(opt_format)
        .format_for_stderr(colored_opt_format)
        .rotate(
            Criterion::Size(10 * 1024 * 1024), // 按大小切分，10MB
            Naming::Timestamps,
            Cleanup::KeepLogFiles(10), // 保留30个日志文件（大约一个月）
        )
        .duplicate_to_stderr(Duplicate::All)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
}



#[macro_export]
macro_rules! time_it {
    ($start:expr, $func:expr) => {{
        let result = $func(); // 直接执行闭包
        let duration = $start.elapsed();
        let elapsed_micros = duration.as_micros();
        let elapsed_secs = duration.as_secs_f64();
        debug!(
            "location={}:{} elapsed={}µs elapsed_secs={:.6e}",
            file!(),
            line!(),
            elapsed_micros,
            elapsed_secs
        );
        result
    }};
}

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
