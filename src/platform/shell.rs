//! Cross-platform shell helpers: drives, file operations, list-view
//! hit-testing, secrets, monitor checks.
//!
//! Porting target of `rust_explorer/src/platform/windows.rs` (SPEC §13:
//! if a helper is missing from `ru_wx`, it is added here). Every
//! function is dual-cfg: the Windows body talks to Win32
//! (`windows_sys`), the non-Windows body uses pure `std` or AppKit so
//! the same call sites compile on every target.

use std::path::{Path, PathBuf};

// ── Shared types ───────────────────────────────────────────────────

/// Valori `GetDriveTypeW` — condivisi fra Windows e le controparti
/// non-Windows di [`enumerate_drives`].
pub mod drive_kind {
    pub const DRIVE_UNKNOWN: u32 = 0;
    pub const DRIVE_NO_ROOT_DIR: u32 = 1;
    pub const DRIVE_REMOVABLE: u32 = 2;
    pub const DRIVE_FIXED: u32 = 3;
    pub const DRIVE_REMOTE: u32 = 4;
    pub const DRIVE_CDROM: u32 = 5;
    pub const DRIVE_RAMDISK: u32 = 6;
}

/// Un logical drive (Windows: `C:\`; macOS/Linux: volume montato).
#[derive(Debug, Clone)]
pub struct DriveInfo {
    /// Lettera di drive (Windows) o lettera sintetica assegnata in
    /// ordine di enumerazione sulle piattaforme senza lettere.
    pub letter: char,
    /// Testo mostrato nel tree: etichetta di volume + tipo.
    pub label: String,
    /// Una costante [`drive_kind`].
    pub kind: u32,
    /// Path radice del volume.
    pub root: PathBuf,
}

/// Errori restituiti dalle operazioni shell.
#[derive(Debug, Clone, Copy)]
pub enum ShellError {
    /// Non ancora implementato nella milestone corrente.
    NotImplemented,
    /// Permesso negato (ACL, UAC, file protetto).
    AccessDenied,
    /// File non trovato (`SE_ERR_FNF = 2`).
    FileNotFound,
    /// Path non trovato (`SE_ERR_PNF = 3`).
    PathNotFound,
    /// Nessuna applicazione associata al tipo di file
    /// (`SE_ERR_NOASSOC = 31`, `SE_ERR_ASSOCINCOMPLETE = 27`).
    NoAssociation,
    /// Il file `.exe` non è valido / immagine non valida
    /// (`SE_ERR_BAD_FORMAT = 11`, `ERROR_BAD_EXE_FORMAT`).
    BadFormat,
    /// File in uso / condivisione violata (`SE_ERR_SHARE = 26`).
    SharingViolation,
    /// DLL richiesta non trovata (`SE_ERR_DLLNOTFOUND = 32`).
    DllNotFound,
    /// Memoria insufficiente (`SE_ERR_OOM = 8`).
    OutOfMemory,
    /// Path sorgente o destinazione non valido.
    InvalidPath,
    /// Cestino pieno (per `recycle_paths`).
    RecycleBinFull,
    /// Altro errore del sistema (codice conservato per debug).
    Other(u32),
}

impl std::fmt::Display for ShellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShellError::NotImplemented => f.write_str("not implemented"),
            ShellError::AccessDenied => f.write_str("access denied"),
            ShellError::FileNotFound => f.write_str("file not found"),
            ShellError::PathNotFound => f.write_str("path not found"),
            ShellError::NoAssociation => f.write_str("no application associated"),
            ShellError::BadFormat => f.write_str("invalid file format"),
            ShellError::SharingViolation => f.write_str("file in use / sharing violation"),
            ShellError::DllNotFound => f.write_str("required DLL not found"),
            ShellError::OutOfMemory => f.write_str("out of memory"),
            ShellError::InvalidPath => f.write_str("invalid path"),
            ShellError::RecycleBinFull => f.write_str("recycle bin full"),
            ShellError::Other(code) => write!(f, "shell error 0x{code:08x}"),
        }
    }
}

impl std::error::Error for ShellError {}

/// Risultato del calcolo ricorsivo della dimensione di una cartella.
#[derive(Debug, Clone, Copy, Default)]
pub struct DirSize {
    /// Numero di file (non cartelle) incontrati.
    pub files: u64,
    /// Numero di sottocartelle (incluse quelle annidate).
    pub dirs: u64,
    /// Dimensione totale occupata in byte.
    pub bytes: u64,
}

fn drive_type_name(t: u32) -> &'static str {
    match t {
        drive_kind::DRIVE_FIXED => "Fixed",
        drive_kind::DRIVE_REMOVABLE => "Removable",
        drive_kind::DRIVE_CDROM => "CDROM",
        drive_kind::DRIVE_REMOTE => "Network",
        drive_kind::DRIVE_RAMDISK => "RAM Disk",
        _ => "Unknown",
    }
}

/// Map di un `std::io::Error` sugli [`ShellError`].
fn map_io_error(e: std::io::Error) -> ShellError {
    use std::io::ErrorKind;
    match e.kind() {
        ErrorKind::NotFound | ErrorKind::InvalidInput => ShellError::InvalidPath,
        ErrorKind::PermissionDenied => ShellError::AccessDenied,
        other => ShellError::Other(other as u32),
    }
}

// ── Edit cue banner ────────────────────────────────────────────────

/// Imposta il "cue banner" (testo grigio che sparisce quando si digita)
/// su un controllo EDIT.
///
/// Windows: `EM_SETCUEBANNER` (`wparam = 1` mostra il banner anche
/// con il focus). macOS: placeholder su `NSTextField`.
pub fn set_edit_cue_banner(hwnd: isize, text: &str) {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
        if hwnd == 0 {
            return;
        }
        let wide = crate::platform::to_wide(text);
        // SAFETY: HWND di un controllo EDIT valido; `wide` vive per
        // tutta la chiamata sincrona `SendMessageW`.
        unsafe {
            SendMessageW(
                hwnd as HWND,
                0x1501, /* EM_SETCUEBANNER */
                1,
                wide.as_ptr() as isize,
            );
        }
    }
    #[cfg(target_os = "macos")]
    {
        crate::platform::appkit::set_text_field_placeholder(hwnd, text);
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = (hwnd, text);
    }
}

// ── Drives ─────────────────────────────────────────────────────────

/// Enumerate the logical drives visible to the system.
///
/// Windows: `GetLogicalDrives` + `GetDriveTypeW` + `GetVolumeInformationW`.
/// macOS/Linux: `/` + le cartelle montate in `/Volumes`.
#[cfg(target_os = "windows")]
pub fn enumerate_drives() -> Vec<DriveInfo> {
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};
    let bitmask = unsafe { GetLogicalDrives() };
    let mut out = Vec::new();
    for i in 0..26u32 {
        if bitmask & (1 << i) == 0 {
            continue;
        }
        let letter = (b'A' + i as u8) as char;
        let root_str = format!("{}:\\", letter);
        let wide: Vec<u16> = root_str.encode_utf16().chain(std::iter::once(0)).collect();
        let dtype = unsafe { GetDriveTypeW(wide.as_ptr()) };

        let label = read_volume_label(letter).unwrap_or_else(|| format!("{}:", letter));
        let kind_label = drive_type_name(dtype);

        let display = if label == format!("{}:", letter) {
            format!("{} ({})", letter, kind_label)
        } else {
            format!("{} ({})", label, kind_label)
        };

        out.push(DriveInfo {
            letter,
            label: display,
            kind: dtype,
            root: PathBuf::from(root_str),
        });
    }
    out
}

#[cfg(not(target_os = "windows"))]
pub fn enumerate_drives() -> Vec<DriveInfo> {
    let mut out = Vec::new();
    push_mount(&mut out, "/");
    if let Ok(rd) = std::fs::read_dir("/Volumes") {
        let mut mounts: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        mounts.sort();
        for m in mounts {
            push_mount(&mut out, &m.to_string_lossy());
        }
    }
    out
}

/// Aggiunge un volume montato all'elenco. I path nascosti in
/// `/Volumes` (`.disk_images`, `.Trashes`…) vengono saltati; un
/// symlink in `/Volumes` è un automount di rete (`DRIVE_REMOTE`).
#[cfg(not(target_os = "windows"))]
fn push_mount(out: &mut Vec<DriveInfo>, mount: &str) {
    let name = Path::new(mount)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(mount);
    if mount.starts_with("/Volumes/") && (name.is_empty() || name.starts_with('.')) {
        return;
    }
    let is_link = std::fs::symlink_metadata(mount)
        .map(|md| md.file_type().is_symlink())
        .unwrap_or(false);
    let kind = if is_link {
        drive_kind::DRIVE_REMOTE
    } else {
        drive_kind::DRIVE_FIXED
    };
    let kind_label = drive_type_name(kind);
    let display = if mount == "/" {
        format!("/ ({kind_label})")
    } else {
        format!("{} ({kind_label})", name)
    };
    // Lettera sintetica: le piattaforme senza lettere assegnano A, B, C…
    // in ordine di enumerazione (nessun codice la legge come path).
    let letter = (b'A' + out.len().min(25) as u8) as char;
    out.push(DriveInfo {
        letter,
        label: display,
        kind,
        root: PathBuf::from(mount),
    });
}

#[cfg(target_os = "windows")]
fn read_volume_label(letter: char) -> Option<String> {
    use windows_sys::Win32::Storage::FileSystem::GetVolumeInformationW;
    let root = format!("{}:\\", letter);
    let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
    let mut name_buf = [0u16; 261];
    let mut fs_buf = [0u16; 261];
    let mut serial = 0u32;
    let mut max_comp = 0u32;
    let mut flags = 0u32;
    let ok = unsafe {
        GetVolumeInformationW(
            wide.as_ptr(),
            name_buf.as_mut_ptr(),
            name_buf.len() as u32,
            &mut serial,
            &mut max_comp,
            &mut flags,
            fs_buf.as_mut_ptr(),
            fs_buf.len() as u32,
        )
    };
    if ok == 0 {
        return None;
    }
    let len = name_buf
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(name_buf.len());
    if len == 0 {
        return None;
    }
    String::from_utf16(&name_buf[..len]).ok()
}

/// `(free_bytes, total_bytes)` per la lettera di drive data.
#[cfg(target_os = "windows")]
pub fn drive_free_space(letter: char) -> Option<(u64, u64)> {
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let root = format!("{}:\\", letter);
    let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
    let mut free_to_caller: u64 = 0;
    let mut total: u64 = 0;
    let mut total_free: u64 = 0;
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free_to_caller,
            &mut total,
            &mut total_free,
        )
    };
    if ok == 0 {
        None
    } else {
        Some((free_to_caller, total))
    }
}

/// Senza `statfs` nel `std`, lo spazio libero di un volume montato non
/// è calcolabile qui: nessun chiamante lo usa sulle piattaforme
/// non-Windows (la API è letter-based, concetto Windows-only).
#[cfg(not(target_os = "windows"))]
pub fn drive_free_space(_letter: char) -> Option<(u64, u64)> {
    None
}

/// `(free_bytes, total_bytes)` per il volume che ospita `path`.
///
/// Equivalente POSIX di [`drive_free_space`] ma path-based, così
/// funziona anche su macOS/Linux dove le lettere di drive non
/// esistono. Usa `libc::statvfs` su Unix e `GetDiskFreeSpaceExW`
/// sull'unità estratta su Windows (cadendo nel ramo Win32).
pub fn volume_free_space(path: &Path) -> Option<(u64, u64)> {
    #[cfg(target_os = "macos")]
    unsafe {
        use libc::statvfs;
        let mut s: statvfs = std::mem::zeroed();
        let p = path.to_string_lossy();
        let cstr = std::ffi::CString::new(p.as_bytes()).ok()?;
        if libc::statvfs(cstr.as_ptr(), &mut s) != 0 {
            return None;
        }
        let bsize = if s.f_frsize > 0 {
            s.f_frsize as u64
        } else {
            s.f_bsize as u64
        };
        let free = (s.f_bavail as u64).saturating_mul(bsize);
        let total = (s.f_blocks as u64).saturating_mul(bsize);
        if total == 0 {
            return None;
        }
        Some((free, total))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    unsafe {
        use libc::{c_int, statvfs};
        let mut s: statvfs = std::mem::zeroed();
        let p = path.to_string_lossy();
        let cstr = std::ffi::CString::new(p.as_bytes()).ok()?;
        if libc::statvfs(cstr.as_ptr(), &mut s) != 0 {
            return None;
        }
        let bsize = if s.f_frsize > 0 {
            s.f_frsize as u64
        } else {
            s.f_bsize as u64
        };
        let free = (s.f_bavail as u64).saturating_mul(bsize);
        let total = (s.f_blocks as u64).saturating_mul(bsize);
        if total == 0 {
            return None;
        }
        Some((free, total))
    }
    #[cfg(target_os = "windows")]
    {
        // Su Windows riusa `drive_free_space` con la lettera
        // estratta dal path (gestita dal chiamante in status_bar).
        if let Some(letter) = path.to_string_lossy().chars().next() {
            drive_free_space(letter)
        } else {
            None
        }
    }
}

// ── Shell icons ────────────────────────────────────────────────────

/// `SHGetFileInfoW` con `SHGFI_ICON | SHGFI_SMALLICON |
/// SHGFI_USEFILEATTRIBUTES`. Non accede al disco — usa solo
/// l'estensione o l'attributo directory per scegliere l'icona.
#[cfg(target_os = "windows")]
pub fn extract_shell_icon_hicon(path: &Path, is_dir: bool) -> Option<isize> {
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL,
    };
    use windows_sys::Win32::UI::Shell::{
        SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_SMALLICON, SHGFI_USEFILEATTRIBUTES,
    };
    let wide = crate::platform::to_wide(&path.to_string_lossy());
    let mut info: SHFILEINFOW = unsafe { std::mem::zeroed() };
    let attrs = if is_dir {
        FILE_ATTRIBUTE_DIRECTORY
    } else {
        FILE_ATTRIBUTE_NORMAL
    };
    let r = unsafe {
        SHGetFileInfoW(
            wide.as_ptr(),
            attrs,
            &mut info,
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_SMALLICON | SHGFI_USEFILEATTRIBUTES,
        )
    };
    if r == 0 || info.hIcon.is_null() {
        return None;
    }
    Some(info.hIcon as isize)
}

/// Le icone native non-Windows non sono `HICON`: M2 le estrae
/// direttamente come PNG altrove.
#[cfg(not(target_os = "windows"))]
pub fn extract_shell_icon_hicon(_path: &Path, _is_dir: bool) -> Option<isize> {
    None
}

/// Converte un `HICON` Win32 in bytes PNG (16×16 RGBA8) pronti per
/// `ru_wx::ImageList::add_image_bytes`. Non implementato (le
/// controparti non-Windows non hanno `HICON`).
pub fn hicon_to_png_bytes(_hicon: isize) -> Option<Vec<u8>> {
    None
}

// ── File operations ────────────────────────────────────────────────

/// Sposta un file o una cartella nel cestino — SPEC §4.3.
///
/// Windows: `SHFileOperationW` con `FO_DELETE | FOF_ALLOWUNDO`.
/// macOS/Linux: `rename` in `~/.Trash` (con fallback copia+rimuovi
/// per i volumi esterni, dove `rename` non attraversa i filesystem).
#[cfg(target_os = "windows")]
pub fn recycle_paths(paths: &[PathBuf]) -> Result<usize, ShellError> {
    use windows_sys::Win32::UI::Shell::{FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_SILENT, FO_DELETE};
    let Some((from, count)) = build_double_null_list(paths) else {
        return Err(ShellError::InvalidPath);
    };
    let flags = (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT) as u16;
    run_shell_file_op(FO_DELETE, &from, None, flags)?;
    Ok(count)
}

#[cfg(not(target_os = "windows"))]
pub fn recycle_paths(paths: &[PathBuf]) -> Result<usize, ShellError> {
    let Some(home) = std::env::var_os("HOME") else {
        return Err(ShellError::InvalidPath);
    };
    let trash = PathBuf::from(home).join(".Trash");
    if !trash.is_dir() {
        std::fs::create_dir_all(&trash).map_err(map_io_error)?;
    }
    let mut count = 0usize;
    for p in paths {
        let abs = shell_absolute(p);
        if !abs.exists() {
            return Err(ShellError::InvalidPath);
        }
        let Some(name) = abs.file_name() else {
            return Err(ShellError::InvalidPath);
        };
        let dest = unique_destination(&trash, name);
        move_to_trash(&abs, &dest)?;
        count += 1;
    }
    Ok(count)
}

/// Apre il file con l'applicazione predefinita registrata per la sua
/// estensione. Per una cartella apre il file manager sulla cartella.
///
/// `parent_hwnd` è l'HWND della finestra principale (opzionale). Se
/// fornito, le dialog di errore che il sistema dovesse mostrare
/// sarebbero parentate a quella finestra. Con `None` (default) il
/// sistema può mostrare dialog modali "orfane" — comportamento
/// storico del vecchio `ShellExecuteW` con `HWND = NULL`.
#[cfg(target_os = "windows")]
pub fn open_with_default_app(
    path: &Path,
    parent_hwnd: Option<crate::platform::NativeHandle>,
) -> Result<(), ShellError> {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW,
    };
    let abs = shell_absolute(path);
    let text = abs.to_string_lossy();
    let wide_file: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let verb: Vec<u16> = crate::platform::to_wide("open")
        .into_iter()
        .chain(std::iter::once(0))
        .collect();

    let hwnd = parent_hwnd
        .map(|h| h as HWND)
        .unwrap_or(std::ptr::null_mut());

    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        // SEE_MASK_FLAG_NO_UI: il sistema NON mostra le sue dialog di
        // errore (es. "impossibile aprire il file") — vogliamo noi
        // un messaggio localizzato e coerente con il resto dell'app.
        // SEE_MASK_NOASYNC: ritorno sincrono (non aspetta che il
        // processo figlio termini), importante perché siamo sul
        // thread UI.
        fMask: SEE_MASK_FLAG_NO_UI | SEE_MASK_NOASYNC,
        hwnd,
        lpVerb: verb.as_ptr(),
        lpFile: wide_file.as_ptr(),
        lpParameters: std::ptr::null(),
        lpDirectory: std::ptr::null(),
        nShow: 1, // SW_SHOWNORMAL
        hInstApp: std::ptr::null_mut(),
        lpIDList: std::ptr::null_mut(),
        lpClass: std::ptr::null(),
        hkeyClass: std::ptr::null_mut(),
        dwHotKey: 0,
        Anonymous: unsafe { std::mem::zeroed() },
        hProcess: std::ptr::null_mut(),
    };

    let ok = unsafe { ShellExecuteExW(&mut info) };
    if ok != 0 {
        return Ok(());
    }

    // Mappatura errori: ShellExecuteExW non ritorna il classico
    // "code > 32" di ShellExecuteW — torna `FALSE` (= 0) e
    // memorizza `GetLastError()`. Convertiamo il codice Win32 nel
    // nostro enum per dare un messaggio utile all'utente.
    let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
    Err(map_shellexec_error(err))
}

#[cfg(target_os = "macos")]
pub fn open_with_default_app(
    path: &Path,
    _parent_hwnd: Option<crate::platform::NativeHandle>,
) -> Result<(), ShellError> {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::{NSString, NSURL};
    let abs = shell_absolute(path);
    let ns = NSString::from_str(&abs.to_string_lossy());
    let url = NSURL::fileURLWithPath(&ns);
    if NSWorkspace::sharedWorkspace().openURL(&url) {
        Ok(())
    } else {
        Err(ShellError::AccessDenied)
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn open_with_default_app(
    path: &Path,
    _parent_hwnd: Option<crate::platform::NativeHandle>,
) -> Result<(), ShellError> {
    let _ = path;
    Err(ShellError::NotImplemented)
}

/// Mappa un codice di errore Win32 (letto con `GetLastError()` dopo
/// un fallimento di `ShellExecuteExW`) sul nostro enum [`ShellError`].
///
/// I codici Win32 qui usati sono quelli che `GetLastError` può
/// effettivamente ritornare dopo `ShellExecuteExW`:
/// * `ERROR_FILE_NOT_FOUND` (2) = file inesistente.
/// * `ERROR_PATH_NOT_FOUND` (3) = directory inesistente.
/// * `ERROR_ACCESS_DENIED` (5) = ACL / UAC.
/// * `ERROR_NOT_ENOUGH_MEMORY` (8) = OOM durante l'avvio.
/// * `ERROR_BAD_EXE_FORMAT` (11) = eseguibile corrotto.
/// * `ERROR_SHARING_VIOLATION` (32) = file in uso.
///
/// I valori `SE_ERR_*` del vecchio `ShellExecuteW` (range 0..=32) si
/// sovrappongono ma hanno semantica leggermente diversa; non li
/// gestiamo qui perché `ShellExecuteExW` non li ritorna — usa
/// `GetLastError()`. Le costanti magiche sono definite da
/// `shellapi.h` ma `windows-sys` le re-exporta, quindi le usiamo
/// direttamente.
#[cfg(target_os = "windows")]
fn map_shellexec_error(err: u32) -> ShellError {
    use windows_sys::Win32::Foundation::{
        ERROR_ACCESS_DENIED, ERROR_BAD_EXE_FORMAT, ERROR_FILE_NOT_FOUND, ERROR_NOT_ENOUGH_MEMORY,
        ERROR_PATH_NOT_FOUND, ERROR_SHARING_VIOLATION,
    };
    match err {
        // 0 = "no error" — non dovrebbe mai capitare, ma difendiamoci
        // con un Other(0) esplicito invece di un default silenzioso.
        0 => ShellError::Other(0),
        // ERROR_FILE_NOT_FOUND (2) / ERROR_PATH_NOT_FOUND (3): il file o la
        // directory mancanti vengono normalizzati a InvalidPath.
        ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => ShellError::InvalidPath,
        ERROR_ACCESS_DENIED => ShellError::AccessDenied,
        ERROR_NOT_ENOUGH_MEMORY => ShellError::OutOfMemory,
        ERROR_BAD_EXE_FORMAT => ShellError::BadFormat,
        ERROR_SHARING_VIOLATION => ShellError::SharingViolation,
        other => ShellError::Other(other),
    }
}

/// Apre la dialog "Proprietà" del sistema sul path dato.
///
/// `parent_hwnd` è l'HWND della finestra principale (opzionale). Se
/// fornito, le dialog di errore sono parentate ad essa. Con `None`
/// (default) il sistema può mostrare dialog modali "orfane".
#[cfg(target_os = "windows")]
pub fn show_properties_dialog(
    path: &Path,
    parent_hwnd: Option<crate::platform::NativeHandle>,
) -> Result<(), ShellError> {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW,
    };
    let abs = shell_absolute(path);
    let text = abs.to_string_lossy();
    let wide_file: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let verb: Vec<u16> = crate::platform::to_wide("properties")
        .into_iter()
        .chain(std::iter::once(0))
        .collect();

    let hwnd = parent_hwnd
        .map(|h| h as HWND)
        .unwrap_or(std::ptr::null_mut());

    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_FLAG_NO_UI | SEE_MASK_NOASYNC,
        hwnd,
        lpVerb: verb.as_ptr(),
        lpFile: wide_file.as_ptr(),
        lpParameters: std::ptr::null(),
        lpDirectory: std::ptr::null(),
        nShow: 1,
        hInstApp: std::ptr::null_mut(),
        lpIDList: std::ptr::null_mut(),
        lpClass: std::ptr::null(),
        hkeyClass: std::ptr::null_mut(),
        dwHotKey: 0,
        Anonymous: unsafe { std::mem::zeroed() },
        hProcess: std::ptr::null_mut(),
    };

    let ok = unsafe { ShellExecuteExW(&mut info) };
    if ok != 0 {
        return Ok(());
    }
    let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
    Err(map_shellexec_error(err))
}

/// macOS non espone la dialog "Proprietà" del Finder via API semplici:
/// milestone best-effort (SPEC §11).
#[cfg(not(target_os = "windows"))]
pub fn show_properties_dialog(
    path: &Path,
    _parent_hwnd: Option<crate::platform::NativeHandle>,
) -> Result<(), ShellError> {
    let _ = path;
    Err(ShellError::NotImplemented)
}

/// Mostra la dialog di sistema "Apri con…" per scegliere
/// un'applicazione non-predefinita con cui aprire `path`.
///
/// La dialog Win32 (`SHOpenWithDialog`) è la stessa che Esplora
/// Risorse mostra nel menu contestuale: elenca le app registrate per
/// l'estensione, permette di sceglierne una o cercarne un'altra
/// ("Cerca un'app in questo PC"), oppure di cliccare "OK" per
/// lanciare l'app predefinita. Se l'utente fa una scelta, l'app
/// viene avviata **direttamente dalla dialog** — la funzione non
/// ritorna il path dell'eseguibile scelto.
///
/// Differisce da [`open_with_default_app`] perché non usa il mapping
/// predefinito del registry, ma chiede esplicitamente all'utente.
///
/// `parent_hwnd` è l'HWND della finestra principale (opzionale).
///
/// Errori tipici:
/// * `ShellError::InvalidPath` — `path` non esiste o non è un file.
/// * `ShellError::NoAssociation` — nessuna app registrata per
///   l'estensione (la dialog Win32 lo segnala prima di aprirsi).
/// * `ShellError::Other(code)` — HRESULT non-zero ritornato da
///   `SHOpenWithDialog` (raro: la dialog gestisce quasi tutto).
#[cfg(target_os = "windows")]
pub fn open_with_dialog(
    path: &Path,
    parent_hwnd: Option<crate::platform::NativeHandle>,
) -> Result<(), ShellError> {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::Shell::{
        SHOpenWithDialog, OAIF_ALLOW_REGISTRATION, OAIF_EXEC, OPENASINFO,
    };
    let abs = shell_absolute(path);
    if !abs.exists() {
        return Err(ShellError::InvalidPath);
    }
    let text = abs.to_string_lossy();
    let wide_file: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let info = OPENASINFO {
        pcszFile: wide_file.as_ptr(),
        // Lasciamo `pcszClass` = NULL: Win32 ricava la classe
        // dall'estensione del file via `AssocQueryStringW`.
        pcszClass: std::ptr::null(),
        // `OAIF_EXEC` chiede alla dialog di lanciare subito l'app
        // scelta dall'utente (anziché limitarci a "Apri" senza
        // effetto). `OAIF_ALLOW_REGISTRATION` mostra il checkbox
        // "Usa sempre questa app" — Esplora Risorse lo fa.
        oaifInFlags: OAIF_EXEC | OAIF_ALLOW_REGISTRATION,
    };
    let hwnd = parent_hwnd
        .map(|h| h as HWND)
        .unwrap_or(std::ptr::null_mut());
    // SAFETY: `info` è una struct POD valida, `hwnd` è un HWND vivo
    // (o NULL), `wide_file` sopravvive alla chiamata sincrona.
    let hr = unsafe { SHOpenWithDialog(hwnd, &info) };
    // `SHOpenWithDialog` ritorna S_OK (= 0) sia se l'utente ha
    // scelto un'app (lanciata dalla dialog) sia se ha solo chiuso
    // la finestra con OK senza scegliere. `S_FALSE` (= 1) è
    // "Cancel". Qualsiasi altro HRESULT è un errore Win32.
    if hr >= 0 {
        Ok(())
    } else {
        // Mappa i codici HRESULT più comuni; gli altri finiscono
        // in `Other` con il valore raw per debug.
        let code = hr as u32;
        match code {
            // NOASSOC = 0x80070483
            0x8007_0483 => Err(ShellError::NoAssociation),
            // ERROR_CANCELLED = 0x800704C7 — l'utente ha chiuso
            // la dialog con Cancel/X. Non è un errore "vero" per
            // l'utente, ma lo segnaliamo come tale perché la UI lo
            // gestirà già (status bar silenziosa). Raramente
            // osservato con `SHOpenWithDialog`, che usa S_FALSE.
            0x8007_04C7 => Err(ShellError::Other(code)),
            other => Err(ShellError::Other(other)),
        }
    }
}

/// `SHOpenWithDialog` non ha un equivalente diretto cross-platform
/// in questa milestone. Su macOS si potrebbe usare
/// `[NSWorkspace chooseApplicationBundleFor:]` ma serve un piccolo
/// bridge Objective-C; lo aggiungeremo quando servirà davvero.
#[cfg(not(target_os = "windows"))]
pub fn open_with_dialog(
    _path: &Path,
    _parent_hwnd: Option<crate::platform::NativeHandle>,
) -> Result<(), ShellError> {
    Err(ShellError::NotImplemented)
}

/// Lancia `path` con privilegi elevati (UAC prompt).
///
/// Su Windows equivalente a `ShellExecuteExW` con verb `"runas"`:
/// il sistema mostra il dialog UAC standard ("Vuoi autorizzare…")
/// e, se l'utente accetta, esegue il file come amministratore.
/// Funziona solo per i tipi di file che Shell può elevare:
/// `.exe`, `.bat`, `.cmd`, `.msi`, `.ps1`, `.vbs`, `.js`,
/// `.scr`, `.com`. Per i documenti (PDF, immagini…) il
/// `runas` non è significativo e ritorniamo
/// `ShellError::BadFormat`.
///
/// `parent_hwnd` è l'HWND della finestra principale (opzionale):
/// il dialog UAC viene parentato ad essa.
#[cfg(target_os = "windows")]
pub fn launch_as_admin(
    path: &Path,
    parent_hwnd: Option<crate::platform::NativeHandle>,
) -> Result<(), ShellError> {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::Shell::{
        ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW,
    };
    let abs = shell_absolute(path);
    if !abs.exists() {
        return Err(ShellError::InvalidPath);
    }
    // Filtra le estensioni non elevabili: tenta `runas` solo su
    // eseguibili / script / installer. Per un PDF o un JPG il
    // sistema non sa cosa elevare (l'app predefinita non gira
    // come Admin) e ShellExecute ritorna errore.
    let ext = abs
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let can_elevate = matches!(
        ext.as_str(),
        "exe" | "bat" | "cmd" | "msi" | "msp" | "ps1" | "vbs" | "js" | "scr" | "com"
    );
    if !can_elevate {
        return Err(ShellError::BadFormat);
    }
    let text = abs.to_string_lossy();
    let wide_file: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let verb: Vec<u16> = crate::platform::to_wide("runas")
        .into_iter()
        .chain(std::iter::once(0))
        .collect();
    let hwnd = parent_hwnd
        .map(|h| h as HWND)
        .unwrap_or(std::ptr::null_mut());
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        // `SEE_MASK_FLAG_NO_UI` sopprime le dialog di errore di
        // sistema: se l'UAC viene annullato dall'utente, vogliamo
        // mostrare un nostro messaggio localizzato invece della
        // dialog Win32.
        fMask: SEE_MASK_FLAG_NO_UI | SEE_MASK_NOASYNC,
        hwnd,
        lpVerb: verb.as_ptr(),
        lpFile: wide_file.as_ptr(),
        lpParameters: std::ptr::null(),
        lpDirectory: std::ptr::null(),
        nShow: 1,
        hInstApp: std::ptr::null_mut(),
        lpIDList: std::ptr::null_mut(),
        lpClass: std::ptr::null(),
        hkeyClass: std::ptr::null_mut(),
        dwHotKey: 0,
        Anonymous: unsafe { std::mem::zeroed() },
        hProcess: std::ptr::null_mut(),
    };
    let ok = unsafe { ShellExecuteExW(&mut info) };
    if ok != 0 {
        return Ok(());
    }
    let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
    // L'utente ha annullato l'UAC: Win32 ritorna
    // `ERROR_CANCELLED = 1223`. Lo segnaliamo come
    // `ShellError::Other(1223)` per distinguerlo dagli errori
    // veri (la UI mostrerà un messaggio generico).
    Err(map_shellexec_error(err))
}

/// L'elevazione UAC è un concetto solo Windows.
#[cfg(not(target_os = "windows"))]
pub fn launch_as_admin(
    _path: &Path,
    _parent_hwnd: Option<crate::platform::NativeHandle>,
) -> Result<(), ShellError> {
    Err(ShellError::NotImplemented)
}

/// Ritorna `true` se il processo corrente gira con privilegi
/// elevati (amministratore su Windows, root su macOS/Linux).
///
/// Windows: `IsUserAnAdmin()` da `shellapi.h` — richiede
/// `#pragma comment(lib, "shell32.lib")` ma il linker di
/// rust-lld risolve `shell32.dll` automaticamente.
///
/// macOS: controlla `getuid() == 0`.
/// Linux: idem.
///
/// Utile per decidere se mostrare/nascondere la voce "Apri come
/// amministratore" nel context menu (nasconderla se siamo già
/// admin evita un prompt UAC inutile).
#[cfg(target_os = "windows")]
pub fn is_running_as_admin() -> bool {
    use windows_sys::Win32::UI::Shell::IsUserAnAdmin;
    // SAFETY: `IsUserAnAdmin` non ha precondizioni.
    unsafe { IsUserAnAdmin() != 0 }
}

#[cfg(target_os = "macos")]
pub fn is_running_as_admin() -> bool {
    // `getuid` è safe in libc; `geteuid` ritorna l'UID effettivo.
    extern "C" {
        fn geteuid() -> u32;
    }
    unsafe { geteuid() == 0 }
}

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
pub fn is_running_as_admin() -> bool {
    extern "C" {
        fn geteuid() -> u32;
    }
    unsafe { geteuid() == 0 }
}

/// Estensioni di file per cui ha senso offrire "Apri come
/// amministratore" (eseguibili, script, installer). Usata dal
/// context menu per decidere se aggiungere la voce.
pub fn can_elevate_extension(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    matches!(
        ext.as_str(),
        "exe" | "bat" | "cmd" | "msi" | "msp" | "ps1" | "vbs" | "js" | "scr" | "com"
    )
}

/// Calcola ricorsivamente la dimensione occupata da una cartella.
///
/// Conta file, sottocartelle e byte totali. Le sottocartelle che non
/// possono essere lette (permessi negati) vengono saltate senza
/// propagare l'errore.
///
/// Windows: `FindFirstFileW`/`FindNextFileW` (attributi e dimensione
/// arrivano già nel `WIN32_FIND_DATAW`, nessuna `stat` aggiuntiva).
/// non-Windows: `read_dir` (su Unix `DirEntry::metadata` usa `lstat`,
/// quindi i symlink non vengono seguiti — nessun rischio di cicli).
pub fn compute_dir_size(path: &Path) -> DirSize {
    let mut out = DirSize::default();
    let mut counter = 0u64;
    scan_dir_size(path, &mut out, &mut counter, &mut |_, _| true);
    out
}

/// Come [`compute_dir_size`], ma chiama `on_progress` (con lo stato
/// parziale e la voce appena esaminata) circa ogni 1000 voci.
///
/// Se `on_progress` ritorna `false`, il calcolo si interrompe subito
/// (annullamento) e restituisce lo stato accumulato fino a quel punto.
pub fn compute_dir_size_cancellable(
    path: &Path,
    on_progress: &mut dyn FnMut(&DirSize, &Path) -> bool,
) -> DirSize {
    let mut out = DirSize::default();
    let mut counter = 0u64;
    scan_dir_size(path, &mut out, &mut counter, on_progress);
    out
}

/// Ricorsione interna condivisa. Ritorna `false` se l'utente ha
/// annullato, `true` altrimenti (anche quando una sottocartella non è
/// leggibile: non è un annullamento).
#[cfg(target_os = "windows")]
fn scan_dir_size(
    path: &Path,
    out: &mut DirSize,
    counter: &mut u64,
    on_progress: &mut dyn FnMut(&DirSize, &Path) -> bool,
) -> bool {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::Storage::FileSystem::{
        FindClose, FindFirstFileW, FindNextFileW, FILE_ATTRIBUTE_DIRECTORY,
        FILE_ATTRIBUTE_REPARSE_POINT, WIN32_FIND_DATAW,
    };

    let search = build_search_path(path);
    let mut data: WIN32_FIND_DATAW = unsafe { std::mem::zeroed() };
    // SAFETY: `search` è un buffer wide NUL-terminato valido; `data` è
    // un puntatore a una struct inizializzata che sopravvive alla chiamata.
    let handle = unsafe { FindFirstFileW(search.as_ptr(), &mut data) };
    if handle == INVALID_HANDLE_VALUE {
        return true;
    }

    loop {
        let name = wide_name_to_string(&data.cFileName);
        if name != "." && name != ".." {
            let child = path.join(&name);
            let is_dir = (data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0;
            let is_reparse = (data.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0;

            if is_dir && !is_reparse {
                out.dirs += 1;
                if !scan_dir_size(&child, out, counter, on_progress) {
                    // SAFETY: chiude l'handle di ricerca valido.
                    unsafe {
                        FindClose(handle);
                    }
                    return false;
                }
            } else {
                // File (o reparse point: non ricorrere per evitare cicli
                // con junction/symlink).
                out.files += 1;
                out.bytes += ((data.nFileSizeHigh as u64) << 32) | (data.nFileSizeLow as u64);
            }

            *counter += 1;
            if (*counter).is_multiple_of(1000) && !on_progress(out, &child) {
                // SAFETY: chiude l'handle di ricerca valido.
                unsafe {
                    FindClose(handle);
                }
                return false;
            }
        }

        // SAFETY: `handle` è un handle di ricerca valido, `data` è valido.
        if unsafe { FindNextFileW(handle, &mut data) } == 0 {
            break;
        }
    }

    // SAFETY: chiude l'handle di ricerca valido.
    unsafe {
        FindClose(handle);
    }
    true
}

#[cfg(not(target_os = "windows"))]
fn scan_dir_size(
    path: &Path,
    out: &mut DirSize,
    counter: &mut u64,
    on_progress: &mut dyn FnMut(&DirSize, &Path) -> bool,
) -> bool {
    let Ok(rd) = std::fs::read_dir(path) else {
        return true;
    };
    for entry in rd {
        let Ok(entry) = entry else {
            continue;
        };
        // `DirEntry::metadata` non segue i symlink (lstat): le symlink
        // contano come file e non generano cicli.
        let Ok(md) = entry.metadata() else {
            continue;
        };
        let child = entry.path();
        if md.is_dir() {
            out.dirs += 1;
            if !scan_dir_size(&child, out, counter, on_progress) {
                return false;
            }
        } else {
            out.files += 1;
            out.bytes += md.len();
        }
        *counter += 1;
        if counter.is_multiple_of(1000) && !on_progress(out, &child) {
            return false;
        }
    }
    true
}

/// Costruisce il pattern di ricerca `path\*` NUL-terminato per
/// `FindFirstFileW`.
#[cfg(target_os = "windows")]
fn build_search_path(path: &Path) -> Vec<u16> {
    let mut w: Vec<u16> = path.to_string_lossy().encode_utf16().collect();
    if w.last().copied() != Some(b'\\' as u16) {
        w.push(b'\\' as u16);
    }
    w.push(b'*' as u16);
    w.push(0);
    w
}

/// Converte il buffer wide NUL-terminato `cFileName` di
/// `WIN32_FIND_DATAW` in una `String`.
#[cfg(target_os = "windows")]
fn wide_name_to_string(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

/// Copia ricorsiva file/cartelle (`copy_paths` non-Windows e fallback
/// cross-device di [`recycle_paths`]).
#[cfg(not(target_os = "windows"))]
fn copy_recursive(src: &Path, dest: &Path) -> Result<(), ShellError> {
    let md = std::fs::symlink_metadata(src).map_err(map_io_error)?;
    if md.file_type().is_symlink() {
        let target = std::fs::read_link(src).map_err(map_io_error)?;
        // Il symlink dest esistente va rimosso: `symlink` fallisce se
        // il path esiste già.
        let _ = std::fs::remove_file(dest);
        std::os::unix::fs::symlink(target, dest).map_err(map_io_error)
    } else if md.is_dir() {
        std::fs::create_dir_all(dest).map_err(map_io_error)?;
        for entry in std::fs::read_dir(src).map_err(map_io_error)? {
            let entry = entry.map_err(map_io_error)?;
            copy_recursive(&entry.path(), &dest.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        std::fs::copy(src, dest).map(|_| ()).map_err(map_io_error)
    }
}

/// Rimozione ricorsiva file/cartelle/symlink.
#[cfg(not(target_os = "windows"))]
fn remove_recursive(p: &Path) -> Result<(), ShellError> {
    let md = std::fs::symlink_metadata(p).map_err(map_io_error)?;
    if md.file_type().is_symlink() || !md.is_dir() {
        std::fs::remove_file(p).map_err(map_io_error)
    } else {
        for entry in std::fs::read_dir(p).map_err(map_io_error)? {
            let entry = entry.map_err(map_io_error)?;
            remove_recursive(&entry.path())?;
        }
        std::fs::remove_dir(p).map_err(map_io_error)
    }
}

/// Sposta `src` in `dest`; se `rename` fallisce (volume esterno,
/// `EXDEV`) copia e rimuove.
#[cfg(not(target_os = "windows"))]
fn move_to_trash(src: &Path, dest: &Path) -> Result<(), ShellError> {
    match std::fs::rename(src, dest) {
        Ok(()) => Ok(()),
        Err(_) => {
            copy_recursive(src, dest)?;
            remove_recursive(src)
        }
    }
}

/// Path `dest` non ancora esistente in `dir`: `name`, poi `name 2`,
/// `name 3`… (come fa Finder quando il nome è già nel Cestino).
#[cfg(not(target_os = "windows"))]
fn unique_destination(dir: &Path, name: &std::ffi::OsStr) -> PathBuf {
    let direct = dir.join(name);
    if !direct.exists() {
        return direct;
    }
    let base = Path::new(name);
    let stem = base
        .file_stem()
        .map(|s| s.to_os_string())
        .unwrap_or_else(|| name.to_os_string());
    let ext = base.extension().map(|e| e.to_os_string());
    for i in 2..10_000u32 {
        let mut candidate = stem.clone();
        candidate.push(format!(" {i}"));
        if let Some(ext) = &ext {
            candidate.push(".");
            candidate.push(ext);
        }
        let path = dir.join(candidate);
        if !path.exists() {
            return path;
        }
    }
    direct
}

/// Copia i path sorgente dentro la directory `dst`.
///
/// Windows: `SHFileOperationW` con `FO_COPY` (sovrascrive, come
/// l'Explorer con conferma disabilitata). non-Windows: copia ricorsiva
/// in `std` con la stessa semantica di sovrascrittura.
#[cfg(target_os = "windows")]
pub fn copy_paths(src: &[PathBuf], dst: &Path) -> Result<usize, ShellError> {
    use windows_sys::Win32::UI::Shell::{FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FO_COPY};
    let Some((from, count)) = build_double_null_list(src) else {
        return Err(ShellError::InvalidPath);
    };
    let Some((to, _)) = build_double_null_list(std::slice::from_ref(&dst.to_path_buf())) else {
        return Err(ShellError::InvalidPath);
    };
    let flags = (FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT) as u16;
    run_shell_file_op(FO_COPY, &from, Some(&to), flags)?;
    Ok(count)
}

#[cfg(not(target_os = "windows"))]
pub fn copy_paths(src: &[PathBuf], dst: &Path) -> Result<usize, ShellError> {
    if src.is_empty() {
        return Err(ShellError::InvalidPath);
    }
    if !dst.is_dir() {
        std::fs::create_dir_all(dst).map_err(map_io_error)?;
    }
    let mut count = 0usize;
    for p in src {
        let Some(name) = p.file_name() else {
            return Err(ShellError::InvalidPath);
        };
        copy_recursive(p, &dst.join(name))?;
        count += 1;
    }
    Ok(count)
}

/// Path assoluto senza il prefisso `\\?\` (che il shell rifiuta).
fn shell_absolute(p: &Path) -> PathBuf {
    let abs = std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf());
    let text = abs.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(stripped) => PathBuf::from(stripped),
        None => abs,
    }
}

/// Costruisce la lista di path UTF-16 richiesta da `SHFileOperationW`:
/// ogni path è NUL-terminato e la lista è chiusa da un NUL aggiuntivo.
/// Ritorna `None` se nessun path è utilizzabile.
#[cfg(target_os = "windows")]
fn build_double_null_list(paths: &[PathBuf]) -> Option<(Vec<u16>, usize)> {
    let mut buf: Vec<u16> = Vec::new();
    let mut count = 0usize;
    for p in paths {
        let abs = shell_absolute(p);
        let text = abs.to_string_lossy();
        if text.is_empty() {
            continue;
        }
        buf.extend(text.encode_utf16());
        buf.push(0);
        count += 1;
    }
    if count == 0 {
        return None;
    }
    buf.push(0);
    Some((buf, count))
}

#[cfg(target_os = "windows")]
fn run_shell_file_op(
    func: u32,
    from: &[u16],
    to: Option<&[u16]>,
    flags: u16,
) -> Result<(), ShellError> {
    use windows_sys::Win32::UI::Shell::{SHFileOperationW, SHFILEOPSTRUCTW};
    let mut op: SHFILEOPSTRUCTW = unsafe { std::mem::zeroed() };
    op.hwnd = std::ptr::null_mut();
    op.wFunc = func;
    op.pFrom = from.as_ptr();
    op.pTo = to.map_or(std::ptr::null(), |t| t.as_ptr());
    op.fFlags = flags;
    // SAFETY: FFI call with a zeroed, fully-initialised struct whose
    // `pFrom` / `pTo` buffers outlive the call.
    let rc = unsafe { SHFileOperationW(&mut op) };
    if rc != 0 {
        return Err(match rc as u32 {
            // ERROR_FILE_NOT_FOUND / ERROR_PATH_NOT_FOUND
            2 | 3 => ShellError::InvalidPath,
            // ERROR_ACCESS_DENIED
            5 => ShellError::AccessDenied,
            // DE_ACCESSDENIEDSRC
            0x78 => ShellError::AccessDenied,
            other => ShellError::Other(other),
        });
    }
    if op.fAnyOperationsAborted != 0 {
        return Err(ShellError::Other(0));
    }
    Ok(())
}

// ── ListView / toolbar hit-testing (Win32 notifications) ───────────

/// Estrae l'indice della riga attivata da una notifica
/// `LVN_ITEMACTIVATE` (doppio click / Invio su una riga della lista).
///
/// `lparam` è il puntatore a un `NMITEMACTIVATE` arrivato tramite
/// `WM_NOTIFY`. Ritorna `None` se il puntatore è nullo o se `iItem` è
/// negativo (click su spazio vuoto / header).
#[cfg(target_os = "windows")]
pub fn list_activate_item_index(lparam: isize) -> Option<usize> {
    use windows_sys::Win32::UI::Controls::NMITEMACTIVATE;
    if lparam == 0 {
        return None;
    }
    // SAFETY: `lparam` è un `NMITEMACTIVATE` valido passato da ListView
    // al proprio parent durante `LVN_ITEMACTIVATE`.
    let item = unsafe { (*(lparam as *const NMITEMACTIVATE)).iItem };
    if item < 0 {
        None
    } else {
        Some(item as usize)
    }
}

#[cfg(target_os = "macos")]
pub fn list_activate_item_index(lparam: isize) -> Option<usize> {
    // Sul backend AppKit non esiste un `NMITEMACTIVATE`: l'indice della
    // riga attivata (doppio clic) viaggia direttamente in `lparam`,
    // codificato dal dispatch di `LISTCTRL_NOTIFY_ITEM_DBLCLK`.
    if lparam < 0 {
        None
    } else {
        Some(lparam as usize)
    }
}

/// Sul backend GTK non esiste un `NMITEMACTIVATE`: l'indice della riga
/// attivata (doppio clic su `TreeView`) viaggia direttamente in `lparam`,
/// codificato da `platform::gtk::dispatch_lv_item_activate`.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn list_activate_item_index(lparam: isize) -> Option<usize> {
    if lparam < 0 {
        None
    } else {
        Some(lparam as usize)
    }
}

/// Estrae l'indice della colonna cliccata da una notifica
/// `LVN_COLUMNCLICK` (click sull'header della ListView).
///
/// `lparam` è il puntatore a un `NMLISTVIEW` arrivato tramite
/// `WM_NOTIFY`. Ritorna `None` se il puntatore è nullo o se
/// `iSubItem` è negativo.
#[cfg(target_os = "windows")]
pub fn list_column_click_index(lparam: isize) -> Option<usize> {
    use windows_sys::Win32::UI::Controls::NMLISTVIEW;
    if lparam == 0 {
        return None;
    }
    // SAFETY: `lparam` è un `NMLISTVIEW` valido passato da ListView
    // al proprio parent durante `LVN_COLUMNCLICK`.
    let sub = unsafe { (*(lparam as *const NMLISTVIEW)).iSubItem };
    if sub < 0 {
        None
    } else {
        Some(sub as usize)
    }
}

/// Sul backend AppKit non esiste un `NMLISTVIEW`: l'indice della colonna
/// cliccata viaggia direttamente in `lparam`, codificato dal dispatch di
/// `LISTCTRL_NOTIFY_COLUMN_CLICK` (delegato
/// `tableView:didClickTableColumn:` in `appkit_list_table`).
#[cfg(target_os = "macos")]
pub fn list_column_click_index(lparam: isize) -> Option<usize> {
    if lparam < 0 {
        None
    } else {
        Some(lparam as usize)
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn list_column_click_index(_lparam: isize) -> Option<usize> {
    None
}

/// Hit test su un `TreeCtrl` Win32: data una coordinata in *client-space*
/// del frame, restituisce l'`HTREEITEM` (come `usize`) sotto il
/// cursore, oppure `None` se il punto non cade su un nodo.
///
/// Implementato con `TVM_HITTEST` + `TVHITTESTINFO`. I flag di ritorno
/// sono controllati per accettare solo hit "on item" (label, icona,
/// state icon, pulsante `[+]`, indent o area a destra): click nello
/// sfondo vuoto del tree non vengono trattati come selezione.
#[cfg(target_os = "windows")]
pub fn tree_view_hit_test(tree_hwnd: isize, screen_x: i32, screen_y: i32) -> Option<usize> {
    use windows_sys::Win32::Foundation::{HWND, POINT};
    use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
    use windows_sys::Win32::UI::Controls::{
        TVHITTESTINFO, TVHT_ONITEM, TVHT_ONITEMBUTTON, TVHT_ONITEMICON, TVHT_ONITEMINDENT,
        TVHT_ONITEMLABEL, TVHT_ONITEMRIGHT, TVHT_ONITEMSTATEICON, TVM_HITTEST,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
    if tree_hwnd == 0 {
        return None;
    }
    let hwnd = tree_hwnd as HWND;
    let mut pt = POINT {
        x: screen_x,
        y: screen_y,
    };
    let translated = unsafe { ScreenToClient(hwnd, &mut pt) };
    if translated == 0 {
        return None;
    }
    let mut info = TVHITTESTINFO {
        pt,
        flags: 0,
        hItem: 0,
    };
    let _res = unsafe { SendMessageW(hwnd, TVM_HITTEST, 0, &mut info as *mut _ as isize) };
    let on_item = TVHT_ONITEM
        | TVHT_ONITEMICON
        | TVHT_ONITEMLABEL
        | TVHT_ONITEMSTATEICON
        | TVHT_ONITEMBUTTON
        | TVHT_ONITEMINDENT
        | TVHT_ONITEMRIGHT;
    if info.hItem == 0 || (info.flags & on_item) == 0 {
        return None;
    }
    Some(info.hItem as usize)
}

#[cfg(not(target_os = "windows"))]
pub fn tree_view_hit_test(_tree_hwnd: isize, _screen_x: i32, _screen_y: i32) -> Option<usize> {
    None
}

/// Trova la riga della lista sotto le coordinate schermo `(x, y)`.
///
/// Implementa `LVM_HITTEST`. Ritorna `None` se il cursore è fuori
/// dall'area item (header, scrollbar, spazio vuoto).
#[cfg(target_os = "windows")]
pub fn list_view_hit_test(list_hwnd: isize, screen_x: i32, screen_y: i32) -> Option<usize> {
    use windows_sys::Win32::Foundation::{HWND, POINT};
    use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
    use windows_sys::Win32::UI::Controls::{
        LVHITTESTINFO, LVHT_ONITEMICON, LVHT_ONITEMLABEL, LVHT_ONITEMSTATEICON, LVM_HITTEST,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
    if list_hwnd == 0 {
        return None;
    }
    let hwnd = list_hwnd as HWND;
    let mut pt = POINT {
        x: screen_x,
        y: screen_y,
    };
    let translated = unsafe { ScreenToClient(hwnd, &mut pt) };
    if translated == 0 {
        return None;
    }
    let mut info = LVHITTESTINFO {
        pt,
        flags: 0,
        iItem: 0,
        iSubItem: 0,
        iGroup: 0,
    };
    let res = unsafe { SendMessageW(hwnd, LVM_HITTEST, 0, &mut info as *mut _ as isize) };
    let on_item_mask = LVHT_ONITEMICON | LVHT_ONITEMLABEL | LVHT_ONITEMSTATEICON;
    if res == -1 || info.iItem < 0 || (info.flags & on_item_mask) == 0 {
        return None;
    }
    Some(info.iItem as usize)
}

#[cfg(not(target_os = "windows"))]
pub fn list_view_hit_test(_list_hwnd: isize, _screen_x: i32, _screen_y: i32) -> Option<usize> {
    None
}

/// Data la coordinata **in client-space** del frame, restituisce
/// l'handle del controllo figlio sotto quel punto (drag source).
#[cfg(target_os = "windows")]
pub fn hwnd_at_point(x: i32, y: i32) -> Option<isize> {
    use windows_sys::Win32::Foundation::{HWND, POINT};
    use windows_sys::Win32::UI::WindowsAndMessaging::ChildWindowFromPoint;
    // `ChildWindowFromPoint` vuole coordinate in **client-space del
    // parent**; il frame le riceve già in quel formato da
    // `WM_MOUSEMOVE` / `WM_LBUTTONDOWN`.
    let parent_raw = frame_hwnd();
    if parent_raw == 0 {
        return None;
    }
    let parent = parent_raw as HWND;
    let pt = POINT { x, y };
    // SAFETY: `parent` è l'HWND del frame (vivo per tutta la durata
    // dell'app), `pt` è una struct POD.
    let hwnd = unsafe { ChildWindowFromPoint(parent, pt) };
    if hwnd.is_null() {
        None
    } else {
        Some(hwnd as isize)
    }
}

#[cfg(not(target_os = "windows"))]
pub fn hwnd_at_point(_x: i32, _y: i32) -> Option<isize> {
    None
}

/// Restituisce l'handle del frame principale, salvato da
/// [`set_frame_hwnd`] quando il frame viene costruito.
pub fn frame_hwnd() -> isize {
    FRAME_HND.load(std::sync::atomic::Ordering::Relaxed)
}

/// Imposta l'handle del frame principale (chiamato da `build_window`
/// subito dopo `Frame::new`).
pub fn set_frame_hwnd(hwnd: isize) {
    FRAME_HND.store(hwnd, std::sync::atomic::Ordering::Relaxed);
}

static FRAME_HND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

// ── ListCtrl style / column helpers ────────────────────────────────

/// Cambia stile di una `ListCtrl` Win32 (`SysListView32`) a runtime.
///
/// `style_bits` è il bitfield `LVS_*` da impostare (es. `LVS_ICON`,
/// `LVS_LIST`, `LVS_REPORT`). Dopo il cambio stile è necessario
/// riassegnare l'ImageList.
#[cfg(target_os = "windows")]
pub fn set_listctrl_style(hwnd: isize, style_bits: u32) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_STYLE,
    };
    if hwnd == 0 {
        return;
    }
    let h = hwnd as HWND;
    // SAFETY: `hwnd` è un HWND ListView vivo.
    unsafe {
        let prev = GetWindowLongPtrW(h, GWL_STYLE) as u32;
        // Mantieni gli stili estesi (border, scrollbar…) e rimpiazza
        // solo i bit LVS_TYPEMASK (i 3 bit bassi dello stile).
        let new_style = (prev & !0x0003) | (style_bits & 0x0003);
        SetWindowLongPtrW(h, GWL_STYLE, new_style as isize);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn set_listctrl_style(_hwnd: isize, _style_bits: u32) {}

/// Cambia la larghezza di una colonna della `ListCtrl` Win32.
/// `col` è 0-based; `width` è in pixel (vedi `LVM_SETCOLUMNWIDTH`).
#[cfg(target_os = "windows")]
pub fn set_listctrl_column_width(hwnd: isize, col: i32, width: i32) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
    if hwnd == 0 {
        return;
    }
    let h = hwnd as HWND;
    // SAFETY: HWND ListView valido; argomenti Win32 POD.
    unsafe {
        SendMessageW(
            h,
            0x101E, /* LVM_SETCOLUMNWIDTH */
            col as usize,
            width as isize,
        );
    }
}

#[cfg(not(target_os = "windows"))]
pub fn set_listctrl_column_width(_hwnd: isize, _col: i32, _width: i32) {}

/// Allinea a destra il testo di una colonna della `ListCtrl` Win32
/// (`LVCFMT_RIGHT`), come fa Esplora risorse con la colonna "Size".
/// `col` è 0-based (vedi `LVM_SETCOLUMNW`).
#[cfg(target_os = "windows")]
pub fn set_listctrl_column_align_right(hwnd: isize, col: i32) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;

    /// `LVM_SETCOLUMNW` (`LVM_FIRST + 96`).
    const LVM_SETCOLUMNW: u32 = 0x1000 + 96;
    /// `LVCF_FMT` — il campo `fmt` è valido.
    const LVCF_FMT: u32 = 0x0001;
    /// `LVCFMT_RIGHT`.
    const LVCFMT_RIGHT: i32 = 0x0001;

    /// Layout `LVCOLUMNW` di `commctrl.h`.
    #[repr(C)]
    #[allow(non_snake_case)]
    #[allow(clippy::upper_case_acronyms)] // mirrors the Win32 SDK struct name
    struct LVCOLUMNW {
        mask: u32,
        fmt: i32,
        cx: i32,
        psz_text: *const u16,
        cch_text_max: i32,
        i_sub_item: i32,
        i_image: i32,
        i_order: i32,
        cx_min: i32,
        cx_default: i32,
        cx_ideal: i32,
    }

    if hwnd == 0 {
        return;
    }
    let h = hwnd as HWND;
    let c = LVCOLUMNW {
        mask: LVCF_FMT,
        fmt: LVCFMT_RIGHT,
        cx: 0,
        psz_text: std::ptr::null(),
        cch_text_max: 0,
        i_sub_item: col,
        i_image: 0,
        i_order: 0,
        cx_min: 0,
        cx_default: 0,
        cx_ideal: 0,
    };
    // SAFETY: HWND ListView valido; `c` è POD e resta vivo per la call.
    unsafe {
        SendMessageW(
            h,
            LVM_SETCOLUMNW,
            col as usize,
            &c as *const LVCOLUMNW as isize,
        );
    }
}

#[cfg(not(target_os = "windows"))]
pub fn set_listctrl_column_align_right(_hwnd: isize, _col: i32) {}

/// Legge la larghezza corrente di una colonna della `ListCtrl` Win32.
/// Ritorna `width` (in pixel) o `None` se l'handle non è valido.
#[cfg(target_os = "windows")]
pub fn get_listctrl_column_width(hwnd: isize, col: i32) -> Option<i32> {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
    if hwnd == 0 {
        return None;
    }
    let h = hwnd as HWND;
    // SAFETY: HWND valido.
    let w = unsafe {
        SendMessageW(h, 0x101D /* LVM_GETCOLUMNWIDTH */, col as usize, 0)
    };
    Some(w as i32)
}

#[cfg(not(target_os = "windows"))]
pub fn get_listctrl_column_width(_hwnd: isize, _col: i32) -> Option<i32> {
    None
}

/// Mostra o nasconde l'indicatore di sort (freccia ↑/↓) sull'header
/// della `ListCtrl` Win32. Se `show == false`, `ascending` è ignorato.
#[cfg(target_os = "windows")]
pub fn set_listctrl_header_sort_indicator(hwnd: isize, col: i32, ascending: bool, show: bool) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::Controls::{
        HDF_SORTDOWN, HDF_SORTUP, HDITEMW, HDI_FORMAT, HDM_SETITEM, LVM_GETHEADER,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
    if hwnd == 0 {
        return;
    }
    let h = hwnd as HWND;
    // HDITEM layout su x64 (offset in byte): mask=0, cxy=4, pszText=8,
    // hbm=16, cchTextMax=24, fmt=28, lParam=32, …
    // Usiamo la struct `HDITEMW` di windows-sys invece di un array
    // `u32` a mano: l'offset di `fmt` era sbagliato ([5] = metà alta
    // di `hbm`) e la freccia di sort non veniva mai mostrata.
    let mut item: HDITEMW = unsafe { std::mem::zeroed() };
    item.mask = HDI_FORMAT;
    item.fmt = if !show {
        0
    } else if ascending {
        HDF_SORTUP
    } else {
        HDF_SORTDOWN
    };
    // SAFETY: HWND valido; HDITEMW è POD allineato come atteso da
    // HDM_SETITEM.
    unsafe {
        // L'header è un controllo figlio della ListView: prendiamo il
        // suo HWND con LVM_GETHEADER e inviamo lì HDM_SETITEM.
        let header_hwnd: HWND = SendMessageW(h, LVM_GETHEADER, 0, 0) as HWND;
        if !header_hwnd.is_null() {
            SendMessageW(
                header_hwnd,
                HDM_SETITEM,
                col as usize,
                &mut item as *mut HDITEMW as isize,
            );
        }
    }
}

/// Su macOS delega al backend `NSTableView`: freccia ↑/↓ sull'header
/// tramite `indicatorImage` della colonna.
#[cfg(target_os = "macos")]
pub fn set_listctrl_header_sort_indicator(hwnd: isize, col: i32, ascending: bool, show: bool) {
    crate::platform::appkit::listctrl_set_header_sort_indicator(hwnd, col, ascending, show);
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn set_listctrl_header_sort_indicator(_hwnd: isize, _col: i32, _ascending: bool, _show: bool) {}

/// Ritorna la posizione `(x, y)` **client-space del frame** del
/// top-left del controllo `hwnd` (es. una `ListCtrl`). `None` se
/// l'handle non è valido.
#[cfg(target_os = "windows")]
pub fn list_position_in_frame(hwnd: isize) -> Option<(i32, i32)> {
    use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;
    if hwnd == 0 {
        return None;
    }
    let h = hwnd as HWND;
    // SAFETY: HWND valido, RECT è POD.
    unsafe {
        let mut r: RECT = std::mem::zeroed();
        if GetWindowRect(h, &mut r) == 0 {
            return None;
        }
        let parent = frame_hwnd() as HWND;
        let mut pt = POINT {
            x: r.left,
            y: r.top,
        };
        if ScreenToClient(parent, &mut pt) == 0 {
            return None;
        }
        Some((pt.x, pt.y))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn list_position_in_frame(_hwnd: isize) -> Option<(i32, i32)> {
    None
}

/// Identifica quale bottone della toolbar (Win32 `ToolbarWindow32`)
/// si trova al punto `(x, y)` in coordinate **del frame**. Ritorna
/// l'ID del comando (`idCommand` del `TBBUTTON`) oppure `None`.
#[cfg(target_os = "windows")]
pub fn toolbar_hit_test(frame_hwnd: isize, tb_hwnd: isize, x: i32, y: i32) -> Option<u16> {
    use windows_sys::Win32::Foundation::{HWND, POINT};
    use windows_sys::Win32::Graphics::Gdi::{ClientToScreen, ScreenToClient};
    use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
    if tb_hwnd == 0 || frame_hwnd == 0 {
        return None;
    }
    let parent = frame_hwnd as HWND;
    let h_tb = tb_hwnd as HWND;
    let mut pt = POINT { x, y };
    // `MouseEvent::position` è già client-space del **frame**; non va
    // riconvertito con ScreenToClient(frame) (doppia traslazione che
    // spostava il punto dell'origine della finestra). TB_HITTEST
    // vuole invece coordinate client-space della **toolbar**:
    // frame-client → screen → toolbar-client.
    // SAFETY: HWND del frame e della toolbar validi; `pt` è POD.
    unsafe {
        if ClientToScreen(parent, &mut pt) == 0 {
            return None;
        }
        if ScreenToClient(h_tb, &mut pt) == 0 {
            return None;
        }
    }
    // SAFETY: HWND della toolbar valido.
    let res = unsafe {
        SendMessageW(
            h_tb,
            0x0445, /* TB_HITTEST */
            0,
            &mut pt as *mut _ as isize,
        )
    } as i32;
    if res < 0 {
        return None;
    }
    // Recupera l'id del comando via TB_GETBUTTON + i.
    let mut tbb: [u32; 6] = [0; 6];
    // SAFETY: TBBUTTON è POD.
    let ok = unsafe {
        SendMessageW(
            h_tb,
            0x0444, /* TB_GETBUTTON */
            res as usize,
            tbb.as_mut_ptr() as isize,
        )
    };
    if ok == 0 {
        return None;
    }
    // TBBUTTON layout (Win32): {iBitmap, idCommand, fsState, fsStyle, ...}
    // Noi riceviamo 6*4 byte, idCommand è il campo [1].
    let id = tbb[1] & 0xFFFF;
    Some(id as u16)
}

#[cfg(not(target_os = "windows"))]
pub fn toolbar_hit_test(_frame_hwnd: isize, _tb_hwnd: isize, _x: i32, _y: i32) -> Option<u16> {
    None
}

/// Ritorna l'`HWND` della `ToolBar` figlia del frame. `0` se non
/// disponibile (dichiarata per simmetria, non usata).
pub fn get_toolbar_hwnd(frame_hwnd: isize) -> isize {
    let _ = frame_hwnd;
    0
}

// ── Splitter ───────────────────────────────────────────────────────

/// Re-parent di una finestra figlia dentro un `SplitterWindow`.
///
/// Necessario perché `SplitterWindow::split_vertically` posiziona le
/// due pane a `(0, 0)` del loro *parent*. Se le pane sono figlie del
/// Frame, finiscono dietro la toolbar. Ri-parentandole dentro lo
/// SplitterWindow le coordinate vengono interpretate correttamente.
#[cfg(target_os = "windows")]
pub fn reparent_to_splitter(child_hwnd: isize, splitter_hwnd: isize) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::SetParent;
    unsafe {
        SetParent(child_hwnd as HWND, splitter_hwnd as HWND);
    }
}

/// Controparte AppKit: sposta la view della pane sotto la view dello
/// splitter mantenendo il frame corrente (`record_parent` aggiorna la
/// catena handle → frame).
#[cfg(target_os = "macos")]
pub fn reparent_to_splitter(child_hwnd: isize, splitter_hwnd: isize) {
    crate::platform::appkit::reparent_view_keeping_frame(child_hwnd, splitter_hwnd);
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn reparent_to_splitter(_child_hwnd: isize, _splitter_hwnd: isize) {}

/// Chiama `SplitterWindow::split_vertically` con handle `isize`,
/// isolando il cast (`isize → HWND` su Windows, identità su macOS)
/// così il chiamante (in genere `app.rs`) non deve importare
/// `windows_sys` direttamente.
#[cfg(target_os = "windows")]
pub fn split_vertically_panes(splitter: &crate::SplitterWindow, pane1: isize, pane2: isize) {
    use windows_sys::Win32::Foundation::HWND;
    splitter.split_vertically(pane1 as HWND, pane2 as HWND);
}

#[cfg(not(target_os = "windows"))]
pub fn split_vertically_panes(splitter: &crate::SplitterWindow, pane1: isize, pane2: isize) {
    splitter.split_vertically(pane1, pane2);
}

// ── Monitor check ──────────────────────────────────────────────────

/// Verifica che la posizione (x, y) sia dentro un monitor attualmente
/// connesso. Usata da `rust_explorer::geometry::clamp_to_visible_monitors`.
///
/// Windows: `MonitorFromPoint(MONITOR_DEFAULTTONULL)`. macOS: i frame
/// di `NSScreen` (stesso spazio coordinate di `mac_dpi_for_point`).
/// Ritorna `true` anche quando il check non è disponibile: meglio non
/// clampare che clampare male.
#[cfg(target_os = "windows")]
pub fn point_is_on_visible_monitor(x: i32, y: i32) -> bool {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTONULL};
    let pt = POINT { x, y };
    let monitor = unsafe { MonitorFromPoint(pt, MONITOR_DEFAULTTONULL) };
    !monitor.is_null()
}

#[cfg(target_os = "macos")]
pub fn point_is_on_visible_monitor(x: i32, y: i32) -> bool {
    use objc2_app_kit::NSScreen;
    use objc2_foundation::{MainThreadMarker, NSPoint};

    let Some(mtm) = MainThreadMarker::new() else {
        return true;
    };
    let point = NSPoint::new(x as f64, y as f64);
    for screen in NSScreen::screens(mtm) {
        let frame = screen.frame();
        if point.x >= frame.origin.x
            && point.y >= frame.origin.y
            && point.x < frame.origin.x + frame.size.width
            && point.y < frame.origin.y + frame.size.height
        {
            return true;
        }
    }
    false
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn point_is_on_visible_monitor(_x: i32, _y: i32) -> bool {
    true
}

/// Clamp della geometria finestra: se la posizione (x, y) è
/// off-screen (monitor disconnesso), ritorna `default_x`/`default_y`,
/// altrimenti la posizione originale. Il punto testato è
/// (x + 50, y + 50), cioè un punto interno alla finestra così anche
/// una finestra che sborda appena dal bordo viene ritenuta visibile.
///
/// Cross-platform: usa [`point_is_on_visible_monitor`].
pub fn clamp_to_visible_monitors(x: i32, y: i32, default_x: i32, default_y: i32) -> (i32, i32) {
    if point_is_on_visible_monitor(x + 50, y + 50) {
        (x, y)
    } else {
        (default_x, default_y)
    }
}

// ── Random / URL / secrets ─────────────────────────────────────────

/// Genera `n` byte casuali crittograficamente sicuri usando l'RNG di
/// sistema (`BCryptGenRandom` su Windows, `/dev/urandom` altrove).
/// Usato per `code_verifier`/`state` PKCE.
#[cfg(target_os = "windows")]
pub fn random_bytes(n: usize) -> Vec<u8> {
    use windows_sys::Win32::Security::Cryptography::BCryptGenRandom;
    const BCRYPT_USE_SYSTEM_PREFERRED_RNG: u32 = 0x0000_0002;
    let mut buf = vec![0u8; n];
    // SAFETY: `buf` è un buffer valido di `n` byte; il flag usa l'RNG
    // di sistema preferito (nessun handle da passare).
    let status = unsafe {
        BCryptGenRandom(
            std::ptr::null_mut(),
            buf.as_mut_ptr(),
            n as u32,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };
    if status < 0 {
        // Fallback deterministico (solo in ambienti degradati): non è
        // crittograficamente sicuro ma evita di restituire tutti zeri.
        for (i, byte) in buf.iter_mut().enumerate() {
            *byte = (i as u8).wrapping_mul(31).wrapping_add(17);
        }
    }
    buf
}

#[cfg(not(target_os = "windows"))]
pub fn random_bytes(n: usize) -> Vec<u8> {
    use std::io::Read;
    let mut buf = vec![0u8; n];
    let filled = std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut buf))
        .is_ok();
    if !filled {
        // Fallback deterministico (solo in ambienti degradati).
        for (i, byte) in buf.iter_mut().enumerate() {
            *byte = (i as u8).wrapping_mul(31).wrapping_add(17);
        }
    }
    buf
}

/// Sezione di "Privacy e Sicurezza" delle Impostazioni di Sistema
/// raggiungibile via URL (solo macOS).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivacyPane {
    /// "File e cartelle": Desktop, Documenti, Download, volumi esterni.
    FilesAndFolders,
    /// "Accesso completo al disco".
    FullDiskAccess,
}

/// Apre "Privacy e Sicurezza" nelle Impostazioni di Sistema.
///
/// Serve quando una cartella protetta da TCC (macOS: Desktop,
/// Documenti, Download, volumi esterni) nega l'accesso: il sistema
/// ricorda il rifiuto e non ripresenta più il dialogo, quindi all'utente
/// non resta che riabilitare l'app a mano. Sulle altre piattaforme non
/// esiste un equivalente e la funzione non fa nulla.
pub fn open_privacy_settings(pane: PrivacyPane) -> Result<(), ShellError> {
    #[cfg(target_os = "macos")]
    {
        // Gli anchor del pannello Privacy sono stabili da Catalina e
        // vengono ancora mappati su "Privacy e Sicurezza" in Ventura+.
        let anchor = match pane {
            PrivacyPane::FilesAndFolders => "Privacy_FilesAndFolders",
            PrivacyPane::FullDiskAccess => "Privacy_AllFiles",
        };
        open_url(&format!(
            "x-apple.systempreferences:com.apple.preference.security?{anchor}"
        ))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = pane;
        Ok(())
    }
}

/// Apre `url` nel browser predefinito (`ShellExecuteW` con "open" su
/// Windows, `NSWorkspace` su macOS — vedi [`crate::core::open_url`]).
pub fn open_url(url: &str) -> Result<(), ShellError> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        let wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
        let open: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: buffer wide NUL-terminati validi che sopravvivono
        // alla chiamata sincrona.
        let rc = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                open.as_ptr(),
                wide.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1, // SW_SHOWNORMAL
            )
        };
        if (rc as isize) > 32 {
            Ok(())
        } else {
            Err(ShellError::AccessDenied)
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if crate::core::open_url::open_url_in_browser(url) {
            Ok(())
        } else {
            Err(ShellError::AccessDenied)
        }
    }
}

/// Apre un terminale con `cwd` come directory di lavoro.
///
/// Piattaforma:
/// * **Windows** — preferisce `pwsh.exe` (PowerShell 7+), poi
///   `powershell.exe` (Windows PowerShell 5.1), poi `cmd.exe`. Usa
///   `ShellExecuteW` con `Verb = "open"`, che lascia al sistema il
///   lookup nel PATH e l'avvio in una nuova finestra. Ritorna
///   `Err(ShellError::NotImplemented)` se nessuno dei tre è trovato.
/// * **macOS** — apre Terminal.app con `open -a Terminal <cwd>`
///   (`open` sa risolvere `Terminal` come applicazione registrata).
/// * **Linux / altri Unix** — tenta nell'ordine `x-terminal-emulator`,
///   `gnome-terminal`, `konsole`, `xfce4-terminal`, `xterm`, lanciati
///   con `--working-directory=<cwd>`. Restituisce
///   `Err(ShellError::NotImplemented)` se nessuno è disponibile.
///
/// La funzione è asincrona rispetto al processo chiamante: il
/// terminale si apre in una nuova finestra senza bloccare il thread UI.
pub fn open_in_terminal(cwd: &std::path::Path) -> Result<(), ShellError> {
    let cwd_str = cwd.to_string_lossy().into_owned();
    if cwd_str.is_empty() {
        return Err(ShellError::InvalidPath);
    }
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        // Cerca i candidati nell'ordine. `which` non esiste su Windows;
        // uso `where` ma solo come euristica. In pratica
        // `ShellExecuteW` con un comando senza path assoluto delega al
        // processo `CreateProcess` la ricerca nel PATH, quindi mi
        // basta scegliere il primo candidato *probabile* e fare
        // fallback sugli altri se fallisce.
        let candidates: [&str; 3] = ["pwsh.exe", "powershell.exe", "cmd.exe"];
        for cmd in &candidates {
            // Su Windows, `powershell.exe -NoExit -Command "cd <cwd>"`
            // tiene aperta la finestra. `cmd.exe /K cd /d <cwd>` apre
            // un prompt classico che resta attivo. Per `pwsh` /
            // `powershell` uso `-NoExit -Command cd <cwd>`; per `cmd`
            // `/K cd /d <cwd>`.
            let params = if cmd.contains("pwsh") || cmd.contains("powershell") {
                format!(
                    "-NoExit -Command \"Set-Location -LiteralPath '{}'\"",
                    cwd_str.replace('\'', "''")
                )
            } else {
                format!("/K cd /d \"{}\"", cwd_str)
            };
            let wide_cmd: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0)).collect();
            let wide_params: Vec<u16> = params.encode_utf16().chain(std::iter::once(0)).collect();
            let open: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
            // SAFETY: i buffer wide sono validi e NUL-terminati per
            // tutta la durata della chiamata sincrona.
            let rc = unsafe {
                ShellExecuteW(
                    std::ptr::null_mut(),
                    open.as_ptr(),
                    wide_cmd.as_ptr(),
                    wide_params.as_ptr(),
                    std::ptr::null(),
                    1, // SW_SHOWNORMAL
                )
            } as isize;
            if rc > 32 {
                return Ok(());
            }
        }
        Err(ShellError::NotImplemented)
    }
    #[cfg(target_os = "macos")]
    {
        // `open -a Terminal <cwd>` apre Terminal.app nella directory.
        // `open` è sempre presente in `/usr/bin/open` su macOS.
        let status = std::process::Command::new("/usr/bin/open")
            .arg("-a")
            .arg("Terminal")
            .arg(&cwd_str)
            .status();
        match status {
            Ok(s) if s.success() => Ok(()),
            _ => Err(ShellError::NotImplemented),
        }
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        // Candidati con il flag per fissare la cwd.
        let candidates: [(&str, &[&str]); 5] = [
            ("x-terminal-emulator", &["--working-directory"]),
            ("gnome-terminal", &["--working-directory"]),
            ("konsole", &["--workdir"]),
            ("xfce4-terminal", &["--working-directory"]),
            ("xterm", &[]), // xterm non supporta --working-directory.
        ];
        for (cmd, flag) in &candidates {
            let mut c = std::process::Command::new(cmd);
            for f in *flag {
                c.arg(f).arg(&cwd_str);
            }
            if flag.is_empty() {
                // xterm: lancia una shell che fa `cd`.
                c.arg("-e").arg("sh").arg("-c").arg(format!(
                    "cd '{}' && exec $SHELL",
                    cwd_str.replace('\'', "'\\''")
                ));
            }
            match c.status() {
                Ok(s) if s.success() => return Ok(()),
                _ => continue,
            }
        }
        Err(ShellError::NotImplemented)
    }
}

/// Cifra `plain` e ritorna il blob in Base64 prefissato `dpapi:`.
///
/// Windows: DPAPI (`CryptProtectData`), legato all'account utente
/// corrente. non-Windows: fallback degradato Base64 (nessun keychain
/// integrato in questa milestone) — stessa sintassi, stesso contract
/// di [`dpapi_decrypt`]; il chiamante tratta `Err` come "lascia in
/// chiaro".
#[cfg(target_os = "windows")]
pub fn dpapi_encrypt(plain: &str) -> Result<String, String> {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let bytes = plain.as_bytes();
    let in_blob = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut out_blob = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // SAFETY: `in_blob` punta a byte validi che sopravvivono alla chiamata
    // sincrona; `out_blob` viene riempito da DPAPI.
    let ok = unsafe {
        CryptProtectData(
            &in_blob,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out_blob,
        )
    };
    if ok == 0 {
        return Err("CryptProtectData failed".into());
    }
    let mut buf = Vec::with_capacity(out_blob.cbData as usize);
    if out_blob.cbData > 0 {
        // SAFETY: `out_blob.pbData` è valido per `cbData` byte.
        unsafe {
            std::ptr::copy_nonoverlapping(
                out_blob.pbData,
                buf.as_mut_ptr(),
                out_blob.cbData as usize,
            );
            buf.set_len(out_blob.cbData as usize);
        }
    }
    // SAFETY: libera il buffer allocato da `CryptProtectData`.
    unsafe { LocalFree(out_blob.pbData as *mut core::ffi::c_void) };
    Ok(format!("dpapi:{}", base64_encode(&buf)))
}

#[cfg(not(target_os = "windows"))]
pub fn dpapi_encrypt(plain: &str) -> Result<String, String> {
    Ok(format!("dpapi:{}", base64_encode(plain.as_bytes())))
}

/// Decifra un valore prodotto da [`dpapi_encrypt`]. Se `encoded` non
/// inizia con `dpapi:` (es. vecchio valore in chiaro) ritorna
/// `Ok(None)`; il chiamante può così mantenere la retrocompatibilità.
#[cfg(target_os = "windows")]
pub fn dpapi_decrypt(encoded: &str) -> Result<Option<String>, String> {
    let Some(b64) = encoded.strip_prefix("dpapi:") else {
        return Ok(None);
    };
    let Some(bytes) = base64_decode(b64) else {
        return Ok(None);
    };
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let in_blob = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut out_blob = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // SAFETY: `in_blob` punta a byte validi che sopravvivono alla chiamata
    // sincrona; `out_blob` viene riempito da DPAPI.
    let ok = unsafe {
        CryptUnprotectData(
            &in_blob,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out_blob,
        )
    };
    if ok == 0 {
        return Err("CryptUnprotectData failed".into());
    }
    let mut out = Vec::with_capacity(out_blob.cbData as usize);
    if out_blob.cbData > 0 {
        // SAFETY: `out_blob.pbData` è valido per `cbData` byte.
        unsafe {
            std::ptr::copy_nonoverlapping(
                out_blob.pbData,
                out.as_mut_ptr(),
                out_blob.cbData as usize,
            );
            out.set_len(out_blob.cbData as usize);
        }
    }
    // SAFETY: libera il buffer allocato da `CryptUnprotectData`.
    unsafe { LocalFree(out_blob.pbData as *mut core::ffi::c_void) };
    let text = String::from_utf8(out).map_err(|_| "DPAPI plaintext non UTF-8".to_string())?;
    Ok(Some(text))
}

#[cfg(not(target_os = "windows"))]
pub fn dpapi_decrypt(encoded: &str) -> Result<Option<String>, String> {
    let Some(b64) = encoded.strip_prefix("dpapi:") else {
        return Ok(None);
    };
    let Some(bytes) = base64_decode(b64) else {
        return Ok(None);
    };
    let text = String::from_utf8(bytes).map_err(|_| "plaintext non UTF-8".to_string())?;
    Ok(Some(text))
}

/// Base64 standard (RFC 4648, alfabeto con padding).
fn base64_encode(bytes: &[u8]) -> String {
    const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let b0 = bytes[i] as u32;
        let b1 = bytes[i + 1] as u32;
        let b2 = bytes[i + 2] as u32;
        out.push(CHARS[(b0 >> 2) as usize] as char);
        out.push(CHARS[(((b0 & 0b11) << 4) | (b1 >> 4)) as usize] as char);
        out.push(CHARS[(((b1 & 0b1111) << 2) | (b2 >> 6)) as usize] as char);
        out.push(CHARS[(b2 & 0b111111) as usize] as char);
        i += 3;
    }
    let rem = bytes.len() - i;
    if rem == 1 {
        let b0 = bytes[i] as u32;
        out.push(CHARS[(b0 >> 2) as usize] as char);
        out.push(CHARS[((b0 & 0b11) << 4) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let b0 = bytes[i] as u32;
        let b1 = bytes[i + 1] as u32;
        out.push(CHARS[(b0 >> 2) as usize] as char);
        out.push(CHARS[(((b0 & 0b11) << 4) | (b1 >> 4)) as usize] as char);
        out.push(CHARS[((b1 & 0b1111) << 2) as usize] as char);
        out.push('=');
    }
    out
}

/// Decodifica Base64 standard; ignora spazi/newline e termina sul
/// padding `=`. Ritorna `None` solo su input malformato (non-base64).
fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    for c in s.bytes() {
        let v: u32 = match c {
            b'A'..=b'Z' => (c - b'A') as u32,
            b'a'..=b'z' => (c - b'a' + 26) as u32,
            b'0'..=b'9' => (c - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            b' ' | b'\r' | b'\n' | b'\t' => continue,
            _ => return None,
        };
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= if bits == 0 { 0 } else { (1u32 << bits) - 1 };
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    /// Regressione Fase D: su macOS il click sull'header colonna non
    /// arrivava mai all'app perché `list_column_click_index` era uno
    /// stub che ritornava sempre `None`. Ora `lparam` trasporta
    /// direttamente l'indice di colonna (come già per
    /// `list_activate_item_index`).
    #[cfg(target_os = "macos")]
    #[test]
    fn list_column_click_index_macos_decodifica_lparam() {
        assert_eq!(super::list_column_click_index(0), Some(0));
        assert_eq!(super::list_column_click_index(3), Some(3));
        assert_eq!(super::list_column_click_index(-1), None);
    }

    /// Regressione spazio libero: su macOS l'API era letter-based
    /// (`drive_free_space('C')`) e ritornava sempre `None`, lasciando
    /// la status bar con `" Free: ---- "`. `volume_free_space` usa
    /// `statvfs` su path POSIX e deve restituire una tupla
    /// `(free, total)` realistica su un path esistente.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn volume_free_space_root_returns_positive_totals() {
        let p = std::path::Path::new("/");
        let (free, total) = super::volume_free_space(p).expect("statvfs su /");
        assert!(total > 0, "total deve essere > 0 (era {total})");
        assert!(free > 0, "free deve essere > 0 (era {free})");
        // Lo spazio libero non può eccedere il totale.
        assert!(free <= total);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn volume_free_space_home_directory_works() {
        let home = std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("/"));
        let (free, total) = super::volume_free_space(&home).expect("statvfs su HOME");
        assert!(total > 0);
        assert!(free > 0);
        assert!(free <= total);
    }
}
