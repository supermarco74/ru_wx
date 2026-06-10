//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
#![windows_subsystem = "windows"]

use ru_wx::{
    Accelerator, AcceleratorEntry, AcceleratorTable, App, BoxSizer, Frame, IconizeEvent,
    MaximizeEvent, MouseEnterEvent, MouseLeaveEvent, ProcessEvent, ScrollWinAxis, ScrollWinEvent,
    SocketServer, StaticText, StatusBar, ThreadHelperSimple, UpdateUIEvent,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — frame events round 19")
        .with_size(480, 220)
        .with_modern_style().build();
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Minimize, maximize, or move the mouse.", 0);
    let _hint = StaticText::new(&frame, "Iconize / maximize / mouse enter-leave:");
    let s = status.clone();
    frame.on_iconize(move |ev: &IconizeEvent| {
        let msg = if ev.iconized { "Iconized" } else { "Restored" };
        s.set_status_text(msg, 0);
    });
    let s2 = status.clone();
    frame.on_maximize(move |ev: &MaximizeEvent| {
        let msg = if ev.maximized { "Maximized" } else { "Unmaximized" };
        s2.set_status_text(msg, 0);
    });
    let s3 = status.clone();
    frame.on_mouse_enter(move |ev: &MouseEnterEvent| {
        s3.set_status_text(
            &format!("Mouse enter {}, {}", ev.position.x, ev.position.y),
            0,
        );
    });
    let s4 = status.clone();
    frame.on_mouse_leave(move |_ev: &MouseLeaveEvent| {
        s4.set_status_text("Mouse leave", 0);
    });
    let mut table = AcceleratorTable::new();
    table.add(AcceleratorEntry::new(
        Accelerator::parse("Ctrl+Shift+F").expect("accel"),
        9001,
    ));
    assert_eq!(table.len(), 1);
    let _scroll = ScrollWinEvent::new(ScrollWinAxis::Vertical, 0);
    let _ui = UpdateUIEvent::new(1);
    let _proc = ProcessEvent::terminate(0);
    let mut server = SocketServer::new();
    let _ = server.listen(0);
    let _worker = ThreadHelperSimple::run(|| ());
    frame.set_sizer(BoxSizer::vertical());
    app.run(frame);
}
