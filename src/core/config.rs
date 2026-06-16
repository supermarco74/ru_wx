//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Configuration and paths (`wxConfig`, `wxStandardPaths`, …).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Key/value settings store (`wxConfig`).
#[derive(Debug, Default)]
pub struct Config {
    path: PathBuf,
    values: HashMap<String, String>,
    dirty: bool,
}

impl Config {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        let mut cfg = Self {
            path,
            values: HashMap::new(),
            dirty: false,
        };
        cfg.read_disk();
        cfg
    }

    pub fn read(&self, key: &str, default: &str) -> String {
        self.values.get(key).cloned().unwrap_or_else(|| default.to_string())
    }

    pub fn write(&mut self, key: &str, value: &str) {
        self.values.insert(key.to_string(), value.to_string());
        self.dirty = true;
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let body: String = self
            .values
            .iter()
            .map(|(k, v)| format!("{k}={v}\n"))
            .collect();
        fs::write(&self.path, body)?;
        self.dirty = false;
        Ok(())
    }

    fn read_disk(&mut self) {
        if let Ok(text) = fs::read_to_string(&self.path) {
            for line in text.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    self.values.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
        }
    }
}

/// Well-known application directories (`wxStandardPaths`).
#[derive(Debug, Clone)]
pub struct StandardPaths {
    app_name: String,
}

impl StandardPaths {
    pub fn new(app_name: &str) -> Self {
        Self {
            app_name: app_name.to_string(),
        }
    }

    pub fn config_dir(&self) -> PathBuf {
        dirs_config().join(&self.app_name)
    }

    pub fn data_dir(&self) -> PathBuf {
        dirs_data().join(&self.app_name)
    }

    pub fn user_config_file(&self, leaf: &str) -> PathBuf {
        self.config_dir().join(leaf)
    }
}

fn dirs_config() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn dirs_data() -> PathBuf {
    dirs_config()
}

/// Locale placeholder (`wxLocale`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Locale {
    language: u32,
}

impl Locale {
    pub const fn new(language: u32) -> Self {
        Self { language }
    }

    pub const fn language(&self) -> u32 {
        self.language
    }
}

/// Replaceable watcher event callback slot.
type WatcherEventHandler = std::cell::RefCell<
    Option<Box<dyn FnMut(&crate::core::filesystem_watcher_event::FileSystemWatcherEvent)>>,
>;

static WATCHER_REGISTRY: std::sync::Mutex<Vec<usize>> = std::sync::Mutex::new(Vec::new());

fn register_watcher(ptr: usize) {
    if ptr == 0 {
        return;
    }
    let mut reg = WATCHER_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    if !reg.contains(&ptr) {
        reg.push(ptr);
    }
}

fn unregister_watcher(ptr: usize) {
    let mut reg = WATCHER_REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    reg.retain(|&p| p != ptr);
}

/// Drain pending events on every watcher that called [`FileSystemWatcher::start`].
/// Called automatically from the frame idle loop; may also be invoked manually.
pub fn poll_registered_filesystem_watchers() {
    let registry = WATCHER_REGISTRY
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    for ptr in registry {
        if ptr != 0 {
            // SAFETY: pointers are registered in `start` and cleared in `stop`/`Drop`.
            unsafe {
                let w = &*(ptr as *const FileSystemWatcher);
                w.poll();
            }
        }
    }
}

/// Directory change notifications (`wxFileSystemWatcher`).
pub struct FileSystemWatcher {
    paths: Vec<PathBuf>,
    on_event: WatcherEventHandler,
    pending: std::sync::Arc<std::sync::Mutex<Vec<crate::core::filesystem_watcher_event::FileSystemWatcherEvent>>>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    threads: std::cell::RefCell<Vec<std::thread::JoinHandle<()>>>,
    running: std::cell::RefCell<bool>,
}

impl Default for FileSystemWatcher {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            on_event: std::cell::RefCell::new(None),
            pending: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            stop: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            threads: std::cell::RefCell::new(Vec::new()),
            running: std::cell::RefCell::new(false),
        }
    }
}

impl std::fmt::Debug for FileSystemWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileSystemWatcher")
            .field("paths", &self.paths)
            .finish_non_exhaustive()
    }
}

impl FileSystemWatcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, path: impl AsRef<Path>) {
        self.paths.push(path.as_ref().to_path_buf());
    }

    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    /// Register a callback for filesystem change notifications.
    pub fn on_event<F: FnMut(&crate::core::filesystem_watcher_event::FileSystemWatcherEvent) + 'static>(
        &self,
        f: F,
    ) {
        *self.on_event.borrow_mut() = Some(Box::new(f));
    }

    /// Start native watcher threads for all registered paths.
    pub fn start(&mut self) {
        if *self.running.borrow() {
            return;
        }
        self.stop.store(false, std::sync::atomic::Ordering::SeqCst);
        for path in self.paths.clone() {
            let pending = std::sync::Arc::clone(&self.pending);
            let stop = std::sync::Arc::clone(&self.stop);
            let handle = std::thread::spawn(move || watch_path(path, pending, stop));
            self.threads.borrow_mut().push(handle);
        }
        *self.running.borrow_mut() = true;
        register_watcher(self as *const _ as usize);
    }

    /// Stop watcher threads.
    pub fn stop(&mut self) {
        unregister_watcher(self as *const _ as usize);
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        while let Some(handle) = self.threads.borrow_mut().pop() {
            let _ = handle.join();
        }
        *self.running.borrow_mut() = false;
    }

    /// Drain pending native events and invoke the registered callback.
    pub fn poll(&self) {
        let events: Vec<_> = self
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .drain(..)
            .collect();
        for event in events {
            if let Some(ref mut cb) = *self.on_event.borrow_mut() {
                cb(&event);
            }
        }
    }

    /// Inject a change event (tests and manual notification).
    pub fn notify_change(
        &self,
        path: impl AsRef<Path>,
        change_type: crate::core::filesystem_watcher_event::FileSystemChangeType,
    ) {
        if let Some(ref mut cb) = *self.on_event.borrow_mut() {
            cb(&crate::core::filesystem_watcher_event::FileSystemWatcherEvent::new(
                path.as_ref().display().to_string(),
                change_type,
            ));
        }
    }
}

impl Drop for FileSystemWatcher {
    fn drop(&mut self) {
        unregister_watcher(self as *const _ as usize);
        if *self.running.borrow() {
            self.stop();
        }
    }
}

use crate::core::filesystem_watcher_event::{FileSystemChangeType, FileSystemWatcherEvent};

fn watch_path(
    path: PathBuf,
    pending: std::sync::Arc<std::sync::Mutex<Vec<FileSystemWatcherEvent>>>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    #[cfg(target_os = "windows")]
    {
        watch_path_windows(path, pending, stop);
    }
    #[cfg(not(target_os = "windows"))]
    {
        watch_path_poll(path, pending, stop);
    }
}

#[cfg(target_os = "windows")]
fn watch_path_windows(
    path: PathBuf,
    pending: std::sync::Arc<std::sync::Mutex<Vec<FileSystemWatcherEvent>>>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    use crate::platform::win32::to_wide;
    use std::ffi::c_void;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, ReadDirectoryChangesW, FILE_ACTION_ADDED, FILE_ACTION_MODIFIED,
        FILE_ACTION_REMOVED, FILE_ACTION_RENAMED_NEW_NAME, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_LIST_DIRECTORY, FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE,
        FILE_NOTIFY_INFORMATION, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
        OPEN_EXISTING,
    };

    let wide = to_wide(&path.display().to_string());
    // SAFETY: directory handle for ReadDirectoryChangesW.
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_LIST_DIRECTORY,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return;
    }

    let mut buffer = vec![0u8; 16 * 1024];
    while !stop.load(std::sync::atomic::Ordering::SeqCst) {
        let mut bytes_returned = 0u32;
        let ok = unsafe {
            ReadDirectoryChangesW(
                handle,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
                1,
                FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_LAST_WRITE,
                &mut bytes_returned,
                std::ptr::null_mut(),
                None,
            )
        };
        if ok == 0 || bytes_returned == 0 {
            std::thread::sleep(std::time::Duration::from_millis(200));
            continue;
        }
        let mut offset = 0usize;
        while offset < bytes_returned as usize {
            let info = unsafe {
                &*(buffer.as_ptr().add(offset) as *const FILE_NOTIFY_INFORMATION)
            };
            let name_len = (info.FileNameLength / 2) as usize;
            let name_wide =
                unsafe { std::slice::from_raw_parts(info.FileName.as_ptr(), name_len) };
            let name = String::from_utf16_lossy(name_wide);
            let full = path.join(name);
            let change_type = match info.Action {
                FILE_ACTION_ADDED | FILE_ACTION_RENAMED_NEW_NAME => FileSystemChangeType::Create,
                FILE_ACTION_REMOVED => FileSystemChangeType::Delete,
                FILE_ACTION_MODIFIED => FileSystemChangeType::Modify,
                _ => FileSystemChangeType::Modify,
            };
            if let Ok(mut queue) = pending.lock() {
                queue.push(FileSystemWatcherEvent::new(
                    full.display().to_string(),
                    change_type,
                ));
            }
            if info.NextEntryOffset == 0 {
                break;
            }
            offset += info.NextEntryOffset as usize;
        }
    }
    unsafe {
        CloseHandle(handle);
    }
}

#[cfg(not(target_os = "windows"))]
fn watch_path_poll(
    path: PathBuf,
    pending: std::sync::Arc<std::sync::Mutex<Vec<FileSystemWatcherEvent>>>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    let mut last = fs::metadata(&path).and_then(|m| m.modified()).ok();
    while !stop.load(std::sync::atomic::Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let modified = fs::metadata(&path).and_then(|m| m.modified()).ok();
        if modified != last {
            last = modified;
            if let Ok(mut queue) = pending.lock() {
                queue.push(FileSystemWatcherEvent::new(
                    path.display().to_string(),
                    FileSystemChangeType::Modify,
                ));
            }
        }
    }
}

/// Single-instance guard (`wxSingleInstanceChecker`).
#[derive(Debug)]
pub struct SingleInstanceChecker {
    name: String,
}

impl SingleInstanceChecker {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn is_another_running(&self) -> bool {
        let lock = StandardPaths::new(&self.name).config_dir().join("instance.lock");
        if lock.exists() {
            return true;
        }
        if let Some(parent) = lock.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&lock, std::process::id().to_string());
        false
    }
}
