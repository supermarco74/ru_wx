//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
#![windows_subsystem = "windows"]

use ru_wx::{
    App, AutoBufferedPaintDC, BoxSizer, CheckBox, CheckBoxEvent, ComboBoxEvent, FileTypeInfo,
    FontEnumerator, Frame, HeaderCtrl, MemoryInputStream, MimeTypesManager, MouseWheelEvent,
    Palette, PaletteChangedEvent, QueryNewPaletteEvent, Rect, StaticText, StatusBar, TextEvent,
    TimerEvent, WxInputStream,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — round 23")
        .with_size(480, 260)
        .build();
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Scroll the mouse wheel.", 0);
    let _hint = StaticText::new(&frame, "DC / MIME / control notify events:");
    let s = status.clone();
    frame.on_mouse_wheel(move |ev: &MouseWheelEvent| {
        s.set_status_text(&format!("Wheel {}", ev.delta), 0);
    });
    let mut header = HeaderCtrl::new(Rect::new(0, 0, 400, 24));
    header.append_column("Name", 200);
    let mime = MimeTypesManager::new();
    let info = mime
        .get_type_from_extension("png")
        .unwrap_or(FileTypeInfo::new("png", "image/png", "PNG"));
    let _ = info;
    let mut fonts = FontEnumerator::new();
    fonts.enumerate();
    let _palette = Palette::new(&[ru_wx::Colour::new(0, 0, 0, 255)]);
    let _buf = AutoBufferedPaintDC::new(100, 100);
    let mut input = MemoryInputStream::from_str("ru_wx");
    let mut buf = [0u8; 4];
    let _ = input.read(&mut buf);
    let _text = TextEvent::new("hello");
    let _combo = ComboBoxEvent::new(0);
    let _check = CheckBoxEvent::new(true);
    let _timer = TimerEvent::new(1);
    let _pal = PaletteChangedEvent::new();
    let _query = QueryNewPaletteEvent::new();
    let cb = CheckBox::new(&frame, "Option");
    let mut sizer = BoxSizer::vertical();
    sizer.add(cb.as_widget_ref());
    frame.set_sizer(sizer);
    app.run(frame);
}
