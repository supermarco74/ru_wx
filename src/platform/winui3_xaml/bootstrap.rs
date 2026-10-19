//! WinRT/WinUI3 bootstrap initialization
//!
//! Handles the Windows App SDK bootstrap initialization required for WinUI3.
//! Based on windows-rs reactor patterns (crates/libs/reactor/src/bootstrap.rs).
//!
//! This MUST be called before using any WinUI3 APIs.
//!
//! # Version targeting
//!
//! The bootstrap channel must match a WinAppRuntime package installed on the
//! machine AND be supported by the `Microsoft.WindowsAppRuntime.Bootstrap.dll`
//! that gets loaded at runtime. The constants below target the **2.0 channel**,
//! mirroring the upstream `windows-reactor` bootstrap
//! (`crates/libs/reactor/src/bindings.rs`: `WINDOWSAPPSDK_RELEASE_MAJORMINOR =
//! 0x00020000`, minimum runtime version `2.0.1.0`), which is what the reactor's
//! generated WinRT bindings are compiled against. Any installed 2.x runtime
//! (e.g. `Microsoft.WindowsAppRuntime.2_2.5.1.0`) satisfies it.
//!
//! To retarget a different channel, change `WINDOWSAPPSDK_RELEASE_MAJORMINOR`
//! and `WINDOWSAPPSDK_RUNTIME_VERSION_UINT64` together.

use windows_core::link;

/// Windows App SDK major.minor version — **channel 2.0** (`0x00020000`).
///
/// Encoding: `(major << 16) | minor`. Matches the upstream reactor and the
/// installed `Microsoft.WindowsAppRuntime.2` framework package.
const WINDOWSAPPSDK_RELEASE_MAJORMINOR: u32 = 0x0002_0000;

/// Version tag for bootstrap (empty string = stable channel).
///
/// Deve essere **NUL-terminato**: la DLL legge una PCWSTR, quindi `&[]` (il cui
/// `as_ptr()` è un dangling non nullo e non terminato) provoca un access
/// violation dentro `microsoft.windowsappruntime.bootstrap.dll`. Il reactor usa
/// `windows_core::w!("")`, equivalente a questa slice `[0u16]`.
const WINDOWSAPPSDK_RELEASE_VERSION_TAG_W: &[u16] = &[0];

/// Minimum acceptable runtime version as `PACKAGE_VERSION` (u64).
///
/// Encoding: `(major << 48) | (minor << 32) | (build << 16) | revision`.
/// `2.0.1.0` — identica al valore del reactor a monte
/// (`WINDOWSAPPSDK_RUNTIME_VERSION_UINT64 = 562949953486848`).
const WINDOWSAPPSDK_RUNTIME_VERSION_UINT64: u64 = (2_u64 << 48) | (1_u64 << 16);

/// Bootstrap options
const MDD_BOOTSTRAP_INITIALIZE_OPTIONS_NONE: u32 = 0;

/// PACKAGE_VERSION struct for bootstrap (mirrors Windows SDK)
#[repr(C)]
#[derive(Clone, Copy)]
#[allow(non_snake_case)] // field name pinned to the Windows SDK PACKAGE_VERSION ABI
struct PACKAGE_VERSION {
    Anonymous: PACKAGE_VERSION_0,
}
#[repr(C)]
#[derive(Clone, Copy)]
#[allow(non_snake_case)] // field name pinned to the Windows SDK PACKAGE_VERSION ABI
union PACKAGE_VERSION_0 {
    Version: u64,
}

/// Initialize the Windows App SDK runtime.
/// This MUST be called before using any WinUI3 APIs.
/// Returns Ok(()) on success, Err on failure.
///
/// Safe to call multiple times: a successful initialization is remembered
/// and subsequent calls return immediately.
pub fn init_platform() -> Result<(), windows_core::Error> {
    use std::sync::OnceLock;
    static INIT_RESULT: OnceLock<Result<(), windows_core::Error>> = OnceLock::new();
    INIT_RESULT.get_or_init(init_platform_inner).clone()
}

fn init_platform_inner() -> Result<(), windows_core::Error> {
    unsafe {
        // Link to Windows App SDK bootstrap API
        // Pattern from windows-rs: windows_core::link!(dll "system" fn ...)
        link!("microsoft.windowsappruntime.bootstrap.dll" "system" fn MddBootstrapInitialize2(
            majorminorversion: u32,
            versiontag: *const u16,
            minversion: PACKAGE_VERSION,
            options: u32,
        ) -> windows_core::HRESULT);

        // Initialize WinAppSDK bootstrap
        let result = MddBootstrapInitialize2(
            WINDOWSAPPSDK_RELEASE_MAJORMINOR,
            WINDOWSAPPSDK_RELEASE_VERSION_TAG_W.as_ptr(),
            PACKAGE_VERSION {
                Anonymous: PACKAGE_VERSION_0 {
                    Version: WINDOWSAPPSDK_RUNTIME_VERSION_UINT64,
                },
            },
            MDD_BOOTSTRAP_INITIALIZE_OPTIONS_NONE,
        );

        result.ok()?;
        Ok(())
    }
}
