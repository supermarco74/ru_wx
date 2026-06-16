//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Credential storage (`wxSecretStore`).

#[cfg(not(target_os = "windows"))]
use std::collections::HashMap;

/// Key/value secret vault (`wxSecretStore`).
#[derive(Debug, Default)]
pub struct SecretStore {
    #[cfg(not(target_os = "windows"))]
    secrets: HashMap<String, String>,
}

impl SecretStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn key(service: &str, user: &str) -> String {
        format!("{service}:{user}")
    }

    pub fn save(&mut self, service: &str, user: &str, secret: &str) -> bool {
        #[cfg(target_os = "windows")]
        {
            write_credential(service, user, secret)
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.secrets
                .insert(Self::key(service, user), secret.to_string());
            true
        }
    }

    pub fn load(&self, service: &str, user: &str) -> Option<String> {
        #[cfg(target_os = "windows")]
        {
            read_credential(service, user)
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.secrets.get(&Self::key(service, user)).cloned()
        }
    }

    pub fn delete(&mut self, service: &str, user: &str) -> bool {
        #[cfg(target_os = "windows")]
        {
            delete_credential(service, user)
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.secrets.remove(&Self::key(service, user)).is_some()
        }
    }
}

#[cfg(target_os = "windows")]
fn credential_target(service: &str, user: &str) -> String {
    format!("ru_wx:{service}:{user}")
}

#[cfg(target_os = "windows")]
fn write_credential(service: &str, user: &str, secret: &str) -> bool {
    use crate::platform::win32::to_wide;
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::Security::Credentials::{
        CredWriteW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW,
    };

    let target = to_wide(&credential_target(service, user));
    let user_wide = to_wide(user);
    let mut blob: Vec<u8> = secret.as_bytes().to_vec();
    let mut cred: CREDENTIALW = unsafe { std::mem::zeroed() };
    cred.Flags = 0;
    cred.Type = CRED_TYPE_GENERIC;
    cred.TargetName = target.as_ptr() as *mut u16;
    cred.Comment = std::ptr::null_mut();
    cred.LastWritten = unsafe { std::mem::zeroed() };
    cred.CredentialBlobSize = blob.len() as u32;
    cred.CredentialBlob = blob.as_mut_ptr();
    cred.Persist = CRED_PERSIST_LOCAL_MACHINE;
    cred.AttributeCount = 0;
    cred.Attributes = std::ptr::null_mut();
    cred.TargetAlias = std::ptr::null_mut();
    cred.UserName = user_wide.as_ptr() as *mut u16;
    // SAFETY: CREDENTIALW points at live wide strings and blob for the duration of the call.
    unsafe {
        CredWriteW(&cred, 0) != 0 || GetLastError() == 0
    }
}

#[cfg(target_os = "windows")]
fn read_credential(service: &str, user: &str) -> Option<String> {
    use crate::platform::win32::to_wide;
    use std::ffi::c_void;
    use windows_sys::Win32::Security::Credentials::{CredFree, CredReadW, CRED_TYPE_GENERIC};

    let target = to_wide(&credential_target(service, user));
    let mut pcred: *mut CREDENTIALW = std::ptr::null_mut();
    // SAFETY: CredReadW returns an allocated credential freed with CredFree.
    unsafe {
        if CredReadW(
            target.as_ptr(),
            CRED_TYPE_GENERIC,
            0,
            &mut pcred,
        ) == 0
            || pcred.is_null()
        {
            return None;
        }
        let cred = &*pcred;
        let bytes = std::slice::from_raw_parts(cred.CredentialBlob, cred.CredentialBlobSize as usize);
        let out = String::from_utf8_lossy(bytes).into_owned();
        CredFree(pcred as *mut c_void);
        Some(out)
    }
}

#[cfg(target_os = "windows")]
use windows_sys::Win32::Security::Credentials::CREDENTIALW;

#[cfg(target_os = "windows")]
fn delete_credential(service: &str, user: &str) -> bool {
    use crate::platform::win32::to_wide;
    use windows_sys::Win32::Security::Credentials::{CredDeleteW, CRED_TYPE_GENERIC};

    let target = to_wide(&credential_target(service, user));
    // SAFETY: CredDeleteW removes a previously stored generic credential.
    unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) != 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_store_round_trip() {
        let mut store = SecretStore::new();
        assert!(store.save("app", "user", "secret"));
        assert_eq!(store.load("app", "user").as_deref(), Some("secret"));
        assert!(store.delete("app", "user"));
        assert!(store.load("app", "user").is_none());
    }
}
