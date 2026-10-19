//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Virtual list box (`wxVListBox`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::controls::list_ctrl::{ListCtrl, ListCtrlStyle};
use crate::core::widget::WidgetRef;
use crate::window::frame::Frame;

/// Owner-data list with on-demand line text (`wxVListBox`).
#[derive(Clone)]
pub struct VListBox {
    list: ListCtrl,
    lines: Rc<RefCell<Vec<String>>>,
}

impl VListBox {
    /// Create a virtual list bound to `frame` for `LVN_GETDISPINFOW` dispatch.
    pub fn new(frame: &Frame) -> Self {
        // `ReportVirtual`: `LVS_OWNERDATA` si decide alla creazione.
        let list = ListCtrl::new(frame, ListCtrlStyle::ReportVirtual);
        list.insert_column(0, "", 200);
        let lines = Rc::new(RefCell::new(Vec::<String>::new()));
        let lines_cb = lines.clone();
        list.on_get_disp_info(frame, move |item| {
            if item.is_text_requested() && item.sub_item() == 0 {
                let idx = item.index();
                let text = lines_cb.borrow().get(idx).cloned().unwrap_or_default();
                let _ = item.set_text(&text);
            }
        });
        Self { list, lines }
    }

    pub fn set_line_count(&self, count: usize) {
        let mut lines = self.lines.borrow_mut();
        lines.resize(count, String::new());
        self.list.set_item_count(count as u32);
    }

    pub fn set_line(&self, index: usize, text: &str) {
        let changed = {
            let mut lines = self.lines.borrow_mut();
            if index < lines.len() {
                lines[index] = text.to_string();
                true
            } else {
                false
            }
        };
        // In owner-data mode the ListView caches the painted text;
        // invalidate the row so it re-queries LVN_GETDISPINFO.
        if changed {
            self.list.redraw_items(index, index);
        }
    }

    pub fn line_count(&self) -> usize {
        self.lines.borrow().len()
    }

    pub fn line(&self, index: usize) -> Option<String> {
        self.lines.borrow().get(index).cloned()
    }

    pub fn selection(&self) -> Option<usize> {
        let sel = self.list.get_selected_items();
        sel.first().copied()
    }

    pub fn as_widget_ref(&self) -> WidgetRef {
        self.list.as_widget_ref()
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    use windows_sys::Win32::UI::Controls::{
        LVIF_TEXT, LVITEMW, LVM_GETITEMCOUNT, LVS_OWNERDATA, NMLVDISPINFOW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SendMessageW, GWL_STYLE, WM_NOTIFY,
    };

    /// Catena completa: stile virtuale alla creazione, count nativo,
    /// e `LVN_GETDISPINFOW` che riempie il testo della riga.
    #[test]
    fn virtuale_con_disp_info_funzionante() {
        let frame = Frame::builder()
            .with_title("vlistbox-test")
            .with_size(200, 120)
            .build();
        let vlist = VListBox::new(&frame);
        vlist.set_line_count(2);
        vlist.set_line(0, "Line 0");

        let hwnd = vlist.list.as_widget_ref().borrow().native_handle();
        // SAFETY: HWND del ListView appena creato, ancora vivo.
        let (style, count) = unsafe {
            (
                GetWindowLongPtrW(hwnd as _, GWL_STYLE) as u32,
                SendMessageW(hwnd as _, LVM_GETITEMCOUNT, 0, 0) as u32,
            )
        };
        assert_ne!(style & LVS_OWNERDATA, 0, "LVS_OWNERDATA non attivo");
        assert_eq!(count, 2, "LVM_GETITEMCOUNT");

        // Simula la richiesta di testo per la riga 0.
        let mut buf = [0u16; 64];
        // SAFETY: strutture POD azzerate; `buf` vive quanto `nm`.
        let mut nm: NMLVDISPINFOW = unsafe { std::mem::zeroed() };
        nm.hdr.idFrom = vlist.list.id() as usize;
        nm.hdr.code = crate::controls::list_ctrl::LVN_GETDISPINFOW;
        let mut item: LVITEMW = unsafe { std::mem::zeroed() };
        item.mask = LVIF_TEXT;
        item.iItem = 0;
        item.iSubItem = 0;
        item.pszText = buf.as_mut_ptr();
        item.cchTextMax = buf.len() as i32;
        nm.item = item;

        let frame_hwnd = crate::platform::window_handle_as_hwnd(&frame);
        // SAFETY: `nm` valida per la chiamata sincrona; frame reale.
        unsafe {
            SendMessageW(
                frame_hwnd,
                WM_NOTIFY,
                vlist.list.id() as usize,
                &mut nm as *mut _ as isize,
            );
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        assert_eq!(String::from_utf16_lossy(&buf[..len]), "Line 0");
    }
}
