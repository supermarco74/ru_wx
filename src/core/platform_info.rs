//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Runtime platform descriptor (`wxPlatformInfo`).

/// Host OS and architecture summary (`wxPlatformInfo`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformInfo {
    pub os_family: OsFamily,
    pub arch: Arch,
    pub little_endian: bool,
}

/// Broad operating-system family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsFamily {
    Windows,
    MacOs,
    Linux,
    Other,
}

/// CPU architecture label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86,
    X86_64,
    Arm64,
    Other,
}

impl PlatformInfo {
    pub fn current() -> Self {
        let little_endian = cfg!(target_endian = "little");
        let os_family = if cfg!(target_os = "windows") {
            OsFamily::Windows
        } else if cfg!(target_os = "macos") {
            OsFamily::MacOs
        } else if cfg!(target_os = "linux") {
            OsFamily::Linux
        } else {
            OsFamily::Other
        };
        let arch = if cfg!(target_arch = "x86") {
            Arch::X86
        } else if cfg!(target_arch = "x86_64") {
            Arch::X86_64
        } else if cfg!(target_arch = "aarch64") {
            Arch::Arm64
        } else {
            Arch::Other
        };
        Self {
            os_family,
            arch,
            little_endian,
        }
    }

    pub fn os_description(&self) -> &'static str {
        match self.os_family {
            OsFamily::Windows => "Windows",
            OsFamily::MacOs => "macOS",
            OsFamily::Linux => "Linux",
            OsFamily::Other => "Other",
        }
    }
}
