//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
#![windows_subsystem = "windows"]

use ru_wx::{
    App, BoxSizer, CredentialEntryDialog, DataViewBitmapRenderer, DataViewChoiceRenderer,
    DataViewRenderer, DataViewToggleRenderer, Frame, GridCellBoolEditor, GridCellBoolRenderer,
    GridCellChoiceEditor, GridCellDateEditor, GridCellFloatEditor, GridCellNumberEditor,
    GridCellNumberRenderer, GridCellStringRenderer, GridCellTextEditor, RichTextAttr,
    RichTextFormattingDialog, RichTextStyle, RichTextStyleSheet, StaticText, StatusBar,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — round 35")
        .with_size(480, 280)
        .with_modern_style().build();
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Round 35: grid editors + data-view renderers.", 0);
    let _hint = StaticText::new(&frame, "GridCell* / DataView* / RichText dialogs:");
    let mut text_ed = GridCellTextEditor::new(0, 0);
    text_ed.begin_edit("A1");
    let _ = text_ed.end_edit(true);
    let mut num_ed = GridCellNumberEditor::new(0, 1);
    num_ed.begin_edit(42);
    let _ = num_ed.end_edit(true);
    let mut float_ed = GridCellFloatEditor::new(1, 0);
    float_ed.begin_edit(2.5);
    let _ = float_ed.end_edit(true);
    let mut bool_ed = GridCellBoolEditor::new(1, 1);
    bool_ed.toggle();
    let mut choice_ed = GridCellChoiceEditor::new(2, 0, vec!["A".into(), "B".into()]);
    choice_ed.set_selection(1);
    let mut date_ed = GridCellDateEditor::new(2, 1);
    date_ed.begin_edit("2026-06-10");
    let _ = date_ed.end_edit(true);
    let str_r = GridCellStringRenderer::new();
    let _ = str_r.render_string("hello");
    let num_r = GridCellNumberRenderer::new();
    let _ = num_r.render_number(12.5);
    let bool_r = GridCellBoolRenderer::new();
    let _ = bool_r.render_bool(true);
    let bmp_r = DataViewBitmapRenderer::new(0);
    let _ = bmp_r.render_text("icon");
    let choice_r = DataViewChoiceRenderer::new(vec!["One".into(), "Two".into()]);
    let _ = choice_r.render_text("1");
    let toggle_r = DataViewToggleRenderer::new();
    let _ = toggle_r.render_text("true");
    let mut sheet = RichTextStyleSheet::new();
    sheet.add_style(RichTextStyle::new("Body"));
    let dlg = RichTextFormattingDialog::new("Format", RichTextAttr::new().bold());
    let _ = dlg.show_modal(&frame);
    let cred = CredentialEntryDialog::new("Login", "Enter credentials")
        .with_defaults("user", "secret");
    let _ = cred.show_modal(&frame);
    let mut sizer = BoxSizer::vertical();
    sizer.add(_hint.as_widget_ref());
    frame.set_sizer(sizer);
    app.run(frame);
}
