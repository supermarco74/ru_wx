//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
#![windows_subsystem = "windows"]

use ru_wx::{
    App, BoxSizer, CountingOutputStream, Frame, GridCoords, GridStringTable, GridTable, HtmlTagHandler,
    IpcClient, IpcServer, MouseCaptureChangedEvent, NcCalcSizeEvent, Rect, RichTextAttr,
    SizerSpacer, StaticSizer, StaticText, StatusBar, WebViewHandler, WxHashMap, WxOutputStream,
    ZipEntry, ZipFSHandler,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — round 32")
        .with_size(480, 280)
        .build();
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Round 32: IPC + grid extras + streams.", 0);
    let _hint = StaticText::new(&frame, "HashMap / GridStringTable / ZipEntry:");
    let mut map = WxHashMap::new();
    map.insert("key", "value");
    let _tag = HtmlTagHandler::new("p").with_attribute("class", "note");
    let mut web_handler = WebViewHandler::new("app");
    web_handler.set_handler(|path| format!("handled:{path}"));
    let _ = web_handler.handle("app://home");
    let mut static_sizer = StaticSizer::vertical();
    static_sizer.add_spacer(8);
    let spacer = SizerSpacer::new(12);
    let mut client = IpcClient::new("ru_wx");
    let _ = client.connect();
    let mut server = IpcServer::new("ru_wx");
    let _ = server.listen();
    server.push_message(b"hello".to_vec());
    let _coords = GridCoords::new(1, 2);
    let mut table = GridStringTable::new();
    table.resize(2, 2);
    table.set_value(0, 0, "A1");
    let _ = table.value(0, 0);
    let _nc = NcCalcSizeEvent::new(Rect::new(0, 0, 100, 100), true);
    let _cap = MouseCaptureChangedEvent::new(false, 0);
    let _attr = RichTextAttr::new().bold().with_font_size(12);
    let mut counter = CountingOutputStream::new();
    let _ = counter.write(b"round32");
    let zip = ZipFSHandler::new();
    zip.add_text("readme.txt", "zip");
    let _entry = ZipEntry::new("readme.txt", 3);
    let _ = zip.list_entries();
    frame.on_mouse_capture_changed(|_| {});
    let mut sizer = BoxSizer::vertical();
    sizer.add_sizer_spacer(spacer);
    sizer.add(_hint.as_widget_ref());
    frame.set_sizer(sizer);
    app.run(frame);
}
