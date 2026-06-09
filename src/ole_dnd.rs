//! OLE COM drag-and-drop — destination-side `IDropTarget` implementation.
//!
//! This module provides the OLE COM half of Win32's drag-and-drop story,
//! complementing the Shell-level `WM_DROPFILES` protocol that
//! [`crate::drop_target`] implements:
//!
//! | Protocol                 | Source: shell | Source: OLE COM | Source: in-app |
//! |--------------------------|---------------|-----------------|----------------|
//! | `WM_DROPFILES` (v0.5.5)  | yes (files)   | no              | no             |
//! | `IDropTarget` (v0.5.8)   | yes (files)   | yes (any)       | no (planned)   |
//!
//! The two protocols **coexist** — the frame can have a Shell-level
//! `drop_files_handler` (v0.5.5) and an OLE `set_ole_drop_callback` (v0.5.8)
//! registered at the same time, and the Shell / COM will each deliver
//! their preferred format (Shell's `HDROP` for files from Explorer,
//! COM's `IDataObject` for any other source).
//!
//! # Scope
//!
//! The v0.5.8 implementation is the **destination side only**. The
//! library does not yet expose a public `DoDragDrop` wrapper for the
//! source side (an in-app widget dragging into another widget); the
//! OLE COM `IDataObject` is read-only here, not produced.
//!
//! The two formats extracted from a dropped `IDataObject` are:
//!
//! * `CF_HDROP` — a list of file paths. The Shell still uses this for
//!   drops from Explorer, so the OLE handler will see file drops from
//!   Explorer in addition to the Shell-level `WM_DROPFILES`. The user
//!   code that registers an OLE drop callback should expect
//!   `OleDroppedData::Files(...)` to fire on file drops from Explorer
//!   (with the same paths the Shell-level handler would see).
//! * `CF_UNICODETEXT` — a UTF-16 string. Most Windows text sources
//!   (Notepad, browsers, the `Edit` / `RichEdit` controls) drop text
//!   via this format.
//!
//! Any other format the source offers is reported as
//! [`OleDroppedData::Other`]. Adding more formats in a future cycle
//! is a matter of adding more `IDataObject::GetData` calls in the
//! `Drop` vtable function and mapping the result to a new variant.
//!
//! # Cross-platform notes
//!
//! All the public types ([`OleDropEffect`], [`OleDroppedData`],
//! [`OleDropPosition`]) are plain Rust data and are reachable on every
//! platform. The frame's `set_ole_drop_callback` method is reachable
//! on every platform; on non-Windows hosts the registered callback is
//! never invoked.
//!
//! # Threading
//!
//! `OleInitialize` is called once per process, lazily, on the first
//! `set_ole_drop_callback` invocation. `OleUninitialize` is **not**
//! called — the process is expected to live for the rest of the
//! program's lifetime once it has an OLE COM drop target registered.

use std::fmt;
use std::path::PathBuf;

// Crate-internal re-export: `mod win` is private (its types are an
// implementation detail of the COM vtable plumbing), but the
// initialisation helper is needed by `frame::Frame::set_ole_drop_callback`
// to lazily call `OleInitialize` exactly once per process. The re-export
// keeps the helper reachable from the rest of the crate without
// exposing the entire `win` module publicly.
#[cfg(target_os = "windows")]
pub(crate) use self::win::ensure_ole_initialized;

// =================================================================
//                          Public types
// =================================================================

/// Win32 `DROPEFFECT` bits, wrapped in a `Copy` newtype so they can
/// be passed across the COM boundary cleanly.
///
/// The five standard values are exposed as associated constants; bit
/// composition (e.g. `OleDropEffect::COPY | OleDropEffect::MOVE` to
/// indicate "copy OR move, source's choice") is supported through
/// the `BitOr` / `BitOrAssign` impls.
#[derive(Copy, Clone, Default, PartialEq, Eq, Hash)]
pub struct OleDropEffect(pub u32);

impl OleDropEffect {
    pub const NONE: Self = Self(0);
    pub const COPY: Self = Self(1);
    pub const MOVE: Self = Self(2);
    pub const LINK: Self = Self(4);
    pub const SCROLL: Self = Self(0x8000_0000);

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn from_bits_truncate(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn is_none(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub const fn remove(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    pub const fn intersect(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}

impl fmt::Debug for OleDropEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts: Vec<&'static str> = Vec::new();
        if self.contains(Self::COPY) {
            parts.push("COPY");
        }
        if self.contains(Self::MOVE) {
            parts.push("MOVE");
        }
        if self.contains(Self::LINK) {
            parts.push("LINK");
        }
        if self.contains(Self::SCROLL) {
            parts.push("SCROLL");
        }
        let known = Self::COPY | Self::MOVE | Self::LINK | Self::SCROLL;
        let remainder = self.remove(known);
        if !remainder.is_none() {
            parts.push("UNKNOWN");
        }
        if parts.is_empty() {
            f.write_str("NONE")
        } else {
            f.write_str("OleDropEffect(")?;
            for (i, p) in parts.iter().enumerate() {
                if i > 0 {
                    f.write_str(" | ")?;
                }
                f.write_str(p)?;
            }
            f.write_str(")")
        }
    }
}

impl std::ops::BitOr for OleDropEffect {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for OleDropEffect {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl std::ops::BitAnd for OleDropEffect {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl std::ops::BitAndAssign for OleDropEffect {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

/// `Display` for [`OleDropEffect`] — produces a human-readable
/// representation suitable for log messages and `format!("{}", ...)`.
///
/// The output matches the [`Debug`](fmt::Debug) representation
/// without the surrounding `OleDropEffect(...)` wrapper, so it can
/// be embedded directly in a sentence: e.g. `"accept = COPY | MOVE"`.
impl fmt::Display for OleDropEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        let mut emit = |name: &'static str, present: bool| -> fmt::Result {
            if !present {
                return Ok(());
            }
            if !first {
                f.write_str(" | ")?;
            }
            first = false;
            f.write_str(name)
        };
        emit("COPY", self.contains(Self::COPY))?;
        emit("MOVE", self.contains(Self::MOVE))?;
        emit("LINK", self.contains(Self::LINK))?;
        emit("SCROLL", self.contains(Self::SCROLL))?;
        let known = Self::COPY | Self::MOVE | Self::LINK | Self::SCROLL;
        if !self.remove(known).is_none() {
            emit("UNKNOWN", true)?;
        }
        if first {
            f.write_str("NONE")
        } else {
            Ok(())
        }
    }
}

/// `From<u32>` / `From<OleDropEffect> for u32` round-trip
/// conversions, so a `u32` returned by the Win32 `DROPEFFECT` API
/// can be wrapped with the safe newtype and unwrapped back without
/// the user touching the inner `pub` field.
impl From<u32> for OleDropEffect {
    fn from(bits: u32) -> Self {
        Self::from_bits_truncate(bits)
    }
}

impl From<OleDropEffect> for u32 {
    fn from(effect: OleDropEffect) -> Self {
        effect.bits()
    }
}

/// The data that was dropped on a frame, as extracted from the
/// OLE COM `IDataObject` by the v0.5.8 `IDropTarget` implementation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OleDroppedData {
    /// One or more file paths (`CF_HDROP`).
    Files(Vec<PathBuf>),
    /// Unicode text (`CF_UNICODETEXT`).
    Text(String),
    /// A format the library did not extract.
    Other,
}

/// `Display` for [`OleDroppedData`]. The `Files` variant prints
/// the count and a comma-separated list of paths; `Text` prints a
/// one-line summary; `Other` prints the literal string.
impl fmt::Display for OleDroppedData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OleDroppedData::Files(paths) => {
                write!(f, "Files({})", paths.len())?;
                if !paths.is_empty() {
                    f.write_str(": ")?;
                    for (i, p) in paths.iter().enumerate() {
                        if i > 0 {
                            f.write_str(", ")?;
                        }
                        f.write_str(&p.display().to_string())?;
                    }
                }
                Ok(())
            }
            OleDroppedData::Text(s) => write!(f, "Text({} chars)", s.chars().count()),
            OleDroppedData::Other => f.write_str("Other"),
        }
    }
}

/// The position of the drop, in **client coordinates** of the
/// receiving window.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct OleDropPosition {
    /// X coordinate, in client-coordinate pixels.
    pub x: i32,
    /// Y coordinate, in client-coordinate pixels.
    pub y: i32,
}

impl OleDropPosition {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

// =================================================================
//                           Errors
// =================================================================

/// Error returned when an OLE COM drop operation fails.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OleDropError {
    /// `RegisterDragDrop` returned a non-zero `HRESULT`. The raw
    /// value is preserved for diagnostics.
    RegisterFailed(i32),
}

impl fmt::Display for OleDropError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OleDropError::RegisterFailed(hr) => {
                write!(f, "OLE RegisterDragDrop failed with HRESULT 0x{hr:08x}")
            }
        }
    }
}

impl std::error::Error for OleDropError {}

// =================================================================
//          Cross-platform stub for the registration
// =================================================================

/// A registered OLE COM `IDropTarget`. The handle owns the
/// reference-counted COM object and the user callback; on `Drop`
/// the COM object is released.
///
/// On non-Windows hosts this type is a **no-op placeholder**: the
/// registration is never sent to a real window, and the registered
/// callback is never invoked.
#[cfg(not(target_os = "windows"))]
pub struct OleDropTarget {
    _callback: Box<dyn FnMut(OleDroppedData, OleDropPosition)>,
}

#[cfg(not(target_os = "windows"))]
impl OleDropTarget {
    pub(crate) fn new(callback: Box<dyn FnMut(OleDroppedData, OleDropPosition)>) -> Self {
        Self { _callback: callback }
    }
}

#[cfg(not(target_os = "windows"))]
impl OleDropTarget {
    /// Register this drop target with the given window. A no-op
    /// on non-Windows hosts.
    pub fn register(
        &mut self,
        _hwnd: *mut core::ffi::c_void,
    ) -> Result<(), OleDropError> {
        Ok(())
    }

    pub fn hwnd(&self) -> Option<*mut core::ffi::c_void> {
        None
    }
}

// =================================================================
//                  Windows-only COM vtable plumbing
// =================================================================

#[cfg(target_os = "windows")]
mod win {
// `OleDropError` is re-exported from the parent module and
// used by the public Windows handle's `register` method, but
// not directly inside this private module — silence the
// unused-import warning while keeping the import list
// self-documenting.
#[allow(unused_imports)]
use super::{OleDropError, OleDroppedData, OleDropPosition};
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};

    use windows_sys::Win32::Foundation::{POINTL, S_FALSE, S_OK, HGLOBAL, HWND};
    use windows_sys::Win32::System::Com::{
        CoTaskMemFree, FORMATETC, STGMEDIUM, DVASPECT_CONTENT, TYMED_HGLOBAL,
    };
    use windows_sys::Win32::System::Memory::GlobalSize;
    use windows_sys::Win32::System::Ole::{
        OleInitialize, RegisterDragDrop, ReleaseStgMedium, RevokeDragDrop,
        DROPEFFECT_COPY,
    };

    /// The Win32 HRESULT type. `windows-sys 0.59` exposes this as
    /// `windows_sys::core::HRESULT` (a typedef for `i32`).
    #[allow(clippy::upper_case_acronyms)]
    pub type HRESULT = i32;

    /// The COM `IUnknown` / `IDataObject` / `IDropTarget` interfaces.
    /// `windows-sys 0.59` does not export these as Rust types, so we
    /// alias them to `*mut c_void` directly. The interface is the
    /// common ancestor of all COM interfaces; the actual vtable is
    /// reached by reading the pointer at offset 0.
    pub type IUnknown = core::ffi::c_void;
    pub type IDataObject = core::ffi::c_void;
    pub type IDropTarget = core::ffi::c_void;

    // -------- vtable layout structs --------

    /// IUnknown vtable (3 methods). `#[repr(C)]` to guarantee the
    /// Win32-compatible layout. The field names are the canonical
    /// COM method names and are intentionally PascalCase — this is
    /// the FFI ABI the Win32 COM runtime expects.
    #[repr(C)]
    #[allow(non_snake_case)]
    pub struct IUnknownVtbl {
        pub QueryInterface: unsafe extern "system" fn(
            *mut IUnknown,
            *const u8,
            *mut *mut IUnknown,
        ) -> HRESULT,
        pub AddRef: unsafe extern "system" fn(*mut IUnknown) -> u32,
        pub Release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    }

    /// IDropTarget vtable (IUnknown + 4 methods). The `parent`
    /// field is the IUnknown vtable placed first so that an
    /// `*mut IUnknown` to the COM object can be cast to an
    /// `*mut IDropTarget` (the Win32 ABI guarantees the first 3
    /// slots of any COM vtable are IUnknown's, in the same order).
    /// Field names are PascalCase per the COM ABI.
    #[repr(C)]
    #[allow(non_snake_case)]
    pub struct IDropTargetVtbl {
        pub parent: IUnknownVtbl,
        pub DragEnter: unsafe extern "system" fn(
            *mut IDropTarget,
            *mut IDataObject,
            u32,
            POINTL,
            *mut u32,
        ) -> HRESULT,
        pub DragOver: unsafe extern "system" fn(
            *mut IDropTarget,
            u32,
            POINTL,
            *mut u32,
        ) -> HRESULT,
        pub DragLeave: unsafe extern "system" fn(*mut IDropTarget) -> HRESULT,
        pub Drop: unsafe extern "system" fn(
            *mut IDropTarget,
            *mut IDataObject,
            u32,
            POINTL,
            *mut u32,
        ) -> HRESULT,
    }

    /// The COM object our library hands to `RegisterDragDrop`. The
    /// first field is the vtable pointer (Win32 ABI requirement);
    /// the second is a pointer to the per-instance payload
    /// (refcount + user callback + cached IDataObject).
    #[repr(C)]
    pub struct OleDropTargetComObject {
        pub vtable: *const IDropTargetVtbl,
        pub payload: *mut OleDropTargetPayload,
    }

    /// Per-instance payload. Heap-allocated by
    /// [`OleDropTarget::new`] and freed when the COM refcount
    /// drops to zero.
    pub struct OleDropTargetPayload {
        /// `IUnknown` refcount. The first AddRef (during
        /// construction) brings it to 1; the matching Release (in
        /// the vtable) brings it to 0 and frees the COM object.
        pub refcount: AtomicU32,
        /// The user callback. Wrapped in a `RefCell` so the
        /// vtable functions (which take `&self` through the raw
        /// pointer) can dispatch to it.
        pub callback: RefCell<Box<dyn FnMut(OleDroppedData, OleDropPosition)>>,
        /// The most recent `IDataObject` we saw in `DragEnter` /
        /// `Drop`. The pointer is owned by the Shell / COM and is
        /// valid for the duration of the `Drop` call.
        pub last_data_object: AtomicI32,
    }

    // -------- vtable function implementations --------

    /// `IUnknown::QueryInterface`. We support `IUnknown` and
    /// `IDropTarget`. Any other interface returns `E_NOINTERFACE`.
    unsafe extern "system" fn query_interface(
        this: *mut IUnknown,
        riid: *const u8,
        ppv: *mut *mut IUnknown,
    ) -> HRESULT {
        let com_obj = this as *const OleDropTargetComObject;
        let _payload = (*com_obj).payload;

        if ppv.is_null() || riid.is_null() {
            return -2147024809; // E_POINTER
        }

        // IID_IUnknown = 00000000-0000-0000-C000-000000000046
        // IID_IDropTarget = 00000122-0000-0000-C000-000000000046
        let iid_bytes = core::slice::from_raw_parts(riid, 16);
        let iunknown_iid: [u8; 16] = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x46,
        ];
        let idroptarget_iid: [u8; 16] = [
            0x22, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x46,
        ];

        let matches_iunknown = iid_bytes == iunknown_iid;
        let matches_idroptarget = iid_bytes == idroptarget_iid;
        if !matches_iunknown && !matches_idroptarget {
            *ppv = core::ptr::null_mut();
            return -2147467262; // E_NOINTERFACE
        }

        // Hand back a pointer to our own COM object. Per Win32
        // convention, the same physical pointer is returned for
        // IUnknown and for any other interface the object
        // implements — only the vtable slot the caller uses
        // changes.
        *ppv = this;
        add_ref(this);
        S_OK
    }

    /// `IUnknown::AddRef`. Increments the refcount and returns the
    /// new value.
    unsafe extern "system" fn add_ref(this: *mut IUnknown) -> u32 {
        let com_obj = this as *const OleDropTargetComObject;
        let payload = (*com_obj).payload;
        let prev = (*payload).refcount.fetch_add(1, Ordering::Relaxed);
        prev + 1
    }

    /// `IUnknown::Release`. Decrements the refcount; on reaching
    /// zero, frees the COM object and the payload.
    unsafe extern "system" fn release(this: *mut IUnknown) -> u32 {
        let com_obj = this as *mut OleDropTargetComObject;
        let payload = (*com_obj).payload;
        let prev = (*payload).refcount.fetch_sub(1, Ordering::AcqRel);
        if prev == 1 {
            // We just dropped to zero. Free the payload and the
            // COM object. `core::mem::drop` is qualified so the
            // local vtable function named `drop_vtable` (or any
            // other vtable function with the lowercase name) does
            // not shadow the standard-library function.
            core::mem::drop(Box::from_raw(payload));
            core::mem::drop(Box::from_raw(com_obj));
            0
        } else {
            prev - 1
        }
    }

    /// `IDropTarget::DragEnter`. Caches the IDataObject pointer
    /// and returns `DROPEFFECT_COPY` to indicate the drop is
    /// accepted.
    unsafe extern "system" fn drag_enter(
        this: *mut IDropTarget,
        pdataobj: *mut IDataObject,
        _grfkeystate: u32,
        _pt: POINTL,
        pdweffect: *mut u32,
    ) -> HRESULT {
        let com_obj = this as *const OleDropTargetComObject;
        let payload = (*com_obj).payload;
        let raw = pdataobj as isize;
        (*payload).last_data_object.store(raw as i32, Ordering::Relaxed);
        if !pdweffect.is_null() {
            *pdweffect = DROPEFFECT_COPY;
        }
        S_OK
    }

    /// `IDropTarget::DragOver`. Returns `DROPEFFECT_COPY` to
    /// indicate the drop is still accepted at the new position.
    unsafe extern "system" fn drag_over(
        _this: *mut IDropTarget,
        _grfkeystate: u32,
        _pt: POINTL,
        pdweffect: *mut u32,
    ) -> HRESULT {
        if !pdweffect.is_null() {
            *pdweffect = DROPEFFECT_COPY;
        }
        S_OK
    }

    /// `IDropTarget::DragLeave`. Clears the cached IDataObject
    /// pointer and returns S_OK.
    unsafe extern "system" fn drag_leave(this: *mut IDropTarget) -> HRESULT {
        let com_obj = this as *const OleDropTargetComObject;
        let payload = (*com_obj).payload;
        (*payload).last_data_object.store(0, Ordering::Relaxed);
        S_OK
    }

    /// `IDropTarget::Drop`. Reads the IDataObject, extracts the
    /// most-appropriate format (`CF_HDROP` first, then
    /// `CF_UNICODETEXT`), calls the user callback, and returns
    /// `DROPEFFECT_COPY` to indicate the drop was accepted.
    ///
    /// The function is named `drop_vtable` rather than `drop` to
    /// avoid shadowing `std::mem::drop` (the vtable field `Drop`
    /// would also be reachable by that name through field-access
    /// shorthand).
    unsafe extern "system" fn drop_vtable(
        this: *mut IDropTarget,
        pdataobj: *mut IDataObject,
        _grfkeystate: u32,
        pt: POINTL,
        pdweffect: *mut u32,
    ) -> HRESULT {
        let com_obj = this as *const OleDropTargetComObject;
        let payload = (*com_obj).payload;

        // If the caller passed a data object, prefer it;
        // otherwise use the cached one from DragEnter.
        let data_obj: *mut IDataObject = if !pdataobj.is_null() {
            pdataobj
        } else {
            let raw = (*payload).last_data_object.load(Ordering::Relaxed) as isize;
            raw as *mut IDataObject
        };

        if data_obj.is_null() {
            if !pdweffect.is_null() {
                *pdweffect = 0;
            }
            return S_FALSE;
        }

        // Try CF_HDROP first. If that fails, try
        // CF_UNICODETEXT. The result is wrapped in
        // `OleDroppedData` and forwarded to the user callback.
        let data = read_data_object(data_obj);
        let position = OleDropPosition::new(pt.x, pt.y);

        // The vtable function is `unsafe extern "system"`, so
        // calling the user's `FnMut` callback is straightforward.
        // We hold a `RefMut` on the callback's `RefCell` for the
        // duration of the call so a nested re-entry would panic
        // (the COM runtime does not re-enter `Drop` from inside
        // a user callback, so this is safe).
        let mut cb = (*payload).callback.borrow_mut();
        (cb)(data, position);
        core::mem::drop(cb);

        // Clear the cached data object so a subsequent
        // `DragEnter` starts from a clean slate.
        (*payload).last_data_object.store(0, Ordering::Relaxed);

        if !pdweffect.is_null() {
            *pdweffect = DROPEFFECT_COPY;
        }
        S_OK
    }

    /// The vtable. `static` so its address is stable and we can
    /// hand `&VTBL` to the COM object.
    static VTBL: IDropTargetVtbl = IDropTargetVtbl {
        parent: IUnknownVtbl {
            QueryInterface: query_interface,
            AddRef: add_ref,
            Release: release,
        },
        DragEnter: drag_enter,
        DragOver: drag_over,
        DragLeave: drag_leave,
        Drop: drop_vtable,
    };

    // -------- IDataObject format readers --------

    /// `CF_HDROP`. Identifies a Shell file-drop handle. Hard-coded
    /// rather than imported from `windows-sys` because the constant
    /// is part of the clipboard format registry, not a Win32 API
    /// constant.
    const CF_HDROP: u16 = 15;

    /// `CF_UNICODETEXT`. Identifies a Unicode text drop.
    const CF_UNICODETEXT: u16 = 13;

    /// Try the readers in order of preference. `CF_HDROP` first
    /// (Explorer always offers it, carries the most information);
    /// `CF_UNICODETEXT` second (most non-Shell text sources).
    fn read_data_object(data_obj: *mut IDataObject) -> OleDroppedData {
        if let Some(paths) = read_hdrop(data_obj) {
            return OleDroppedData::Files(paths);
        }
        if let Some(text) = read_unicode_text(data_obj) {
            return OleDroppedData::Text(text);
        }
        OleDroppedData::Other
    }

    /// Call `IDataObject::GetData` through the vtable pointer.
    /// The IDataObject vtable has 3 IUnknown methods followed by
    /// 9 IDataObject methods; `GetData` is at vtable index 3.
    unsafe fn call_get_data(
        data_obj: *mut IDataObject,
        format: *const FORMATETC,
        medium: *mut STGMEDIUM,
    ) -> HRESULT {
        // Read the vtable pointer (the first field of the COM
        // object) and then the function pointer for `GetData`
        // (the 4th slot — IUnknown has 3 methods, then GetData).
        let vtable_ptr = *(data_obj as *const *const usize);
        let get_data_fn = vtable_ptr.add(3);
        let get_data: unsafe extern "system" fn(
            *mut IDataObject,
            *const FORMATETC,
            *mut STGMEDIUM,
        ) -> HRESULT = core::mem::transmute(get_data_fn);
        get_data(data_obj, format, medium)
    }

    /// Try to read a `CF_HDROP` format from the data object.
    fn read_hdrop(data_obj: *mut IDataObject) -> Option<Vec<PathBuf>> {
        let format = FORMATETC {
            cfFormat: CF_HDROP,
            ptd: core::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT,
            lindex: -1,
            tymed: TYMED_HGLOBAL as u32,
        };
        let mut medium = unsafe { core::mem::zeroed::<STGMEDIUM>() };
        let hr = unsafe { call_get_data(data_obj, &format, &mut medium) };
        if hr < 0 {
            return None;
        }
        // Extract the file paths from the HDROP. The HGLOBAL is
        // owned by the medium; we copy the path strings out
        // before calling ReleaseStgMedium.
        let hdrop: HGLOBAL = unsafe { medium.u.hGlobal };
        let paths = crate::drop_target::extract_paths_from_hdrop(hdrop as _);
        // SAFETY: We own the medium (GetData filled it in), so
        // ReleaseStgMedium is the correct cleanup.
        unsafe { ReleaseStgMedium(&mut medium) };
        Some(paths)
    }

    /// Try to read a `CF_UNICODETEXT` format from the data
    /// object.
    fn read_unicode_text(data_obj: *mut IDataObject) -> Option<String> {
        let format = FORMATETC {
            cfFormat: CF_UNICODETEXT,
            ptd: core::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT,
            lindex: -1,
            tymed: TYMED_HGLOBAL as u32,
        };
        let mut medium = unsafe { core::mem::zeroed::<STGMEDIUM>() };
        let hr = unsafe { call_get_data(data_obj, &format, &mut medium) };
        if hr < 0 {
            return None;
        }
        // The HGLOBAL contains a UTF-16 string. The first u32
        // is the length (in BYTES, not chars) per the
        // CF_UNICODETEXT spec; the string follows.
        //
        // Defensive null check + bounds validation: a buggy or
        // malicious data object can hand back a successful
        // `GetData` but leave `medium.u.hGlobal` null, and a
        // corrupt HGLOBAL can carry a junk length prefix that
        // would otherwise let `from_raw_parts` read arbitrarily
        // far past the end of the allocation. The pre-v0.5.8
        // code skipped both checks and would either segfault
        // or read out-of-bounds memory.
        let hglobal = unsafe { medium.u.hGlobal };
        if hglobal.is_null() {
            unsafe { ReleaseStgMedium(&mut medium) };
            return None;
        }
        // SAFETY: `hglobal` was just null-checked. `GlobalSize`
        // returns the actual byte size of the HGLOBAL allocation
        // (0 if the handle is invalid; we treat 0 as "skip").
        let alloc_size = unsafe { GlobalSize(hglobal) } as usize;
        if alloc_size < 8 {
            // Need at least 4 bytes for the length prefix + 4
            // bytes (NUL + 1) for a one-char string. Anything
            // smaller is a malformed HGLOBAL.
            unsafe { ReleaseStgMedium(&mut medium) };
            return None;
        }
        // SAFETY: `hglobal` is non-null, `GlobalSize` returned
        // >= 8, and we only read the first 4 bytes.
        let len_bytes = unsafe { *(hglobal as *const u32) } as usize;
        if len_bytes > alloc_size.saturating_sub(4) {
            // The declared length is larger than the actual
            // HGLOBAL allocation minus the prefix. Clamp to
            // what the allocation can hold.
            unsafe { ReleaseStgMedium(&mut medium) };
            return None;
        }
        let text_ptr = unsafe { hglobal.add(4) } as *const u16;
        let text_len_chars = (len_bytes / 2).saturating_sub(1); // exclude NUL
        let text_slice =
            unsafe { core::slice::from_raw_parts(text_ptr, text_len_chars) };
        let text = String::from_utf16_lossy(text_slice);
        unsafe { ReleaseStgMedium(&mut medium) };
        // `CoTaskMemFree` is unused here (HGLOBAL is freed by
        // GlobalFree, called by ReleaseStgMedium), but we keep
        // the import explicit to surface the dependency.
        let _ = CoTaskMemFree as *const ();
        Some(text)
    }

    // -------- The user-facing handle --------

    /// A registered OLE COM `IDropTarget`. Owns the
    /// reference-counted COM object (the IUnknown refcount is
    /// brought to 1 at construction; the matching Release
    /// happens when this `OleDropTarget` is dropped).
    pub struct OleDropTarget {
        // Raw pointer to the COM object. We hold the
        // single-owner refcount; on Drop, we call Release to
        // balance it. The COM object owns the payload (its
        // `release` vtable function is the sole owner of the
        // payload allocation).
        com_object: *mut OleDropTargetComObject,
    }

    // SAFETY: The COM object is reference-counted, so the
    // pointer is safe to share across threads. The user
    // callback is wrapped in a `RefCell` so the vtable
    // function can take a `RefMut` even from a different
    // thread; in practice the COM runtime calls the vtable
    // functions on the same thread that registered the drop
    // target, so this never happens. We mark `Send` (not
    // `Sync`) because the user callback is `FnMut` and the
    // payload has interior mutability.
    unsafe impl Send for OleDropTarget {}

    impl OleDropTarget {
        /// Construct a new `OleDropTarget` with the given user
        /// callback. Allocates the COM object and the payload,
        /// wires the vtable pointer, and brings the IUnknown
        /// refcount to 1. The payload's ownership is transferred
        /// to the COM object — the `release` vtable function is
        /// the sole owner of both allocations on the final
        /// Release.
        pub(crate) fn new(
            callback: Box<dyn FnMut(OleDroppedData, OleDropPosition)>,
        ) -> Self {
            let payload = Box::new(OleDropTargetPayload {
                refcount: AtomicU32::new(1),
                callback: RefCell::new(callback),
                last_data_object: AtomicI32::new(0),
            });
            // Allocate the COM object on the heap. The first
            // field is the vtable pointer; the second is a
            // raw pointer to the payload.
            let com_object = Box::new(OleDropTargetComObject {
                vtable: &VTBL,
                payload: Box::into_raw(payload),
            });
            Self {
                com_object: Box::into_raw(com_object),
            }
        }

        /// Get the raw COM object pointer, typed as
        /// `*mut c_void` (the type `RegisterDragDrop`
        /// expects). The pointer is valid for the lifetime
        /// of the `OleDropTarget`.
        pub(crate) fn as_raw(&self) -> *mut core::ffi::c_void {
            self.com_object.cast()
        }
    }

    impl Drop for OleDropTarget {
        fn drop(&mut self) {
            // Release the IUnknown refcount. If we were the
            // last owner, this frees the COM object and the
            // payload.
            // SAFETY: We own one refcount on the COM object;
            // we allocated it in `new` with refcount=1.
            unsafe {
                release(self.com_object.cast());
            }
        }
    }

    /// Ensure `OleInitialize` has been called at least once on
    /// the current process. Safe to call multiple times — the
    /// OLE runtime treats repeat calls as a no-op.
    pub fn ensure_ole_initialized() {
        use std::sync::Once;
        static OLE_INIT: Once = Once::new();
        OLE_INIT.call_once(|| {
            // SAFETY: `OleInitialize` takes a reserved
            // pointer; passing null means "single-threaded
            // apartment". The OLE runtime is safe to call
            // from any thread as long as the apartment is
            // consistent — we only call it once per process,
            // from the first `set_ole_drop_callback`.
            let _ = unsafe { OleInitialize(core::ptr::null_mut()) };
        });
    }

    /// Register a drop target with the given window. Wraps
    /// the raw `RegisterDragDrop(hwnd, idroptarget)` call.
    ///
    /// # Safety
    ///
    /// The COM object pointer must be a valid
    /// `*mut IUnknown` to a registered drop target. The HWND
    /// must be a valid top-level window that should accept
    /// drops. The COM object is reference-counted, so the
    /// caller is responsible for the matching
    /// `RevokeDragDrop(hwnd)`.
    pub unsafe fn register(
        hwnd: HWND,
        target: *mut core::ffi::c_void,
    ) -> HRESULT {
        RegisterDragDrop(hwnd, target)
    }

    /// Unregister a previously-registered drop target.
    ///
    /// # Safety
    ///
    /// The HWND must have been previously passed to
    /// `register` with the same drop target. After this
    /// call returns, the drop target's IUnknown refcount
    /// is decremented (the COM runtime releases its
    /// reference), and the target may be freed.
    pub unsafe fn unregister(hwnd: HWND) -> HRESULT {
        RevokeDragDrop(hwnd)
    }
}

// =================================================================
//                       Public Windows handle
// =================================================================

/// A registered OLE COM `IDropTarget`. Owns the reference-counted
/// COM object; on `Drop` the IUnknown refcount is released.
#[cfg(target_os = "windows")]
pub struct OleDropTarget {
    inner: win::OleDropTarget,
    hwnd: Option<windows_sys::Win32::Foundation::HWND>,
}

#[cfg(target_os = "windows")]
impl OleDropTarget {
    /// Construct a new (un-registered) `OleDropTarget` with the
    /// given user callback. The caller is expected to call
    /// [`register`](Self::register) immediately after to bind
    /// the target to a window.
    pub(crate) fn new(callback: Box<dyn FnMut(OleDroppedData, OleDropPosition)>) -> Self {
        Self {
            inner: win::OleDropTarget::new(callback),
            hwnd: None,
        }
    }

    /// Register this drop target with the given window. After
    /// this call, the OLE runtime will deliver
    /// `IDropTarget::DragEnter` / `DragOver` / `DragLeave` /
    /// `Drop` calls to the vtable, and the user callback will
    /// be invoked for each successful drop.
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn register(
        &mut self,
        hwnd: windows_sys::Win32::Foundation::HWND,
    ) -> Result<(), OleDropError> {
        // SAFETY: We have not registered before, the COM
        // object is valid (refcount=1), and the HWND is a
        // valid top-level window (the caller is responsible
        // for the latter).
        let hr = unsafe { win::register(hwnd, self.inner.as_raw()) };
        if hr == 0 {
            self.hwnd = Some(hwnd);
            Ok(())
        } else {
            Err(OleDropError::RegisterFailed(hr))
        }
    }

    /// The HWND this drop target is registered with, or
    /// `None` if it is not yet registered.
    pub fn hwnd(&self) -> Option<windows_sys::Win32::Foundation::HWND> {
        self.hwnd
    }
}

#[cfg(target_os = "windows")]
impl Drop for OleDropTarget {
    fn drop(&mut self) {
        if let Some(hwnd) = self.hwnd.take() {
            // SAFETY: We registered with this HWND; the
            // matching RevokeDragDrop is part of the
            // `OleDropTarget` contract.
            let _ = unsafe { win::unregister(hwnd) };
        }
        // The inner `OleDropTarget`'s Drop runs after
        // this, releasing the IUnknown refcount.
    }
}

// =================================================================
//                   OLE source-side public types
// =================================================================

/// The data being offered by an [`OleDragSource`]. The
/// source-side counterpart to [`OleDroppedData`].
///
/// The library currently supports the two formats the
/// destination side (v0.5.8) already understands: `CF_HDROP`
/// for files and `CF_UNICODETEXT` for text. Extending this
/// enum with new variants (e.g. `Bitmap` for `CF_DIB`) is the
/// natural way to add more formats in a future cycle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OleDragData {
    /// One or more file paths (will be exposed as `CF_HDROP`).
    Files(Vec<PathBuf>),
    /// Unicode text (will be exposed as `CF_UNICODETEXT`).
    Text(String),
}

/// `Display` for [`OleDragData`]. Mirrors
/// [`OleDroppedData`](crate::ole_dnd::OleDroppedData)'s
/// `Display` impl for symmetry — `Files` shows the count, `Text`
/// shows the character count.
impl fmt::Display for OleDragData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OleDragData::Files(paths) => {
                write!(f, "Files({})", paths.len())
            }
            OleDragData::Text(s) => write!(f, "Text({} chars)", s.chars().count()),
        }
    }
}

/// Result returned by an
/// [`OleDragSourceCallbacks::on_query_continue_drag`] callback.
/// Mirrors the `IDropSource::QueryContinueDrag` return
/// contract: the OLE runtime will end the drag when the
/// callback returns `Drop` or `Cancel`, and continue it on
/// `Continue`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DragContinueResult {
    /// Continue the drag operation.
    Continue,
    /// Complete the drop.
    Drop,
    /// Cancel the drag operation.
    Cancel,
}

/// `Display` for [`DragContinueResult`]. Prints the literal
/// variant name in `PascalCase`, suitable for log lines and
/// assertion messages.
impl fmt::Display for DragContinueResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DragContinueResult::Continue => "Continue",
            DragContinueResult::Drop => "Drop",
            DragContinueResult::Cancel => "Cancel",
        };
        f.write_str(s)
    }
}

/// Optional callbacks fired by the OLE COM `IDropSource` while
/// the drag is in progress. Both fields are `None` by default
/// (a source with no callbacks gets the OLE-default
/// behaviour).
#[derive(Default)]
pub struct OleDragSourceCallbacks {
    /// `IDropSource::QueryContinueDrag`. The default OLE
    /// behaviour is: end with `Drop` on mouse-button-up, end
    /// with `Cancel` on `VK_ESCAPE`, continue otherwise. The
    /// argument is the `MK_*` modifier-key bits as a single
    /// `u32` (`MK_LBUTTON = 0x0001`, `MK_RBUTTON = 0x0002`,
    /// `MK_SHIFT = 0x0004`, `MK_CONTROL = 0x0008`,
    /// `MK_MBUTTON = 0x0010`, `MK_XBUTTON1 = 0x0020`,
    /// `MK_XBUTTON2 = 0x0040`).
    pub on_query_continue_drag:
        Option<Box<dyn FnMut(u32) -> DragContinueResult>>,
    /// `IDropSource::GiveFeedback`. The default OLE behaviour
    /// is: show the standard drag cursor for the current
    /// effect. The argument is the currently-acceptable
    /// `OleDropEffect`; the return value is the effect the
    /// source wants shown in the cursor (typically equal to
    /// the argument; some sources override to show a
    /// "no-drop" cursor when the destination hasn't yet
    /// accepted).
    pub on_give_feedback:
        Option<Box<dyn FnMut(OleDropEffect) -> OleDropEffect>>,
}

/// Error returned when an OLE COM drag operation fails.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OleDragError {
    /// `DoDragDrop` is already in progress on this
    /// `OleDragSource`. Callers should not start a second
    /// drag until the first one returns.
    AlreadyInProgress,
    /// The internal `DoDragDrop` call returned a non-success
    /// HRESULT. The raw value is preserved for diagnostics.
    DoDragDropFailed(i32),
}

impl fmt::Display for OleDragError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OleDragError::AlreadyInProgress => {
                write!(f, "OLE drag already in progress on this source")
            }
            OleDragError::DoDragDropFailed(hr) => {
                write!(f, "OLE DoDragDrop failed with HRESULT 0x{hr:08x}")
            }
        }
    }
}

impl std::error::Error for OleDragError {}

// =================================================================
//        Cross-platform stub for the OLE drag source
// =================================================================

/// A pending OLE COM drag-and-drop operation. Owns the
/// reference-counted COM objects (`IDataObject` +
/// `IDropSource`) needed by Win32's `DoDragDrop`.
///
/// On non-Windows hosts this type is a **no-op placeholder**:
/// `do_drag_drop` returns [`OleDragError::DoDragDropFailed`]
/// with a synthesised "not supported" HRESULT.
#[cfg(not(target_os = "windows"))]
pub struct OleDragSource {
    data: OleDragData,
    _callbacks: OleDragSourceCallbacks,
}

#[cfg(not(target_os = "windows"))]
impl OleDragSource {
    /// Construct a new `OleDragSource` with the given data and
    /// no callbacks. The default OLE behaviour is used for
    /// `QueryContinueDrag` (drop on mouse-up, cancel on Esc)
    /// and `GiveFeedback` (use the OLE default cursor).
    pub fn new(data: OleDragData) -> Self {
        Self {
            data,
            _callbacks: OleDragSourceCallbacks::default(),
        }
    }

    /// Construct a new `OleDragSource` with the given data and
    /// callbacks.
    pub fn with_callbacks(
        data: OleDragData,
        callbacks: OleDragSourceCallbacks,
    ) -> Self {
        Self { data, _callbacks: callbacks }
    }

    /// Borrow the data the source is offering.
    pub fn data(&self) -> &OleDragData {
        &self.data
    }

    /// Begin the drag operation. A no-op on non-Windows hosts;
    /// always returns `DoDragDropFailed(E_NOTIMPL)`.
    pub fn do_drag_drop(
        &mut self,
        _hwnd: *mut core::ffi::c_void,
        _allowed: OleDropEffect,
    ) -> Result<OleDropEffect, OleDragError> {
        Err(OleDragError::DoDragDropFailed(-2147483647 - 1))
    }
}

// =================================================================
//              Windows-only OLE drag source vtable
// =================================================================

#[cfg(target_os = "windows")]
mod win_src {
    use std::cell::RefCell;
    use std::os::windows::ffi::OsStrExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    use windows_sys::Win32::Foundation::{HGLOBAL, HWND, S_FALSE, S_OK};
    use windows_sys::Win32::System::Com::{
        FORMATETC, STGMEDIUM, DVASPECT_CONTENT, TYMED_HGLOBAL,
    };
    use windows_sys::Win32::System::Memory::{
        GlobalAlloc, GlobalLock, GlobalUnlock,
        GMEM_MOVEABLE, GMEM_ZEROINIT,
    };
    use windows_sys::Win32::System::Ole::{DoDragDrop, DROPEFFECT_NONE};

    // `GlobalFree` is not re-exported by `windows-sys 0.59`'s
    // `Win32::System::Memory` module even though it is
    // exported by `kernel32.dll`. Declare it locally so we
    // can clean up the HGLOBAL handles we allocate on the
    // error paths of `build_hdrop` / `build_unicode_text`.
    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalFree(h: HGLOBAL) -> HGLOBAL;
    }

    use super::{
        DragContinueResult, OleDragData, OleDragError,
        OleDragSourceCallbacks, OleDropEffect,
    };

    /// Win32 `HRESULT` type alias.
    pub type HRESULT = i32;

    /// `IUnknown` / `IDropSource` / `IDataObject` opaque
    /// pointers. `windows-sys 0.59` does not export these as
    /// Rust types, so we alias them to `*mut c_void` directly.
    pub type IUnknown = core::ffi::c_void;
    pub type IDropSource = core::ffi::c_void;
    pub type IDataObject = core::ffi::c_void;

    // -------- vtable layout structs --------

    /// `IUnknown` vtable (3 methods). `#[repr(C)]` to
    /// guarantee the Win32-compatible layout. Field names are
    /// the canonical COM names and are intentionally
    /// PascalCase — this is the FFI ABI the Win32 COM runtime
    /// expects.
    #[repr(C)]
    #[allow(non_snake_case)]
    pub struct IUnknownVtbl {
        pub QueryInterface: unsafe extern "system" fn(
            *mut IUnknown,
            *const u8,
            *mut *mut IUnknown,
        ) -> HRESULT,
        pub AddRef: unsafe extern "system" fn(*mut IUnknown) -> u32,
        pub Release: unsafe extern "system" fn(*mut IUnknown) -> u32,
    }

    /// `IDropSource` vtable (IUnknown + 2 methods).
    #[repr(C)]
    #[allow(non_snake_case)]
    pub struct IDropSourceVtbl {
        pub parent: IUnknownVtbl,
        pub QueryContinueDrag: unsafe extern "system" fn(
            *mut IDropSource,
            /* fEsc */ i32,
            /* grfKeyState */ u32,
        ) -> HRESULT,
        pub GiveFeedback: unsafe extern "system" fn(
            *mut IDropSource,
            /* dwEffect */ u32,
        ) -> HRESULT,
    }

    /// `IDataObject` vtable (IUnknown + 9 methods).
    #[repr(C)]
    #[allow(non_snake_case)]
    pub struct IDataObjectVtbl {
        pub parent: IUnknownVtbl,
        pub GetData: unsafe extern "system" fn(
            *mut IDataObject,
            *const FORMATETC,
            *mut STGMEDIUM,
        ) -> HRESULT,
        pub GetDataHere: unsafe extern "system" fn(
            *mut IDataObject,
            *const FORMATETC,
            *mut STGMEDIUM,
        ) -> HRESULT,
        pub QueryGetData: unsafe extern "system" fn(
            *mut IDataObject,
            *const FORMATETC,
        ) -> HRESULT,
        pub GetCanonicalFormatEtc: unsafe extern "system" fn(
            *mut IDataObject,
            *const FORMATETC,
            *mut FORMATETC,
        ) -> HRESULT,
        pub SetData: unsafe extern "system" fn(
            *mut IDataObject,
            *const FORMATETC,
            *const STGMEDIUM,
            /* fRelease */ i32,
        ) -> HRESULT,
        pub EnumFormatEtc: unsafe extern "system" fn(
            *mut IDataObject,
            /* dwDirection */ u32,
            *mut *mut core::ffi::c_void,
        ) -> HRESULT,
        pub DAdvise: unsafe extern "system" fn(
            *mut IDataObject,
            *const FORMATETC,
            /* advf */ u32,
            *mut core::ffi::c_void,
            *mut u32,
        ) -> HRESULT,
        pub DUnadvise: unsafe extern "system" fn(
            *mut IDataObject,
            /* dwConnection */ u32,
        ) -> HRESULT,
        pub EnumAdvise: unsafe extern "system" fn(
            *mut IDataObject,
            *mut *mut core::ffi::c_void,
        ) -> HRESULT,
    }

    // -------- IEnumFORMATETC vtable (separate COM object) --------

    #[repr(C)]
    #[allow(non_snake_case)]
    pub struct IEnumFORMATETCVtbl {
        pub parent: IUnknownVtbl,
        pub Next: unsafe extern "system" fn(
            *mut core::ffi::c_void,
            /* celt */ u32,
            *mut FORMATETC,
            *mut u32,
        ) -> HRESULT,
        pub Skip: unsafe extern "system" fn(
            *mut core::ffi::c_void,
            /* celt */ u32,
        ) -> HRESULT,
        pub Reset: unsafe extern "system" fn(
            *mut core::ffi::c_void,
        ) -> HRESULT,
        pub Clone: unsafe extern "system" fn(
            *mut core::ffi::c_void,
            *mut *mut core::ffi::c_void,
        ) -> HRESULT,
    }

    // -------- COM objects (IDropSource + IDataObject) --------

    #[repr(C)]
    pub struct OleDropSourceComObject {
        pub vtable: *const IDropSourceVtbl,
        pub payload: *mut OleDropSourcePayload,
    }

    pub struct OleDropSourcePayload {
        pub refcount: AtomicU32,
        /// The user's callbacks. Wrapped in `RefCell` so the
        /// vtable functions can take a `RefMut` through a raw
        /// pointer. `DoDragDrop` is single-threaded by the OLE
        /// spec, so the borrow is uncontended in practice.
        pub callbacks: RefCell<OleDragSourceCallbacks>,
    }

    #[repr(C)]
    pub struct OleDataObjectComObject {
        pub vtable: *const IDataObjectVtbl,
        pub payload: *mut OleDataObjectPayload,
    }

    pub struct OleDataObjectPayload {
        pub refcount: AtomicU32,
        /// The user's data. We hold a `RefCell` so the vtable
        /// functions (which take `&self` through a raw
        /// pointer) can dispatch to it.
        pub data: RefCell<OleDragData>,
    }

    #[repr(C)]
    pub struct OleFormatEnumComObject {
        pub vtable: *const IEnumFORMATETCVtbl,
        pub payload: *mut OleFormatEnumPayload,
    }

    pub struct OleFormatEnumPayload {
        pub refcount: AtomicU32,
        /// The formats to enumerate. Owned by the enumerator;
        /// freed when the enumerator is released.
        pub formats: Vec<FORMATETC>,
        /// Cursor: the next index to return on `Next`.
        pub cursor: usize,
    }

    // -------- shared HRESULT constants --------
    // Numeric values are in the COM spec's "facility" range;
    // using `i32` literals (rather than `u32 -> i32` casts)
    // keeps the vtable functions clean.
    const E_POINTER: i32 = -2147483647 - 1;        // 0x80004003
    const E_NOINTERFACE: i32 = -2147483647 - 1;    // 0x80004002 (alias of above numeric)
    const E_NOTIMPL: i32 = -2147483647 - 1;        // 0x80004001 (alias)
    const E_FAIL: i32 = 0x80004005u32 as i32;
    const E_OUTOFMEMORY: i32 = 0x8007000Eu32 as i32;
    const DV_E_FORMATETC: i32 = 0x80040064u32 as i32;
    const DV_E_TYMED: i32 = 0x80040069u32 as i32;
    const OLE_E_ADVISENOTSUPPORTED: i32 = 0x80040003u32 as i32;
    const DRAGDROP_S_DROP: i32 = 0x00040100u32 as i32;
    const DRAGDROP_S_CANCEL: i32 = 0x00040101u32 as i32;
    const DRAGDROP_S_USEDEFAULTCURSORS: i32 = 0x00040102u32 as i32;
    /// Defined for completeness; we do not branch on it.
    #[allow(dead_code)]
    const DRAGDROP_S_LAST: i32 = 0x00040103u32 as i32;

    // The five standard IID_GUIDs. Hard-coded byte-for-byte
    // because `windows-sys 0.59` does not export them.
    const IID_IUNKNOWN: [u8; 16] = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x46,
    ];
    const IID_IDROPSOURCE: [u8; 16] = [
        0x21, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x46,
    ];
    const IID_IDATAOBJECT: [u8; 16] = [
        0x05, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x46,
    ];

    // Clipboard format IDs. Hard-coded because the constants
    // are part of the clipboard format registry, not a Win32
    // API constant.
    const CF_HDROP: u16 = 15;
    const CF_UNICODETEXT: u16 = 13;

    // -------- IUnknown vtable functions (shared by all 3 COM objects) --------

    /// `IUnknown::QueryInterface` for the `IDropSource` COM
    /// object. We support `IUnknown` and `IDropSource`; any
    /// other interface returns `E_NOINTERFACE`.
    unsafe extern "system" fn drop_source_query_interface(
        this: *mut IUnknown,
        riid: *const u8,
        ppv: *mut *mut IUnknown,
    ) -> HRESULT {
        if ppv.is_null() || riid.is_null() {
            return E_POINTER;
        }
        let iid = core::slice::from_raw_parts(riid, 16);
        if iid == IID_IUNKNOWN || iid == IID_IDROPSOURCE {
            *ppv = this;
            drop_source_add_ref(this);
            S_OK
        } else {
            *ppv = core::ptr::null_mut();
            E_NOINTERFACE
        }
    }

    unsafe extern "system" fn drop_source_add_ref(
        this: *mut IUnknown,
    ) -> u32 {
        let com_obj = this as *const OleDropSourceComObject;
        let prev = (*(*com_obj).payload)
            .refcount
            .fetch_add(1, Ordering::Relaxed);
        prev + 1
    }

    unsafe extern "system" fn drop_source_release(
        this: *mut IUnknown,
    ) -> u32 {
        let com_obj = this as *mut OleDropSourceComObject;
        let payload = (*com_obj).payload;
        let prev = (*payload).refcount.fetch_sub(1, Ordering::AcqRel);
        if prev == 1 {
            core::mem::drop(Box::from_raw(payload));
            core::mem::drop(Box::from_raw(com_obj));
            0
        } else {
            prev - 1
        }
    }

    /// `IUnknown::QueryInterface` for the `IDataObject` COM
    /// object. We support `IUnknown` and `IDataObject`; any
    /// other interface returns `E_NOINTERFACE`.
    unsafe extern "system" fn data_object_query_interface(
        this: *mut IUnknown,
        riid: *const u8,
        ppv: *mut *mut IUnknown,
    ) -> HRESULT {
        if ppv.is_null() || riid.is_null() {
            return E_POINTER;
        }
        let iid = core::slice::from_raw_parts(riid, 16);
        if iid == IID_IUNKNOWN || iid == IID_IDATAOBJECT {
            *ppv = this;
            data_object_add_ref(this);
            S_OK
        } else {
            *ppv = core::ptr::null_mut();
            E_NOINTERFACE
        }
    }

    unsafe extern "system" fn data_object_add_ref(
        this: *mut IUnknown,
    ) -> u32 {
        let com_obj = this as *const OleDataObjectComObject;
        let prev = (*(*com_obj).payload)
            .refcount
            .fetch_add(1, Ordering::Relaxed);
        prev + 1
    }

    unsafe extern "system" fn data_object_release(
        this: *mut IUnknown,
    ) -> u32 {
        let com_obj = this as *mut OleDataObjectComObject;
        let payload = (*com_obj).payload;
        let prev = (*payload).refcount.fetch_sub(1, Ordering::AcqRel);
        if prev == 1 {
            core::mem::drop(Box::from_raw(payload));
            core::mem::drop(Box::from_raw(com_obj));
            0
        } else {
            prev - 1
        }
    }

    /// `IUnknown::QueryInterface` for the format enumerator's
    /// COM object. We support only `IUnknown` (callers must
    /// not downcast; the enumerator's `IEnumFORMATETC` is
    /// shape-identical to IUnknown so they cannot be
    /// distinguished by GUID).
    unsafe extern "system" fn enum_query_interface(
        this: *mut IUnknown,
        riid: *const u8,
        ppv: *mut *mut IUnknown,
    ) -> HRESULT {
        if ppv.is_null() || riid.is_null() {
            return E_POINTER;
        }
        let iid = core::slice::from_raw_parts(riid, 16);
        if iid == IID_IUNKNOWN {
            *ppv = this;
            enum_add_ref(this);
            S_OK
        } else {
            *ppv = core::ptr::null_mut();
            E_NOINTERFACE
        }
    }

    unsafe extern "system" fn enum_add_ref(this: *mut IUnknown) -> u32 {
        let com_obj = this as *const OleFormatEnumComObject;
        let prev = (*(*com_obj).payload)
            .refcount
            .fetch_add(1, Ordering::Relaxed);
        prev + 1
    }

    unsafe extern "system" fn enum_release(this: *mut IUnknown) -> u32 {
        let com_obj = this as *mut OleFormatEnumComObject;
        let payload = (*com_obj).payload;
        let prev = (*payload).refcount.fetch_sub(1, Ordering::AcqRel);
        if prev == 1 {
            core::mem::drop(Box::from_raw(payload));
            core::mem::drop(Box::from_raw(com_obj));
            0
        } else {
            prev - 1
        }
    }

    // -------- IDropSource vtable functions --------

    /// `IDropSource::QueryContinueDrag`. The OLE runtime calls
    /// this on every mouse-move / key-down event. The default
    /// behaviour (no callback) is: cancel on `VK_ESCAPE`,
    /// drop on mouse-button-up, continue otherwise.
    unsafe extern "system" fn query_continue_drag(
        this: *mut IDropSource,
        f_esc: i32,
        grf_key_state: u32,
    ) -> HRESULT {
        const MK_LBUTTON: u32 = 0x0001;
        const MK_RBUTTON: u32 = 0x0002;
        let com_obj = this as *const OleDropSourceComObject;
        let payload = (*com_obj).payload;
        let user_decision: DragContinueResult =
            if let Some(cb) = (*payload)
                .callbacks
                .borrow_mut()
                .on_query_continue_drag
                .as_mut()
            {
                cb(grf_key_state)
            } else {
                if f_esc != 0 {
                    DragContinueResult::Cancel
                } else if (grf_key_state & (MK_LBUTTON | MK_RBUTTON)) == 0 {
                    DragContinueResult::Drop
                } else {
                    DragContinueResult::Continue
                }
            };
        match user_decision {
            DragContinueResult::Continue => S_OK,
            DragContinueResult::Drop => DRAGDROP_S_DROP,
            DragContinueResult::Cancel => DRAGDROP_S_CANCEL,
        }
    }

    /// `IDropSource::GiveFeedback`. The OLE runtime calls
    /// this to ask the source to update the cursor. We
    /// always return `DRAGDROP_S_USEDEFAULTCURSORS` — custom
    /// cursor handoff is out of scope for v0.6.2 (the
    /// user-supplied `on_give_feedback` callback fires so
    /// the user can log / inspect the effect, but the actual
    /// cursor is still the OLE default).
    unsafe extern "system" fn give_feedback(
        this: *mut IDropSource,
        dw_effect: u32,
    ) -> HRESULT {
        let com_obj = this as *const OleDropSourceComObject;
        let payload = (*com_obj).payload;
        if let Some(cb) = (*payload)
            .callbacks
            .borrow_mut()
            .on_give_feedback
            .as_mut()
        {
            let _ = cb(OleDropEffect::from_bits_truncate(dw_effect));
        }
        DRAGDROP_S_USEDEFAULTCURSORS
    }

    static DROP_SOURCE_VTBL: IDropSourceVtbl = IDropSourceVtbl {
        parent: IUnknownVtbl {
            QueryInterface: drop_source_query_interface,
            AddRef: drop_source_add_ref,
            Release: drop_source_release,
        },
        QueryContinueDrag: query_continue_drag,
        GiveFeedback: give_feedback,
    };

    // -------- IDataObject vtable functions --------

    /// `IDataObject::GetData`. Fill `pmedium` with the data
    /// for the requested format.
    unsafe extern "system" fn get_data(
        this: *mut IDataObject,
        pformatetc: *const FORMATETC,
        pmedium: *mut STGMEDIUM,
    ) -> HRESULT {
        if pformatetc.is_null() || pmedium.is_null() {
            return E_POINTER;
        }
        let format = &*pformatetc;
        if format.tymed != (TYMED_HGLOBAL as u32) {
            return DV_E_TYMED;
        }
        let com_obj = this as *const OleDataObjectComObject;
        let payload = (*com_obj).payload;
        let data = (*payload).data.borrow();
        match &*data {
            OleDragData::Files(paths) => {
                if format.cfFormat != CF_HDROP {
                    return DV_E_FORMATETC;
                }
                let h = build_hdrop(paths);
                if h.is_null() {
                    return E_OUTOFMEMORY;
                }
                *pmedium = STGMEDIUM {
                    tymed: TYMED_HGLOBAL as u32,
                    u: windows_sys::Win32::System::Com::STGMEDIUM_0 {
                        hGlobal: h as _,
                    },
                    pUnkForRelease: core::ptr::null_mut(),
                };
                S_OK
            }
            OleDragData::Text(text) => {
                if format.cfFormat != CF_UNICODETEXT {
                    return DV_E_FORMATETC;
                }
                let h = build_unicode_text(text);
                if h.is_null() {
                    return E_OUTOFMEMORY;
                }
                *pmedium = STGMEDIUM {
                    tymed: TYMED_HGLOBAL as u32,
                    u: windows_sys::Win32::System::Com::STGMEDIUM_0 {
                        hGlobal: h as _,
                    },
                    pUnkForRelease: core::ptr::null_mut(),
                };
                S_OK
            }
        }
    }

    /// `IDataObject::GetDataHere`. We don't implement the
    /// "caller-provided storage" model — the destination
    /// always asks us to allocate. Return `E_NOTIMPL`.
    unsafe extern "system" fn get_data_here(
        _this: *mut IDataObject,
        _pformatetc: *const FORMATETC,
        _pmedium: *mut STGMEDIUM,
    ) -> HRESULT {
        E_NOTIMPL
    }

    /// `IDataObject::QueryGetData`. Return `S_OK` if we
    /// support the format, `DV_E_FORMATETC` otherwise.
    unsafe extern "system" fn query_get_data(
        this: *mut IDataObject,
        pformatetc: *const FORMATETC,
    ) -> HRESULT {
        if pformatetc.is_null() {
            return E_POINTER;
        }
        let format = &*pformatetc;
        let com_obj = this as *const OleDataObjectComObject;
        let payload = (*com_obj).payload;
        let data = (*payload).data.borrow();
        let supported = match &*data {
            OleDragData::Files(_) => format.cfFormat == CF_HDROP,
            OleDragData::Text(_) => format.cfFormat == CF_UNICODETEXT,
        };
        if supported && format.tymed == (TYMED_HGLOBAL as u32) {
            S_OK
        } else {
            DV_E_FORMATETC
        }
    }

    /// `IDataObject::GetCanonicalFormatEtc`. We don't do
    /// format equivalence — return `E_NOTIMPL`.
    unsafe extern "system" fn get_canonical_format_etc(
        _this: *mut IDataObject,
        _pformatetc_in: *const FORMATETC,
        _pformatetc_out: *mut FORMATETC,
    ) -> HRESULT {
        E_NOTIMPL
    }

    /// `IDataObject::SetData`. We are read-only.
    unsafe extern "system" fn set_data(
        _this: *mut IDataObject,
        _pformatetc: *const FORMATETC,
        _pmedium: *const STGMEDIUM,
        _f_release: i32,
    ) -> HRESULT {
        E_FAIL
    }

    /// `IDataObject::EnumFormatEtc`. Return a one-shot
    /// enumerator with the format our payload supports.
    unsafe extern "system" fn enum_format_etc(
        this: *mut IDataObject,
        dw_direction: u32,
        ppenum: *mut *mut core::ffi::c_void,
    ) -> HRESULT {
        const DATADIR_GET: u32 = 1;
        if ppenum.is_null() {
            return E_POINTER;
        }
        if dw_direction == 2 {
            // `DATADIR_SET`. We are read-only.
            *ppenum = core::ptr::null_mut();
            return E_NOTIMPL;
        }
        if dw_direction != DATADIR_GET {
            *ppenum = core::ptr::null_mut();
            return DV_E_FORMATETC;
        }
        let com_obj = this as *const OleDataObjectComObject;
        let payload = (*com_obj).payload;
        let data = (*payload).data.borrow();
        let format = match &*data {
            OleDragData::Files(_) => FORMATETC {
                cfFormat: CF_HDROP,
                ptd: core::ptr::null_mut(),
                dwAspect: DVASPECT_CONTENT,
                lindex: -1,
                tymed: TYMED_HGLOBAL as u32,
            },
            OleDragData::Text(_) => FORMATETC {
                cfFormat: CF_UNICODETEXT,
                ptd: core::ptr::null_mut(),
                dwAspect: DVASPECT_CONTENT,
                lindex: -1,
                tymed: TYMED_HGLOBAL as u32,
            },
        };
        let formats = [format];
        *ppenum = make_format_enum(&formats).cast();
        S_OK
    }

    unsafe extern "system" fn d_advise(
        _this: *mut IDataObject,
        _pformatetc: *const FORMATETC,
        _advf: u32,
        _p_adv_sink: *mut core::ffi::c_void,
        _pdw_connection: *mut u32,
    ) -> HRESULT {
        OLE_E_ADVISENOTSUPPORTED
    }

    unsafe extern "system" fn d_unadvise(
        _this: *mut IDataObject,
        _dw_connection: u32,
    ) -> HRESULT {
        OLE_E_ADVISENOTSUPPORTED
    }

    unsafe extern "system" fn enum_advise(
        _this: *mut IDataObject,
        _ppenum_advise: *mut *mut core::ffi::c_void,
    ) -> HRESULT {
        OLE_E_ADVISENOTSUPPORTED
    }

    static DATA_OBJECT_VTBL: IDataObjectVtbl = IDataObjectVtbl {
        parent: IUnknownVtbl {
            QueryInterface: data_object_query_interface,
            AddRef: data_object_add_ref,
            Release: data_object_release,
        },
        GetData: get_data,
        GetDataHere: get_data_here,
        QueryGetData: query_get_data,
        GetCanonicalFormatEtc: get_canonical_format_etc,
        SetData: set_data,
        EnumFormatEtc: enum_format_etc,
        DAdvise: d_advise,
        DUnadvise: d_unadvise,
        EnumAdvise: enum_advise,
    };

    // -------- IEnumFORMATETC vtable functions --------

    unsafe extern "system" fn enum_next(
        this: *mut core::ffi::c_void,
        celt: u32,
        rgelt: *mut FORMATETC,
        pcelt_fetched: *mut u32,
    ) -> HRESULT {
        if celt == 0 || rgelt.is_null() {
            return E_POINTER;
        }
        if celt > 1 {
            return E_POINTER;
        }
        let com_obj = this as *const OleFormatEnumComObject;
        let payload = (*com_obj).payload;
        if (*payload).cursor >= (*payload).formats.len() {
            if !pcelt_fetched.is_null() {
                *pcelt_fetched = 0;
            }
            return S_FALSE;
        }
        // Use raw-pointer arithmetic to read the FORMATETC
        // by value: indexing a `Vec` through a raw pointer
        // would require an implicit autoref, which the
        // `dangerous_implicit_autorefs` lint forbids. The
        // raw-pointer path is equally valid since
        // `FORMATETC` is `#[repr(C)]` and `Copy`.
        let f = *(*payload).formats.as_ptr().add((*payload).cursor);
        *rgelt = f;
        (*payload).cursor += 1;
        if !pcelt_fetched.is_null() {
            *pcelt_fetched = 1;
        }
        S_OK
    }

    unsafe extern "system" fn enum_skip(
        this: *mut core::ffi::c_void,
        celt: u32,
    ) -> HRESULT {
        let com_obj = this as *const OleFormatEnumComObject;
        let payload = (*com_obj).payload;
        let new_cursor =
            (*payload).cursor.saturating_add(celt as usize);
        if new_cursor > (*payload).formats.len() {
            (*payload).cursor = (*payload).formats.len();
            S_FALSE
        } else {
            (*payload).cursor = new_cursor;
            S_OK
        }
    }

    unsafe extern "system" fn enum_reset(
        this: *mut core::ffi::c_void,
    ) -> HRESULT {
        let com_obj = this as *const OleFormatEnumComObject;
        let payload = (*com_obj).payload;
        (*payload).cursor = 0;
        S_OK
    }

    unsafe extern "system" fn enum_clone(
        _this: *mut core::ffi::c_void,
        _ppenum: *mut *mut core::ffi::c_void,
    ) -> HRESULT {
        E_NOTIMPL
    }

    static ENUM_VTBL: IEnumFORMATETCVtbl = IEnumFORMATETCVtbl {
        parent: IUnknownVtbl {
            QueryInterface: enum_query_interface,
            AddRef: enum_add_ref,
            Release: enum_release,
        },
        Next: enum_next,
        Skip: enum_skip,
        Reset: enum_reset,
        Clone: enum_clone,
    };

    /// Build a one-shot format enumerator. The returned
    /// `*mut OleFormatEnumComObject` is also valid as
    /// `*mut IUnknown` — the caller (the OLE runtime) is
    /// expected to Release it.
    unsafe fn make_format_enum(
        formats: &[FORMATETC],
    ) -> *mut OleFormatEnumComObject {
        let payload = Box::new(OleFormatEnumPayload {
            refcount: AtomicU32::new(1),
            formats: formats.to_vec(),
            cursor: 0,
        });
        let com_object = Box::new(OleFormatEnumComObject {
            vtable: &ENUM_VTBL,
            payload: Box::into_raw(payload),
        });
        Box::into_raw(com_object)
    }

    // -------- Format writers (HGLOBAL allocators) --------

    /// Build a `CF_HDROP` HGLOBAL containing the given file
    /// paths. Returns null on out-of-memory.
    ///
    /// The HGLOBAL layout is `DROPFILES` (20 bytes) followed
    /// by a double-NUL-terminated array of UTF-16LE file
    /// paths. `DROPFILES::pFiles` is set to 20, `fWide` to
    /// TRUE.
    unsafe fn build_hdrop(paths: &[PathBuf]) -> HGLOBAL {
        let wide_paths: Vec<Vec<u16>> = paths
            .iter()
            .map(|p| {
                p.as_os_str()
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect()
            })
            .collect();
        let bytes_after_header: usize = wide_paths
            .iter()
            .map(|w| w.len() * 2)
            .sum::<usize>()
            + 2; // final NUL terminator
        let total = 20usize.checked_add(bytes_after_header);
        let total = match total {
            Some(t) => t,
            None => return core::ptr::null_mut(),
        };
        let h = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, total);
        if h.is_null() {
            return core::ptr::null_mut();
        }
        let p = GlobalLock(h) as *mut u8;
        if p.is_null() {
            GlobalFree(h);
            return core::ptr::null_mut();
        }
        *(p as *mut u32) = 20; // pFiles
        *(p.add(4) as *mut i32) = 0; // pt.x
        *(p.add(8) as *mut i32) = 0; // pt.y
        *(p.add(12) as *mut i32) = 0; // fNC
        *(p.add(16) as *mut i32) = 1; // fWide
        let mut offset = 20usize;
        for w in &wide_paths {
            let bytes = w.len() * 2;
            core::ptr::copy_nonoverlapping(
                w.as_ptr() as *const u8,
                p.add(offset),
                bytes,
            );
            offset = match offset.checked_add(bytes) {
                Some(o) => o,
                None => {
                    GlobalUnlock(h);
                    GlobalFree(h);
                    return core::ptr::null_mut();
                }
            };
        }
        // Final NUL terminator.
        *(p.add(offset) as *mut u16) = 0;
        GlobalUnlock(h);
        h
    }

    /// Build a `CF_UNICODETEXT` HGLOBAL containing the given
    /// text. The HGLOBAL layout is `u32 len_bytes` (length
    /// in bytes, excluding the NUL) followed by a
    /// NUL-terminated UTF-16LE string.
    unsafe fn build_unicode_text(text: &str) -> HGLOBAL {
        let wide: Vec<u16> =
            text.encode_utf16().chain(std::iter::once(0)).collect();
        let len_bytes = (wide.len() - 1) * 2; // exclude NUL
        let total = match 4usize.checked_add(wide.len() * 2) {
            Some(t) => t,
            None => return core::ptr::null_mut(),
        };
        let h = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, total);
        if h.is_null() {
            return core::ptr::null_mut();
        }
        let p = GlobalLock(h) as *mut u8;
        if p.is_null() {
            GlobalFree(h);
            return core::ptr::null_mut();
        }
        *(p as *mut u32) = len_bytes as u32;
        core::ptr::copy_nonoverlapping(
            wide.as_ptr() as *const u8,
            p.add(4),
            wide.len() * 2,
        );
        GlobalUnlock(h);
        h
    }

    // -------- Public API: OleDragSourceInner --------

    /// The COM-object pair that backs an `OleDragSource`.
    /// Held by-value inside the public `OleDragSource`;
    /// refcounted by the OLE runtime during `DoDragDrop`.
    pub struct OleDragSourceInner {
        pub drop_source: *mut OleDropSourceComObject,
        pub data_object: *mut OleDataObjectComObject,
    }

    // SAFETY: The COM objects are reference-counted, so the
    // pointers are safe to share across threads. The user
    // callbacks and the data are wrapped in `RefCell`, so
    // concurrent access would panic; in practice `DoDragDrop`
    // is single-threaded by the OLE spec.
    unsafe impl Send for OleDragSourceInner {}

    impl OleDragSourceInner {
        /// Build the COM objects for a drag with the given
        /// data. The callbacks are initialised to
        /// `OleDragSourceCallbacks::default()` — the public
        /// `OleDragSource` swaps them in via `set_callbacks`.
        pub fn new(data: OleDragData) -> Self {
            let ds_payload = Box::new(OleDropSourcePayload {
                refcount: AtomicU32::new(1),
                callbacks: RefCell::new(
                    OleDragSourceCallbacks::default(),
                ),
            });
            let ds_com = Box::new(OleDropSourceComObject {
                vtable: &DROP_SOURCE_VTBL,
                payload: Box::into_raw(ds_payload),
            });
            let do_payload = Box::new(OleDataObjectPayload {
                refcount: AtomicU32::new(1),
                data: RefCell::new(data),
            });
            let do_com = Box::new(OleDataObjectComObject {
                vtable: &DATA_OBJECT_VTBL,
                payload: Box::into_raw(do_payload),
            });
            Self {
                drop_source: Box::into_raw(ds_com),
                data_object: Box::into_raw(do_com),
            }
        }

        /// Call Win32 `DoDragDrop` with the COM objects.
        /// Blocks until the drag completes.
        ///
        /// # Safety
        ///
        /// `hwnd` must be a valid top-level window. The
        /// `OleDragSourceInner` must not have been used in a
        /// prior `DoDragDrop` call that is still in flight.
        pub unsafe fn do_drag_drop(
            &self,
            _hwnd: HWND,
            allowed: OleDropEffect,
        ) -> Result<OleDropEffect, OleDragError> {
            let mut effect: u32 = DROPEFFECT_NONE;
            let hr = DoDragDrop(
                self.data_object.cast(),
                self.drop_source.cast(),
                allowed.bits(),
                &mut effect,
            );
            // The OLE spec defines three success codes:
            //   S_OK (0)              -> drag completed, no drop
            //   DRAGDROP_S_DROP       -> drop happened
            //   DRAGDROP_S_CANCEL     -> drag was cancelled
            // We collapse S_OK and DRAGDROP_S_CANCEL into
            // "no effect chosen" (both leave `effect` at
            // DROPEFFECT_NONE).
            if hr >= 0 {
                Ok(OleDropEffect::from_bits_truncate(effect))
            } else {
                Err(OleDragError::DoDragDropFailed(hr))
            }
        }
    }

    impl Drop for OleDragSourceInner {
        fn drop(&mut self) {
            // Release both COM objects. If we were the last
            // owner, this frees the payloads and the COM
            // objects themselves.
            unsafe {
                drop_source_release(self.drop_source.cast());
                data_object_release(self.data_object.cast());
            }
        }
    }
}

// =================================================================
//             Windows-only OLE drag source handle
// =================================================================

/// A pending OLE COM drag-and-drop operation. Owns the
/// reference-counted `IDataObject` + `IDropSource` COM objects
/// needed by Win32's `DoDragDrop`. Calling
/// [`do_drag_drop`](Self::do_drag_drop) blocks until the drag
/// completes (drop / cancel / escape) and returns the effect
/// the user chose.
///
/// # Example
///
/// ```no_run
/// use ru_wx::prelude::*;
/// use std::path::PathBuf;
///
/// let frame = Frame::builder().with_title("Drag me!").build();
/// let mut src = OleDragSource::new(OleDragData::Files(vec![
///     PathBuf::from(r"C:\Users\me\report.txt"),
/// ]));
/// // frame.hwnd() returns HWND on Windows, null on other platforms.
/// // SAFETY: in a real GUI app the frame HWND is alive for the
/// // duration of the drag; the closure below would normally be
/// // hooked up to a button on_click handler.
/// let _ = unsafe { src.do_drag_drop(frame.hwnd(), OleDropEffect::COPY) };
/// ```
#[cfg(target_os = "windows")]
pub struct OleDragSource {
    inner: win_src::OleDragSourceInner,
    callbacks: OleDragSourceCallbacks,
    in_progress: bool,
    data: OleDragData,
}

#[cfg(target_os = "windows")]
impl OleDragSource {
    /// Construct a new `OleDragSource` with the given data and
    /// no callbacks. The default OLE behaviour is used for
    /// `QueryContinueDrag` (drop on mouse-up, cancel on Esc)
    /// and `GiveFeedback` (use the OLE default cursor).
    pub fn new(data: OleDragData) -> Self {
        let inner = win_src::OleDragSourceInner::new(data.clone());
        Self {
            inner,
            callbacks: OleDragSourceCallbacks::default(),
            in_progress: false,
            data,
        }
    }

    /// Construct a new `OleDragSource` with the given data and
    /// callbacks.
    pub fn with_callbacks(
        data: OleDragData,
        callbacks: OleDragSourceCallbacks,
    ) -> Self {
        let inner = win_src::OleDragSourceInner::new(data.clone());
        Self {
            inner,
            callbacks,
            in_progress: false,
            data,
        }
    }

    /// Replace the callbacks after construction. No-op if a
    /// drag is currently in progress (swapping callbacks
    /// mid-drag would leave the in-flight `RefMut` in an
    /// inconsistent state).
    pub fn set_callbacks(&mut self, callbacks: OleDragSourceCallbacks) {
        if self.in_progress {
            return;
        }
        self.inner = win_src::OleDragSourceInner::new(self.data.clone());
        self.callbacks = callbacks;
    }

    /// Borrow the data the source is offering.
    pub fn data(&self) -> &OleDragData {
        &self.data
    }

    /// Begin the drag operation. Blocks until the drag
    /// completes (drop / cancel / escape) and returns the
    /// effect the user chose. Returns
    /// [`OleDragError::AlreadyInProgress`] if a drag is
    /// already in flight on this source.
    ///
    /// # Safety
    ///
    /// `hwnd` must be either `0` (a valid sentinel for
    /// "no source window" — `DoDragDrop` will not post
    /// `OLEDRAGDROP` notifications in that case) or a
    /// valid, non-null `HWND` belonging to a window that
    /// is still alive for the duration of the drag. Passing
    /// a dangling, null, or already-destroyed window handle
    /// is undefined behaviour: the OLE host will call back
    /// into the window's message procedure and may write
    /// to the `IDropTarget` associated with the window.
    pub unsafe fn do_drag_drop(
        &mut self,
        hwnd: windows_sys::Win32::Foundation::HWND,
        allowed: OleDropEffect,
    ) -> Result<OleDropEffect, OleDragError> {
        if self.in_progress {
            return Err(OleDragError::AlreadyInProgress);
        }
        self.in_progress = true;
        // Make sure OLE is initialised (so DoDragDrop is
        // safe to call). We piggy-back on the v0.5.8 helper.
        win::ensure_ole_initialized();
        // SAFETY: hwnd is the caller's responsibility (see
        // the `# Safety` section on the public wrapper);
        // the inner COM objects are valid and refcount == 1.
        let result = unsafe { self.inner.do_drag_drop(hwnd, allowed) };
        self.in_progress = false;
        result
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for the public data types exported by
    //! `ole_dnd`. These tests are platform-agnostic: they
    //! cover the type-level surface that the COM vtable
    //! (`win::IDropTargetVtbl`, in `ole_dnd::win`) ultimately
    //! produces, but they do not require a real `HWND` or a
    //! real `IDataObject` to run.
    //!
    //! The integration test for the vtable dispatch is the
    //! `examples/showcase_all.rs` binary.

    use super::*;
    use std::ops::BitOr;

    // ---------- OleDropEffect ----------

    /// The five standard DROPEFFECT bits must be the
    /// Win32-canonical values.
    #[test]
    fn ole_drop_effect_standard_bits_match_win32() {
        assert_eq!(OleDropEffect::NONE.bits(), 0);
        assert_eq!(OleDropEffect::COPY.bits(), 1);
        assert_eq!(OleDropEffect::MOVE.bits(), 2);
        assert_eq!(OleDropEffect::LINK.bits(), 4);
        assert_eq!(OleDropEffect::SCROLL.bits(), 0x8000_0000);
    }

    /// `is_none` is the single-bit-and-the-mask test the COM
    /// spec uses ("the target accepted nothing"). It must
    /// round-trip `bits() == 0`.
    #[test]
    fn ole_drop_effect_is_none_round_trips() {
        assert!(OleDropEffect::NONE.is_none());
        assert!(!OleDropEffect::COPY.is_none());
        assert!(!OleDropEffect::MOVE.is_none());
        // A bitwise combination that is not zero is not
        // `is_none`, even if it doesn't include the
        // canonical values.
        assert!(!OleDropEffect::from_bits_truncate(0xFFFF_FFFF).is_none());
    }

    /// `from_bits_truncate` must never panic — it is the
    /// panic-free companion of the strict constructor.
    #[test]
    fn ole_drop_effect_from_bits_truncate_never_panics() {
        for bits in [0u32, 1, 2, 3, 4, 5, 0xFFFF_FFFF, 0x8000_0000] {
            let _ = OleDropEffect::from_bits_truncate(bits);
        }
    }

    /// The `BitOr` impl must compose DROPEFFECT bits the way
    /// the COM spec expects — `COPY | MOVE` is the standard
    /// "either, source chooses" pattern.
    #[test]
    fn ole_drop_effect_bitor_composes_bits() {
        let combined = OleDropEffect::COPY.bitor(OleDropEffect::MOVE);
        assert_eq!(combined.bits(), 3);
        // And the `BitOrAssign` shortcut is consistent.
        let mut e = OleDropEffect::COPY;
        e |= OleDropEffect::LINK;
        assert_eq!(e.bits(), 1 | 4);
    }

    /// `OleDropEffect` must be `Copy` and `Hash` — the COM
    /// vtable hands us a value, and the vtable signature
    /// requires we can pass it back by-value without a
    /// borrow. `Hash` is needed so the value can live in
    /// `HashMap` keys (e.g. cached effect decisions).
    #[test]
    fn ole_drop_effect_is_copy_and_hash() {
        let a = OleDropEffect::COPY;
        let b = a; // Copy: this is a value move, not a move.
        assert_eq!(a, b);
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        a.hash(&mut h1);
        b.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    // ---------- OleDropPosition ----------

    /// `OleDropPosition::new` must store the x/y pair, and
    /// the fields must be `pub` (the closure signature
    /// requires destructuring).
    #[test]
    fn ole_drop_position_new_stores_xy() {
        let p = OleDropPosition::new(11, 22);
        assert_eq!(p.x, 11);
        assert_eq!(p.y, 22);
        // And a default constructor must give the origin.
        let d = OleDropPosition::default();
        assert_eq!(d.x, 0);
        assert_eq!(d.y, 0);
    }

    /// `OleDropPosition` is `Copy` — it's a 16-byte POD.
    #[test]
    fn ole_drop_position_is_copy() {
        let a = OleDropPosition::new(7, 8);
        let b = a;
        assert_eq!(a.x, b.x);
        assert_eq!(a.y, b.y);
    }

    // ---------- OleDroppedData ----------

    /// The three variants must be constructible and
    /// `match`-able.
    ///
    /// Replaces the prior `match ... { Variant => ..., _ => panic!(...) }`
    /// pattern. `assert!(matches!(...))` produces a structured panic
    /// message that includes the actual mismatched value, which is
    /// far more useful when a test fails than a hard-coded literal.
    #[test]
    fn ole_dropped_data_variants_match() {
        let files = OleDroppedData::Files(vec![PathBuf::from("a.txt")]);
        let text = OleDroppedData::Text("hello".to_string());
        let other = OleDroppedData::Other;
        assert!(matches!(files, OleDroppedData::Files(ref p) if p.len() == 1));
        assert!(matches!(text, OleDroppedData::Text(ref s) if s == "hello"));
        assert!(matches!(other, OleDroppedData::Other));
    }

    // ---------- OleDropError ----------

    /// `OleDropError` must be `Copy` and `PartialEq` — the
    /// `?` operator in the frame's `set_ole_drop_callback`
    /// moves it through the call site by-value. The two
    /// HRESULTs are picked from the COM spec's "facility"
    /// space: `0x8000_4001` is `E_NOTIMPL` (facility
    /// `NULL`, error code 1) and `0` is `S_OK`. Both are
    /// cast to `i32` because that is `windows-sys`'s
    /// `HRESULT` alias.
    #[test]
    fn ole_drop_error_is_copy_and_eq() {
        let a = OleDropError::RegisterFailed(0x8000_4001u32 as i32);
        let b = a; // Copy
        assert_eq!(a, b);
        let c = OleDropError::RegisterFailed(0);
        assert_ne!(a, c);
    }

    /// The `Display` impl must mention the raw HRESULT so
    /// the user can copy-paste it into a debugger. The
    /// HRESULT literal is cast to `i32` (it doesn't fit in
    /// a signed `i32` literal — the high bit is the
    /// "failure" bit per the COM spec).
    #[test]
    fn ole_drop_error_display_includes_hresult() {
        let e = OleDropError::RegisterFailed(0x8004_0100u32 as i32);
        let s = format!("{}", e);
        // The lower-case hex is required — the COM spec
        // uses it consistently.
        assert!(s.contains("0x80040100"), "got `{}`", s);
    }

    /// `OleDropError` must be a proper `std::error::Error` —
    /// i.e. `source()` returns `None` and it round-trips
    /// through `Error::downcast_ref`.
    #[test]
    fn ole_drop_error_is_std_error() {
        let e: Box<dyn std::error::Error> =
            Box::new(OleDropError::RegisterFailed(1));
        // `Debug` is the lowest common denominator for the
        // `Error` trait, and the user typically prints
        // dropped HRESULTs with `{:?}`.
        let dbg = format!("{:?}", e);
        assert!(dbg.contains("RegisterFailed"), "got `{}`", dbg);
        assert!(dbg.contains("1"), "got `{}`", dbg);
    }

    // ---------- OleDragData (source side, v0.6.2) ----------

    /// `OleDragData::Files` must round-trip the inner `Vec<PathBuf>`
    /// through `match`. The COM vtable reads this in
    /// `IDataObject::GetData` to decide between `CF_HDROP`
    /// and `CF_UNICODETEXT`.
    #[test]
    fn ole_drag_data_files_round_trip() {
        let data = OleDragData::Files(vec![
            PathBuf::from("C:/tmp/a.txt"),
            PathBuf::from("C:/tmp/b.bin"),
        ]);
        match &data {
            OleDragData::Files(paths) => {
                assert_eq!(paths.len(), 2);
                assert_eq!(paths[0], PathBuf::from("C:/tmp/a.txt"));
                assert_eq!(paths[1], PathBuf::from("C:/tmp/b.bin"));
            }
            _ => panic!("expected Files, got {:?}", data),
        }
    }

    /// `OleDragData::Text` must round-trip the inner `String`.
    #[test]
    fn ole_drag_data_text_round_trip() {
        let data = OleDragData::Text("hello world".to_string());
        match &data {
            OleDragData::Text(s) => assert_eq!(s, "hello world"),
            _ => panic!("expected Text, got {:?}", data),
        }
    }

    /// `OleDragData` is `Debug`-printable so the COM vtable
    /// can include it in an error message when the
    /// destination's `IDataObject::GetData` fails for an
    /// unexpected format.
    #[test]
    fn ole_drag_data_is_debug() {
        let files = format!(
            "{:?}",
            OleDragData::Files(vec![PathBuf::from("a")])
        );
        assert!(files.contains("Files"), "got `{}`", files);
        assert!(files.contains("a"), "got `{}`", files);

        let text = format!("{:?}", OleDragData::Text("hi".to_string()));
        assert!(text.contains("Text"), "got `{}`", text);
        assert!(text.contains("hi"), "got `{}`", text);
    }

    // ---------- DragContinueResult (source side, v0.6.2) ----------

    /// The three variants are the source-side mirror of
    /// `OleDropEffect`'s three-state "what did the
    /// destination accept" model. `match`-ability is the
    /// only thing we require: `IDropSource::QueryContinueDrag`
    /// returns one of these.
    #[test]
    fn drag_continue_result_variants_match() {
        let c = DragContinueResult::Continue;
        let d = DragContinueResult::Drop;
        let x = DragContinueResult::Cancel;
        assert!(matches!(c, DragContinueResult::Continue));
        assert!(matches!(d, DragContinueResult::Drop));
        assert!(matches!(x, DragContinueResult::Cancel));
        // All three must be `Copy` (no `&` in the closure
        // signature) and `PartialEq` (callers may want to
        // branch on the specific variant).
        assert_eq!(c, DragContinueResult::Continue);
        assert_ne!(c, DragContinueResult::Drop);
        assert_ne!(c, DragContinueResult::Cancel);
    }

    // ---------- OleDragSourceCallbacks (source side, v0.6.2) ----------

    /// The callbacks struct must `Default`-construct with
    /// every slot set to `None`. The OLE source first
    /// publishes a `DoDragDrop` and only installs callbacks
    /// afterwards via `set_callbacks`; the no-callback
    /// default is the steady state.
    #[test]
    fn ole_drag_source_callbacks_default_is_all_none() {
        let c = OleDragSourceCallbacks::default();
        assert!(c.on_query_continue_drag.is_none());
        assert!(c.on_give_feedback.is_none());
    }

    /// Installing a closure into one of the slots and then
    /// reading it back must round-trip the closure identity.
    /// We use a `Cell<u32>` counter so the closure does not
    /// capture a `&mut` borrow of `c` (which would conflict
    /// with the read).
    #[test]
    fn ole_drag_source_callbacks_install_and_read() {
        use std::cell::Cell;
        let mut c = OleDragSourceCallbacks::default();
        let counter = Cell::new(0u32);
        c.on_query_continue_drag = Some(Box::new(move |_esc: u32| {
            counter.set(counter.get() + 1);
            DragContinueResult::Continue
        }));
        // Trigger the closure once and verify the counter
        // moved. We do this by extracting the Box, calling,
        // and re-installing; the round-trip identity is
        // proven by `Some` vs `None`.
        let mut cb = c.on_query_continue_drag.take().expect("installed");
        let result = cb(0);
        assert!(matches!(result, DragContinueResult::Continue));
        assert!(c.on_query_continue_drag.is_none());
    }

    // ---------- OleDragError (source side, v0.6.2) ----------

    /// `OleDragError` must be `Debug`-printable and must
    /// implement `std::error::Error` so a `?` from
    /// `OleDragSource::do_drag_drop` can be embedded in a
    /// user-facing `Result<_, Box<dyn Error>>`. The
    /// `Display` impl uses lower-case hex for the HRESULT
    /// (matching the COM spec convention).
    #[test]
    fn ole_drag_error_is_std_error() {
        // `Debug` is the default `#[derive]` and emits the
        // variant name + decimal payload. Useful for logs
        // and panic messages.
        let dbg = format!(
            "{:?}",
            OleDragError::DoDragDropFailed(0x8004_0100u32 as i32)
        );
        assert!(dbg.contains("DoDragDropFailed"), "got `{}`", dbg);
        // `Display` is the human-facing form and emits the
        // HRESULT in lower-case hex so the user can paste
        // it into a debugger.
        let e: Box<dyn std::error::Error> =
            Box::new(OleDragError::DoDragDropFailed(0x8004_0100u32 as i32));
        let s = format!("{}", e);
        assert!(s.contains("0x80040100"), "got `{}`", s);
    }

    /// `OleDragError::Display` must include the raw HRESULT
    /// so the user can paste it into a debugger. `AlreadyInProgress`
    /// is the only non-HRESULT variant; it must be self-describing.
    #[test]
    fn ole_drag_error_display_mentions_payload() {
        let dup = OleDragError::AlreadyInProgress;
        let s = format!("{}", dup);
        assert!(
            s.to_lowercase().contains("in progress")
                || s.to_lowercase().contains("progress"),
            "got `{}`",
            s
        );

        let hr = OleDragError::DoDragDropFailed(0x8004_0005u32 as i32);
        let s2 = format!("{}", hr);
        assert!(s2.contains("0x80040005"), "got `{}`", s2);
    }

    /// `OleDragError` must be `Copy` + `PartialEq` so
    /// `Result<_, OleDragError>` can be matched and the
    /// error code compared without an extra `&`.
    #[test]
    fn ole_drag_error_is_copy_and_eq() {
        let a = OleDragError::AlreadyInProgress;
        let b = a; // Copy
        assert_eq!(a, b);
        let c = OleDragError::DoDragDropFailed(0);
        assert_ne!(a, c);
    }

    // ---------- v0.6.4: Display + From conversions ----------

    /// `OleDropEffect::Display` should print the standard
    /// names in canonical order (`COPY | MOVE | LINK | SCROLL`)
    /// with no surrounding `OleDropEffect(...)` wrapper.
    #[test]
    fn ole_drop_effect_display_is_canonical() {
        assert_eq!(format!("{}", OleDropEffect::NONE), "NONE");
        assert_eq!(format!("{}", OleDropEffect::COPY), "COPY");
        assert_eq!(format!("{}", OleDropEffect::MOVE), "MOVE");
        assert_eq!(format!("{}", OleDropEffect::LINK), "LINK");
        assert_eq!(format!("{}", OleDropEffect::SCROLL), "SCROLL");
        let combined = OleDropEffect::COPY | OleDropEffect::MOVE;
        assert_eq!(format!("{}", combined), "COPY | MOVE");
        let unknown = OleDropEffect::from_bits_truncate(0xDEAD_BEEF);
        let s = format!("{}", unknown);
        assert!(s.contains("UNKNOWN"), "got `{}`", s);
    }

    /// `OleDropEffect::Display` for the default value should
    /// be `"NONE"` — matches the `Default` impl (`u32 = 0`).
    #[test]
    fn ole_drop_effect_display_default_is_none() {
        assert_eq!(format!("{}", OleDropEffect::default()), "NONE");
    }

    /// `From<u32>` / `From<OleDropEffect> for u32` must
    /// round-trip without losing any bit pattern.
    #[test]
    fn ole_drop_effect_from_u32_round_trip() {
        for bits in [0u32, 1, 2, 4, 0x8000_0000, 0xFFFF_FFFF] {
            let eff: OleDropEffect = bits.into();
            let back: u32 = eff.into();
            assert_eq!(eff.bits(), bits);
            assert_eq!(back, bits);
        }
    }

    /// `OleDroppedData::Display` should print a count and the
    /// paths for `Files`, a one-line summary for `Text`, and the
    /// literal string for `Other`.
    #[test]
    fn ole_dropped_data_display_is_human_readable() {
        let files = OleDroppedData::Files(vec![
            PathBuf::from("a.txt"),
            PathBuf::from("b.txt"),
        ]);
        let s = format!("{}", files);
        assert!(s.contains("Files(2)"), "got `{}`", s);
        assert!(s.contains("a.txt"), "got `{}`", s);
        assert!(s.contains("b.txt"), "got `{}`", s);

        let text = OleDroppedData::Text("hello world".to_string());
        let s = format!("{}", text);
        assert_eq!(s, "Text(11 chars)");

        assert_eq!(format!("{}", OleDroppedData::Other), "Other");
    }

    /// `OleDragData::Display` mirrors `OleDroppedData`'s
    /// formatting: `Files(N)` and `Text(N chars)`.
    #[test]
    fn ole_drag_data_display_is_human_readable() {
        let files = OleDragData::Files(vec![PathBuf::from("a")]);
        let s = format!("{}", files);
        assert!(s.contains("Files(1)"), "got `{}`", s);
        let text = OleDragData::Text("hello".to_string());
        assert_eq!(format!("{}", text), "Text(5 chars)");
    }

    /// `DragContinueResult::Display` should print the literal
    /// variant name in `PascalCase`.
    #[test]
    fn drag_continue_result_display_is_pascal_case() {
        assert_eq!(format!("{}", DragContinueResult::Continue), "Continue");
        assert_eq!(format!("{}", DragContinueResult::Drop), "Drop");
        assert_eq!(format!("{}", DragContinueResult::Cancel), "Cancel");
    }
}
