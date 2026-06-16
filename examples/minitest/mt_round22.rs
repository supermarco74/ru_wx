//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
#![windows_subsystem = "windows"]

use ru_wx::{
    App, BoxSizer, Colour, ColourPickerEvent, DatePickerEvent, DirPickerEvent, FileCtrlEvent,
    FilePickerEvent, FontPickerEvent, Frame, HyperlinkCtrl, HyperlinkEvent, ItemContainer,
    ListBox, MemoryOutputStream, NotebookEvent, SecretStore, SizerEvent, StaticText, StatusBar,
    TempFile, WxOutputStream,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — round 22")
        .with_size(520, 280)
        .build();
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Click the hyperlink.", 0);
    let _hint = StaticText::new(&frame, "Picker events / streams / ItemContainer:");
    let link = HyperlinkCtrl::new(&frame, "Docs", "https://example.com");
    let s = status.clone();
    link.on_link(&frame, move |ev: &HyperlinkEvent| {
        s.set_status_text(&ev.url, 0);
    });
    let list = ListBox::new(&frame);
    list.append("one");
    assert_eq!(ItemContainer::count(&list), 1);
    let _date = DatePickerEvent::new(None);
    let _colour = ColourPickerEvent::new(Colour::new(0, 0, 255, 255));
    let _file = FilePickerEvent::new("C:\\temp\\a.txt");
    let _dir = DirPickerEvent::new("C:\\temp");
    let _font = FontPickerEvent::new("Segoe UI", 10);
    let _fc = FileCtrlEvent::new("demo.txt");
    let _sizer = SizerEvent::new(ru_wx::Size::new(100, 100));
    let _nb = NotebookEvent::new(0);
    let mut mem = MemoryOutputStream::new();
    mem.write(b"ru_wx").expect("write");
    let mut secrets = SecretStore::new();
    secrets.save("app", "user", "token");
    let _ = TempFile::new("ru_wx_test");
    let mut sizer = BoxSizer::vertical();
    sizer.add(link.as_widget_ref());
    sizer.add(list.as_widget_ref());
    frame.set_sizer(sizer);
    app.run(frame);
}
