//! Comprehensive WinUI3 widgets showcase.
//!
//! Run with:
//! ```bash
//! cargo run --release --example winui3_widgets_showcase --features winui3
//! ```

#![windows_subsystem = "windows"]

use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::UI::WindowsAndMessaging::{GetClientRect, SetWindowPos};

use ru_wx::platform::winui3::{
    set_frame_icon, svg_to_hicon, App, Frame, Hyperlink, Panel, ProgressBar, Slider, SpinCtrl,
    Splitter, StaticBox, StaticLine, StatusBar, TextCtrl, Timer, ToolBar, TreeCtrl,
};

fn main() {
    let app = App::new();

    let frame = Frame::new("winui3 widgets showcase", 1024, 720);
    frame.center();
    frame.show();

    let svg: &[u8] = b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'><rect x='4' y='4' width='24' height='24' rx='4' fill='%234f46e5'/><text x='16' y='22' text-anchor='middle' fill='white' font-size='18'>R</text></svg>";
    if let Some(h) = svg_to_hicon(svg, 32) {
        set_frame_icon(frame.hwnd(), h);
    }

    // Toolbar.
    let mut tb = ToolBar::new(&frame);
    let _save = tb.add_tool("Save");
    let _open = tb.add_tool("Open");
    tb.add_separator();
    let _about = tb.add_tool("About");
    let _quit = tb.add_tool("Quit");

    // TreeCtrl on the LEFT.
    let mut tree = TreeCtrl::new(&frame);
    let root = tree.add_root("src/");
    let ui = tree.append_item(root, "ui/");
    tree.append_item(ui, "button.rs");
    tree.append_item(ui, "panel.rs");
    tree.append_item(root, "net.rs");
    tree.append_item(root, "Cargo.toml");
    tree.select_item(root);

    // RIGHT panel that hosts every WinUI3 widget.
    let right = Panel::new(&frame);
    let group = StaticBox::new(&right, "Demo widgets");
    let line = StaticLine::new(&right);
    let link = Hyperlink::new(&right, "Visit ru_wx docs", "https://example.com");
    let spin = SpinCtrl::new(&right, 0, 100, 50);
    let slider = Slider::new(&right, 0, 100);
    let progress = ProgressBar::new(&right);
    progress.set_range(0, 100);
    progress.set_value(33);
    let search = TextCtrl::new(&right, "search something...");
    let _splitter = Splitter::new(&right);

    slider.on_change(&frame, |v| println!("slider = {v}"));

    // StatusBar.
    let mut status = StatusBar::new(&frame);
    status.set_part_count(3);
    status.set_part_text(0, "Ready");
    status.set_part_text(1, "winui3 widgets");
    status.set_part_text(2, "v0.7.1");

    // ----- Manual layout (deterministic, debug-friendly) -----
    //
    // Layout zones in frame client area (cw × ch):
    //
    //   y=0                          toolbar        (cw wide × 32 tall)
    //   y=32..ch-24                  tree | right   (left=240 / right=cw-240)
    //   y=ch-24..ch                  status bar     (cw wide × 24 tall)
    //
    let mut client: RECT = unsafe { std::mem::zeroed() };
    unsafe {
        GetClientRect(frame.hwnd(), &mut client);
    }
    let cw = client.right - client.left;
    let ch = client.bottom - client.top;
    let no_zorder = 0x0004u32; // SWP_NOZORDER

    let pos = |h: windows_sys::Win32::Foundation::HWND, x: i32, y: i32, w: i32, hgt: i32| unsafe {
        SetWindowPos(h, std::ptr::null_mut(), x, y, w, hgt, no_zorder);
    };

    // Top: toolbar full width, height 32.
    pos(tb.hwnd(), 0, 0, cw, 32);

    // Middle: tree on left (240 wide) | right panel (rest).
    pos(tree.hwnd(), 0, 32, 240, ch - 32 - 24);
    pos(right.hwnd(), 240, 32, cw - 240, ch - 32 - 24);

    // Right panel content stack: title bar above each item.
    let rp_x = 240 + 12;
    let rp_w = cw - 240 - 24;
    let mut y = 32 + 8;
    pos(group.hwnd(), rp_x, y, rp_w, 24);
    y += 24;
    pos(line.hwnd(), rp_x, y, rp_w, 4);
    y += 8;
    pos(link.hwnd(), rp_x, y, rp_w, 22);
    y += 26;
    pos(spin.hwnd(), rp_x, y, 24, 24);
    y += 28;
    pos(slider.hwnd(), rp_x, y, rp_w, 30);
    y += 34;
    pos(progress.hwnd(), rp_x, y, rp_w, 18);
    y += 22;
    pos(search.hwnd(), rp_x, y, rp_w, 24);
    y += 28;
    pos(_splitter.hwnd(), rp_x, y, rp_w, 4);

    // Timer.
    let _timer = Timer::new(&frame, 500, || {});

    println!("winui3 widgets showcase ready.");
    app.run(&frame, || {});
}
