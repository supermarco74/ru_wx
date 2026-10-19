//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Reorder list dialog (`wxRearrangeDialog`).

use crate::window::frame::Frame;

#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use std::cell::RefCell;
#[cfg(target_os = "windows")]
use std::rc::Rc;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{GetStockObject, UpdateWindow, DEFAULT_GUI_FONT, HBRUSH};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[cfg(target_os = "windows")]
const REARRANGE_CLASS: &str = "RuWxRearrangeDialogClass";
#[cfg(target_os = "windows")]
const IDOK_I: i32 = 1;
#[cfg(target_os = "windows")]
const IDCANCEL_I: i32 = 2;
#[cfg(target_os = "windows")]
const IDUP_I: i32 = 3;
#[cfg(target_os = "windows")]
const IDDOWN_I: i32 = 4;

#[cfg(target_os = "windows")]
struct RearrangeInner {
    hwnd: HWND,
    hwnd_list: HWND,
    items: Vec<String>,
    result: Option<Vec<String>>,
    is_done: bool,
}

/// Modal dialog to reorder a list of strings (`wxRearrangeDialog`).
pub struct RearrangeDialog {
    title: String,
    items: Vec<String>,
}

impl RearrangeDialog {
    pub fn new(title: &str, items: Vec<String>) -> Self {
        Self {
            title: title.to_string(),
            items,
        }
    }

    pub fn items(&self) -> &[String] {
        &self.items
    }

    pub fn move_up(&mut self, index: usize) {
        if index > 0 && index < self.items.len() {
            self.items.swap(index, index - 1);
        }
    }

    pub fn move_down(&mut self, index: usize) {
        if index + 1 < self.items.len() {
            self.items.swap(index, index + 1);
        }
    }

    /// Show modally. Returns reordered items, or `None` if cancelled.
    pub fn show_modal(self, frame: &Frame) -> Option<Vec<String>> {
        #[cfg(target_os = "windows")]
        {
            let inner = build_rearrange_dialog(frame, &self.title, self.items);
            inner.borrow_mut().is_done = false;
            inner.borrow_mut().result = None;
            let hwnd = inner.borrow().hwnd;
            run_rearrange_modal_loop(hwnd);
            let result = inner.borrow_mut().result.take();
            result
        }
        #[cfg(target_os = "macos")]
        {
            use std::cell::{Cell, RefCell};
            use std::rc::Rc;

            use crate::core::geometry::Rect;
            use crate::platform::{
                appkit::{
                    create_button, create_dialog_window, create_list_box, create_static_text,
                    dialog_close, dialog_run_until, listbox_append_item, listbox_clear,
                    listbox_get_selection, listbox_set_selection,
                },
                next_control_id, window_handle,
            };

            let items = self.items;
            let dlg = create_dialog_window(window_handle(frame), &self.title, 400, 340);
            create_static_text(
                dlg,
                "Select an item and use Up/Down to reorder:",
                Rect::new(12, 12, 360, 18),
            );
            let list_id = next_control_id();
            let list = create_list_box(dlg, list_id, Rect::new(12, 36, 376, 220), false);
            for item in &items {
                listbox_append_item(list, item);
            }
            listbox_set_selection(list, 0);

            let id_up = next_control_id();
            let id_down = next_control_id();
            let id_ok = next_control_id();
            let id_cancel = next_control_id();
            create_button(dlg, "Up", id_up, Rect::new(12, 268, 60, 28));
            create_button(dlg, "Down", id_down, Rect::new(80, 268, 60, 28));
            create_button(dlg, "OK", id_ok, Rect::new(220, 268, 80, 28));
            create_button(dlg, "Cancel", id_cancel, Rect::new(308, 268, 80, 28));

            let working = Rc::new(RefCell::new(items));
            let selected = Rc::new(Cell::new(0usize));
            let running = Rc::new(Cell::new(true));
            let result: Rc<Cell<Option<Option<Vec<String>>>>> = Rc::new(Cell::new(None));

            let working_up = working.clone();
            let sel_up = selected.clone();
            frame.register_command_handler(id_up, {
                Box::new(move || {
                    let mut items = working_up.borrow_mut();
                    let idx = sel_up.get();
                    if idx > 0 {
                        items.swap(idx, idx - 1);
                        sel_up.set(idx - 1);
                        let snapshot = items.clone();
                        drop(items);
                        listbox_clear(list);
                        for item in &snapshot {
                            listbox_append_item(list, item);
                        }
                        listbox_set_selection(list, sel_up.get());
                    }
                })
            });

            let working_down = working.clone();
            let sel_down = selected.clone();
            frame.register_command_handler(id_down, {
                Box::new(move || {
                    let mut items = working_down.borrow_mut();
                    let idx = sel_down.get();
                    if idx + 1 < items.len() {
                        items.swap(idx, idx + 1);
                        sel_down.set(idx + 1);
                        let snapshot = items.clone();
                        drop(items);
                        listbox_clear(list);
                        for item in &snapshot {
                            listbox_append_item(list, item);
                        }
                        listbox_set_selection(list, sel_down.get());
                    }
                })
            });

            let result_ok = result.clone();
            let working_ok = working.clone();
            let running_ok = running.clone();
            frame.register_command_handler(id_ok, {
                Box::new(move || {
                    result_ok.set(Some(Some(working_ok.borrow().clone())));
                    running_ok.set(false);
                })
            });

            let result_cancel = result.clone();
            let running_cancel = running.clone();
            frame.register_command_handler(id_cancel, {
                Box::new(move || {
                    result_cancel.set(Some(None));
                    running_cancel.set(false);
                })
            });

            frame.register_command_handler(list_id, {
                let sel_list = selected.clone();
                Box::new(move || {
                    if let Some(idx) = listbox_get_selection(list) {
                        sel_list.set(idx);
                    }
                })
            });

            dialog_run_until(dlg, || running.get());
            dialog_close(dlg);
            result.take().flatten()
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            let _ = frame;
            console_rearrange_dialog(&self.title, self.items)
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn console_rearrange_dialog(title: &str, mut items: Vec<String>) -> Option<Vec<String>> {
    use std::io::{self, Write};

    let mut selected = 0usize;
    loop {
        println!("\n{title}");
        for (i, item) in items.iter().enumerate() {
            let mark = if i == selected { ">" } else { " " };
            println!("{mark} {}. {item}", i + 1);
        }
        print!("Command [u=p up, d=down, Enter=OK, q=cancel]: ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_err() {
            return None;
        }
        match line.trim() {
            "" => return Some(items),
            "q" | "Q" => return None,
            "u" | "U" if selected > 0 => {
                items.swap(selected, selected - 1);
                selected -= 1;
            }
            "d" | "D" if selected + 1 < items.len() => {
                items.swap(selected, selected + 1);
                selected += 1;
            }
            n if n
                .parse::<usize>()
                .ok()
                .filter(|&v| v >= 1 && v <= items.len())
                .is_some() =>
            {
                selected = n.parse::<usize>().unwrap() - 1;
            }
            _ => {}
        }
    }
}

#[cfg(target_os = "windows")]
fn register_rearrange_class() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        let wide = to_wide(REARRANGE_CLASS);
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(rearrange_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: GetModuleHandleW(std::ptr::null()),
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: (6_isize + 1) as HBRUSH,
            lpszMenuName: std::ptr::null(),
            lpszClassName: wide.as_ptr(),
        };
        RegisterClassW(&wc);
    });
}

#[cfg(target_os = "windows")]
fn build_rearrange_dialog(
    frame: &Frame,
    title: &str,
    items: Vec<String>,
) -> Rc<RefCell<RearrangeInner>> {
    register_rearrange_class();
    unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let parent = frame.hwnd();
        let wide_class = to_wide(REARRANGE_CLASS);
        let wide_title = to_wide(title);
        let dlg_w = 420;
        let dlg_h = 320;
        let hwnd = CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            wide_class.as_ptr(),
            wide_title.as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            dlg_w,
            dlg_h,
            parent as *mut std::ffi::c_void,
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null_mut(),
        );
        let hwnd_list = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            to_wide("LISTBOX").as_ptr(),
            std::ptr::null(),
            WS_CHILD | WS_VISIBLE | WS_VSCROLL | (LBS_NOTIFY as u32),
            10,
            10,
            dlg_w - 120,
            dlg_h - 60,
            hwnd,
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null_mut(),
        );
        for item in &items {
            let wide = to_wide(item);
            SendMessageW(hwnd_list, LB_ADDSTRING, 0, wide.as_ptr() as LPARAM);
        }
        let hfont = GetStockObject(DEFAULT_GUI_FONT);
        SendMessageW(hwnd_list, WM_SETFONT, hfont as usize, 1);
        let btn_w = 90;
        let btn_h = 28;
        let btn_x = dlg_w - btn_w - 15;
        create_button(hwnd, hinstance, "Su", btn_x, 10, btn_w, btn_h, IDUP_I);
        create_button(hwnd, hinstance, "Giù", btn_x, 46, btn_w, btn_h, IDDOWN_I);
        create_button(
            hwnd,
            hinstance,
            "OK",
            btn_x,
            dlg_h - 90,
            btn_w,
            btn_h,
            IDOK_I,
        );
        create_button(
            hwnd,
            hinstance,
            "Annulla",
            btn_x,
            dlg_h - 54,
            btn_w,
            btn_h,
            IDCANCEL_I,
        );
        let inner = Rc::new(RefCell::new(RearrangeInner {
            hwnd,
            hwnd_list,
            items,
            result: None,
            is_done: false,
        }));
        let raw = Rc::into_raw(inner.clone());
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw as isize);
        inner
    }
}

#[cfg(target_os = "windows")]
#[allow(clippy::too_many_arguments)]
unsafe fn create_button(
    parent: HWND,
    hinstance: HINSTANCE,
    label: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    id: i32,
) -> HWND {
    let style = if id == IDOK_I {
        WS_CHILD | WS_VISIBLE | (BS_DEFPUSHBUTTON as u32)
    } else {
        WS_CHILD | WS_VISIBLE | (BS_PUSHBUTTON as u32)
    };
    let hwnd = CreateWindowExW(
        0,
        to_wide("BUTTON").as_ptr(),
        to_wide(label).as_ptr(),
        style,
        x,
        y,
        w,
        h,
        parent,
        id as usize as HMENU,
        hinstance,
        std::ptr::null_mut(),
    );
    let hfont = GetStockObject(DEFAULT_GUI_FONT);
    SendMessageW(hwnd, WM_SETFONT, hfont as usize, 1);
    hwnd
}

#[cfg(target_os = "windows")]
fn run_rearrange_modal_loop(hwnd: HWND) {
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
        loop {
            while PeekMessageW(&mut msg, hwnd, 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    return;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let inner_ptr =
                GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<RearrangeInner>;
            if !inner_ptr.is_null() && (*inner_ptr).borrow().is_done {
                break;
            }
            WaitMessage();
        }
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn rearrange_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let inner_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<RearrangeInner>;
    match msg {
        WM_COMMAND => {
            if inner_ptr.is_null() {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }
            let id = (wparam & 0xFFFF) as i32;
            let mut inner = (*inner_ptr).borrow_mut();
            match id {
                IDOK_I => {
                    inner.items = read_list_items(inner.hwnd_list);
                    inner.result = Some(inner.items.clone());
                    inner.is_done = true;
                    DestroyWindow(hwnd);
                }
                IDCANCEL_I => {
                    inner.result = None;
                    inner.is_done = true;
                    DestroyWindow(hwnd);
                }
                IDUP_I => {
                    move_selection(inner.hwnd_list, -1);
                }
                IDDOWN_I => {
                    move_selection(inner.hwnd_list, 1);
                }
                _ => {}
            }
            0
        }
        WM_DESTROY => {
            if !inner_ptr.is_null() {
                (*inner_ptr).borrow_mut().is_done = true;
            }
            0
        }
        WM_CLOSE => {
            PostMessageW(hwnd, WM_COMMAND, IDCANCEL_I as usize, 0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(target_os = "windows")]
unsafe fn read_list_items(hwnd_list: HWND) -> Vec<String> {
    let count = SendMessageW(hwnd_list, LB_GETCOUNT, 0, 0) as i32;
    let mut out = Vec::new();
    for i in 0..count {
        let len = SendMessageW(hwnd_list, LB_GETTEXTLEN, i as usize, 0) as usize;
        let mut buf = vec![0u16; len + 1];
        SendMessageW(
            hwnd_list,
            LB_GETTEXT,
            i as usize,
            buf.as_mut_ptr() as LPARAM,
        );
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        out.push(String::from_utf16_lossy(&buf[..end]));
    }
    out
}

#[cfg(target_os = "windows")]
unsafe fn move_selection(hwnd_list: HWND, delta: i32) {
    let sel = SendMessageW(hwnd_list, LB_GETCURSEL, 0, 0) as i32;
    if sel < 0 {
        return;
    }
    let new_sel = sel + delta;
    let count = SendMessageW(hwnd_list, LB_GETCOUNT, 0, 0) as i32;
    if new_sel < 0 || new_sel >= count {
        return;
    }
    let len = SendMessageW(hwnd_list, LB_GETTEXTLEN, sel as usize, 0) as usize;
    let mut buf = vec![0u16; len + 1];
    SendMessageW(
        hwnd_list,
        LB_GETTEXT,
        sel as usize,
        buf.as_mut_ptr() as LPARAM,
    );
    SendMessageW(hwnd_list, LB_DELETESTRING, sel as usize, 0);
    let insert_at = new_sel as usize;
    SendMessageW(
        hwnd_list,
        LB_INSERTSTRING,
        insert_at,
        buf.as_ptr() as LPARAM,
    );
    SendMessageW(hwnd_list, LB_SETCURSEL, insert_at, 0);
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn move_up_down_reorders_items() {
        let mut dlg = RearrangeDialog::new("Test", vec!["a".into(), "b".into(), "c".into()]);
        dlg.move_up(1);
        assert_eq!(dlg.items(), &["b", "a", "c"]);
        dlg.move_down(0);
        assert_eq!(dlg.items(), &["a", "b", "c"]);
    }
}
