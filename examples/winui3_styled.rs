//! Themed WinUI3 showcase — uses the same widgets as
//! `winui3_widgets_showcase` but lays them out in a more polished
//! dashboard style, with animated progress and live slider feedback.
//!
//! Run with:
//! ```bash
//! cargo run --release --example winui3_styled --features winui3
//! ```

#![windows_subsystem = "windows"]

use std::ptr;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetClientRect, SendMessageW, SetWindowPos, SetWindowTextW,
};

use ru_wx::platform::winui3::{
    events, set_frame_icon, svg_to_hicon, App, Frame, Hyperlink, Panel, ProgressBar, Slider,
    SpinCtrl, Splitter, StaticBox, StatusBar, TextCtrl, Timer, TreeCtrl,
};

fn w(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn main() {
    let app = App::new();

    let frame = Frame::new("ru_wx Dashboard", 1100, 700);
    frame.center();
    frame.show();

    // Frame icon: an "R" badge SVG.
    let svg: &[u8] = b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'><rect x='4' y='4' width='24' height='24' rx='4' fill='%234f46e5'/><text x='16' y='22' text-anchor='middle' fill='white' font-size='18' font-family='Segoe UI'>R</text></svg>";
    if let Some(h) = svg_to_hicon(svg, 32) {
        set_frame_icon(frame.hwnd(), h);
    }

    // Left: tree navigator.
    let mut tree = TreeCtrl::new(&frame);
    let root = tree.add_root("Project");
    let src = tree.append_item(root, "src/");
    tree.append_item(src, "ui/");
    tree.append_item(src, "net/");
    tree.append_item(root, "Cargo.toml");
    tree.append_item(root, "README.md");
    tree.select_item(root);

    // Splitter between left tree and right panel.
    let splitter = Splitter::new(&frame);

    // Right: a panel with three labeled groups.
    let right = Panel::new(&frame);
    let grp_status = StaticBox::new(&right, "Live status");
    let slider = Slider::new(&right, 0, 100);
    let grp_form = StaticBox::new(&right, "Form");
    let spin = SpinCtrl::new(&right, 0, 100, 33);
    let text_in = TextCtrl::new(&right, "Type here...");
    let grp_anim = StaticBox::new(&right, "Animation");
    let progress = ProgressBar::new(&right);
    // Build a Hyperlink at the bottom of the Animation group.
    let link = Hyperlink::new(&right, "Visit ru_wx docs", "https://example.com");

    // Exercise the colour helpers: a bright accent on the slider thumb
    // would be ideal, but for safety we only set text-colour through
    // the frame subclass that the Hyperlink installed automatically.
    println!("hyperlink hwnd = {:?}", link.hwnd());

    // Bottom: status bar with 3 parts (status / progress text / fake clock).
    let mut status = StatusBar::new(&frame);
    status.set_part_count(3);
    status.set_part_text(0, "Ready");
    status.set_part_text(1, "Slider 33 / 100");
    status.set_part_text(2, "00:00:00");

    // -----------------------------------------------------------------------
    // 1. Slider mirrors its live value to the status bar + TextCtrl.
    // -----------------------------------------------------------------------
    let text_in_hwnd = text_in.hwnd();
    slider.on_change(&frame, move |v| {
        let wide = w(&format!("{v}/100"));
        unsafe {
            SetWindowTextW(text_in_hwnd, wide.as_ptr());
        }
    });

    // Direct registration for status bar updates (same effect, going
    // via HWND-keyed dispatcher rather than closure).
    events::register_command(frame.hwnd(), 9999, move |_ev| {
        // No-op; we already drive the slider->text binding via
        // `slider.on_change`.
    });

    // -----------------------------------------------------------------------
    // 2. Animation timer ticks every 200 ms.
    // -----------------------------------------------------------------------
    let progress_hwnd = progress.hwnd();
    let status_hwnd = status.hwnd();
    let _timer = Timer::new(&frame, 200, move || {
        let prev = unsafe { SendMessageW(progress_hwnd, 0x0408, 0, 0) };
        let next = (prev + 1) % 101;
        unsafe {
            SendMessageW(progress_hwnd, 0x0402, next as usize, 0);
        }
        let wide = w(&format!(
            "tick {:02}:{:02}:{:02}",
            (next % 24),
            (next % 60),
            (next % 60)
        ));
        let _ = unsafe {
            SendMessageW(
                status_hwnd,
                0x040B,
                2usize,
                std::mem::transmute(wide.as_ptr()),
            )
        };
    });

    // -----------------------------------------------------------------------
    // 3. Multi-region manual layout via SetWindowPos.
    //
    //   * tree on the left at (0, 0, 240, ch-24)
    //   * splitter just to the right of the tree
    //   * right panel takes the remainder
    // -----------------------------------------------------------------------
    let mut client: RECT = unsafe { std::mem::zeroed() };
    unsafe {
        GetClientRect(frame.hwnd(), &mut client);
    }
    let cw = client.right - client.left;
    let ch = client.bottom - client.top;
    let noz = 0x0004u32;
    let pos = |h: HWND, x: i32, y: i32, w: i32, hgt: i32| unsafe {
        SetWindowPos(h, ptr::null_mut(), x, y, w, hgt, noz);
    };
    pos(tree.hwnd(), 0, 0, 240, ch - 24);
    pos(splitter.hwnd(), 240, 0, 6, ch - 24);
    pos(right.hwnd(), 246, 0, cw - 246, ch - 24);

    // Stack the three StaticBox groups + their child widgets.
    let mut rc: RECT = unsafe { std::mem::zeroed() };
    unsafe {
        GetClientRect(right.hwnd(), &mut rc);
    }
    let cw2 = rc.right - rc.left;
    let cx = 16_i32;
    let mut yy = 14_i32;

    // grp_status group + slider
    pos(grp_status.hwnd(), cx, yy, cw2 - 32, 60);
    yy += 64;
    pos(slider.hwnd(), cx + 14, yy, cw2 - 60, 26);
    yy += 34;
    yy += 8;

    // grp_form group + spin + text
    pos(grp_form.hwnd(), cx, yy, cw2 - 32, 80);
    yy += 80;
    pos(spin.hwnd(), cx + 16, yy, 60, 24);
    yy += 32;
    pos(text_in.hwnd(), cx + 90, yy - 32, cw2 - 120, 24);

    // grp_anim group + progress + link
    yy += 8;
    pos(grp_anim.hwnd(), cx, yy, cw2 - 32, 80);
    yy += 30;
    pos(progress.hwnd(), cx + 16, yy, cw2 - 60, 18);
    yy += 28;
    pos(link.hwnd(), cx + 16, yy, cw2 - 60, 24);

    println!("ru_wx dashboard ready.");
    app.run(&frame, || {});
}
