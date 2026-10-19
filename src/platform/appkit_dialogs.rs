//! macOS system dialogs: file/dir pickers, colour panel, text entry alerts.

use crate::core::geometry::Colour;

use objc2::rc::Retained;
use objc2::MainThreadOnly;
use objc2_app_kit::{
    NSAlert, NSAlertStyle, NSApplication, NSColor, NSColorPanel, NSModalResponse,
    NSModalResponseOK, NSOpenPanel, NSPopUpButton, NSSavePanel, NSSecureTextField, NSTextField,
};
use objc2_foundation::{MainThreadMarker, NSArray, NSString, NSURL};

/// Options shared by open/save file panels.
pub struct FilePanelOptions {
    pub title: String,
    pub directory: String,
    pub filename: String,
    pub wildcard: String,
    pub multi_select: bool,
    pub save: bool,
}

fn parse_wildcard_extensions(wildcard: &str) -> Option<Retained<NSArray<NSString>>> {
    let mut exts: Vec<Retained<NSString>> = Vec::new();
    for (i, part) in wildcard.split('|').enumerate() {
        if i % 2 != 1 {
            continue;
        }
        for pat in part.split(';') {
            let pat = pat.trim();
            if let Some(ext) = pat.strip_prefix("*.").or_else(|| pat.strip_prefix('.')) {
                if !ext.is_empty() && ext != "*" {
                    exts.push(NSString::from_str(ext));
                }
            }
        }
    }
    if exts.is_empty() {
        None
    } else {
        Some(NSArray::from_retained_slice(&exts))
    }
}

fn url_to_path(url: &NSURL) -> Option<String> {
    url.path().map(|s| s.to_string())
}

// `-[NSSavePanel setAllowedFileTypes:]` is deprecated by Apple in favour of
// `-allowedContentTypes`, but the replacement needs `UTType` values from the
// `objc2-uniform-type-identifiers` crate, which is not a dependency here. The wildcard
// is given as plain extension strings, so the deprecated call is kept on purpose.
#[allow(deprecated)]
fn configure_panel(panel: &NSSavePanel, opts: &FilePanelOptions) {
    if !opts.title.is_empty() {
        panel.setTitle(Some(&NSString::from_str(&opts.title)));
    }
    if !opts.directory.is_empty() {
        let url = NSURL::fileURLWithPath(&NSString::from_str(&opts.directory));
        panel.setDirectoryURL(Some(&url));
    }
    if !opts.filename.is_empty() {
        panel.setNameFieldStringValue(&NSString::from_str(&opts.filename));
    }
    if let Some(exts) = parse_wildcard_extensions(&opts.wildcard) {
        panel.setAllowedFileTypes(Some(&exts));
    }
}

fn panel_accepted(response: NSModalResponse) -> bool {
    response == NSModalResponseOK
}

pub fn run_file_panel(opts: &FilePanelOptions) -> Option<String> {
    run_file_panel_multi(opts).into_iter().next()
}

pub fn run_file_panel_multi(opts: &FilePanelOptions) -> Vec<String> {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    if opts.save {
        let panel = NSSavePanel::savePanel(mtm);
        configure_panel(&panel, opts);
        if !panel_accepted(panel.runModal()) {
            return Vec::new();
        }
        return panel
            .URL()
            .and_then(|u| url_to_path(&u))
            .into_iter()
            .collect();
    }

    let panel = NSOpenPanel::openPanel(mtm);
    panel.setCanChooseFiles(true);
    panel.setCanChooseDirectories(false);
    panel.setAllowsMultipleSelection(opts.multi_select);
    configure_panel(&panel, opts);
    if !panel_accepted(panel.runModal()) {
        return Vec::new();
    }
    let urls = panel.URLs();
    (0..urls.count())
        .filter_map(|i| url_to_path(&urls.objectAtIndex(i)))
        .collect()
}

pub fn run_dir_panel(title: &str, initial_dir: &str) -> Option<String> {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let panel = NSOpenPanel::openPanel(mtm);
    panel.setCanChooseFiles(false);
    panel.setCanChooseDirectories(true);
    panel.setAllowsMultipleSelection(false);
    if !title.is_empty() {
        panel.setTitle(Some(&NSString::from_str(title)));
    }
    if !initial_dir.is_empty() {
        let url = NSURL::fileURLWithPath(&NSString::from_str(initial_dir));
        panel.setDirectoryURL(Some(&url));
    }
    if !panel_accepted(panel.runModal()) {
        return None;
    }
    panel.URL().and_then(|u| url_to_path(&u))
}

pub fn run_color_panel(initial: Colour) -> Option<Colour> {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let app = NSApplication::sharedApplication(mtm);
    let panel = NSColorPanel::sharedColorPanel(mtm);
    panel.setShowsAlpha(false);
    let ns = NSColor::colorWithCalibratedRed_green_blue_alpha(
        initial.r as f64 / 255.0,
        initial.g as f64 / 255.0,
        initial.b as f64 / 255.0,
        initial.a as f64 / 255.0,
    );
    panel.setColor(&ns);
    let _initial_color = panel.color();
    panel.makeKeyAndOrderFront(None);
    let _response = app.runModalForWindow(&panel);
    panel.orderOut(None);
    let picked = panel.color();

    // Note: NSColorPanel doesn't have built-in cancel detection.
    // We return the picked color regardless of whether user cancelled.
    // For proper cancel support, would need accessory view with buttons.

    Some(Colour::new(
        (picked.redComponent() * 255.0).round().clamp(0.0, 255.0) as u8,
        (picked.greenComponent() * 255.0).round().clamp(0.0, 255.0) as u8,
        (picked.blueComponent() * 255.0).round().clamp(0.0, 255.0) as u8,
        255,
    ))
}

/// Single-line text entry via [`NSAlert`] + accessory [`NSTextField`].
pub fn run_text_entry_alert(
    title: &str,
    message: &str,
    default_value: &str,
    password: bool,
) -> Option<String> {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let alert = NSAlert::new(mtm);
    if title.is_empty() {
        alert.setMessageText(&NSString::from_str("Input"));
    } else {
        alert.setMessageText(&NSString::from_str(title));
    }
    if !message.is_empty() {
        alert.setInformativeText(&NSString::from_str(message));
    }
    alert.setAlertStyle(NSAlertStyle::Informational);
    alert.addButtonWithTitle(&NSString::from_str("OK"));
    alert.addButtonWithTitle(&NSString::from_str("Cancel"));

    let field: Retained<NSTextField> = if password {
        let f = NSSecureTextField::initWithFrame(
            NSSecureTextField::alloc(mtm),
            objc2_foundation::NSRect::new(
                objc2_foundation::NSPoint::new(0.0, 0.0),
                objc2_foundation::NSSize::new(300.0, 24.0),
            ),
        );
        f.setStringValue(&NSString::from_str(default_value));
        f.setEditable(true);
        f.setBezeled(true);
        f.setDrawsBackground(true);
        f.into_super()
    } else {
        let f = NSTextField::initWithFrame(
            NSTextField::alloc(mtm),
            objc2_foundation::NSRect::new(
                objc2_foundation::NSPoint::new(0.0, 0.0),
                objc2_foundation::NSSize::new(300.0, 24.0),
            ),
        );
        f.setStringValue(&NSString::from_str(default_value));
        f.setEditable(true);
        f.setBezeled(true);
        f.setDrawsBackground(true);
        f
    };
    alert.setAccessoryView(Some(&field));

    if alert.runModal() != NSModalResponseOK {
        return None;
    }
    Some(field.stringValue().to_string())
}

/// Single-choice picker via [`NSAlert`] + accessory [`NSPopUpButton`].
pub fn run_single_choice_alert(
    message: &str,
    caption: &str,
    choices: &[&str],
    initial: Option<usize>,
) -> Option<usize> {
    if choices.is_empty() {
        return None;
    }
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let alert = NSAlert::new(mtm);
    alert.setMessageText(&NSString::from_str(caption));
    if !message.is_empty() {
        alert.setInformativeText(&NSString::from_str(message));
    }
    alert.setAlertStyle(NSAlertStyle::Informational);
    let popup = NSPopUpButton::initWithFrame(
        NSPopUpButton::alloc(mtm),
        objc2_foundation::NSRect::new(
            objc2_foundation::NSPoint::new(0.0, 0.0),
            objc2_foundation::NSSize::new(300.0, 26.0),
        ),
    );
    for choice in choices {
        popup.addItemWithTitle(&NSString::from_str(choice));
    }
    if let Some(idx) = initial {
        if idx < choices.len() {
            popup.selectItemAtIndex(idx as isize);
        }
    }
    alert.setAccessoryView(Some(&popup));
    alert.addButtonWithTitle(&NSString::from_str("OK"));
    alert.addButtonWithTitle(&NSString::from_str("Cancel"));
    if alert.runModal() != NSModalResponseOK {
        return None;
    }
    Some(popup.indexOfSelectedItem() as usize)
}

/// Modal date picker via [`NSAlert`] + [`NSDatePicker`] accessory.
#[cfg(target_os = "macos")]
pub fn run_date_picker_dialog(
    message: &str,
    caption: &str,
    initial: Option<(i32, u32, u32)>,
) -> Option<(i32, u32, u32)> {
    use objc2_app_kit::{NSDatePicker, NSDatePickerElementFlags, NSDatePickerStyle};
    use objc2_foundation::{NSDateFormatter, NSLocale};

    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let alert = NSAlert::new(mtm);
    alert.setMessageText(&NSString::from_str(if caption.is_empty() {
        "Select date"
    } else {
        caption
    }));
    if !message.is_empty() {
        alert.setInformativeText(&NSString::from_str(message));
    }
    alert.setAlertStyle(NSAlertStyle::Informational);

    let picker = NSDatePicker::initWithFrame(
        NSDatePicker::alloc(mtm),
        objc2_foundation::NSRect::new(
            objc2_foundation::NSPoint::new(0.0, 0.0),
            objc2_foundation::NSSize::new(260.0, 26.0),
        ),
    );
    picker.setDatePickerStyle(NSDatePickerStyle::TextFieldAndStepper);
    picker.setDatePickerElements(
        NSDatePickerElementFlags::YearMonthDay | NSDatePickerElementFlags::Era,
    );
    if let Some((y, m, d)) = initial {
        let fmt = NSDateFormatter::new();
        fmt.setDateFormat(Some(&NSString::from_str("yyyy-MM-dd")));
        fmt.setLocale(Some(&NSLocale::currentLocale()));
        let text = format!("{y:04}-{m:02}-{d:02}");
        if let Some(date) = fmt.dateFromString(&NSString::from_str(&text)) {
            picker.setDateValue(&date);
        }
    }
    alert.setAccessoryView(Some(&picker));
    alert.addButtonWithTitle(&NSString::from_str("OK"));
    alert.addButtonWithTitle(&NSString::from_str("Cancel"));
    if alert.runModal() != NSModalResponseOK {
        return None;
    }
    let fmt = NSDateFormatter::new();
    fmt.setDateFormat(Some(&NSString::from_str("yyyy-MM-dd")));
    let text = fmt.stringFromDate(&picker.dateValue()).to_string();
    let mut parts = text.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    Some((year, month, day))
}
