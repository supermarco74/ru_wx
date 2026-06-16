//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
#![windows_subsystem = "windows"]

use ru_wx::{
    App, ArrayDouble, ArrayInt, ArrayLong, BoxSizer, FFileInputStream, FtpClient, Frame,
    HttpClient, IpcConnection, ObjectClientData, Point2D, Rect2D, ScopedPtr, SizerItem, Socket,
    StaticText, StatusBar, StringClientData, StringList, Variant, WxAny,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — round 30")
        .with_size(480, 280)
        .build();
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Round 30: arrays + IPC + geometry2d.", 0);
    let _hint = StaticText::new(&frame, "ScopedPtr / WxAny / Socket:");
    let _ptr = ScopedPtr::new(42_u32);
    let _any = WxAny::from(99_i64);
    let _str_data = StringClientData::new("item");
    let _obj_data = ObjectClientData::new(Variant::from("meta"));
    let mut ints = ArrayInt::new();
    ints.add(1);
    let mut longs = ArrayLong::new();
    longs.add(2_i64);
    let mut doubles = ArrayDouble::new();
    doubles.add(2.5);
    let mut list = StringList::from_slice(&["a", "b"]);
    list.append("c");
    let _pt = Point2D::new(1.0, 2.0);
    let _rect = Rect2D::new(0.0, 0.0, 10.0, 10.0);
    let _item = SizerItem::stretch(1);
    let mut socket = Socket::new();
    socket.on_socket_event(|ev| {
        let _ = ev.kind;
    });
    socket.connect("127.0.0.1", 8080).ok();
    socket.notify_input(4);
    let mut ftp = FtpClient::new();
    let _ = ftp.connect("ftp.example.com", "user", "pass");
    let mut http = HttpClient::new();
    http.set_base_url("https://example.com");
    let _ = http.get("/");
    let mut ipc = IpcConnection::new("ru_wx");
    ipc.connect().ok();
    let _ = ipc.send(b"ping");
    let _ = ipc.receive();
    let mut sizer = BoxSizer::vertical();
    sizer.add(_hint.as_widget_ref());
    frame.set_sizer(sizer);
    let _ = FFileInputStream::open("Cargo.toml");
    app.run(frame);
}
