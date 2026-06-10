//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
#![windows_subsystem = "windows"]

use ru_wx::{
    App, BoxSizer, BufferedInputStream, DisplayChangedEvent, FileInputStream, FilterInputStream,
    FloatingPointValidator, Frame, GenericValidator, InfoBar, InfoBarMessageType, IntegerValidator,
    NativeWindow, PopupWindowEvent, PopupWindowEventKind, Rect, RibbonBar, ScrollLineEvent,
    StaticText, StatusBar, Timer, TimerEvent, UiScrollAxis, Validator, WxInputStream, WxModule,
    WxObject,
    ZlibInputStream,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — round 26")
        .with_size(480, 280)
        .with_modern_style().build();
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Round 26: validators + I/O + events.", 0);
    let _hint = StaticText::new(&frame, "Modules / streams / validators:");
    let _disp = DisplayChangedEvent::new(0);
    let _scroll = ScrollLineEvent::new(UiScrollAxis::Vertical, 1);
    let _popup = PopupWindowEvent::new(PopupWindowEventKind::Show);
    let _obj = WxObject::new();
    let mut module = WxModule::new("demo");
    module.initialize();
    let _timer_ev = TimerEvent::new(1);
    let timer = Timer::new(&frame);
    timer.on_timer_event(|ev| {
        let _ = ev.timer_id;
    });
    let mut bar = InfoBar::new(&frame);
    bar.show_message("Hello", InfoBarMessageType::Info);
    bar.on_info_bar_event(&frame, |_| {});
    let ribbon = RibbonBar::new(&frame);
    ribbon.on_ribbon_event(&frame, |_| {});
    let _native = NativeWindow::from_handle(std::ptr::null_mut(), Rect::new(0, 0, 10, 10));
    let mut buf_in = BufferedInputStream::new(b"ru_wx".to_vec(), 4);
    let mut read_buf = [0u8; 5];
    let _ = buf_in.read(&mut read_buf);
    let mut filt = FilterInputStream::new(b"a\r\n".to_vec()).with_strip_cr(true);
    let _ = filt.read(&mut read_buf);
    let mut zlib = ZlibInputStream::from_decompressed(b"zlib".to_vec());
    let _ = zlib.read(&mut read_buf);
    let _gen = GenericValidator;
    let _int = IntegerValidator;
    let _float = FloatingPointValidator;
    assert!(_int.validate("42"));
    assert!(_float.validate("3.14"));
    let mut sizer = BoxSizer::vertical();
    sizer.add(_hint.as_widget_ref());
    frame.set_sizer(sizer);
    let _ = module.is_initialized();
    let _ = FileInputStream::open(std::env::temp_dir().join("ru_wx_round26_missing.tmp"));
    app.run(frame);
}
