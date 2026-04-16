use std::io::Write;
use std::path::Path;

pub struct Reporter {
    log_file: Option<std::fs::File>,
}

impl Reporter {
    /// Opens (or creates) the log file if `log_path` is Some.
    pub fn new(log_path: Option<&Path>) -> Self {
        let log_file = log_path.and_then(|p| {
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(p)
                .ok()
        });
        Self { log_file }
    }

    pub fn log(&mut self, line: &str) {
        println!("{line}");
        if let Some(f) = &mut self.log_file {
            let _ = writeln!(f, "{line}");
        }
    }
}
