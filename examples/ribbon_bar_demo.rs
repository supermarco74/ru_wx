//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! RibbonBar demo — Office-style ribbon with pages, panels, and 3D icon buttons.
//!
//! Run with:
//! ```bash
//! cargo run --example ribbon_bar_demo
//! ```

use image::GenericImageView;
use ru_wx::{
    App, BoxSizer, Frame, Panel, RibbonBar, RibbonBarEvent, RibbonBarEventKind, RibbonButtonBar,
    RibbonImagePosition, RibbonPage, RibbonPanel, StaticText, TextCtrl,
};

/// Load a PNG image from file and return RGBA data.
fn load_icon_rgba(path: &str) -> Option<(u32, u32, Vec<u8>)> {
    let img = image::open(path).ok()?;
    let (w, h) = img.dimensions();
    let rgba = img.to_rgba8().into_raw();
    Some((w, h, rgba))
}

/// Create a "hover" version of an RGBA image by blending with a blue tint.
/// This simulates the effect of a button changing appearance on hover.
fn make_hover_image((w, h, rgba): (u32, u32, Vec<u8>)) -> (u32, u32, Vec<u8>) {
    let mut hover_rgba = rgba.clone();
    // Blend with blue: add 40 to blue channel, reduce red/green slightly
    for i in (0..hover_rgba.len()).step_by(4) {
        hover_rgba[i] = (hover_rgba[i] as f32 * 0.7).min(255.0) as u8; // R * 0.7
        hover_rgba[i + 1] = (hover_rgba[i + 1] as f32 * 0.7).min(255.0) as u8; // G * 0.7
        hover_rgba[i + 2] = ((hover_rgba[i + 2] as f32 * 1.3).min(255.0)) as u8;
        // B * 1.3 (brighter blue)
    }
    (w, h, hover_rgba)
}

// ── Tool IDs ──────────────────────────────────────────────────────────
const ID_NEW: u16 = 3001;
const ID_OPEN: u16 = 3002;
const ID_SAVE: u16 = 3003;
const ID_SAVEAS: u16 = 3004;
const ID_UNDO: u16 = 3010;
const ID_REDO: u16 = 3011;
const ID_BOLD: u16 = 3020;
const ID_ITALIC: u16 = 3021;
const ID_UNDERLINE: u16 = 3022;
const ID_ZOOM_IN: u16 = 3030;
const ID_ZOOM_OUT: u16 = 3031;
const ID_FULLSCREEN: u16 = 3032;
// Hover example IDs
const ID_HOVER_RED: u16 = 3040;
const ID_HOVER_GREEN: u16 = 3041;
const ID_HOVER_BLUE: u16 = 3042;
const ID_HOVER_TEXT: u16 = 3043;
const ID_TEXT_ONLY: u16 = 3044;
const ID_COLORED_BTN: u16 = 3045;

// ── Helpers ──────────────────────────────────────────────────────────

fn append_log(ctrl: &TextCtrl, msg: &str) {
    let existing = ctrl.get_value();
    let new_msg = format!("{}\n[ribbon] {}", existing, msg);
    ctrl.set_value(new_msg.trim_start());
}

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("RibbonBar Demo — Office-style Ribbon")
        .with_size(960, 680)
        .build();

    // ── Log panel (bottom) ───────────────────────────────────────────
    let log_panel = Panel::new(&frame);
    let log_ctrl = TextCtrl::new(&log_panel, "");
    log_ctrl.set_value("[ribbon] RibbonBar demo started");

    let mut log_sizer = BoxSizer::vertical();
    let lbl_log = StaticText::new(&log_panel, "Activity log:");
    log_sizer.add(lbl_log.as_widget_ref());
    log_sizer.add(log_ctrl.as_widget_ref());
    log_panel.set_sizer(log_sizer);

    // ── Status bar (below ribbon) ────────────────────────────────────
    let status_panel = Panel::new(&frame);
    let status_text = StaticText::new(&status_panel, "Ready");
    let mut status_sizer = BoxSizer::horizontal();
    let status_label = StaticText::new(&status_panel, "Status:");
    status_sizer.add(status_label.as_widget_ref());
    status_sizer.add_spacer(6);
    status_sizer.add(status_text.as_widget_ref());
    status_sizer.add_stretch(1);
    status_panel.set_sizer(status_sizer);

    // ── Ribbon pages ─────────────────────────────────────────────────
    // Base path for ribbon icons (absolute path from cargo run CWD)
    let icon_base = "/Users/marco/cursor_mix/ru_wx/ru_wx/examples/ribbon_icons/";

    // ----- Page 1: Home -------------------------------------------------
    let mut home_page = RibbonPage::new("Home");

    // Group 1: ImageLeading - icon to the left of label (loaded from PNG files)
    let mut leading_group = RibbonPanel::new("Leading (Icon ← Text)");
    {
        let mut bar = RibbonButtonBar::new();
        if let Some(icon) = load_icon_rgba(&format!("{}new.png", icon_base)) {
            bar.add_button_with_icon_and_position(
                ID_NEW,
                "New",
                icon,
                RibbonImagePosition::ImageLeading,
            );
        }
        if let Some(icon) = load_icon_rgba(&format!("{}open.png", icon_base)) {
            bar.add_button_with_icon_and_position(
                ID_OPEN,
                "Open",
                icon,
                RibbonImagePosition::ImageLeading,
            );
        }
        if let Some(icon) = load_icon_rgba(&format!("{}save.png", icon_base)) {
            bar.add_button_with_icon_and_position(
                ID_SAVE,
                "Save",
                icon,
                RibbonImagePosition::ImageLeading,
            );
        }
        leading_group.add_bar(bar);
    }
    home_page.add_panel(leading_group);

    // Group 2: ImageAbove - icon above label
    let mut above_group = RibbonPanel::new("Above (Icon ↑ Text)");
    {
        let mut bar = RibbonButtonBar::new();
        if let Some(icon) = load_icon_rgba(&format!("{}save.png", icon_base)) {
            bar.add_button_with_icon_and_position(
                ID_SAVEAS,
                "Save As",
                icon,
                RibbonImagePosition::ImageAbove,
            );
        }
        if let Some(icon) = load_icon_rgba(&format!("{}undo.png", icon_base)) {
            bar.add_button_with_icon_and_position(
                ID_UNDO,
                "Undo",
                icon,
                RibbonImagePosition::ImageAbove,
            );
        }
        if let Some(icon) = load_icon_rgba(&format!("{}redo.png", icon_base)) {
            bar.add_button_with_icon_and_position(
                ID_REDO,
                "Redo",
                icon,
                RibbonImagePosition::ImageAbove,
            );
        }
        above_group.add_bar(bar);
    }
    home_page.add_panel(above_group);

    // Group 3: ImageTrailing - icon to the right of label
    let mut trailing_group = RibbonPanel::new("Trailing (Text → Icon)");
    {
        let mut bar = RibbonButtonBar::new();
        if let Some(icon) = load_icon_rgba(&format!("{}bold.png", icon_base)) {
            bar.add_button_with_icon_and_position(
                ID_BOLD,
                "Bold",
                icon,
                RibbonImagePosition::ImageTrailing,
            );
        }
        if let Some(icon) = load_icon_rgba(&format!("{}italic.png", icon_base)) {
            bar.add_button_with_icon_and_position(
                ID_ITALIC,
                "Italic",
                icon,
                RibbonImagePosition::ImageTrailing,
            );
        }
        if let Some(icon) = load_icon_rgba(&format!("{}underline.png", icon_base)) {
            bar.add_button_with_icon_and_position(
                ID_UNDERLINE,
                "Underline",
                icon,
                RibbonImagePosition::ImageTrailing,
            );
        }
        trailing_group.add_bar(bar);
    }
    home_page.add_panel(trailing_group);

    // Group 4: ImageOnly - icon only, no label
    let mut icon_only_group = RibbonPanel::new("Icon Only (no text)");
    {
        let mut bar = RibbonButtonBar::new();
        if let Some(icon) = load_icon_rgba(&format!("{}new.png", icon_base)) {
            bar.add_button_with_icon_and_position(ID_NEW, "", icon, RibbonImagePosition::ImageOnly);
        }
        if let Some(icon) = load_icon_rgba(&format!("{}open.png", icon_base)) {
            bar.add_button_with_icon_and_position(
                ID_OPEN,
                "",
                icon,
                RibbonImagePosition::ImageOnly,
            );
        }
        if let Some(icon) = load_icon_rgba(&format!("{}save.png", icon_base)) {
            bar.add_button_with_icon_and_position(
                ID_SAVE,
                "",
                icon,
                RibbonImagePosition::ImageOnly,
            );
        }
        icon_only_group.add_bar(bar);
    }
    home_page.add_panel(icon_only_group);

    // ----- Page 2: Insert ------------------------------------------------
    let mut insert_page = RibbonPage::new("Insert");

    // Group: Hover Effect Examples - buttons that change image on hover
    let mut hover_group = RibbonPanel::new("Hover Effect (image changes on mouse-over)");
    {
        let mut bar = RibbonButtonBar::new();
        // Button with hover image (blue-tinted version)
        if let Some(icon) = load_icon_rgba(&format!("{}new.png", icon_base)) {
            let hover_icon = make_hover_image(icon.clone());
            bar.add_button_with_icon_and_hover_and_position(
                ID_HOVER_RED,
                "Hover Blue",
                icon,
                hover_icon,
                RibbonImagePosition::ImageLeading,
            );
        }
        // Button with hover image for open
        if let Some(icon) = load_icon_rgba(&format!("{}open.png", icon_base)) {
            let hover_icon = make_hover_image(icon.clone());
            bar.add_button_with_icon_and_hover_and_position(
                ID_HOVER_GREEN,
                "Green Hover",
                icon,
                hover_icon,
                RibbonImagePosition::ImageAbove,
            );
        }
        // Button with hover image for save
        if let Some(icon) = load_icon_rgba(&format!("{}save.png", icon_base)) {
            let hover_icon = make_hover_image(icon.clone());
            bar.add_button_with_icon_and_hover_and_position(
                ID_HOVER_BLUE,
                "Image Above",
                icon,
                hover_icon,
                RibbonImagePosition::ImageAbove,
            );
        }
        hover_group.add_bar(bar);
    }
    insert_page.add_panel(hover_group);

    // Group: Text Only Buttons (no image)
    let mut text_only_group = RibbonPanel::new("Text Only (no image)");
    {
        let mut bar = RibbonButtonBar::new();
        bar.add_button_with_color_and_position(
            ID_TEXT_ONLY,
            "Text Btn",
            100,
            100,
            100,
            RibbonImagePosition::ImageLeading,
        );
        bar.add_button_with_color_and_position(
            ID_COLORED_BTN,
            "Colored",
            180,
            80,
            80,
            RibbonImagePosition::ImageLeading,
        );
        text_only_group.add_bar(bar);
    }
    insert_page.add_panel(text_only_group);

    // Group: Image Below with Hover
    let mut below_hover_group = RibbonPanel::new("Image Below + Hover");
    {
        let mut bar = RibbonButtonBar::new();
        if let Some(icon) = load_icon_rgba(&format!("{}undo.png", icon_base)) {
            let hover_icon = make_hover_image(icon.clone());
            bar.add_button_with_icon_and_hover_and_position(
                ID_UNDO,
                "Undo Hover",
                icon,
                hover_icon,
                RibbonImagePosition::ImageBelow,
            );
        }
        if let Some(icon) = load_icon_rgba(&format!("{}redo.png", icon_base)) {
            let hover_icon = make_hover_image(icon.clone());
            bar.add_button_with_icon_and_hover_and_position(
                ID_REDO,
                "Redo Hover",
                icon,
                hover_icon,
                RibbonImagePosition::ImageBelow,
            );
        }
        below_hover_group.add_bar(bar);
    }
    insert_page.add_panel(below_hover_group);

    // ----- Page 3: View --------------------------------------------------
    let mut view_page = RibbonPage::new("View");

    let mut window_group = RibbonPanel::new("Window");
    {
        let mut window_bar = RibbonButtonBar::new();
        if let Some(icon) = load_icon_rgba(&format!("{}save.png", icon_base)) {
            window_bar.add_button_with_icon_and_position(
                ID_FULLSCREEN,
                "Fullscreen",
                icon,
                RibbonImagePosition::ImageLeading,
            );
        }
        window_group.add_bar(window_bar);
    }
    view_page.add_panel(window_group);

    // ── Create RibbonBar ─────────────────────────────────────────────
    let ribbon = RibbonBar::new(&frame);
    ribbon.add_page(home_page);
    ribbon.add_page(insert_page);
    ribbon.add_page(view_page);

    ribbon.on_ribbon_event(&frame, move |ev: &RibbonBarEvent| {
        let msg = match ev.kind {
            RibbonBarEventKind::TabChanged => {
                format!("Tab changed to page {}", ev.tool_id)
            }
            RibbonBarEventKind::ToolClick => {
                let name = match ev.tool_id {
                    ID_NEW => "New",
                    ID_OPEN => "Open",
                    ID_SAVE => "Save",
                    ID_SAVEAS => "Save As",
                    ID_UNDO => "Undo",
                    ID_REDO => "Redo",
                    ID_BOLD => "Bold",
                    ID_ITALIC => "Italic",
                    ID_UNDERLINE => "Underline",
                    ID_ZOOM_IN => "Zoom In",
                    ID_ZOOM_OUT => "Zoom Out",
                    ID_FULLSCREEN => "Fullscreen",
                    ID_HOVER_RED => "Hover Blue (ImageLeading)",
                    ID_HOVER_GREEN => "Green Hover (ImageAbove)",
                    ID_HOVER_BLUE => "Image Above",
                    ID_HOVER_TEXT => "Hover Text",
                    ID_TEXT_ONLY => "Text Button",
                    ID_COLORED_BTN => "Colored Button",
                    _ => "Unknown",
                };
                format!("Tool «{}» clicked", name)
            }
        };
        append_log(&log_ctrl, &msg);
        status_text.set_label(&msg);
    });

    ribbon.realize();

    // ── Layout: ribbon at top, status bar below, log at bottom ──────
    let mut sizer = BoxSizer::vertical();
    sizer.add(ribbon.tab_widget()); // tab strip at top
    sizer.add(status_panel.as_widget_ref()); // status bar
    sizer.add(log_panel.as_widget_ref()); // log fills remaining space
    frame.set_sizer(sizer);

    // Lay out the ribbon's page panels (which are NOT in the sizer)
    ribbon.layout(0, 0, 960, 680);

    // ── Show window ──────────────────────────────────────────────────
    frame.clone().show();
    app.run(frame);
}
