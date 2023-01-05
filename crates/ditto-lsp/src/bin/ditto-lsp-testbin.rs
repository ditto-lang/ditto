// This binary is used solely for testing purposes (see `tests` dir)
//
// Ideally it would only be built when running tests,
// but I'm not sure how to do that rn.

use log::{LevelFilter, Metadata, Record};
use std::{
    io::Write,
    sync::{Arc, Mutex},
};

struct FileLogger {
    file: Arc<Mutex<std::fs::File>>,
}

impl log::Log for FileLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut file = self.file.lock().unwrap();
            let target = record.target().to_string();
            let message = record.args().to_string();

            // capture our (ditto-lsp) log messages
            if target.starts_with("ditto_lsp") {
                // let level = record.level().to_string();
                writeln!(file, "{message}\n").unwrap();
            }
        }
    }
    fn flush(&self) {
        let mut file = self.file.lock().unwrap();
        file.flush().expect("error flushing log file");
    }
}

#[tokio::main]
async fn main() {
    let file = std::fs::File::create("logs.txt").unwrap();

    let logger = FileLogger {
        file: Arc::new(Mutex::new(file)),
    };
    log::set_boxed_logger(Box::new(logger)).unwrap();
    log::set_max_level(LevelFilter::Trace);
    ditto_lsp::main_test().await
}
