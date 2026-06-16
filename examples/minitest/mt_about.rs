//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
#![windows_subsystem = "windows"]

use ru_wx::{
    AboutDialog, App, BannerWindow, BoxSizer, Button, Frame, InfoBar, InfoBarMessageType,
    StaticText, StatusBar,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — About / InfoBar / Banner")
        .with_size(520, 280)
        .build();
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Click About or dismiss the banner.", 0);
    let _h = StaticText::new(&frame, "wxAboutDialog, wxInfoBar, wxBannerWindow:");
    let mut info = InfoBar::new(&frame);
    info.show_message("Update available.", InfoBarMessageType::Info);
    info.bind_dismiss(&frame);
    let banner = BannerWindow::new(&frame, "Welcome to ru_wx");
    banner.bind_close(&frame);
    let about_btn = Button::new(&frame, "About…");
    let s = status.clone();
    let f = frame.clone();
    about_btn.on_click(&frame, move || {
        let dlg = AboutDialog::new("ru_wx")
            .with_version("0.6.4")
            .with_description("Native GUI for Rust.")
            .with_copyright("© 2026");
        dlg.show_modal(&f);
        s.set_status_text("About closed.", 0);
    });
    let mut sizer = BoxSizer::vertical();
    sizer.add(info.label_widget());
    sizer.add(banner.message_widget());
    sizer.add(about_btn.as_widget_ref());
    frame.set_sizer(sizer);
    app.run(frame);
}
