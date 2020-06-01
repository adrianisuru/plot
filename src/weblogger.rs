use log::{Level, Metadata, Record};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn console_log(s: &str);
}

pub struct WebLogger;

impl log::Log for WebLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            console_log(&format!("[{}] {}", record.level(), record.args()));
        }
    }

    fn flush(&self) {}
}
