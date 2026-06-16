//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
#![windows_subsystem = "windows"]

use ru_wx::{
    AddRemoveCtrl, App, BoxSizer, CalendarDateAttr, CharHookEvent, ColourDatabase, Date, Frame,
    GridEvent, ItemActivateEvent, ListEvent, ListEventKind, MouseState, RichToolTip, StaticText,
    StatusBar, ThreadEvent, TreeEvent, TreeEventKind, Uri, WindowCreateEvent, WindowDestroyEvent,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — round 21")
        .with_size(520, 300)
        .build();
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Type a key or use Add/Remove.", 0);
    let hint = StaticText::new(&frame, "Char hook / AddRemove / RichToolTip:");
    let s = status.clone();
    frame.on_char_hook(move |ev: &CharHookEvent| {
        if ev.unicode == b'q' as u32 {
            ev.veto();
            s.set_status_text("Char hook vetoed 'q'", 0);
        }
    });
    let mut tip = RichToolTip::new("Hint");
    tip.set_message("Add items with the text field.");
    tip.show_for(&hint.as_widget_ref());
    let add_remove = AddRemoveCtrl::new(&frame, &frame);
    let db = ColourDatabase::new();
    assert!(db.find("red").is_some());
    let uri = Uri::parse("https://example.com/path").expect("uri");
    assert_eq!(uri.host, "example.com");
    let _tree = TreeEvent::new(TreeEventKind::SelectionChanged, 1);
    let _list = ListEvent::new(ListEventKind::ItemSelected, 0, 0);
    let _grid = GridEvent::new(1, 2, true);
    let _act = ItemActivateEvent::new(0);
    let _create = WindowCreateEvent::new(1);
    let _destroy = WindowDestroyEvent::new(1);
    let _mouse = MouseState::new(ru_wx::Point::new(0, 0));
    let _thread = ThreadEvent::new(1, "done");
    let _attr = CalendarDateAttr::new(Date::new(2026, 6, 10), db.find("yellow").unwrap(), db.find("black").unwrap());
    let mut sizer = BoxSizer::vertical();
    sizer.add(hint.as_widget_ref());
    sizer.add(add_remove.entry_widget());
    sizer.add(add_remove.add_widget());
    sizer.add(add_remove.remove_widget());
    sizer.add(add_remove.list_widget());
    frame.set_sizer(sizer);
    app.run(frame);
}
