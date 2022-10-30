// This binary is used solely for testing purposes (see `tests` dir)
//
// Ideally it would only be built when running tests,
// but I'm not sure how to do that rn.
use log::{LevelFilter, Metadata, Record};
use std::{
    io::Write,
    sync::{Arc, Mutex},
};

struct ConsoleLogger {
    file: Arc<Mutex<std::fs::File>>,
}

impl log::Log for ConsoleLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut file = self.file.lock().unwrap();
            let target = record.target().to_string();
            let message = record.args().to_string();

            // capture lsp_server log messages
            if target.starts_with("lsp_server") {
                // stdin
                if message.starts_with("< ") {
                    let value: serde_json::Value =
                        serde_json::from_str(message.trim_start_matches("< ")).unwrap();

                    // Filter out the initialize message?
                    //if let serde_json::Value::Object(ref object) = value {
                    //    if let Some(serde_json::Value::String(method)) = object.get("method") {
                    //        if method == "initialize" {
                    //            writeln!(file, "INITIALIZE\n").unwrap();
                    //            return;
                    //        }
                    //    }
                    //}

                    let pretty = serde_json::to_string_pretty(&value).unwrap();
                    writeln!(file, "LSP INPUT\n{pretty}\n").unwrap();
                    return;
                }
                // stdout
                if message.starts_with("> ") {
                    let value: serde_json::Value =
                        serde_json::from_str(message.trim_start_matches("> ")).unwrap();
                    let pretty = serde_json::to_string_pretty(&value).unwrap();
                    writeln!(file, "LSP_OUTPUT\n{pretty}\n").unwrap();
                }

                // unreachable?
            }
            // capture our (ditto-lsp) log messages
            else if target.starts_with("ditto_lsp") {
                let level = record.level().to_string();
                writeln!(file, "{level}\n{message}\n").unwrap();
            }
        }
    }
    fn flush(&self) {
        let mut file = self.file.lock().unwrap();
        file.flush().expect("error flushing log file");
    }
}

fn main() -> miette::Result<()> {
    let file = std::fs::File::create("logs.txt").unwrap();

    let logger = ConsoleLogger {
        file: Arc::new(Mutex::new(file)),
    };
    log::set_boxed_logger(Box::new(logger)).unwrap();
    log::set_max_level(LevelFilter::Trace);
    ditto_lsp::main()
}
