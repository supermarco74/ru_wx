//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Dynamic library loader (`wxDynamicLibrary`).

use std::ffi::CString;
use std::path::Path;

/// Loaded shared library (`wxDynamicLibrary`).
pub struct DynamicLibrary {
    #[cfg(target_os = "windows")]
    handle: windows_sys::Win32::Foundation::HMODULE,
    #[cfg(not(target_os = "windows"))]
    path: String,
}

impl DynamicLibrary {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        #[cfg(target_os = "windows")]
        {
            use crate::platform::win32::to_wide;
            let wide = to_wide(&path.display().to_string());
            // SAFETY: LoadLibraryW with a valid null-terminated path.
            let handle = unsafe { windows_sys::Win32::System::LibraryLoader::LoadLibraryW(wide.as_ptr()) };
            if handle.is_null() {
                return Err(format!("failed to load {}", path.display()));
            }
            Ok(Self { handle })
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = path;
            Err("DynamicLibrary not implemented on this platform".to_string())
        }
    }

    pub fn symbol(&self, name: &str) -> Result<*const (), String> {
        #[cfg(target_os = "windows")]
        {
            let cname = CString::new(name).map_err(|e| e.to_string())?;
            // SAFETY: GetProcAddress on a loaded module with a valid symbol name.
            let ptr = unsafe {
                windows_sys::Win32::System::LibraryLoader::GetProcAddress(
                    self.handle,
                    cname.as_ptr() as *const u8,
                )
            };
            if ptr.is_none() {
                return Err(format!("symbol not found: {name}"));
            }
            Ok(ptr.unwrap() as *const ())
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (self, name);
            Err("DynamicLibrary not implemented on this platform".to_string())
        }
    }

    pub fn is_loaded(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            !self.handle.is_null()
        }
        #[cfg(not(target_os = "windows"))]
        {
            false
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        // SAFETY: FreeLibrary on a handle returned by LoadLibraryW.
        unsafe {
            let _ = windows_sys::Win32::Foundation::FreeLibrary(self.handle);
        }
    }
}
