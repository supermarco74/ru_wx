//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
#![windows_subsystem = "windows"]

use ru_wx::{
    App, BoxSizer, CollapsiblePane, ContextMenuEvent, DropFilesEvent, Frame, FullScreenEvent,
    GaugeEvent, JoystickEvent, RearrangeList, SetCursorEvent, SliderEvent, SpinEvent,
    StaticText, StatusBar, SysColourChangedEvent, TaskBarIconEvent, TaskBarIconEventKind,
    UiScrollAxis, UiScrollEvent,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — round 20 events")
        .with_size(520, 280)
        .with_modern_style().build();
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Right-click or reorder the list.", 0);
    let _hint = StaticText::new(&frame, "Context menu / rearrange / events:");
    let s = status.clone();
    frame.on_context_menu(move |ev: &ContextMenuEvent| {
        s.set_status_text(
            &format!("Context menu {}, {}", ev.position.x, ev.position.y),
            0,
        );
    });
    let list = RearrangeList::new(&frame, &frame, &["Alpha", "Beta", "Gamma"]);
    let pane_body = StaticText::new(&frame, "Collapsible body");
    let pane = CollapsiblePane::new(&frame, "Section", pane_body.as_widget_ref());
    pane.bind_toggle_with_event(&frame, |_| {});
    let _sys = SysColourChangedEvent::new();
    let _joy = JoystickEvent::button_press(1, ru_wx::Point::new(0, 0));
    let _cursor = SetCursorEvent::new(1, ru_wx::Point::new(0, 0));
    let _full = FullScreenEvent::entered();
    let _spin = SpinEvent::new(5, 1);
    let _slider = SliderEvent::new(50, false);
    let _gauge = GaugeEvent::new(75);
    let _scroll = UiScrollEvent::new(UiScrollAxis::Vertical, 0, 0);
    let _drop = DropFilesEvent::new(vec![], ru_wx::Point::new(0, 0));
    let _tray = TaskBarIconEvent::new(TaskBarIconEventKind::LeftClick);
    let mut sizer = BoxSizer::vertical();
    sizer.add(list.list_widget());
    sizer.add(list.up_widget());
    sizer.add(list.down_widget());
    sizer.add(pane.toggle_widget());
    frame.set_sizer(sizer);
    app.run(frame);
}
