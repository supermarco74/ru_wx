//! Advanced UI workbench — Ribbon, AuiManager, images, Grid, WebView,
//! AnimationCtrl, PropertyGrid extras, dialogs and more in one window.
//!
//! Exercises composite / advanced `ru_wx` modules exported from `lib.rs`
//! on the Win32 backend.
//!
//! ```bash
//! cargo run --example advanced_ui_demo
//! ```

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!(
        "advanced_ui_demo requires Windows (Win32).\n\
         For cross-platform stubs see stub_app_demo or cross_platform_stubs."
    );
}

#[cfg(target_os = "windows")]
mod workbench {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use ru_wx::{
        Animation, AnimationCtrl, App, AuiToolBar, BitmapButton,
        BitmapBundle, BoxSizer, Button, Cell as GridCell, Colour, ColourDialog, ColourProperty,
        DataViewCtrl, FileDialog, FileDialogStyle, FontDesc, FontProperty, Frame, Gauge, Grid,
        HtmlWindow, HyperlinkCtrl, ImageList, InMemoryDataViewModel, Menu, MenuBar, MessageBoxIcon,
        MessageDialog, MessageDialogStyle, Panel, PropertyCategory, PropertyColumnSplitter,
        PropertyGrid, PropertyGridExtras, PropertyGridIterator, PropertyHelpStrip,
        PropertyValue, RibbonArtProvider, RibbonBar, RibbonBarEventKind, RibbonButtonBar,
        RibbonGallery, RibbonPage, RibbonPanel, RichTextCtrl, ScrolledWindow, SizeEvent, SizeType,
        Slider, StaticBitmap, StaticText, StatusBar, Tab, WebView, WebViewEventKind,
    };
    use ru_wx::DataViewModel;
    use ru_wx::Widget;

    const RIBBON_BAND: u32 = 96;
    const LEFT_W: u32 = 260;
    const RIGHT_W: u32 = 280;
    const BOTTOM_BAR_H: u32 = 40;
    /// Matches [`ru_wx::StatusBar`] internal height so panes do not overlap it.
    const STATUS_H: u32 = 22;

    const ID_PASTE: u16 = 4010;
    const ID_CUT: u16 = 4011;
    const ID_COPY: u16 = 4012;
    const ID_BOLD: u16 = 4020;
    const ID_ITALIC: u16 = 4021;
    const ID_INSERT_ROW: u16 = 4030;
    const ID_PREVIEW: u16 = 4040;

    const ID_GALLERY_PREV: u16 = 4050;
    const ID_GALLERY_NEXT: u16 = 4051;
    const ID_ZOOM_IN: u16 = 4052;
    const ID_ZOOM_OUT: u16 = 4053;
    const ID_PICK_COLOUR: u16 = 4054;
    const ID_OPEN_FILE: u16 = 4055;

    const ID_QNEW: u16 = 4101;
    const ID_QSAVE: u16 = 4102;
    const ID_QHELP: u16 = 4103;

    const ICON_NEW: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#4F46E5"/><path d="M12 7v10M7 12h10" stroke="white" stroke-width="2" stroke-linecap="round"/></svg>"##;
    const ICON_SAVE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#10B981"/><path d="M6 4h10l3 3v13H5z" fill="none" stroke="white" stroke-width="1.6"/></svg>"##;
    const ICON_HELP: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#0EA5E9"/><text x="12" y="17" text-anchor="middle" fill="white" font-size="14" font-family="sans-serif">?</text></svg>"##;
    const ICON_PLAY: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="10" fill="#22C55E"/><path d="M10 8l6 4-6 4z" fill="white"/></svg>"##;
    const ICON_STOP: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="10" fill="#EF4444"/><rect x="9" y="9" width="6" height="6" fill="white"/></svg>"##;

    const ASSET_SVGS: &[(&str, &[u8])] = &[
        ("Star", include_bytes!("../assets/icons/star.svg")),
        ("Heart", include_bytes!("../assets/icons/heart-fill.svg")),
        ("Trophy", include_bytes!("../assets/icons/trophy-fill.svg")),
        ("Gem", include_bytes!("../assets/icons/gem.svg")),
        ("Cloud", include_bytes!("../assets/icons/cloud-fill.svg")),
        ("Book", include_bytes!("../assets/icons/book-fill.svg")),
        ("Cart", include_bytes!("../assets/icons/cart-fill.svg")),
        ("Bolt", include_bytes!("../assets/icons/lightning-charge-fill.svg")),
    ];

    const GIF_SIDE: u32 = 64;
    const GIF_FRAMES: u32 = 10;

    struct LayoutCtx {
        ribbon: RibbonBar,
        aui_toolbar_hwnd: isize,
    }

    fn layout_chrome(_frame: &Frame, ctx: &LayoutCtx, ev: &SizeEvent) {
        let w = ev.size.width.max(0) as u32;
        let h = ev.size.height.max(0) as u32;

        ctx.ribbon.layout(0, 0, w, RIBBON_BAND);

        let bottom_y = h.saturating_sub(STATUS_H + BOTTOM_BAR_H);
        #[cfg(target_os = "windows")]
        // SAFETY: HWND registered by `AuiToolBar::realize`.
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::MoveWindow;
            MoveWindow(
                ctx.aui_toolbar_hwnd as _,
                0,
                bottom_y as i32,
                w as i32,
                BOTTOM_BAR_H as i32,
                1,
            );
        }
    }

    #[cfg(target_os = "windows")]
    fn frame_client_size(frame: &Frame) -> ru_wx::Size {
        use windows_sys::Win32::Foundation::RECT;
        use windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect;
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // SAFETY: live frame HWND from the running demo.
        unsafe {
            GetClientRect(frame.hwnd(), &mut rect);
        }
        ru_wx::Size::new(
            (rect.right - rect.left).max(0),
            (rect.bottom - rect.top).max(0),
        )
    }

    fn build_gif_bytes() -> Vec<u8> {
        use image::codecs::gif::{GifEncoder, Repeat};
        use image::{Delay, Frame, Rgba, RgbaImage};

        let palette: [(u8, u8, u8); GIF_FRAMES as usize] = [
            (230, 70, 60),
            (240, 130, 40),
            (245, 190, 40),
            (170, 210, 60),
            (90, 190, 90),
            (60, 200, 170),
            (60, 170, 220),
            (70, 120, 230),
            (120, 90, 230),
            (180, 80, 220),
        ];

        let centre = GIF_SIDE as f32 / 2.0;
        let orbit = GIF_SIDE as f32 * 0.30;
        let radius = GIF_SIDE as f32 * 0.13;

        let mut bytes = Vec::new();
        {
            let mut encoder = GifEncoder::new(&mut bytes);
            let _ = encoder.set_repeat(Repeat::Infinite);
            for i in 0..GIF_FRAMES {
                let angle = i as f32 / GIF_FRAMES as f32 * std::f32::consts::TAU;
                let bx = centre + orbit * angle.cos();
                let by = centre + orbit * angle.sin();
                let (br, bg, bb) = palette[i as usize];
                let img = RgbaImage::from_fn(GIF_SIDE, GIF_SIDE, |x, y| {
                    let dx = x as f32 - bx;
                    let dy = y as f32 - by;
                    if dx * dx + dy * dy <= radius * radius {
                        Rgba([br, bg, bb, 255])
                    } else {
                        let cx = x as f32 - centre;
                        let cy = y as f32 - centre;
                        let d = (cx * cx + cy * cy).sqrt() / centre;
                        let shade = (40.0 + 50.0 * (1.0 - d.min(1.0))) as u8;
                        Rgba([shade / 2, shade / 2, shade, 255])
                    }
                });
                let frame = Frame::from_parts(img, 0, 0, Delay::from_numer_denom_ms(80, 1));
                let _ = encoder.encode_frame(frame);
            }
        }
        bytes
    }

    fn library_checklist() -> &'static str {
        "RibbonBar · RibbonGallery · AuiToolBar · PropertyGrid · PropertyGridExtras · \
         PropertyGridIterator · DataViewCtrl · Grid · RichTextCtrl · HtmlWindow · WebView · \
         Tab · StaticBitmap · AnimationCtrl · Gauge · Slider · ColourDialog · FileDialog"
    }

    pub fn run() {
        let app = App::new();
        let frame = Frame::builder()
            .with_title("ru_wx — Advanced UI workbench (images + Grid + WebView + dialogs)")
            .with_size(1280, 820)
            .build();

        let status = StatusBar::new(&frame, 2);
        status.set_status_text("Ready — Ribbon, assets gallery, Grid, WebView", 0);
        status.set_status_text(library_checklist(), 1);

        let zoom_pct = Rc::new(Cell::new(100i32));
        let gallery = Rc::new(RefCell::new({
            let mut g = RibbonGallery::new();
            g.append("Flat");
            g.append("Gradient");
            g.append("Glass");
            g.append("Neon");
            g.append("Classic");
            g
        }));
        let _ribbon_art = RibbonArtProvider::new();

        // ── Ribbon ───────────────────────────────────────────────────
        let ribbon = RibbonBar::new(&frame);

        let mut home = RibbonPage::new("Home");
        let mut clipboard = RibbonPanel::new("Clipboard");
        let mut clip_bar = RibbonButtonBar::new();
        clip_bar.add_button(ID_PASTE, "Paste");
        clip_bar.add_button(ID_CUT, "Cut");
        clip_bar.add_button(ID_COPY, "Copy");
        clipboard.add_bar(clip_bar);
        home.add_panel(clipboard);

        let mut font_panel = RibbonPanel::new("Font");
        let mut font_bar = RibbonButtonBar::new();
        font_bar.add_button(ID_BOLD, "Bold");
        font_bar.add_button(ID_ITALIC, "Italic");
        font_panel.add_bar(font_bar);
        home.add_panel(font_panel);
        ribbon.add_page(home);

        let mut insert = RibbonPage::new("Insert");
        let mut data_panel = RibbonPanel::new("Data");
        let mut data_bar = RibbonButtonBar::new();
        data_bar.add_button(ID_INSERT_ROW, "Add row");
        data_bar.add_button(ID_PREVIEW, "Preview HTML");
        data_panel.add_bar(data_bar);
        insert.add_panel(data_panel);
        ribbon.add_page(insert);

        let mut view = RibbonPage::new("View");
        let mut styles_panel = RibbonPanel::new("Gallery");
        let mut styles_bar = RibbonButtonBar::new();
        styles_bar.add_button(ID_GALLERY_PREV, "Prev style");
        styles_bar.add_button(ID_GALLERY_NEXT, "Next style");
        styles_panel.add_bar(styles_bar);
        view.add_panel(styles_panel);

        let mut zoom_panel = RibbonPanel::new("Zoom & I/O");
        let mut zoom_bar = RibbonButtonBar::new();
        zoom_bar.add_button(ID_ZOOM_IN, "Zoom +");
        zoom_bar.add_button(ID_ZOOM_OUT, "Zoom −");
        zoom_bar.add_button(ID_PICK_COLOUR, "Colour…");
        zoom_bar.add_button(ID_OPEN_FILE, "Open…");
        zoom_panel.add_bar(zoom_bar);
        view.add_panel(zoom_panel);
        ribbon.add_page(view);
        ribbon.realize();

        // ── Bottom AuiToolBar ────────────────────────────────────────
        let aui_toolbar = AuiToolBar::new(&frame);
        aui_toolbar.set_toolbar_height(36);
        let sizes = [(32, 32), (36, 36)];
        let images = ImageList::new(36, 36);
        for (i, bytes) in [ICON_NEW, ICON_SAVE, ICON_HELP].iter().enumerate() {
            if let Some(bmp) = BitmapBundle::from_svg_bytes(bytes, &sizes).best_for_size((36, 36)) {
                images.add_bitmap(bmp.hbitmap);
            }
            let id = [ID_QNEW, ID_QSAVE, ID_QHELP][i];
            let label = ["New", "Save", "Help"][i];
            aui_toolbar.add_tool(id, label, i as i32);
        }
        aui_toolbar.set_image_list(&images);
        aui_toolbar.realize();
        let aui_toolbar_hwnd = aui_toolbar.hwnd() as isize;

        // ── PropertyGrid + manager + extras ──────────────────────────
        let mut prop_grid = PropertyGrid::new(&frame);
        prop_grid.append_category(&PropertyCategory::new("Document"));
        prop_grid.append("Title", PropertyValue::String("Advanced UI demo".into()));
        prop_grid.append("Author", PropertyValue::String("ru_wx".into()));
        prop_grid.append_colour(
            "Accent",
            &ColourProperty::new(Colour::new(79, 70, 229, 255)),
        );
        prop_grid.append_font(
            "Body font",
            &FontProperty::new(FontDesc::new("Segoe UI", 11)),
        );
        prop_grid.append("Zoom", PropertyValue::Int(100));
        prop_grid.append("ReadOnly", PropertyValue::Bool(false));
        prop_grid.append(
            "Theme",
            PropertyValue::Choice {
                options: vec!["Light".into(), "Dark".into(), "System".into()],
                selected: 0,
            },
        );
        prop_grid.set_column_split(PropertyColumnSplitter::new(120));
        let mut help = PropertyHelpStrip::default();
        help.set_text("Edit properties — Zoom drives the Assets slider and gauge.");
        prop_grid.set_help_strip(&help);

        // ── Center tabs ──────────────────────────────────────────────
        let center_tab = Tab::new(&frame);

        // Document
        let doc_panel = Panel::new(&frame);
        let rich = RichTextCtrl::new(&doc_panel);
        rich.set_value(
            "Welcome to the ru_wx Advanced UI workbench.\n\n\
             • Home → Bold/Italic formats the selection\n\
             • View → Gallery cycles RibbonGallery styles\n\
             • Assets tab — SVG icons + GIF AnimationCtrl\n\
             • Grid tab — function-based cells with icons\n\
             • WebView tab — loads ru_wx.dev\n\
             • View → Colour / Open invoke standard dialogs",
        );
        let mut doc_sizer = BoxSizer::vertical();
        doc_sizer.set_padding(8);
        doc_sizer.add_with_proportion(rich.as_widget_ref(), 1);
        doc_panel.set_sizer(doc_sizer);
        center_tab.add_page("Document", &doc_panel);

        // HTML preview
        let preview_panel = Panel::new(&frame);
        let html = Rc::new(RefCell::new(HtmlWindow::new(&preview_panel)));
        html.borrow_mut().set_page(
            "<h3>ru_wx Advanced UI</h3>\
             <p>Ribbon + AUI + PropertyGrid + DataView + images</p>\
             <p>Insert → Preview HTML refreshes this pane.</p>",
        );
        let preview_label = StaticText::new(&preview_panel, "HTML preview (HtmlWindow):");
        let link = HyperlinkCtrl::new(&preview_panel, "Project site", "https://github.com/");
        let mut preview_sizer = BoxSizer::vertical();
        preview_sizer.set_padding(8);
        preview_sizer.add(preview_label.as_widget_ref());
        preview_sizer.add(link.as_widget_ref());
        preview_sizer.add_with_proportion(html.borrow().as_widget_ref(), 1);
        preview_panel.set_sizer(preview_sizer);
        center_tab.add_page("Preview", &preview_panel);

        // Assets — ScrolledWindow + StaticBitmap gallery + AnimationCtrl
        let assets_panel = Panel::new(&frame);
        let assets_title = StaticText::new(&assets_panel, "SVG asset gallery (compile-time icons):");
        let scrolled = ScrolledWindow::new(&assets_panel);

        let icon_size = (48u32, 48u32);
        let bundle_sizes = [(32, 32), (48, 48), (64, 64)];
        let mut row_y = 8i32;
        let mut col_x = 8i32;
        for (i, (_label, bytes)) in ASSET_SVGS.iter().enumerate() {
            let bundle = BitmapBundle::from_svg_bytes(bytes, &bundle_sizes);
            let bmp = StaticBitmap::new(&scrolled, &bundle, icon_size);
            Widget::set_position(&mut *bmp.as_widget_ref().borrow_mut(), col_x, row_y);
            col_x += icon_size.0 as i32 + 12;
            if (i + 1) % 4 == 0 {
                col_x = 8;
                row_y += icon_size.1 as i32 + 28;
            }
        }
        let virtual_h = row_y + icon_size.1 as i32 + 40;
        scrolled.set_virtual_size(520, virtual_h.max(260));

        let mut anim = Animation::new();
        let gif = build_gif_bytes();
        let anim_ok = anim.load_from_memory(&gif).is_ok();
        let anim_ctrl = AnimationCtrl::with_size(&assets_panel, GIF_SIDE, GIF_SIDE);
        if anim_ok {
            anim_ctrl.set_animation(anim.clone());
        }
        let play_btn = BitmapButton::new_from_svg_bytes(&assets_panel, ICON_PLAY, 36, 36);
        let stop_btn = BitmapButton::new_from_svg_bytes(&assets_panel, ICON_STOP, 36, 36);

        let zoom_slider = Slider::new(&assets_panel, 50, 200, zoom_pct.get());
        let load_gauge = Gauge::new(&assets_panel, 100);
        load_gauge.set_value(zoom_pct.get());

        let anim_hint_label = if anim_ok {
            format!("Runtime GIF: {} frames", anim.frame_count())
        } else {
            "GIF encode failed — AnimationCtrl empty".to_string()
        };
        let anim_hint = StaticText::new(&assets_panel, &anim_hint_label);

        let mut assets_row_anim = BoxSizer::horizontal();
        assets_row_anim.add(anim_ctrl.as_widget_ref());
        assets_row_anim.add_spacer(8);
        assets_row_anim.add(play_btn.as_widget_ref());
        assets_row_anim.add(stop_btn.as_widget_ref());
        assets_row_anim.add_spacer(16);
        assets_row_anim.add(anim_hint.as_widget_ref());

        let slider_label = StaticText::new(&assets_panel, "Zoom %:");
        let mut assets_row_zoom = BoxSizer::horizontal();
        assets_row_zoom.add(slider_label.as_widget_ref());
        assets_row_zoom.add_spacer(8);
        assets_row_zoom.add(zoom_slider.as_widget_ref());
        assets_row_zoom.add_spacer(16);
        assets_row_zoom.add(StaticText::new(&assets_panel, "Load:").as_widget_ref());
        assets_row_zoom.add(load_gauge.as_widget_ref());

        let mut assets_sizer = BoxSizer::vertical();
        assets_sizer.set_padding(8);
        assets_sizer.add(assets_title.as_widget_ref());
        assets_sizer.add_with_proportion(scrolled.as_widget_ref(), 1);
        assets_sizer.add_sizer(assets_row_anim);
        assets_sizer.add_sizer(assets_row_zoom);
        assets_panel.set_sizer(assets_sizer);
        center_tab.add_page("Assets", &assets_panel);

        // Mini Grid
        let grid_panel = Panel::new(&frame);
        let grid_title = StaticText::new(&grid_panel, "Mini Grid — function cells + ImageList:");
        let grid = Grid::new(&grid_panel);
        let grid_images = ImageList::new(16, 16);
        for (_name, bytes) in ASSET_SVGS.iter().take(5) {
            if let Some(bmp) = BitmapBundle::from_svg_bytes(bytes, &[(16, 16)]).best_for_size((16, 16))
            {
                grid_images.add_bitmap(bmp.hbitmap);
            }
        }
        grid.set_image_list(&grid_images);
        grid.set_checkboxes(true);
        grid.enable_interactive_features(&frame);
        grid.apply_win11_theme(&frame);
        grid.append_column("Module", 140);
        grid.append_column("Icon", 100);
        grid.append_column("Status", 80);
        grid.set_value_provider(|row, col| match col {
            0 => GridCell::Text(format!("ru_wx::{}", ["RibbonBar", "Grid", "WebView", "AnimationCtrl", "HeaderCtrl"][row % 5])),
            1 => GridCell::Image {
                idx: (row % 5) as i32,
                text: format!("icon {row}"),
            },
            _ => GridCell::Badge {
                kind: ru_wx::BadgeKind::Ok,
                text: "exported".into(),
            },
        });
        grid.set_row_count(8);
        let refresh_grid_btn = Button::new(&grid_panel, "Refresh grid");
        let mut grid_sizer = BoxSizer::vertical();
        grid_sizer.set_padding(8);
        grid_sizer.add(grid_title.as_widget_ref());
        grid_sizer.add(refresh_grid_btn.as_widget_ref());
        grid_sizer.add_with_proportion(grid.as_widget_ref(), 1);
        grid_panel.set_sizer(grid_sizer);
        center_tab.add_page("Grid", &grid_panel);

        // WebView
        let web_panel = Panel::new(&frame);
        let web_title = StaticText::new(&web_panel, "WebView (stub backend → HtmlWindow):");
        let web = Rc::new(RefCell::new(WebView::new(&web_panel)));
        web.borrow_mut().load_url("https://example.com/");
        let reload_web = Button::new(&web_panel, "Reload");
        let mut web_sizer = BoxSizer::vertical();
        web_sizer.set_padding(8);
        web_sizer.add(web_title.as_widget_ref());
        web_sizer.add(reload_web.as_widget_ref());
        web_sizer.add_with_proportion(web.borrow().as_widget_ref(), 1);
        web_panel.set_sizer(web_sizer);
        center_tab.add_page("WebView", &web_panel);

        // ── Left column title + right DataView panel ─────────────────
        let left_title = StaticText::new(&frame, "Properties");

        // ── Right: DataView ──────────────────────────────────────────
        let right_panel = Panel::new(&frame);
        let dv_title = StaticText::new(&right_panel, "DataView — project files");

        let data_view = DataViewCtrl::new(&right_panel);
        data_view.append_column("Name", 140);
        data_view.append_column("Kind", 72);
        data_view.append_column("Size", 64);
        let model = Rc::new(RefCell::new(InMemoryDataViewModel {
            rows: vec![
                vec!["main.rs".into(), "Rust".into(), "12 KB".into()],
                vec!["lib.rs".into(), "Rust".into(), "48 KB".into()],
                vec!["advanced_ui_demo.rs".into(), "Example".into(), "18 KB".into()],
                vec!["README.md".into(), "Doc".into(), "4 KB".into()],
                vec!["Cargo.toml".into(), "Config".into(), "2 KB".into()],
            ],
        }));
        data_view.set_model(model.clone());

        let mut right_sizer = BoxSizer::vertical();
        right_sizer.set_padding(6);
        right_sizer.add(dv_title.as_widget_ref());
        right_sizer.add_with_proportion(data_view.as_widget_ref(), 1);
        right_panel.set_sizer(right_sizer);

        // ── Frame sizer (3-column workbench + chrome spacers) ─────────
        prop_grid
            .as_widget_ref()
            .borrow_mut()
            .set_size(LEFT_W, 320);
        right_panel
            .as_widget_ref()
            .borrow_mut()
            .set_size(RIGHT_W, 320);

        let mut left_col = BoxSizer::vertical();
        left_col.set_padding(6);
        left_col.add(left_title.as_widget_ref());
        left_col.add_with_proportion(prop_grid.as_widget_ref(), 1);

        let mut work_row = BoxSizer::horizontal();
        work_row.set_padding(8);
        work_row.add_sizer(left_col);
        work_row.add_with_proportion(center_tab.as_widget_ref(), 1);
        work_row.add(right_panel.as_widget_ref());

        let mut frame_col = BoxSizer::vertical();
        frame_col.set_padding(0);
        frame_col.add_spacer(RIBBON_BAND as i32);
        frame_col.add_sizer_with_proportion(work_row, 1);
        frame_col.add_spacer(BOTTOM_BAR_H as i32);
        frame_col.add_spacer(STATUS_H as i32);

        let layout = Rc::new(RefCell::new(LayoutCtx {
            ribbon: ribbon.clone(),
            aui_toolbar_hwnd,
        }));

        // ── Menus ────────────────────────────────────────────────────
        let mut file_menu = Menu::new("&File");
        file_menu.append("Open…", &frame, {
            let frame = frame.clone();
            let status = status.clone();
            move || {
                let mut dlg = FileDialog::new(&frame, FileDialogStyle::Open);
                dlg.set_title("Open document");
                dlg.set_wildcard("Text files (*.txt)|*.txt|All files (*.*)|*.*");
                if let Some(path) = dlg.show_modal() {
                    status.set_status_text(&format!("Opened: {path}"), 0);
                }
            }
        });
        file_menu.append("Reset layout", &frame, {
            let layout = Rc::clone(&layout);
            let frame = frame.clone();
            let status = status.clone();
            move || {
                let ev = SizeEvent {
                    size: frame_client_size(&frame),
                    size_type: SizeType::ShowNormal,
                };
                layout_chrome(&frame, &layout.borrow(), &ev);
                status.set_status_text("Layout reset", 0);
            }
        });
        file_menu.append_separator();
        file_menu.append("Exit", &frame, {
            let frame = frame.clone();
            move || frame.close()
        });

        let mut view_menu = Menu::new("&View");
        view_menu.append("Pick colour…", &frame, {
            let frame = frame.clone();
            let status = status.clone();
            move || {
                if let Some(c) = ColourDialog::builder(&frame)
                    .with_initial_color(Colour::new(64, 128, 255, 255))
                    .show_modal()
                {
                    status.set_status_text(
                        &format!("Colour picked: #{:02x}{:02x}{:02x}", c.r, c.g, c.b),
                        0,
                    );
                }
            }
        });
        view_menu.append("List properties", &frame, {
            let prop_grid = prop_grid.clone();
            let frame = frame.clone();
            move || {
                let mut lines = Vec::new();
                for (_idx, name, value) in PropertyGridIterator::new(&prop_grid) {
                    lines.push(format!("{name}: {value:?}"));
                }
                MessageDialog::new(
                    &frame,
                    &lines.join("\n"),
                    "PropertyGridIterator",
                    MessageDialogStyle::Ok,
                    MessageBoxIcon::Information,
                )
                .show_modal();
            }
        });
        view_menu.append("Go to Assets tab", &frame, {
            let center_tab = center_tab.clone();
            let status = status.clone();
            move || {
                center_tab.set_selection(2);
                status.set_status_text("View → Assets tab", 0);
            }
        });

        let frame_about = frame.clone();
        let mut help_menu = Menu::new("&Help");
        help_menu.append("Library exports", &frame, {
            let frame = frame.clone();
            move || {
                MessageDialog::new(
                    &frame,
                    library_checklist(),
                    "ru_wx public API used in this demo",
                    MessageDialogStyle::Ok,
                    MessageBoxIcon::Information,
                )
                .show_modal();
            }
        });
        help_menu.append("About", &frame, move || {
            MessageDialog::new(
                &frame_about,
                "Advanced UI workbench\n\nDemonstrates composite widgets,\nimages, Grid, WebView, dialogs.",
                "ru_wx",
                MessageDialogStyle::Ok,
                MessageBoxIcon::Information,
            )
            .show_modal();
        });

        let mut menu_bar = MenuBar::new();
        menu_bar.append(file_menu);
        menu_bar.append(view_menu);
        menu_bar.append(help_menu);
        frame.set_menu_bar(menu_bar);
        frame.set_sizer(frame_col);

        let chrome_ev = SizeEvent {
            size: frame_client_size(&frame),
            size_type: SizeType::ShowNormal,
        };
        layout_chrome(&frame, &layout.borrow(), &chrome_ev);

        frame.on_size_event({
            let layout = Rc::clone(&layout);
            let frame = frame.clone();
            let status = status.clone();
            move |ev| {
                layout_chrome(&frame, &layout.borrow(), ev);
                status.set_status_text(
                    &format!("Client {}×{}", ev.size.width, ev.size.height),
                    1,
                );
            }
        });

        // ── Control events ───────────────────────────────────────────
        link.on_click(&frame, {
            let status = status.clone();
            move || {
                status.set_status_text("HyperlinkCtrl clicked", 0);
            }
        });

        play_btn.on_click(&frame, {
            let status = status.clone();
            let anim = anim_ctrl.clone();
            move || {
                anim.play();
                status.set_status_text("AnimationCtrl: play", 0);
            }
        });
        stop_btn.on_click(&frame, {
            let status = status.clone();
            let anim = anim_ctrl.clone();
            move || {
                anim.stop();
                status.set_status_text("AnimationCtrl: stop", 0);
            }
        });

        refresh_grid_btn.on_click(&frame, {
            let grid = grid.clone();
            let status = status.clone();
            move || {
                grid.refresh();
                status.set_status_text("Grid refreshed", 0);
            }
        });

        reload_web.on_click(&frame, {
            let web = Rc::clone(&web);
            let status = status.clone();
            move || {
                web.borrow_mut().load_url("https://example.com/?t=reload");
                status.set_status_text("WebView reloaded", 0);
            }
        });
        let status_web = status.clone();
        web.borrow().on_event(move |ev| {
            if ev.kind == WebViewEventKind::NavigationComplete {
                status_web.set_status_text(&format!("WebView: {}", ev.url), 0);
            }
        });

        // ── Ribbon events ────────────────────────────────────────────
        let rich_for_ribbon = rich.clone();
        let html_for_ribbon = Rc::clone(&html);
        let data_view_for_ribbon = data_view.clone();
        let status_ribbon = status.clone();
        let frame_ribbon = frame.clone();
        let zoom_for_ribbon = Rc::clone(&zoom_pct);
        let gallery_for_ribbon = Rc::clone(&gallery);
        let gauge_ribbon = load_gauge.clone();
        let slider_ribbon = zoom_slider.clone();
        ribbon.on_ribbon_event(&frame, move |ev| {
            if ev.kind != RibbonBarEventKind::ToolClick {
                return;
            }
            match ev.tool_id {
                ID_BOLD => {
                    rich_for_ribbon.set_bold(true);
                    status_ribbon.set_status_text("Ribbon: Bold on selection", 0);
                }
                ID_ITALIC => {
                    rich_for_ribbon.set_italic(true);
                    status_ribbon.set_status_text("Ribbon: Italic on selection", 0);
                }
                ID_PASTE => status_ribbon.set_status_text("Ribbon: Paste (demo)", 0),
                ID_CUT | ID_COPY => {
                    status_ribbon.set_status_text(&format!("Ribbon: tool {}", ev.tool_id), 0);
                }
                ID_INSERT_ROW => {
                    model.borrow_mut().push_row(vec![
                        "new_item.rs".into(),
                        "Rust".into(),
                        "1 KB".into(),
                    ]);
                    data_view_for_ribbon.refresh();
                    status_ribbon.set_status_text("DataView: row added", 0);
                }
                ID_PREVIEW => {
                    let rows = model.borrow().row_count();
                    let style = gallery_for_ribbon
                        .borrow()
                        .selected_label()
                        .map(str::to_string)
                        .unwrap_or_else(|| "default".to_string());
                    html_for_ribbon.borrow_mut().set_page(&format!(
                        "<h3>Preview refreshed</h3>\
                         <p>Rows: {rows} · Gallery style: {style}</p>"
                    ));
                    status_ribbon.set_status_text("HtmlWindow preview updated", 0);
                }
                ID_GALLERY_PREV => {
                    let mut g = gallery_for_ribbon.borrow_mut();
                    let cur = g.selection().unwrap_or(0);
                    let n = g.items().len().max(1);
                    g.set_selection(if cur == 0 { n - 1 } else { cur - 1 });
                    status_ribbon.set_status_text(
                        &format!("RibbonGallery: {}", g.selected_label().unwrap_or("?")),
                        0,
                    );
                }
                ID_GALLERY_NEXT => {
                    let mut g = gallery_for_ribbon.borrow_mut();
                    let cur = g.selection().unwrap_or(0);
                    let n = g.items().len().max(1);
                    g.set_selection((cur + 1) % n);
                    status_ribbon.set_status_text(
                        &format!("RibbonGallery: {}", g.selected_label().unwrap_or("?")),
                        0,
                    );
                }
                ID_ZOOM_IN | ID_ZOOM_OUT => {
                    let delta = if ev.tool_id == ID_ZOOM_IN { 10 } else { -10 };
                    let v = (zoom_for_ribbon.get() + delta).clamp(50, 200);
                    zoom_for_ribbon.set(v);
                    slider_ribbon.set_value(v);
                    gauge_ribbon.set_value(v);
                    status_ribbon.set_status_text(&format!("Zoom: {v}%"), 0);
                }
                ID_PICK_COLOUR => {
                    if let Some(c) = ColourDialog::builder(&frame_ribbon)
                        .show_modal()
                    {
                        status_ribbon.set_status_text(
                            &format!("Colour: #{:02x}{:02x}{:02x}", c.r, c.g, c.b),
                            0,
                        );
                    }
                }
                ID_OPEN_FILE => {
                    let mut dlg = FileDialog::new(&frame_ribbon, FileDialogStyle::Open);
                    dlg.set_title("Ribbon — open file");
                    if let Some(path) = dlg.show_modal() {
                        status_ribbon.set_status_text(&format!("Open: {path}"), 0);
                    }
                }
                _ => {}
            }
        });

        aui_toolbar.on_tool_clicked(&frame, {
            let status = status.clone();
            move |id| {
                status.set_status_text(&format!("AuiToolBar: tool {id} clicked"), 0);
            }
        });

        let status_props = status.clone();
        let gauge_props = load_gauge.clone();
        let slider_props = zoom_slider.clone();
        let zoom_props = Rc::clone(&zoom_pct);
        let prop_grid_watch = prop_grid.clone();
        prop_grid.on_change(move |idx| {
            status_props.set_status_text(&format!("PropertyGrid: property #{idx} changed"), 0);
            if let Some(PropertyValue::Int(z)) = prop_grid_watch.get_value(idx) {
                let z = z.clamp(50, 200);
                zoom_props.set(z);
                gauge_props.set_value(z);
                slider_props.set_value(z);
            }
        });

        app.run(frame);
    }
}

#[cfg(target_os = "windows")]
fn main() {
    workbench::run();
}
