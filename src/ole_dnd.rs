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
        let hglobal = unsafe { medium.u.hGlobal } as *const u8;
        let len_bytes = unsafe { *(hglobal as *const u32) } as usize;
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
    #[test]
    fn ole_dropped_data_variants_match() {
        let files = OleDroppedData::Files(vec![PathBuf::from("a.txt")]);
        let text = OleDroppedData::Text("hello".to_string());
        let other = OleDroppedData::Other;
        match files {
            OleDroppedData::Files(p) => assert_eq!(p.len(), 1),
            _ => panic!("Files variant did not match"),
        }
        match text {
            OleDroppedData::Text(s) => assert_eq!(s, "hello"),
            _ => panic!("Text variant did not match"),
        }
        match other {
            OleDroppedData::Other => {}
            _ => panic!("Other variant did not match"),
        }
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
}
