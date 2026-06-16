//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
#![windows_subsystem = "windows"]

use ru_wx::{
    App, BitmapToggleButton, BoxSizer, CollapsibleHeaderCtrl, ComboCtrl, DragImage, Frame,
    GenericDirCtrl, LayerWindow, MenuButton, PopupCtrl, PropertyGrid, PropertyGridManager,
    PropertyValue, Rect, RibbonArtProvider, RibbonBar, RibbonButtonBar, RibbonGallery,
    RibbonPage, RibbonPanel, StaticText, StatusBar, VListBox,
};
use ru_wx::Bitmap;

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — round 34")
        .with_size(520, 320)
        .build();
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Round 34: ribbon + controls + drag image.", 0);
    let _hint = StaticText::new(&frame, "RibbonPage / VListBox / GenericDirCtrl:");
    let ribbon = RibbonBar::new(&frame);
    let mut page = RibbonPage::new("Home");
    let mut panel = RibbonPanel::new("Clipboard");
    let mut bar = RibbonButtonBar::new();
    bar.add_button(101, "Paste");
    panel.add_bar(bar);
    page.add_panel(panel);
    let _ = ribbon.add_page(page);
    let _art = RibbonArtProvider::new();
    let mut gallery = RibbonGallery::new();
    gallery.append("Style A");
    let _combo = ComboCtrl::new(&frame);
    let _popup = PopupCtrl::new(&frame);
    let _dir = GenericDirCtrl::new(&frame);
    let vlist = VListBox::new(&frame);
    vlist.set_line_count(2);
    vlist.set_line(0, "Line 0");
    let _header = CollapsibleHeaderCtrl::new(&frame, "Section");
    let menu_btn = MenuButton::new(&frame, "Menu");
    menu_btn.menu_mut().append("Item", &frame, || {});
    menu_btn.bind_popup(&frame);
    let bmp = Bitmap::new(16, 16);
    let _toggle = BitmapToggleButton::new(&frame, &bmp, 16, 16);
    let mgr = PropertyGridManager::new(&frame);
    let mut grid = PropertyGrid::new(&frame);
    grid.append("Name", PropertyValue::String("ru_wx".into()));
    mgr.add_page("General", grid);
    let _layer = LayerWindow::new(&frame, Rect::new(0, 0, 80, 40));
    let mut drag = DragImage::new(bmp);
    drag.show(10, 10);
    let _ = ribbon.page_count();
    let mut sizer = BoxSizer::vertical();
    sizer.add(_hint.as_widget_ref());
    frame.set_sizer(sizer);
    app.run(frame);
}
