use crate::db::storage::data_root;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

const MAX_LOG_SIZE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_LOG_FILES_KEPT: usize = 10;

pub fn logs_dir() -> PathBuf {
    data_root().join("logs")
}

fn generate_timestamped_log_path() -> PathBuf {
    let now = chrono::Local::now();
    let filename = now.format("pawstash_%Y-%m-%d_%H-%M-%S.log").to_string();
    logs_dir().join(filename)
}

fn cleanup_old_logs(dir: &PathBuf, max_files: usize) {
    if let Ok(entries) = fs::read_dir(dir) {
        let mut log_files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.is_file()
                    && p.extension().and_then(|ext| ext.to_str()) == Some("log")
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with("pawstash_"))
                        .unwrap_or(false)
            })
            .collect();

        log_files.sort();

        if log_files.len() > max_files {
            let to_remove = log_files.len() - max_files;
            for path in log_files.iter().take(to_remove) {
                let _ = fs::remove_file(path);
            }
        }
    }
}

pub fn find_latest_log_path() -> PathBuf {
    let dir = logs_dir();
    if let Ok(entries) = fs::read_dir(&dir) {
        let mut log_files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.is_file()
                    && p.extension().and_then(|ext| ext.to_str()) == Some("log")
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with("pawstash_"))
                        .unwrap_or(false)
            })
            .collect();

        log_files.sort();
        if let Some(latest) = log_files.pop() {
            return latest;
        }
    }
    generate_timestamped_log_path()
}

pub fn log_file_path() -> PathBuf {
    if let Ok(guard) = FILE_WRITER.lock() {
        if let Some(writer) = guard.as_ref() {
            return writer.path.clone();
        }
    }
    find_latest_log_path()
}

struct RotatingFileWriter {
    path: PathBuf,
    max_size: u64,
    file: Option<File>,
}

impl RotatingFileWriter {
    fn new(path: PathBuf, max_size: u64) -> Self {
        let mut writer = Self {
            path,
            max_size,
            file: None,
        };
        writer.open_file();
        writer
    }

    fn open_file(&mut self) {
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
            cleanup_old_logs(&parent.to_path_buf(), MAX_LOG_FILES_KEPT);
        }
        self.file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .ok();
    }

    fn rotate_if_needed(&mut self) {
        if let Ok(metadata) = fs::metadata(&self.path) {
            if metadata.len() >= self.max_size {
                self.file = None;
                self.path = generate_timestamped_log_path();
                self.open_file();
            }
        }
    }
}

impl Write for RotatingFileWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.rotate_if_needed();
        if self.file.is_none() {
            self.open_file();
        }
        if let Some(file) = &mut self.file {
            file.write(buf)
        } else {
            Ok(buf.len())
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(file) = &mut self.file {
            file.flush()
        } else {
            Ok(())
        }
    }
}

static FILE_WRITER: Mutex<Option<RotatingFileWriter>> = Mutex::new(None);

struct SharedFileWriter;

impl Write for SharedFileWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Ok(mut guard) = FILE_WRITER.lock() {
            if let Some(writer) = guard.as_mut() {
                return writer.write(buf);
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Ok(mut guard) = FILE_WRITER.lock() {
            if let Some(writer) = guard.as_mut() {
                return writer.flush();
            }
        }
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for SharedFileWriter {
    type Writer = SharedFileWriter;

    fn make_writer(&'a self) -> Self::Writer {
        SharedFileWriter
    }
}

pub fn init_logging() {
    let dir = logs_dir();
    let _ = fs::create_dir_all(&dir);

    let initial_path = generate_timestamped_log_path();
    let writer = RotatingFileWriter::new(initial_path, MAX_LOG_SIZE_BYTES);
    if let Ok(mut guard) = FILE_WRITER.lock() {
        *guard = Some(writer);
    }

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new(
                "info,pawstash=debug,pawstash_lib=debug,frontend=debug,keyring=warn,hyper=warn,reqwest=warn,h2=warn,tower=warn,tokio=warn,rusqlite=warn"
            )
        });

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(SharedFileWriter)
        .with_ansi(false)
        .with_target(true)
        .with_level(true);

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_target(true)
        .with_level(true);

    let _ = tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(stdout_layer)
        .try_init();

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
        "Pawstash initialized"
    );
}

pub fn read_recent_logs(max_lines: usize) -> Result<String, String> {
    let path = log_file_path();
    if !path.exists() {
        return Ok(String::new());
    }

    let file = File::open(&path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = Vec::new();

    for l in reader.lines().map_while(Result::ok) {
        lines.push(l);
    }

    if lines.len() > max_lines {
        let start = lines.len() - max_lines;
        Ok(lines[start..].join("\n"))
    } else {
        Ok(lines.join("\n"))
    }
}

pub fn clear_log_file() -> Result<(), String> {
    if let Ok(mut guard) = FILE_WRITER.lock() {
        if let Some(writer) = guard.as_mut() {
            writer.file = None;
        }
    }
    let dir = logs_dir();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let p = entry.path();
            if p.is_file() && p.extension().and_then(|ext| ext.to_str()) == Some("log") {
                let _ = fs::remove_file(p);
            }
        }
    }
    let new_path = generate_timestamped_log_path();
    if let Ok(mut guard) = FILE_WRITER.lock() {
        *guard = Some(RotatingFileWriter::new(new_path, MAX_LOG_SIZE_BYTES));
    }
    Ok(())
}
