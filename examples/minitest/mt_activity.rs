//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
#![windows_subsystem = "windows"]

use ru_wx::{
    ActivityIndicator, App, BoxSizer, Button, FileCtrl, FileHistory, Frame, StaticText, StatusBar,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — ActivityIndicator / FileCtrl")
        .with_size(480, 220)
        .with_modern_style().build();
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Activity indicator running.", 0);
    let _h = StaticText::new(&frame, "wxActivityIndicator + wxFileCtrl + wxFileHistory:");
    let spinner = ActivityIndicator::new(&frame);
    spinner.start();
    let file = FileCtrl::new(&frame);
    file.set_filename("C:\\temp\\demo.txt");
    let mut history = FileHistory::new(5);
    history.add_file("C:\\temp\\demo.txt");
    let stop = Button::new(&frame, "Stop spinner");
    let s = spinner.clone();
    stop.on_click(&frame, move || {
        s.stop();
    });
    let mut sizer = BoxSizer::vertical();
    sizer.add(spinner.as_widget_ref());
    sizer.add(file.as_widget_ref());
    sizer.add(stop.as_widget_ref());
    frame.set_sizer(sizer);
    let _ = history;
    app.run(frame);
}
