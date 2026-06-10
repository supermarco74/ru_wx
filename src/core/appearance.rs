//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Light / dark appearance detection (`wxAppearance`).

/// Application colour scheme preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    Light,
    Dark,
    /// Follow the OS setting.
    System,
}

impl Appearance {
    /// Best-effort OS dark-mode detection (Win10+ via `uxtheme.dll`).
    #[cfg(target_os = "windows")]
    pub fn system_is_dark() -> bool {
        type ShouldAppsUseDarkModeFn = unsafe extern "system" fn() -> i32;
        // SAFETY: Optional system export; null proc → assume light theme.
        unsafe {
            let dll = windows_sys::Win32::System::LibraryLoader::LoadLibraryW(
                crate::platform::win32::to_wide("uxtheme.dll").as_ptr(),
            );
            if dll.is_null() {
                return false;
            }
            let proc = windows_sys::Win32::System::LibraryLoader::GetProcAddress(
                dll,
                c"ShouldAppsUseDarkMode".as_ptr().cast(),
            );
            if let Some(f) = proc {
                let check: ShouldAppsUseDarkModeFn = std::mem::transmute(f);
                return check() != 0;
            }
            false
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn system_is_dark() -> bool {
        false
    }

    /// Resolve `System` to light or dark.
    pub fn resolve(self) -> bool {
        match self {
            Appearance::Light => false,
            Appearance::Dark => true,
            Appearance::System => Self::system_is_dark(),
        }
    }
}
