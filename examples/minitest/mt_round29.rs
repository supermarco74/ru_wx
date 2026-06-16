//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
#![windows_subsystem = "windows"]

use std::rc::Rc;

use ru_wx::{
    App, BlockListFilter, BoxSizer, ClassInfo, EventFilter, FFileOutputStream, Frame, PassThroughFilter,
    Protocol, RefCounter, RegEx, SizerFlags, StaticBox, StaticBoxSizer, StaticText, StatusBar,
    TextBuffer, Translation, ULongLong, Url, Variant, WeakRef, WindowUpdateLocker, WxOutputStream,
    get_translation,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — round 29")
        .with_size(480, 280)
        .build();
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Round 29: variants + sizers + URL.", 0);
    let _hint = StaticText::new(&frame, "RegEx / Variant / StaticBoxSizer:");
    let _regex = RegEx::new("*.rs");
    assert!(_regex.matches("main.rs"));
    let _var = Variant::from(42_i64);
    let _info = ClassInfo::with_base("Frame", "TopLevelWindow");
    let _counter = RefCounter::with_count(1);
    let data = Rc::new(7_i32);
    let _weak = WeakRef::new(&data);
    let mut buf = TextBuffer::from_str("ru_wx");
    buf.append(" round29");
    let mut tr = Translation::new("it");
    tr.add("Hello", "Ciao");
    let _ = get_translation(&tr, "Hello");
    let _url = Url::parse("https://example.com/docs").unwrap();
    let _ = Protocol::from_scheme("https");
    let _ull = ULongLong::new(100) + ULongLong::new(1);
    let mut filter = BlockListFilter::new();
    filter.block(999);
    let _pass = PassThroughFilter;
    let _ = filter.filter_event(1, 0);
    let box_ctrl = StaticBox::new(&frame, "Group");
    let mut box_sizer = StaticBoxSizer::new(box_ctrl.as_widget_ref());
    box_sizer.add(_hint.as_widget_ref());
    let mut sizer = BoxSizer::vertical();
    let flags = SizerFlags::new().proportion(1).border(4).expand();
    sizer.add_with_flags(box_ctrl.as_widget_ref(), flags);
    sizer.add(status.as_widget_ref());
    frame.set_sizer(sizer);
    #[cfg(target_os = "windows")]
    {
        let locker = WindowUpdateLocker::new(&frame);
        locker.unlock();
    }
    let tmp = std::env::temp_dir().join("ru_wx_round29_ffile.tmp");
    let mut out = FFileOutputStream::create(&tmp).unwrap();
    let _ = out.write(b"round29");
    app.run(frame);
}
