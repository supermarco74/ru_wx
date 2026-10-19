//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Minitest: `TaskDialog` — sostituto moderno di `MessageBoxW`.
//!
//! Dimostra:
//! - dialogo semplice OK/Annulla con casella di verifica;
//! - command link con contenuto espanso (`TDF_USE_COMMAND_LINKS`);
//! - dialogo di errore con icona rossa e pulsanti Riprova/Chiudi;
//! - lettura dell'esito e dello stato della casella di verifica in un
//!   `StaticText`.
//!
//! Run con:
//! ```bash
//! cargo run --example mt_task_dialog
//! ```

#![windows_subsystem = "windows"]

use ru_wx::dialogs::task_dialog::{TaskDialog, TaskDialogButton, TaskDialogIcon};
use ru_wx::{App, BoxSizer, Button, Frame, StaticText};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — TaskDialog")
        .with_size(560, 300)
        .build();

    let hint = StaticText::new(&frame, "Premi un pulsante per aprire un TaskDialog modale:");
    let esito = StaticText::new(&frame, "Risultato: (nessuno)");

    // (1) Dialogo semplice OK/Annulla con casella di verifica.
    let btn_semplice = Button::new(&frame, "1 — Semplice OK/Annulla + checkbox");
    {
        let frame_c = frame.clone();
        let esito = esito.clone();
        btn_semplice.on_click(&frame, move || {
            let dialog = TaskDialog::new()
                .with_parent(&frame_c)
                .with_title("Conferma operazione")
                .with_main_instruction("Vuoi davvero continuare?")
                .with_content("L'operazione partirà subito dopo la conferma.")
                .with_icon(TaskDialogIcon::Warning)
                .with_common_buttons(TaskDialogButton::Ok.flag() | TaskDialogButton::Cancel.flag())
                .with_verification_checkbox("Non mostrare più questo avviso", false);
            let risultato = dialog.show();
            let spuntata = dialog.verification_checked();
            esito.set_label(&format!(
                "1: risultato {risultato:?} — checkbox: {spuntata}"
            ));
        });
    }

    // (2) Command link con contenuto espanso già aperto.
    let btn_link = Button::new(&frame, "2 — Command link + contenuto espanso");
    {
        let frame_c = frame.clone();
        let esito = esito.clone();
        btn_link.on_click(&frame, move || {
            let dialog = TaskDialog::new()
                .with_parent(&frame_c)
                .with_title("Importazione guidata")
                .with_main_instruction("Come vuoi importare i dati?")
                .with_content("Scegli una modalità di importazione.")
                .with_icon(TaskDialogIcon::Information)
                .with_command_link(101, "Importa tutto", "Copia tutti i record dalla sorgente.")
                .with_command_link(
                    102,
                    "Solo selezione",
                    "Importa solo gli elementi selezionati.",
                )
                .with_button(TaskDialogButton::Cancel)
                .with_expanded_content(
                    "Sorgente: C:\\dati\\import.csv — 1024 record, 3 campi ignorati.",
                )
                .with_expanded(true);
            let risultato = dialog.show();
            esito.set_label(&format!("2: risultato {risultato:?}"));
        });
    }

    // (3) Dialogo di errore con Riprova/Chiudi.
    let btn_errore = Button::new(&frame, "3 — TaskDialog di errore");
    {
        let frame_c = frame.clone();
        let esito = esito.clone();
        btn_errore.on_click(&frame, move || {
            let dialog = TaskDialog::new()
                .with_parent(&frame_c)
                .with_title("Errore di scrittura")
                .with_main_instruction("Impossibile salvare il file.")
                .with_content("Codice 0x80070057: argomento non valido.")
                .with_icon(TaskDialogIcon::Error)
                .with_button(TaskDialogButton::Retry)
                .with_button(TaskDialogButton::Close);
            let risultato = dialog.show();
            esito.set_label(&format!("3: risultato {risultato:?}"));
        });
    }

    // Layout: testo di testa, esito e i tre pulsanti impilati in verticale.
    let mut sizer = BoxSizer::vertical();
    sizer.add(hint.as_widget_ref());
    sizer.add(esito.as_widget_ref());
    sizer.add(btn_semplice.as_widget_ref());
    sizer.add(btn_link.as_widget_ref());
    sizer.add(btn_errore.as_widget_ref());
    frame.set_sizer(sizer);

    app.run(frame);
}
