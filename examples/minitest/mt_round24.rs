//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
#![windows_subsystem = "windows"]

use ru_wx::{
    adv::media_ctrl::MediaState, AnimationCtrlEvent, App, BoxSizer, Choice, ChoiceEvent,
    EvtLoopActivator, EventLoop, Frame, HeaderButtonClickEvent, HeaderCtrl, ItemContainer,
    ItemContainerImmutable, ListBoxEvent, MediaCtrlEvent, MemoryFSHandler, ProgressEvent,
    PropertyGridEvent, RadioBoxEvent, Rect, SearchCtrlEvent, StaticText, StatusBar, SVGBitmap,
    ToggleButtonEvent, WizardEvent, WizardEventKind,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — round 24")
        .with_size(480, 280)
        .build();
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Round 24: adv events + VFS + SVG.", 0);
    let _hint = StaticText::new(&frame, "Events / handlers / utilities:");
    let s = status.clone();
    let mut header = HeaderCtrl::new(Rect::new(0, 0, 400, 24));
    header.append_column("Name", 200);
    header.on_button_click(move |ev: &HeaderButtonClickEvent| {
        s.set_status_text(&format!("Header col {}", ev.column), 0);
    });
    header.click_column(0);
    let vfs = MemoryFSHandler::new();
    vfs.add_text("demo.txt", "hello memory fs");
    let _ = vfs.get_file("demo.txt");
    let mut loop_aux = EventLoop::new();
    let _activator = EvtLoopActivator::new(&mut loop_aux);
    let choice = Choice::new(&frame);
    choice.append("One");
    choice.append("Two");
    assert_eq!(ItemContainer::count(&choice), 2);
    assert_eq!(ItemContainerImmutable::get_string(&choice, 0).as_deref(), Some("One"));
    let _pg = PropertyGridEvent::new(0, true);
    let _wiz = WizardEvent::new(WizardEventKind::PageChanged, 1);
    let _prog = ProgressEvent::new(3, 10);
    let _media = MediaCtrlEvent::new(MediaState::Stopped);
    let _anim = AnimationCtrlEvent::new(0, false);
    let _radio = RadioBoxEvent::new(1);
    let _toggle = ToggleButtonEvent::new(true);
    let _list = ListBoxEvent::new(0);
    let _ch = ChoiceEvent::new(0);
    let _search = SearchCtrlEvent::new("query");
    let mut svg = SVGBitmap::new(32, 32);
    let _ = svg.load_from_bytes(
        br#"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><circle cx="16" cy="16" r="12" fill="blue"/></svg>"#,
    );
    let mut sizer = BoxSizer::vertical();
    sizer.add(choice.as_widget_ref());
    frame.set_sizer(sizer);
    app.run(frame);
}

