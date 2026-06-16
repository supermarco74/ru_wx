//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Registry config (`wxRegConfig`).

#[cfg(not(target_os = "windows"))]
use std::collections::HashMap;

/// Registry hive.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum RegRoot {
    #[default]
    CurrentUser,
    LocalMachine,
}

/// Registry-backed settings (`wxRegConfig`).
#[derive(Debug)]
pub struct RegConfig {
    root: RegRoot,
    path: String,
    #[cfg(not(target_os = "windows"))]
    values: HashMap<String, String>,
}

impl RegConfig {
    pub fn new(root: RegRoot, path: &str) -> Self {
        Self {
            root,
            path: path.to_string(),
            #[cfg(not(target_os = "windows"))]
            values: HashMap::new(),
        }
    }

    pub fn read(&self, name: &str, default: &str) -> String {
        #[cfg(target_os = "windows")]
        {
            read_reg_string(self.root, &self.path, name).unwrap_or_else(|| default.to_string())
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.values
                .get(name)
                .cloned()
                .unwrap_or_else(|| default.to_string())
        }
    }

    pub fn write(&mut self, name: &str, value: &str) {
        #[cfg(target_os = "windows")]
        {
            let _ = write_reg_string(self.root, &self.path, name, value);
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.values.insert(name.to_string(), value.to_string());
        }
    }

    pub fn root(&self) -> RegRoot {
        self.root
    }

    pub fn key_path(&self) -> &str {
        &self.path
    }
}

#[cfg(target_os = "windows")]
fn root_hkey(root: RegRoot) -> windows_sys::Win32::System::Registry::HKEY {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    match root {
        RegRoot::CurrentUser => HKEY_CURRENT_USER,
        RegRoot::LocalMachine => HKEY_LOCAL_MACHINE,
    }
}

#[cfg(target_os = "windows")]
fn read_reg_string(root: RegRoot, subkey: &str, name: &str) -> Option<String> {
    use crate::platform::win32::to_wide;
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, KEY_READ, REG_SZ,
    };

    let subkey_wide = to_wide(subkey);
    let name_wide = to_wide(name);
    let mut hkey: HKEY = std::ptr::null_mut();
    // SAFETY: Win32 registry read.
    unsafe {
        let open = RegOpenKeyExW(
            root_hkey(root),
            subkey_wide.as_ptr(),
            0,
            KEY_READ,
            &mut hkey,
        );
        if open != ERROR_SUCCESS || hkey.is_null() {
            return None;
        }
        let mut kind = 0u32;
        let mut size = 0u32;
        let q1 = RegQueryValueExW(
            hkey,
            name_wide.as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            std::ptr::null_mut(),
            &mut size,
        );
        if q1 != ERROR_SUCCESS || kind != REG_SZ || size < 2 {
            RegCloseKey(hkey);
            return None;
        }
        let mut buf = vec![0u16; (size as usize / 2) + 1];
        let q2 = RegQueryValueExW(
            hkey,
            name_wide.as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            buf.as_mut_ptr() as *mut u8,
            &mut size,
        );
        RegCloseKey(hkey);
        if q2 != ERROR_SUCCESS {
            return None;
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..len]))
    }
}

#[cfg(target_os = "windows")]
fn write_reg_string(root: RegRoot, subkey: &str, name: &str, value: &str) -> bool {
    use crate::platform::win32::to_wide;
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY, KEY_WRITE, REG_OPTION_NON_VOLATILE,
        REG_SZ,
    };

    let subkey_wide = to_wide(subkey);
    let name_wide = to_wide(name);
    let mut value_wide: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
    let mut hkey: HKEY = std::ptr::null_mut();
    let mut disp = 0u32;
    // SAFETY: Win32 registry write.
    unsafe {
        let created = RegCreateKeyExW(
            root_hkey(root),
            subkey_wide.as_ptr(),
            0,
            std::ptr::null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            std::ptr::null(),
            &mut hkey,
            &mut disp,
        );
        if created != ERROR_SUCCESS || hkey.is_null() {
            return false;
        }
        let byte_len = (value_wide.len() * 2) as u32;
        let set = RegSetValueExW(
            hkey,
            name_wide.as_ptr(),
            0,
            REG_SZ,
            value_wide.as_mut_ptr() as *mut u8,
            byte_len,
        );
        RegCloseKey(hkey);
        set == ERROR_SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reg_config_round_trip_non_windows_or_memory() {
        let mut cfg = RegConfig::new(RegRoot::CurrentUser, "Software\\ru_wx_test");
        cfg.write("answer", "42");
        assert_eq!(cfg.read("answer", "0"), "42");
        assert_eq!(cfg.read("missing", "default"), "default");
    }
}
