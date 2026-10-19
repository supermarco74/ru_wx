//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! `TaskDialog` — dialogo modale Win32 basato su `TaskDialogIndirect`.
//!
//! Sostituisce `MessageBoxW` quando servono: istruzione principale e
//! contenuto separati, icone moderne (incluso scudo UAC), command link con
//! nota, sezione espandibile e casella di verifica.
//!
//! Scelte architetturali:
//! - `hwndParent` è l'HWND del frame quando fornito con
//!   [`TaskDialog::with_parent`], altrimenti la finestra è top-level;
//! - contenuto espanso e casella di verifica sono gestiti con flag a tempo
//!   di costruzione (`TDF_EXPANDED_BY_DEFAULT`, `TDF_VERIFICATION_FLAG_CHECKED`)
//!   e campi `pszExpandedInformation` / `pszVerificationText`: la strada più
//!   semplice, senza usare il callback `pfnCallback`;
//! - i command link usano `TDF_USE_COMMAND_LINKS`; la nota del link è la
//!   seconda riga del testo del pulsante (separata da `\n`), perché
//!   `TASKDIALOG_BUTTON` non ha un campo note dedicato.
//!
//! Esempio:
//! ```no_run
//! use ru_wx::dialogs::task_dialog::{TaskDialog, TaskDialogButton, TaskDialogIcon};
//!
//! let frame = ru_wx::Frame::builder().with_title("App").with_size(400, 300).build();
//! let dialog = TaskDialog::new()
//!     .with_parent(&frame)
//!     .with_title("Salvataggio")
//!     .with_main_instruction("Salvare le modifiche?")
//!     .with_content("Le modifiche non salvate andranno perse.")
//!     .with_icon(TaskDialogIcon::Warning)
//!     .with_button(TaskDialogButton::Yes)
//!     .with_button(TaskDialogButton::No)
//!     .with_verification_checkbox("Non chiedermelo più", false);
//! let esito = dialog.show();
//! let spuntata = dialog.verification_checked();
//! ```

use std::cell::Cell;

use crate::core::widget::Window;
use crate::window::frame::Frame;

#[cfg(target_os = "windows")]
use crate::platform::win32::to_wide;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{BOOL, HWND};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Controls::{
    TaskDialogIndirect, TASKDIALOGCONFIG, TASKDIALOG_BUTTON, TDF_ALLOW_DIALOG_CANCELLATION,
    TDF_EXPANDED_BY_DEFAULT, TDF_USE_COMMAND_LINKS, TDF_VERIFICATION_FLAG_CHECKED, TD_ERROR_ICON,
    TD_INFORMATION_ICON, TD_SHIELD_ICON, TD_WARNING_ICON,
};

// Bandiere comuni `TDCBF_*` (valori identici a commctrl.h / windows-sys):
// definite qui così l'enum compila anche fuori da Windows.
const TDCBF_OK: i32 = 1;
const TDCBF_YES: i32 = 2;
const TDCBF_NO: i32 = 4;
const TDCBF_CANCEL: i32 = 8;
const TDCBF_RETRY: i32 = 16;
const TDCBF_CLOSE: i32 = 32;

// Id standard dei pulsanti restituiti da `TaskDialogIndirect`.
#[cfg(target_os = "windows")]
const IDOK: i32 = 1;
#[cfg(target_os = "windows")]
const IDCANCEL: i32 = 2;
#[cfg(target_os = "windows")]
const IDRETRY: i32 = 4;
#[cfg(target_os = "windows")]
const IDYES: i32 = 6;
#[cfg(target_os = "windows")]
const IDNO: i32 = 7;
#[cfg(target_os = "windows")]
const IDCLOSE: i32 = 8;

/// Icona principale della [`TaskDialog`] (`TD_*_ICON`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskDialogIcon {
    /// Icona informativa (default).
    Information,
    /// Icona di avviso (triangolo giallo).
    Warning,
    /// Icona di errore (cerchio rosso).
    Error,
    /// Icona scudo UAC.
    Shield,
}

/// Pulsante comune richiedibile (`TDCBF_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskDialogButton {
    Ok,
    Yes,
    No,
    Cancel,
    Retry,
    Close,
}

impl TaskDialogButton {
    /// Restituisce la bandiera `TDCBF_*` corrispondente, da sommare per
    /// comporre [`TaskDialog::with_common_buttons`].
    pub fn flag(self) -> i32 {
        match self {
            TaskDialogButton::Ok => TDCBF_OK,
            TaskDialogButton::Yes => TDCBF_YES,
            TaskDialogButton::No => TDCBF_NO,
            TaskDialogButton::Cancel => TDCBF_CANCEL,
            TaskDialogButton::Retry => TDCBF_RETRY,
            TaskDialogButton::Close => TDCBF_CLOSE,
        }
    }
}

/// Esito dell'interazione con la [`TaskDialog`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskDialogResult {
    Ok,
    Yes,
    No,
    Cancel,
    Retry,
    Close,
    /// Command link scelto dall'utente, con il suo id (quello passato a
    /// [`TaskDialog::with_command_link`]).
    CommandLink(i32),
}

/// Builder della [`TaskDialog`]: configura il dialogo e lo mostra con
/// [`TaskDialog::show`].
pub struct TaskDialog {
    /// HWND del genitore (0 = nessun genitore). Memorizzato come `isize`
    /// perché il tipo esiste anche fuori da Windows.
    parent: isize,
    title: String,
    main_instruction: String,
    content: String,
    icon: TaskDialogIcon,
    /// Bandiere `TDCBF_*` dei pulsanti comuni.
    common_buttons: i32,
    /// Command link: (id, etichetta, nota).
    command_links: Vec<(i32, String, String)>,
    expanded_content: String,
    expanded: bool,
    verification_text: String,
    verification_checked: bool,
    /// Stato della casella di verifica letto dall'ultima [`show`](TaskDialog::show).
    last_verification: Cell<bool>,
}

impl TaskDialog {
    /// Crea un dialogo vuoto: nessun genitore, pulsante OK, icona
    /// informativa. Usare i metodi `with_*` per la configurazione.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for TaskDialog {
    fn default() -> Self {
        TaskDialog {
            parent: 0,
            title: String::new(),
            main_instruction: String::new(),
            content: String::new(),
            icon: TaskDialogIcon::Information,
            common_buttons: TDCBF_OK,
            command_links: Vec::new(),
            expanded_content: String::new(),
            expanded: false,
            verification_text: String::new(),
            verification_checked: false,
            last_verification: Cell::new(false),
        }
    }
}

impl TaskDialog {
    /// Imposta la finestra genitore (di norma il frame): il suo HWND finirà
    /// in `hwndParent` della `TASKDIALOGCONFIG`.
    pub fn with_parent<W: Window>(self, parent: &W) -> Self {
        #[cfg(target_os = "windows")]
        {
            let mut this = self;
            this.parent = crate::platform::window_handle_as_hwnd(parent) as isize;
            this
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = parent;
            self
        }
    }

    /// Testo della barra del titolo.
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    /// Istruzione principale (testo in grande, in verde per i temi classici).
    pub fn with_main_instruction(mut self, text: &str) -> Self {
        self.main_instruction = text.to_string();
        self
    }

    /// Contenuto secondario sotto l'istruzione principale.
    pub fn with_content(mut self, text: &str) -> Self {
        self.content = text.to_string();
        self
    }

    /// Icona principale del dialogo.
    pub fn with_icon(mut self, icon: TaskDialogIcon) -> Self {
        self.icon = icon;
        self
    }

    /// Bandiere `TDCBF_*` dei pulsanti comuni (sommate bitwise, es.
    /// `TDCBF_OK | TDCBF_CANCEL` di `windows-sys`).
    pub fn with_common_buttons(mut self, flags: i32) -> Self {
        self.common_buttons = flags;
        self
    }

    /// Aggiunge un pulsante comune. Equivale a ORare la sua bandiera
    /// `TDCBF_*` in [`with_common_buttons`](TaskDialog::with_common_buttons).
    pub fn with_button(mut self, button: TaskDialogButton) -> Self {
        self.common_buttons |= button.flag();
        self
    }

    /// Aggiunge un command link (pulsante grande con nota secondaria).
    /// `id` deve essere un id personalizzato (per convenzione >= 100) e
    /// viene restituito in [`TaskDialogResult::CommandLink`].
    pub fn with_command_link(mut self, id: i32, label: &str, note: &str) -> Self {
        self.command_links
            .push((id, label.to_string(), note.to_string()));
        self
    }

    /// Testo della sezione espandibile (`TDE_EXPANDED_INFORMATION`).
    pub fn with_expanded_content(mut self, text: &str) -> Self {
        self.expanded_content = text.to_string();
        self
    }

    /// Se `true`, la sezione espandibile è aperta già all'apertura del
    /// dialogo (`TDF_EXPANDED_BY_DEFAULT`). Senza testo espanso non ha effetto.
    pub fn with_expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// Aggiunge la casella di verifica in basso con il testo indicato;
    /// `checked` ne fissa lo stato iniziale (`TDF_VERIFICATION_FLAG_CHECKED`).
    pub fn with_verification_checkbox(mut self, text: &str, checked: bool) -> Self {
        self.verification_text = text.to_string();
        self.verification_checked = checked;
        self
    }

    /// Mostra il dialogo in modo modale e blocca fino alla chiusura.
    /// Lo stato della casella di verifica si legge con
    /// [`verification_checked`](TaskDialog::verification_checked).
    #[cfg(target_os = "windows")]
    pub fn show(&self) -> TaskDialogResult {
        // Stringhe wide: devono restare vive fino a fine chiamata.
        let wide_title = wide_opt(&self.title);
        let wide_main = wide_opt(&self.main_instruction);
        let wide_content = wide_opt(&self.content);
        let wide_expanded = wide_opt(&self.expanded_content);
        let wide_verification = wide_opt(&self.verification_text);

        // Testi dei command link: la nota è la seconda riga del testo.
        let link_texts: Vec<Vec<u16>> = self
            .command_links
            .iter()
            .map(|(_, label, note)| {
                if note.is_empty() {
                    to_wide(label)
                } else {
                    to_wide(&format!("{label}\n{note}"))
                }
            })
            .collect();
        let buttons: Vec<TASKDIALOG_BUTTON> = self
            .command_links
            .iter()
            .zip(link_texts.iter())
            .map(|((id, _, _), text)| TASKDIALOG_BUTTON {
                nButtonID: *id,
                pszButtonText: text.as_ptr(),
            })
            .collect();

        // Bandiere a tempo di costruzione (nessun pfnCallback necessario).
        let mut flags = TDF_ALLOW_DIALOG_CANCELLATION;
        if !self.command_links.is_empty() {
            flags |= TDF_USE_COMMAND_LINKS;
        }
        if self.expanded && !self.expanded_content.is_empty() {
            flags |= TDF_EXPANDED_BY_DEFAULT;
        }
        if self.verification_checked && !self.verification_text.is_empty() {
            flags |= TDF_VERIFICATION_FLAG_CHECKED;
        }

        // SAFETY: struttura azzerata e tutti i campi imposti prima della
        // chiamata; le stringhe wide e `buttons` restano vive per tutta la
        // durata del dialogo modale; le icone sono identificatori di
        // risorsa (MAKEINTRESOURCE), non puntatori a memoria.
        let mut cfg: TASKDIALOGCONFIG = unsafe { std::mem::zeroed() };
        cfg.cbSize = std::mem::size_of::<TASKDIALOGCONFIG>() as u32;
        cfg.hwndParent = self.parent as HWND;
        cfg.dwFlags = flags;
        cfg.dwCommonButtons = self.common_buttons;
        cfg.pszWindowTitle = pcw(&wide_title);
        cfg.Anonymous1.pszMainIcon = match self.icon {
            TaskDialogIcon::Information => TD_INFORMATION_ICON,
            TaskDialogIcon::Warning => TD_WARNING_ICON,
            TaskDialogIcon::Error => TD_ERROR_ICON,
            TaskDialogIcon::Shield => TD_SHIELD_ICON,
        };
        cfg.pszMainInstruction = pcw(&wide_main);
        cfg.pszContent = pcw(&wide_content);
        cfg.cButtons = buttons.len() as u32;
        cfg.pButtons = if buttons.is_empty() {
            std::ptr::null()
        } else {
            buttons.as_ptr()
        };
        cfg.pszVerificationText = pcw(&wide_verification);
        cfg.pszExpandedInformation = pcw(&wide_expanded);

        let mut pressed: i32 = 0;
        let mut radio: i32 = 0;
        let mut verification: BOOL = 0;
        // SAFETY: `cfg` è completa e valida; gli output sono puntatori a
        // variabili locali.
        let hr = unsafe { TaskDialogIndirect(&cfg, &mut pressed, &mut radio, &mut verification) };
        let _ = radio; // nessun gruppo di radio button in questo builder
        self.last_verification.set(verification != 0);
        if hr < 0 {
            // Apertura fallita: nessun pulsante premuto.
            return TaskDialogResult::Cancel;
        }

        match pressed {
            IDOK => TaskDialogResult::Ok,
            IDCANCEL => TaskDialogResult::Cancel,
            IDRETRY => TaskDialogResult::Retry,
            IDYES => TaskDialogResult::Yes,
            IDNO => TaskDialogResult::No,
            IDCLOSE => TaskDialogResult::Close,
            // Id personalizzato: nell'uso di questo crate sono i command link.
            id => TaskDialogResult::CommandLink(id),
        }
    }

    /// Mostra il dialogo in modo modale. Fallback fuori da Windows:
    /// `TaskDialogIndirect` esiste solo su Win32, quindi restituisce
    /// [`TaskDialogResult::Cancel`] senza mostrare nulla.
    #[cfg(not(target_os = "windows"))]
    pub fn show(&self) -> TaskDialogResult {
        self.last_verification.set(self.verification_checked);
        TaskDialogResult::Cancel
    }

    /// Stato della casella di verifica dopo l'ultima [`show`](TaskDialog::show).
    pub fn verification_checked(&self) -> bool {
        self.last_verification.get()
    }
}

/// Converte in wide una stringa non vuota; `None` per le stringhe vuote,
/// così il campo corrispondente della `TASKDIALOGCONFIG` resta null.
#[cfg(target_os = "windows")]
fn wide_opt(testo: &str) -> Option<Vec<u16>> {
    if testo.is_empty() {
        None
    } else {
        Some(to_wide(testo))
    }
}

/// Puntatore `PCWSTR` da [`wide_opt`]: null per l'assenza di stringa.
#[cfg(target_os = "windows")]
fn pcw(w: &Option<Vec<u16>>) -> windows_sys::core::PCWSTR {
    match w {
        Some(v) => v.as_ptr(),
        None => std::ptr::null(),
    }
}

/// Alias funzionale: mostra subito una [`TaskDialog`] con i pulsanti comuni
/// indicati (bandiere `TDCBF_*` sommate bitwise).
pub fn task_dialog_show(
    parent: &Frame,
    title: &str,
    main_instruction: &str,
    content: &str,
    buttons: i32,
) -> TaskDialogResult {
    TaskDialog::new()
        .with_parent(parent)
        .with_title(title)
        .with_main_instruction(main_instruction)
        .with_content(content)
        .with_common_buttons(buttons)
        .show()
}
