//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
#![windows_subsystem = "windows"]

use ru_wx::{
    ActivateAppEvent, App, BoxSizer, Cell, Frame, FunctionGridTable, Grid, GridCellEditor,
    GridCellRenderer, GridTable, IPAddressCtrl, KillFocusEvent, NcPaintEvent, ObjectRefData,
    ProcessExitEvent, Rect, RichTextBuffer, SetFocusEvent, Size2D, StaticText, StatusBar,
    StreamError, SysCommandEvent, Variant, WxHashSet,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — round 31")
        .with_size(480, 280)
        .with_modern_style().build();
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Round 31: events + grid helpers.", 0);
    let _hint = StaticText::new(&frame, "GridTable / IPAddress / RichTextBuffer:");
    let _kill = KillFocusEvent::new(0);
    let _set = SetFocusEvent::new(0);
    let _nc = NcPaintEvent::new(Rect::new(0, 0, 10, 10));
    let _sys = SysCommandEvent::close();
    let _app_ev = ActivateAppEvent::activated();
    let _exit = ProcessExitEvent::new(1, 0);
    let _err = StreamError::new("eof");
    let _size = Size2D::new(100.0, 50.0);
    let _ref = ObjectRefData::new(Variant::from("data"));
    let mut set = WxHashSet::new();
    set.insert("ru_wx");
    let mut editor = GridCellEditor::new(0, 0);
    editor.begin_edit("cell");
    let _ = editor.end_edit(true);
    let renderer = GridCellRenderer::new();
    let _ = renderer.render_text(&Cell::Text("ok".into()), Rect::new(0, 0, 40, 20));
    let table = FunctionGridTable::new(2, 2, |row, col| {
        Cell::Text(format!("{row},{col}"))
    });
    let _ = table.value(0, 0);
    let rows = table.row_count();
    let grid = Grid::new(&frame);
    grid.append_column("A", 80);
    grid.set_value_provider(move |row, col| Cell::Text(format!("{row},{col}")));
    grid.set_row_count(rows);
    let ip = IPAddressCtrl::new(&frame);
    ip.set_address("192.168.0.1");
    let _ = ip.is_valid_ipv4();
    let mut buf = RichTextBuffer::from_plain("round 31");
    buf.add_bold_range(0, 5);
    frame.on_kill_focus(|_| {});
    frame.on_set_focus(|_| {});
    let mut sizer = BoxSizer::vertical();
    sizer.add(_hint.as_widget_ref());
    sizer.add(ip.as_widget_ref());
    frame.set_sizer(sizer);
    app.run(frame);
}
