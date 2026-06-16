//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
#![windows_subsystem = "windows"]

use ru_wx::{
    adv::rich_text_event::RichTextEventKind, adv::web_view_event::WebViewEventKind, App,
    AuiToolBarEvent, AuiToolBarEventKind, BoxSizer, BufferedDC, BufferedPaintDC, ButtonEvent,
    ChildFocusEvent, DataViewEvent, DataViewEventKind, Display, FileSystemChangeType,
    FileSystemWatcher, Frame, HeaderColumnEvent, HeaderCtrl, HtmlLinkEvent, Rect, RichTextEvent,
    StaticText, StatusBar, StreamBuffer, TextInputStream, TextOutputStream, WebViewEvent, WxFFile,
    WxInputStream, WxOutputStream,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — round 25")
        .with_size(480, 280)
        .build();
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Round 25: DC buffers + streams + display.", 0);
    let _hint = StaticText::new(&frame, "DC / I/O / events:");
    let s = status.clone();
    let mut header = HeaderCtrl::new(Rect::new(0, 0, 400, 24));
    header.append_column("Name", 200);
    header.on_column_event(move |ev: &HeaderColumnEvent| {
        s.set_status_text(&format!("Col {} -> {}px", ev.column, ev.width), 0);
    });
    header.resize_column(0, 180);
    let displays = Display::enumerate();
    let _primary = Display::primary();
    let mut buf_dc = BufferedDC::new(100, 100);
    let _ = buf_dc.memory_dc();
    let mut paint_dc = BufferedPaintDC::new(80, 60);
    let _ = paint_dc.memory_dc();
    let _btn = ButtonEvent::new(1);
    let _child = ChildFocusEvent::new(2);
    let _dv = DataViewEvent::new(DataViewEventKind::SelectionChanged, 0, 1);
    let _html = HtmlLinkEvent::new("https://example.com", "Example");
    let _rt = RichTextEvent::new(RichTextEventKind::TextUpdated, 0, 10);
    let _wv = WebViewEvent::new(WebViewEventKind::NavigationComplete, "https://ru_wx.dev");
    let _aui = AuiToolBarEvent::new(AuiToolBarEventKind::ToolClick, 100);
    let mut stream_buf = StreamBuffer::new();
    stream_buf.write(b"ru_wx").unwrap();
    stream_buf.reset_read();
    let mut read_buf = [0u8; 5];
    stream_buf.read(&mut read_buf).unwrap();
    let mut tin = TextInputStream::from_str("line1\nline2");
    let _l1 = tin.read_line().unwrap();
    let mut tout = TextOutputStream::new();
    tout.write_line("hello").unwrap();
    let _bytes = tout.into_bytes();
    let mut watcher = FileSystemWatcher::new();
    let s2 = status.clone();
    watcher.on_event(move |ev| {
        s2.set_status_text(&format!("{} {:?}", ev.path, ev.change_type), 0);
    });
    watcher.add(".");
    watcher.notify_change("demo.txt", FileSystemChangeType::Modify);
    if let Ok(mut f) = WxFFile::create(std::env::temp_dir().join("ru_wx_round25.tmp")) {
        let _ = f.write(b"test");
        let _ = f.flush();
    }
    let mut sizer = BoxSizer::vertical();
    sizer.add(_hint.as_widget_ref());
    frame.set_sizer(sizer);
    let _ = displays.len();
    app.run(frame);
}
