//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! File-based log target with size-based rotation.
//!
//! [`RotatingFileTarget`] writes formatted log records to a file on
//! disk so that errors can be inspected locally after the fact.
//! When the active file grows past `max_bytes`, it is rotated:
//! `app.log` becomes `app.1.log`, the previous `app.1.log` becomes
//! `app.2.log` and so on, keeping at most `max_backups` backups.
//!
//! Records at [`LogLevel::Error`] or [`LogLevel::FatalError`] are
//! flushed immediately so that a crash right after the error does
//! not lose the message.

use super::formatter::LogFormatter;
use super::levels::LogLevel;
use super::record::LogRecord;
use super::target::LogTarget;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// A log target that appends records to a file with size rotation.
///
/// All methods are thread-safe; writes are serialized through an
/// internal mutex. I/O errors (disk full, permission denied, …) are
/// never propagated to the caller and never panic: logging must not
/// take the application down.
pub struct RotatingFileTarget {
    inner: Mutex<FileInner>,
    path: PathBuf,
    max_bytes: u64,
    max_backups: u32,
    formatter: LogFormatter,
}

struct FileInner {
    file: Option<File>,
    written: u64,
}

impl RotatingFileTarget {
    /// Default maximum file size before rotation: 1 MiB.
    pub const DEFAULT_MAX_BYTES: u64 = 1024 * 1024;
    /// Default number of rotated backups kept on disk.
    pub const DEFAULT_MAX_BACKUPS: u32 = 5;

    /// Open (creating if needed) the log file at `path` with default
    /// rotation limits. Parent directories are created as needed.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self::with_limits(path, Self::DEFAULT_MAX_BYTES, Self::DEFAULT_MAX_BACKUPS)
    }

    /// Like [`RotatingFileTarget::new`] with explicit rotation limits.
    ///
    /// * `max_bytes` — rotate when the active file exceeds this size.
    /// * `max_backups` — number of `<name>.N.log` backups to keep.
    pub fn with_limits(path: impl Into<PathBuf>, max_bytes: u64, max_backups: u32) -> Self {
        let path = path.into();
        let inner = Mutex::new(FileInner {
            file: None,
            written: 0,
        });
        let target = Self {
            inner,
            path,
            max_bytes: max_bytes.max(1024),
            max_backups,
            formatter: LogFormatter::new().with_thread(true),
        };
        target.open_if_needed();
        target
    }

    /// Open the log file in append mode, creating parent dirs.
    /// Records the current size so rotation accounting is correct
    /// across restarts.
    fn open_if_needed(&self) {
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        if inner.file.is_some() {
            return;
        }
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let opened = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path);
        match opened {
            Ok(file) => {
                let written = file.metadata().map(|m| m.len()).unwrap_or(0);
                inner.file = Some(file);
                inner.written = written;
            }
            Err(_) => {
                inner.file = None;
            }
        }
    }

    /// Rotate the log files: `app.log` → `app.1.log`, …, dropping the
    /// oldest backup when `max_backups` is exceeded.
    fn rotate(&self, inner: &mut FileInner) {
        // Close the current file before renaming it.
        inner.file = None;
        inner.written = 0;

        let backup_path = |n: u32| -> PathBuf { numbered_backup(&self.path, n) };

        // Drop the oldest backup.
        if self.max_backups > 0 {
            let _ = fs::remove_file(backup_path(self.max_backups));
            // Shift backups: N-1 → N.
            for n in (1..self.max_backups).rev() {
                let _ = fs::rename(backup_path(n), backup_path(n + 1));
            }
            // Active file → first backup.
            let _ = fs::rename(&self.path, backup_path(1));
        } else {
            let _ = fs::remove_file(&self.path);
        }

        // Reopen a fresh active file.
        let opened = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path);
        if let Ok(file) = opened {
            inner.file = Some(file);
        }
    }

    /// The path of the active log file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl LogTarget for RotatingFileTarget {
    fn log_record(&self, record: &LogRecord) {
        let line = self.formatter.format(record);
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        if inner.file.is_none() {
            drop(inner);
            self.open_if_needed();
            inner = match self.inner.lock() {
                Ok(g) => g,
                Err(poisoned) => poisoned.into_inner(),
            };
        }
        let bytes = line.as_bytes();
        if inner.written > 0 && inner.written + bytes.len() as u64 + 1 > self.max_bytes {
            self.rotate(&mut inner);
        }
        // Borrow `inner.file` first; update `inner.written` only after
        // the file borrow ends (disjoint field borrows do not work
        // through a `MutexGuard`).
        let write_ok = if let Some(ref mut file) = inner.file {
            let wrote = file.write_all(bytes).and_then(|_| file.write_all(b"\n"));
            // Errors and above: make sure the message hits the disk
            // before anything else can go wrong.
            if wrote.is_ok() && record.level <= LogLevel::Error {
                let _ = file.flush();
            }
            wrote.is_ok()
        } else {
            false
        };
        if write_ok {
            inner.written += bytes.len() as u64 + 1;
        }
    }

    fn flush(&self) {
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Some(ref mut file) = inner.file {
            let _ = file.flush();
        }
    }
}

/// Compute the path of the `n`-th backup for `path`:
/// `/dir/app.log` → `/dir/app.1.log` for `n == 1`.
fn numbered_backup(path: &Path, n: u32) -> PathBuf {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "log".to_string());
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_else(|| "log".to_string());
    let name = format!("{}.{}.{}", stem, n, ext);
    match path.parent() {
        Some(dir) => dir.join(name),
        None => PathBuf::from(name),
    }
}

impl io::Write for RotatingFileTarget {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let msg = String::from_utf8_lossy(buf);
        self.log_record(&LogRecord::new(
            LogLevel::Message,
            "",
            msg.trim_end_matches('\n').to_string(),
        ));
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        LogTarget::flush(self);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Arc;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ru_wx_log_test_{}_{}_{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn writes_records_to_file() {
        let dir = temp_dir("write");
        let path = dir.join("app.log");
        let target = RotatingFileTarget::new(&path);
        target.log_record(&LogRecord::new(LogLevel::Error, "test", "boom".into()));
        LogTarget::flush(&target);
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("[ERROR]"));
        assert!(content.contains("[test]"));
        assert!(content.contains("boom"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rotates_when_max_bytes_exceeded() {
        let dir = temp_dir("rotate");
        let path = dir.join("app.log");
        let target = RotatingFileTarget::with_limits(&path, 1024, 2);
        for i in 0..100 {
            target.log_record(&LogRecord::new(
                LogLevel::Message,
                "test",
                format!("line number {:03} with some padding to fill space", i),
            ));
        }
        LogTarget::flush(&target);
        assert!(path.exists());
        assert!(numbered_backup(&path, 1).exists());
        // With max_backups == 2 the third backup must not exist.
        assert!(!numbered_backup(&path, 3).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RotatingFileTarget>();
    }

    #[test]
    fn works_as_shared_target() {
        let dir = temp_dir("shared");
        let path = dir.join("app.log");
        let target: Arc<dyn LogTarget> = Arc::new(RotatingFileTarget::new(&path));
        let t2 = Arc::clone(&target);
        let h = std::thread::spawn(move || {
            t2.log_record(&LogRecord::new(LogLevel::Message, "worker", "hi".into()));
        });
        h.join().unwrap();
        LogTarget::flush(target.as_ref());
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("hi"));
        let _ = fs::remove_dir_all(&dir);
    }
}
