#[macro_use]
extern crate lazy_static;

use std::sync::RwLock;

const MAX_STORED_LOGS: usize = 400;

lazy_static! {
    pub static ref CONSOLE_LOGS: RwLock<Vec<String>> = RwLock::new(Vec::new());
}

#[macro_export]
macro_rules! console_info {
    () => (println!(""));
    ($($arg:tt)+) => {
        println!("[INFO] {}", format!($($arg)+));
        bevy_kajiya_logger::push_console_log(format!("[INFO] {}", format!($($arg)+)))
    };
}

#[macro_export]
macro_rules! console_debug {
    () => (println!(""));
    ($($arg:tt)+) => {
        println!("[DEBUG] {}", format!($($arg)+));
        bevy_kajiya_logger::push_console_log(format!("[DEBUG] {}", format!($($arg)+)))
    };
}

pub fn push_console_log(log_message: String) {
    let mut logs = crate::CONSOLE_LOGS.write().unwrap();
    if logs.len() > MAX_STORED_LOGS {
        logs.remove(0);
    }
    logs.push(log_message);
}

pub fn get_console_logs() -> Vec<String> {
    crate::CONSOLE_LOGS.read().unwrap().to_vec()
}