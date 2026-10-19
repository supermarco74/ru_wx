//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Dynamic library loader (`wxDynamicLibrary`).

#[cfg(target_os = "windows")]
use std::ffi::CString;
use std::path::Path;

/// Loaded shared library (`wxDynamicLibrary`).
pub struct DynamicLibrary {
    #[cfg(target_os = "windows")]
    handle: windows_sys::Win32::Foundation::HMODULE,
    #[cfg(target_os = "macos")]
    handle: isize,
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
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
            let handle =
                unsafe { windows_sys::Win32::System::LibraryLoader::LoadLibraryW(wide.as_ptr()) };
            if handle.is_null() {
                return Err(format!("failed to load {}", path.display()));
            }
            Ok(Self { handle })
        }
        #[cfg(target_os = "macos")]
        {
            use std::ffi::CString;
            use std::os::raw::{c_char, c_void};
            extern "C" {
                fn dlopen(path: *const c_char, mode: i32) -> *mut c_void;
                fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
                fn dlclose(handle: *mut c_void) -> i32;
            }
            const RTLD_LAZY: i32 = 0x1;
            let path_c =
                CString::new(path.to_string_lossy().as_ref()).map_err(|e| e.to_string())?;
            let handle = unsafe { dlopen(path_c.as_ptr(), RTLD_LAZY) };
            if handle.is_null() {
                return Err(format!("failed to load {}", path.display()));
            }
            Ok(Self {
                handle: handle as isize,
            })
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            let _ = path;
            Err("DynamicLibrary not implemented on this platform".to_string())
        }
    }

    pub fn symbol(&self, name: &str) -> Result<*const (), String> {
        #[cfg(target_os = "windows")]
        let cname = CString::new(name).map_err(|e| e.to_string())?;
        #[cfg(target_os = "windows")]
        {
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
        #[cfg(target_os = "macos")]
        {
            use std::ffi::CString;
            use std::os::raw::{c_char, c_void};
            extern "C" {
                fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
            }
            let cname = CString::new(name).map_err(|e| e.to_string())?;
            let ptr = unsafe { dlsym(self.handle as *mut c_void, cname.as_ptr()) };
            if ptr.is_null() {
                return Err(format!("symbol not found: {name}"));
            }
            Ok(ptr as *const ())
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
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
        #[cfg(target_os = "macos")]
        {
            self.handle != 0
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
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

#[cfg(target_os = "macos")]
impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        if self.handle != 0 {
            extern "C" {
                fn dlclose(handle: *mut std::os::raw::c_void) -> i32;
            }
            unsafe {
                let _ = dlclose(self.handle as *mut std::os::raw::c_void);
            }
        }
    }
}
