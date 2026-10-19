//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! DataGrid virtualizzata: `ListCtrl` in modalità `LVS_OWNERDATA`
//! (righe **non** memorizzate nel controllo) con sort per colonna e
//! filtro testuale, alimentata da una [`DataTable`] del layer `data`.
//!
//! Perché virtualizzare: una tabella con centinaia di migliaia di righe
//! non può essere inserita fisicamente nel ListView Win32 (ogni riga è
//! un `LVITEMW` allocato dal controllo). In modalità virtuale la
//! griglia tiene solo gli **indici** delle righe visibili
//! ([`DataGrid::visible_row_indices`]) e chiama il modello a ogni
//! paint tramite `LVN_GETDISPINFOW` — vedi
//! [`ListCtrl::on_get_disp_info`](crate::controls::list_ctrl::ListCtrl::on_get_disp_info).
//!
//! Il ciclo di vita è:
//!
//! 1. [`DataGrid::new`] crea la lista report (griglia + riga piena);
//! 2. [`DataGrid::set_data`] carica la `DataTable`, allinea le colonne
//!    e registra gli handler (`bind` è idempotente);
//! 3. [`DataGrid::set_filter`] / [`DataGrid::set_sort`] ricalcolano la
//!    vista; il sort si innesca anche cliccando l'intestazione di
//!    colonna;
//! 4. [`DataGrid::refresh`] ricarica il modello dopo che l'applicazione
//!    ha modificato la `DataTable` (es. tramite un form).

use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

use crate::controls::list_ctrl::{ListCtrl, ListCtrlStyle, ListItem};
use crate::core::widget::{WidgetRef, Window};
use crate::data::{DataColumn, DataFieldType, DataTable, DataValue};
use crate::window::frame::Frame;

// ── Stili estesi ListView (valori documentati Win32) ─────────────────

/// `LVS_EX_GRIDLINES` — disegna la griglia attorno alle celle.
const LVS_EX_GRIDLINES: u32 = 0x0001;
/// `LVS_EX_HEADERDRAGDROP` — trascinamento intestazioni per riordinare
/// le colonne.
const LVS_EX_HEADERDRAGDROP: u32 = 0x0010;
/// `LVS_EX_FULLROWSELECT` — cliccando una cella si seleziona l'intera
/// riga.
const LVS_EX_FULLROWSELECT: u32 = 0x0020;
/// `LVS_EX_DOUBLEBUFFER` — paint in memoria: niente sfarfallio
/// durante scroll, sort e refresh delle celle virtuali.
const LVS_EX_DOUBLEBUFFER: u32 = 0x0001_0000;

/// Larghezza predefinita delle colonne quando il nome non dà indizi.
const DEFAULT_COLUMN_WIDTH: i32 = 140;

/// Stato condiviso della griglia.
///
/// È il modello **vista**: `view` contiene gli indici di riga della
/// [`DataTable`] che superano il filtro, nell'ordine corrente del sort.
/// Il `ListCtrl` virtuale non vede altro che questi indici.
type CellEditCallback = Box<dyn FnMut(usize, usize, DataValue)>;
struct DataGridState {
    /// Copia dei dati sorgente (autorità per celle e sort).
    data: DataTable,
    /// Indici di riga visibili: `view[i]` è la riga sorgente mostrata
    /// alla posizione display `i`.
    view: Vec<usize>,
    /// Colonna su cui si sta ordinando (indice sorgente).
    sort_column: Option<usize>,
    /// `true` = ordine crescente.
    sort_ascending: bool,
    /// Filtro testuale applicato a tutte le celle (case-insensitive).
    filter: String,
    /// Intestazioni attualmente create nel `ListCtrl`: serve per
    /// sapere quante colonne eliminare/inserire quando cambia lo
    /// schema e per rilevare ridenominazioni.
    col_titles: Vec<String>,
    /// `true` quando gli handler (`LVN_GETDISPINFOW`, click header)
    /// sono già registrati sul `Frame`.
    bound: bool,
    /// Colonna checkbox nativa (`LVS_EX_CHECKBOXES`) abilitata.
    checkboxes: bool,
    /// Righe **sorgente** spuntate (stato UI, non va nel database).
    checked: std::collections::HashSet<usize>,
    /// Image list attaccata al ListView (slot small): mantiene vivo
    /// l'`HIMAGELIST` finché vive la griglia.
    images: Option<crate::ImageList>,
    /// Icona per riga **sorgente**: indice dentro `images`.
    row_images: std::collections::HashMap<usize, i32>,
    /// Callback opzionale invocata al toggle di una checkbox
    /// (riceve riga sorgente e nuovo stato).
    on_check: Option<Box<dyn FnMut(usize, bool)>>,
    /// Edit in-place abilitato (doppio click su una cella apre un
    /// EDIT flottante nativo).
    editable: bool,
    /// Callback per il doppio click su riga non editabile.
    on_row_activated_cb: Option<Box<dyn FnMut(usize)>>,
    /// Callback dopo il commit di un edit in-place: (riga sorgente,
    /// colonna, nuovo valore).
    on_cell_edited: Option<CellEditCallback>,
    /// Id riservato all'EDIT flottante (assegnato in `bind`): il
    /// handler `EN_KILLFOCUS` è registrato una volta sola su questo id.
    edit_id: u16,
    /// Sessione di edit in corso (una alla volta per griglia).
    #[cfg(target_os = "windows")]
    edit: Option<Rc<RefCell<cell_edit::EditSession>>>,
}

/// DataGrid virtualizzata costruita su [`ListCtrl`].
///
/// Il tipo è `Clone`: tutte le cloni condividono lo stesso stato
/// (`Rc<RefCell<…>>`) e lo stesso controllo nativo.
#[derive(Clone)]
pub struct DataGrid {
    list: ListCtrl,
    state: Rc<RefCell<DataGridState>>,
}

impl DataGrid {
    /// Crea una griglia vuota dentro `parent`.
    ///
    /// La lista nasce in modalità report con griglia, riga piena e
    /// header trascinabili; il conteggio righe parte da 0 e la
    /// modalità virtuale è già attiva, per cui le prime richieste di
    /// paint (prima di [`set_data`](Self::set_data)) restituiscono
    /// celle vuote fino a quando non vengono registrati gli handler
    /// con [`bind`](Self::bind) / [`set_data`](Self::set_data).
    pub fn new<W: Window>(parent: &W) -> Self {
        // `ReportVirtual` = `LVS_REPORT | LVS_OWNERDATA` alla
        // creazione: il bit owner-data non è modificabile dopo.
        let list = ListCtrl::new(parent, ListCtrlStyle::ReportVirtual);
        // mask = 0 in `set_extended_style`: i bit indicati sono gli
        // **unici** attivi, quindi va chiamato una sola volta (o con
        // l'intero insieme) prima di eventuali `set_checkboxes`.
        list.set_extended_style(
            LVS_EX_GRIDLINES | LVS_EX_HEADERDRAGDROP | LVS_EX_FULLROWSELECT | LVS_EX_DOUBLEBUFFER,
        );
        let state = Rc::new(RefCell::new(DataGridState {
            data: DataTable::new(String::new()),
            view: Vec::new(),
            sort_column: None,
            sort_ascending: true,
            filter: String::new(),
            col_titles: Vec::new(),
            bound: false,
            checkboxes: false,
            checked: std::collections::HashSet::new(),
            images: None,
            row_images: std::collections::HashMap::new(),
            on_check: None,
            editable: false,
            on_row_activated_cb: None,
            on_cell_edited: None,
            edit_id: 0,
            #[cfg(target_os = "windows")]
            edit: None,
        }));
        // Attiva subito `LVS_OWNERDATA` (anche con 0 righe).
        list.set_item_count(0);
        DataGrid { list, state }
    }

    // ── Bind degli handler ────────────────────────────────────────

    /// Registra sul `Frame` gli handler che rendono la griglia
    /// interattiva: erogazione celle (`LVN_GETDISPINFOW`) e sort per
    /// clic sull'intestazione (`LVN_COLUMNCLICK`).
    ///
    /// Idempotente: essere chiamata più volte non duplica nulla.
    /// [`set_data`](Self::set_data) la invoca automaticamente, quindi
    /// nella stragrande maggioranza dei casi non è necessaria.
    pub fn bind(&self, frame: &Frame) {
        {
            let mut st = self.state.borrow_mut();
            if st.bound {
                return;
            }
            st.bound = true;
        }

        // (1) Erogazione celle: richiesta display → cella del modello.
        let state = self.state.clone();
        self.list
            .on_get_disp_info(frame, move |item: &mut ListItem| {
                // Prestito immutabile: non modifichiamo mai lo stato qui
                // dentro, quindi non ci sono conflitti con `refresh`.
                let st = state.borrow();
                let Some(&source_row) = st.view.get(item.index()) else {
                    return;
                };
                // Checkbox nativa: in modalità virtuale il controllo
                // chiede lo state image a ogni paint (`LVIF_STATE`).
                if st.checkboxes && item.is_state_requested() {
                    item.set_state_image_checked(st.checked.contains(&source_row));
                }
                // Icona di riga: richiesta `LVIF_IMAGE` (solo sub-item
                // 0 in report mode); l'indice vive nel modello, quindi
                // sopravvive a sort, filtro e scroll.
                if item.is_image_requested() {
                    if let Some(&index) = st.row_images.get(&source_row) {
                        item.set_image(index);
                    }
                }
                if !item.is_text_requested() {
                    // Richieste di solo stato/immagine già risposte
                    // sopra: qui resta solo il testo delle celle.
                    return;
                }
                let Some(value) = st.data.cell(source_row, item.sub_item()) else {
                    return;
                };
                let text = value.to_display_string();
                let _ = item.set_text(&text);
            });

        // (1b) Toggle checkbox: `LVN_ITEMCHANGED` con `NMLISTVIEW`
        // completo (`uOldState`/`uNewState` portano lo state image).
        let state = self.state.clone();
        let id = self.list.id();
        frame.register_lv_item_changed_handler(
            id,
            Box::new(move |lparam| {
                // SAFETY: `lparam` punta a un NMLISTVIEW valido per la
                // durata della notifica (dispatch sincrono del frame).
                #[cfg(target_os = "windows")]
                {
                    let nm = lparam as *const windows_sys::Win32::UI::Controls::NMLISTVIEW;
                    if nm.is_null() {
                        return;
                    }
                    let nm = unsafe { &*nm };
                    const LVIF_STATE: u32 = 0x8;
                    if (nm.uChanged & LVIF_STATE) == 0 {
                        return;
                    }
                    let was = (nm.uOldState & 0x2000) != 0;
                    let is = (nm.uNewState & 0x2000) != 0;
                    if was == is {
                        return;
                    }
                    let mut st = state.borrow_mut();
                    if !st.checkboxes {
                        return;
                    }
                    let Some(&source) = st.view.get(nm.iItem as usize) else {
                        return;
                    };
                    if is {
                        st.checked.insert(source);
                    } else {
                        st.checked.remove(&source);
                    }
                    if let Some(mut cb) = st.on_check.take() {
                        drop(st);
                        cb(source, is);
                        let mut st = state.borrow_mut();
                        if st.on_check.is_none() {
                            st.on_check = Some(cb);
                        }
                    }
                }
                #[cfg(not(target_os = "windows"))]
                let _ = lparam;
            }),
        );

        // (2) Sort per clic sull'intestazione di colonna.
        let this = self.clone();
        let id = self.list.id();
        frame.register_lv_column_click_handler(
            id,
            Box::new(move |lparam| {
                let Some(column) = crate::platform::shell::list_column_click_index(lparam) else {
                    return;
                };
                this.toggle_sort(column);
                this.apply_view();
            }),
        );

        // (3) Doppio click: edit in-place se abilitato, altrimenti
        // `on_row_activated`. `LVN_ITEMACTIVATE` porta `NMITEMACTIVATE`
        // con `iItem`/`iSubItem`.
        let state = self.state.clone();
        let list = self.list.clone();
        let frame_c = frame.clone();
        frame.register_lv_item_activate_handler(
            id,
            Box::new(move |lparam| {
                // SAFETY: `lparam` punta a un NMITEMACTIVATE valido per
                // la durata della notifica.
                #[cfg(target_os = "windows")]
                {
                    let nm = lparam as *const windows_sys::Win32::UI::Controls::NMITEMACTIVATE;
                    if nm.is_null() {
                        return;
                    }
                    let (display, col) = unsafe { ((*nm).iItem, (*nm).iSubItem) };
                    if display < 0 || col < 0 {
                        return;
                    }
                    if state.borrow().editable {
                        Self::start_cell_edit(
                            &frame_c,
                            &list,
                            &state,
                            display as usize,
                            col as usize,
                        );
                    }
                }
                #[cfg(not(target_os = "windows"))]
                let _ = lparam;
            }),
        );

        // (4) Commit dell'edit in-place alla perdita di focus: l'EDIT
        // flottante è figlio del frame con id fisso (`edit_id`), quindi
        // `EN_KILLFOCUS` arriva qui via `WM_COMMAND`.
        #[cfg(target_os = "windows")]
        {
            let edit_id = crate::platform::next_control_id();
            self.state.borrow_mut().edit_id = edit_id;
            let state = self.state.clone();
            frame.register_command_notify_handler(
                edit_id,
                cell_edit::EN_KILLFOCUS,
                Box::new(move || {
                    let session = state.borrow().edit.clone();
                    if let Some(session) = session {
                        cell_edit::commit(&session, true);
                    }
                }),
            );
        }
    }

    // ── Dati ──────────────────────────────────────────────────────

    /// Carica una nuova [`DataTable`], allinea le colonne della griglia
    /// e ricostruisce la vista.
    ///
    /// Le colonne non presenti nel nuovo schema vengono eliminate, le
    /// nuove inserite, quelle rinominate aggiornate. Un sort che punta
    /// a una colonna non più esistente viene azzerato; il filtro resta
    /// attivo (le celle vengono ricercate nella tabella nuova).
    pub fn set_data(&self, frame: &Frame, data: DataTable) {
        self.bind(frame);
        self.sync_columns(&data);
        {
            let mut st = self.state.borrow_mut();
            if st.sort_column.is_some_and(|c| c >= data.column_count()) {
                st.sort_column = None;
            }
            st.data = data;
        }
        self.apply_view();
    }

    /// Legge lo stato corrente dei dati senza clonarli.
    ///
    /// Il prestito è limitato alla durata di `f`: utile per il form
    /// di editing (Lotto E) che deve leggere la riga selezionata.
    pub fn with_data<R>(&self, f: impl FnOnce(&DataTable) -> R) -> R {
        let st = self.state.borrow();
        f(&st.data)
    }

    /// Modifica i dati in posto e ricarica la vista.
    ///
    /// Tipico uso: aggiornare la riga di un form e rivederla
    /// immediatamente in griglia.
    pub fn update_data<R>(&self, f: impl FnOnce(&mut DataTable) -> R) -> R {
        let result = {
            let mut st = self.state.borrow_mut();
            f(&mut st.data)
        };
        self.apply_view();
        result
    }

    /// Valore clonato di una cella (**indice di riga sorgente**, non
    /// la posizione nella vista filtrata).
    pub fn cell(&self, row: usize, column: usize) -> Option<DataValue> {
        let st = self.state.borrow();
        st.data.cell(row, column).cloned()
    }

    /// Numero di righe della tabella (non filtrate).
    pub fn data_row_count(&self) -> usize {
        let st = self.state.borrow();
        st.data.row_count()
    }

    /// Numero di colonne dello schema corrente.
    pub fn column_count(&self) -> usize {
        let st = self.state.borrow();
        st.data.column_count()
    }

    // ── Checkbox native (LVS_EX_CHECKBOXES) ─────────────────────────

    /// Abilita/disabilita la colonna checkbox **nativa** del ListView
    /// (`LVS_EX_CHECKBOXES`). Lo stato di spunta vive nel modello
    /// vista (`checked`) e viene ri-erogato a ogni paint via
    /// `LVIF_STATE` — le spunte sopravvivono a sort, filtro e scroll.
    pub fn set_checkboxes(&self, enabled: bool) {
        self.list.set_checkboxes(enabled);
        self.state.borrow_mut().checkboxes = enabled;
        self.refresh();
    }

    /// `true` se la colonna checkbox è abilitata.
    pub fn checkboxes_enabled(&self) -> bool {
        self.state.borrow().checkboxes
    }

    /// `true` se la riga **sorgente** è spuntata.
    pub fn is_checked(&self, source_row: usize) -> bool {
        self.state.borrow().checked.contains(&source_row)
    }

    /// Spunta/dispunta una riga sorgente via codice e ridisegna la
    /// riga corrispondente (se visibile).
    pub fn set_checked(&self, source_row: usize, checked: bool) {
        let display = {
            let mut st = self.state.borrow_mut();
            if checked {
                st.checked.insert(source_row);
            } else {
                st.checked.remove(&source_row);
            }
            st.view.iter().position(|&s| s == source_row)
        };
        if let Some(d) = display {
            self.list.redraw_items(d, d);
        }
    }

    /// Indici sorgente delle righe spuntate, in ordine crescente.
    pub fn checked_rows(&self) -> Vec<usize> {
        let mut rows: Vec<usize> = self.state.borrow().checked.iter().copied().collect();
        rows.sort_unstable();
        rows
    }

    /// Callback invocata quando l'utente spunta/dispunta una riga
    /// (riceve l'indice **sorgente** e il nuovo stato).
    pub fn on_check_changed<F: FnMut(usize, bool) + 'static>(&self, callback: F) {
        self.state.borrow_mut().on_check = Some(Box::new(callback));
    }

    // ── Icone di riga (ImageList nativa) ─────────────────────────────

    /// Attacca una [`ImageList`](crate::ImageList) al ListView (slot
    /// `LVSIL_SMALL`): le icone vengono disegnate a sinistra del testo
    /// della **prima colonna**. La griglia ne conserva un riferimento,
    /// quindi la lista resta viva finché vive la griglia.
    pub fn set_image_list(&self, images: crate::ImageList) {
        self.list.set_image_list(&images);
        self.state.borrow_mut().images = Some(images);
        self.refresh();
    }

    /// Associa un'icona (indice nella image list) a una riga
    /// **sorgente**; `None` la rimuove. Ridisegna la riga se visibile.
    pub fn set_row_image(&self, source_row: usize, image_index: Option<i32>) {
        let display = {
            let mut st = self.state.borrow_mut();
            match image_index {
                Some(index) => {
                    st.row_images.insert(source_row, index);
                }
                None => {
                    st.row_images.remove(&source_row);
                }
            }
            st.view.iter().position(|&s| s == source_row)
        };
        if let Some(d) = display {
            self.list.redraw_items(d, d);
        }
    }

    /// Indice icona associato alla riga sorgente, se presente.
    pub fn row_image(&self, source_row: usize) -> Option<i32> {
        self.state.borrow().row_images.get(&source_row).copied()
    }

    // ── Edit in-place (EDIT nativo sopra la cella) ─────────────────

    /// Abilita l'edit in-place: doppio click su una cella apre un
    /// controllo `EDIT` nativo sopra la cella; **Enter** o la perdita
    /// di focus committano nel modello, **Esc** annulla. Il valore è
    /// parsato secondo il tipo della colonna (testo non valido non
    /// viene salvato).
    pub fn set_editable(&self, enabled: bool) {
        self.state.borrow_mut().editable = enabled;
    }

    /// `true` se l'edit in-place è attivo.
    pub fn is_editable(&self) -> bool {
        self.state.borrow().editable
    }

    /// Apre programmaticamente l'editor sulla cella (riga **sorgente**,
    /// colonna). Ritorna `false` se la riga non è visibile, l'editing
    /// è disattivato o la colonna non è editabile (Blob).
    pub fn begin_edit(&self, frame: &Frame, source_row: usize, column: usize) -> bool {
        if !self.is_editable() {
            return false;
        }
        let display = {
            let st = self.state.borrow();
            st.view.iter().position(|&s| s == source_row)
        };
        let Some(display) = display else {
            return false;
        };
        Self::start_cell_edit(frame, &self.list, &self.state, display, column)
    }

    /// Callback invocata dopo il commit di un edit in-place: riceve
    /// (riga **sorgente**, colonna, nuovo valore). Tipico uso:
    /// persistere la modifica sul database.
    pub fn on_cell_edited<F: FnMut(usize, usize, DataValue) + 'static>(&self, callback: F) {
        self.state.borrow_mut().on_cell_edited = Some(Box::new(callback));
    }

    #[cfg(target_os = "windows")]
    fn start_cell_edit(
        frame: &Frame,
        list: &ListCtrl,
        state: &Rc<RefCell<DataGridState>>,
        display: usize,
        column: usize,
    ) -> bool {
        cell_edit::start(frame, list, state, display, column)
    }

    #[cfg(not(target_os = "windows"))]
    fn start_cell_edit(
        _frame: &Frame,
        _list: &ListCtrl,
        _state: &Rc<RefCell<DataGridState>>,
        _display: usize,
        _column: usize,
    ) -> bool {
        false
    }

    // ── Vista: filtro + sort ──────────────────────────────────────

    /// Applica il filtro testuale (ricerca case-insensitive in tutte
    /// le celle) e ricostruisce la vista.
    pub fn set_filter(&self, filter: &str) {
        self.state.borrow_mut().filter = filter.to_string();
        self.apply_view();
    }

    /// Testo del filtro corrente.
    pub fn filter(&self) -> String {
        self.state.borrow().filter.clone()
    }

    /// Imposta la colonna di ordinamento direttamente (indice
    /// sorgente). `None` torna all'ordine naturale dei dati.
    pub fn set_sort(&self, column: Option<usize>, ascending: bool) {
        {
            let mut st = self.state.borrow_mut();
            st.sort_column = column.filter(|&c| c < st.data.column_count());
            st.sort_ascending = ascending;
        }
        self.apply_view();
    }

    /// Colonna di ordinamento corrente e senso: `(colonna, crescente)`.
    pub fn sort_order(&self) -> Option<(usize, bool)> {
        let st = self.state.borrow();
        st.sort_column.map(|c| (c, st.sort_ascending))
    }

    /// Inverte il senso se `column` è già la colonna di sort, altrimenti
    /// ordina per la nuova colonna in senso crescente.
    fn toggle_sort(&self, column: usize) {
        let mut st = self.state.borrow_mut();
        if st.sort_column == Some(column) {
            st.sort_ascending = !st.sort_ascending;
        } else {
            st.sort_column = Some(column);
            st.sort_ascending = true;
        }
    }

    /// Ricalcola la vista (filtro + sort), aggiorna il conteggio righe
    /// del `ListCtrl` virtuale e forza il repaint.
    ///
    /// Ogni cella verrà riesa a ogni paint attraverso
    /// `LVN_GETDISPINFOW`, quindi invalidare l'area è sufficiente a
    /// mostrare i nuovi dati.
    fn apply_view(&self) {
        let visible = {
            let mut st = self.state.borrow_mut();
            st.view = compute_view(&st.data, &st.filter, st.sort_column, st.sort_ascending);
            st.view.len()
        };
        // Il pretitolo `st` è già rilasciato: `set_item_count` può
        // inviare notifiche che rientrano nella closure del disp-info
        // (la quale a sua volta legge lo stato) senza ribaltare un
        // `RefCell` già preso in prestito.
        self.list.set_item_count(visible as u32);
        self.list.invalidate();
        #[cfg(target_os = "windows")]
        self.update_sort_arrow();
    }

    /// Aggiorna la freccia di sort **nativa** sull'header
    /// (`HDF_SORTUP`/`HDF_SORTDOWN` via `HDM_SETITEMW`): la colonna di
    /// sort mostra la freccia nel senso corrente, le altre la perdono.
    ///
    /// Con hwnd nullo (test headless) è un no-op.
    #[cfg(target_os = "windows")]
    fn update_sort_arrow(&self) {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::Controls::HDITEMW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;

        /// `LVM_GETHEADER` = `LVM_FIRST + 31`.
        const LVM_GETHEADER: u32 = 0x1000 + 31;
        /// `HDM_GETITEMW` = `HDM_FIRST + 11`.
        const HDM_GETITEMW: u32 = 0x1200 + 11;
        /// `HDM_SETITEMW` = `HDM_FIRST + 12`.
        const HDM_SETITEMW: u32 = 0x1200 + 12;
        /// `HDI_FORMAT` — maschera per leggere/scrivere `fmt`.
        const HDI_FORMAT: u32 = 0x0004;
        /// `HDF_SORTUP` / `HDF_SORTDOWN` — freccia nativa di sort.
        const HDF_SORTUP: i32 = 0x0400;
        const HDF_SORTDOWN: i32 = 0x0200;

        let hwnd = self.list.as_widget_ref().borrow().native_handle() as HWND;
        if hwnd.is_null() {
            return;
        }
        // SAFETY: hwnd è il ListView vivo della griglia.
        let header = unsafe { SendMessageW(hwnd, LVM_GETHEADER, 0, 0) as HWND };
        if header.is_null() {
            return;
        }
        let (sort_col, ascending, cols) = {
            let st = self.state.borrow();
            (st.sort_column, st.sort_ascending, st.col_titles.len())
        };
        for index in 0..cols {
            // SAFETY: `item` è una HDITEMW valida; l'header è vivo.
            unsafe {
                let mut item: HDITEMW = std::mem::zeroed();
                item.mask = HDI_FORMAT;
                if SendMessageW(header, HDM_GETITEMW, index, &mut item as *mut _ as isize) == 0 {
                    continue;
                }
                item.fmt &= !(HDF_SORTUP | HDF_SORTDOWN);
                if Some(index) == sort_col {
                    item.fmt |= if ascending { HDF_SORTUP } else { HDF_SORTDOWN };
                }
                SendMessageW(header, HDM_SETITEMW, index, &mut item as *mut _ as isize);
            }
        }
    }

    /// Ricarica la vista dopo aver modificato i dati (equivalente a
    /// un refresh dei soli contenuti; lo schema colonne resta quello di
    /// [`set_data`](Self::set_data)).
    pub fn refresh(&self) {
        self.apply_view();
    }

    /// Indici di riga **sorgente** visibili dopo l'ultimo
    /// filtro/sort.
    pub fn visible_row_indices(&self) -> Vec<usize> {
        self.state.borrow().view.clone()
    }

    /// Converte una posizione della vista (quella mostrata in griglia e
    /// restituita dagli eventi) nell'indice di riga sorgente.
    pub fn display_to_source(&self, display: usize) -> Option<usize> {
        let st = self.state.borrow();
        st.view.get(display).copied()
    }

    /// Converte un indice di riga sorgente nella posizione che occupa
    /// nella vista corrente, o `None` se il filtro l'ha esclusa.
    pub fn source_to_display(&self, source: usize) -> Option<usize> {
        let st = self.state.borrow();
        st.view.iter().position(|&r| r == source)
    }

    /// Righe visibili in griglia (dopo filtro/sort).
    pub fn row_count(&self) -> usize {
        self.state.borrow().view.len()
    }

    /// Righe selezionate, come indici **sorgente**.
    pub fn selected_rows(&self) -> Vec<usize> {
        let st = self.state.borrow();
        self.list
            .get_selected_items()
            .iter()
            .filter_map(|&display| st.view.get(display).copied())
            .collect()
    }

    // ── Eventi ────────────────────────────────────────────────────

    /// Callback per la selezione di una riga: riceve l'indice
    /// **sorgente** della riga selezionata, oppure `None` quando la
    /// selezione viene azzerata.
    ///
    /// Gli indici sono quelli della [`DataTable`] (non le posizioni
    /// filtrate), così da poter leggere i valori con
    /// [`cell`](Self::cell) senza conversioni.
    pub fn on_selection_changed<F: FnMut(Option<usize>) + 'static>(
        &self,
        frame: &Frame,
        mut callback: F,
    ) {
        let state = self.state.clone();
        self.list.on_item_selected(frame, move |selection| {
            let mapped = selection.and_then(|display| {
                let st = state.borrow();
                st.view.get(display).copied()
            });
            callback(mapped);
        });
    }

    /// Callback per il doppio click su una riga (attivazione): riceve
    /// l'indice **sorgente** della riga. Scatta solo se l'edit
    /// in-place è disattivato ([`set_editable`](Self::set_editable)):
    /// con l'editing attivo il doppio click apre l'editor di cella.
    pub fn on_row_activated<F: FnMut(usize) + 'static>(&self, _frame: &Frame, callback: F) {
        self.state.borrow_mut().on_row_activated_cb = Some(Box::new(callback));
    }

    // ── Accesso al controllo sottostante ──────────────────────────

    /// Il `ListCtrl` sottostante: permette di usare le API non
    /// coperte dalla griglia (checkbox, colonne extra, ricerca
    /// incremental, …).
    ///
    /// Attenzione: `set_extended_style` usa mask 0 e sovrascrive tutti
    /// i bit; per aggiungere le checkbox senza perdere la selezione a
    /// riga intera usi [`ListCtrl::set_checkboxes`](crate::controls::list_ctrl::ListCtrl::set_checkboxes),
    /// che opera con la maschera corretta.
    pub fn list(&self) -> &ListCtrl {
        &self.list
    }

    /// Riferimento al widget, per inserirlo in un sizer.
    pub fn as_widget_ref(&self) -> WidgetRef {
        self.list.as_widget_ref()
    }

    /// Allinea le colonne del `ListCtrl` con quelle della `DataTable`.
    ///
    /// Vengono toccate solo le colonne che sono cambiate: il numero di
    /// colonne diverso comporta inserimenti/eliminazioni, un semplice
    /// cambio di intestazione comporta la ricostruzione dell'intera
    /// riga di intestazioni.
    fn sync_columns(&self, data: &DataTable) {
        let titles: Vec<String> = data.columns.iter().map(|c| c.name.clone()).collect();
        let current: Vec<String> = self.state.borrow().col_titles.clone();

        if titles == current {
            return;
        }

        let have = current.len();
        let want = titles.len();

        if have != want {
            // Elimina in coda → testa (gli indici colonna si
            // ri-compattano da soli), poi inserisce quelle nuove.
            for index in (want..have).rev() {
                self.list.delete_column(index as u32);
            }
            for (index, column) in data.columns.iter().enumerate().skip(have) {
                self.list
                    .insert_column(index as u32, &column.name, column_width(column));
            }
        } else {
            // Stesso numero di colonne ma nomi diversi: ricostruire
            // l'intera testata è l'unico modo per aggiornare i titoli
            // (non esiste un `set_column_title` nel ListView Win32
            // usato da `ListCtrl`).
            for index in (0..have).rev() {
                self.list.delete_column(index as u32);
            }
            for (index, column) in data.columns.iter().enumerate() {
                self.list
                    .insert_column(index as u32, &column.name, column_width(column));
            }
        }

        self.state.borrow_mut().col_titles = titles;
    }
}

// ── Funzioni pure (testabili senza UI) ───────────────────────────────

/// Calcola gli indici di riga visibili: filtro case-insensitive su
/// tutte le celle, poi ordinamento per `sort_column` (i valori
/// numerici restano numerici, `NULL` precedono i valori).
fn compute_view(
    data: &DataTable,
    filter: &str,
    sort_column: Option<usize>,
    ascending: bool,
) -> Vec<usize> {
    let needle = filter.trim().to_lowercase();
    let mut view: Vec<usize> = (0..data.row_count())
        .filter(|&row| {
            if needle.is_empty() {
                return true;
            }
            data.row(row).is_some_and(|cells| {
                cells
                    .iter()
                    .any(|value| value.to_display_string().to_lowercase().contains(&needle))
            })
        })
        .collect();

    if let Some(column) = sort_column {
        if column < data.column_count() {
            view.sort_by(|&a, &b| {
                let ordering = match (data.cell(a, column), data.cell(b, column)) {
                    (Some(left), Some(right)) => compare_values(left, right),
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => Ordering::Equal,
                };
                if ascending {
                    ordering
                } else {
                    ordering.reverse()
                }
            });
        }
    }
    view
}

/// Confronto tipi-consapevole tra due celle: due valori convertibili
/// in numero si confrontano numericamente (`2 < 10`, non `"10" < "2"`),
/// altrimenti si confronta il testo in minuscolo.
fn compare_values(left: &DataValue, right: &DataValue) -> Ordering {
    match (left.as_f64(), right.as_f64()) {
        (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
        _ => left
            .to_display_string()
            .to_lowercase()
            .cmp(&right.to_display_string().to_lowercase()),
    }
}

/// Larghezza predefinita di una colonna: spazio per il nome
/// dell'intestazione (fra 140 e 280 px), esteso a un minimo legato al
/// tipo quando serve (es. un `DATETIME` non sta in 140 px).
fn column_width(column: &DataColumn) -> i32 {
    let type_min = match column.data_type {
        DataFieldType::DateTime => 150,
        DataFieldType::Date => 110,
        DataFieldType::Integer | DataFieldType::Real => 90,
        DataFieldType::Blob => 90,
        DataFieldType::Boolean => 70,
        DataFieldType::Text => 140,
    };
    let by_name = (column.name.chars().count() as i32) * 8 + 40;
    by_name.clamp(DEFAULT_COLUMN_WIDTH, 280).max(type_min)
}

// ── Edit in-place: EDIT nativo flottante ─────────────────────────────

/// Meccanica Win32 dell'editor di cella.
///
/// L'`EDIT` è figlio del **frame** (non del ListView) così
/// `EN_KILLFOCUS` arriva al dispatch `WM_COMMAND` del frame; le
/// coordinate della cella (`LVM_GETSUBITEMRECT`, client del ListView)
/// sono convertite in coordinate client del frame via
/// `ClientToScreen`/`ScreenToClient`. Enter/Esc sono intercettati da
/// una wndproc di subclass (`GWLP_WNDPROC`); il commit è idempotente
/// (flag `done`) perché Enter → `DestroyWindow` → `EN_KILLFOCUS`
/// chiamerebbe due volte.
#[cfg(target_os = "windows")]
mod cell_edit {
    use super::*;
    use std::collections::HashMap;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
    use windows_sys::Win32::Graphics::Gdi::{ClientToScreen, ScreenToClient};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{SetFocus, VK_ESCAPE, VK_RETURN};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW,
        GetWindowTextW, SendMessageW, SetWindowLongPtrW, GWLP_WNDPROC, HMENU, WM_GETFONT,
        WM_KEYDOWN, WM_NCDESTROY, WM_SETFONT, WS_BORDER, WS_CHILD, WS_VISIBLE,
    };

    use crate::platform::win32::to_wide;

    /// `LVM_GETSUBITEMRECT` = `LVM_FIRST + 56`.
    const LVM_GETSUBITEMRECT: u32 = 0x1000 + 56;
    /// `LVIR_BOUNDS` — rect completo della cella.
    const LVIR_BOUNDS: i32 = 0;
    /// `EM_SETSEL` — selezione del testo dell'EDIT.
    const EM_SETSEL: u32 = 0x00B1;
    /// `EN_KILLFOCUS` — notification code `WM_COMMAND` (HIWORD).
    pub const EN_KILLFOCUS: u16 = 0x0200;
    /// `ES_AUTOHSCROLL` — scroll orizzontale automatico dell'EDIT
    /// (in windows-sys è `i32`, lo ridefiniamo `u32` per lo style).
    const ES_AUTOHSCROLL: u32 = 0x0080;

    /// Sessione di edit attiva su una cella.
    pub struct EditSession {
        pub edit_hwnd: HWND,
        pub source_row: usize,
        pub display_row: usize,
        pub column: usize,
        pub done: bool,
        pub original_wndproc: isize,
        pub state: Rc<RefCell<DataGridState>>,
        pub list: ListCtrl,
    }

    thread_local! {
        /// Sessioni vive, mappate per HWND dell'EDIT (serve alla
        /// wndproc statica di subclass).
        static SESSIONS: RefCell<HashMap<isize, Rc<RefCell<EditSession>>>> =
            RefCell::new(HashMap::new());
    }

    /// Parsa il testo editato secondo il tipo della colonna.
    /// `None` = valore non valido (non salvato).
    fn parse_cell_value(column: &DataColumn, text: &str) -> Option<DataValue> {
        let t = text.trim();
        if t.is_empty() {
            return if column.nullable {
                Some(DataValue::Null)
            } else {
                None
            };
        }
        match column.data_type {
            DataFieldType::Text => Some(DataValue::Text(text.to_string())),
            DataFieldType::Integer => t.parse::<i64>().ok().map(DataValue::Int),
            DataFieldType::Real => t.replace(',', ".").parse::<f64>().ok().map(DataValue::Real),
            DataFieldType::Boolean => match t.to_lowercase().as_str() {
                "true" | "1" | "si" | "sì" | "yes" => Some(DataValue::Bool(true)),
                "false" | "0" | "no" => Some(DataValue::Bool(false)),
                _ => None,
            },
            // Date/DateTime sono `Text` ISO nella DataTable: il parsing
            // rigoroso è compito del form (DatePickerCtrl).
            DataFieldType::Date | DataFieldType::DateTime => {
                Some(DataValue::Text(text.to_string()))
            }
            DataFieldType::Blob => None,
        }
    }

    /// Apre l'EDIT sopra la cella (`display`, `column`).
    pub fn start(
        frame: &Frame,
        list: &ListCtrl,
        state: &Rc<RefCell<DataGridState>>,
        display: usize,
        column: usize,
    ) -> bool {
        // Chiude (committando) un eventuale edit precedente.
        let previous = state.borrow().edit.clone();
        if let Some(previous) = previous {
            commit(&previous, true);
        }

        let (text, edit_id, source) = {
            let st = state.borrow();
            let Some(&source) = st.view.get(display) else {
                return false;
            };
            let Some(col_meta) = st.data.columns.get(column) else {
                return false;
            };
            if col_meta.data_type == DataFieldType::Blob || st.edit_id == 0 {
                return false;
            }
            let text = st
                .data
                .cell(source, column)
                .map(|v| v.to_display_string())
                .unwrap_or_default();
            (text, st.edit_id, source)
        };

        let list_hwnd = list.as_widget_ref().borrow().native_handle() as HWND;
        let frame_hwnd = crate::platform::window_handle_as_hwnd(frame);

        // Rect della cella in coordinate client del ListView.
        // MSDN: in ingresso `RECT.top` = indice subitem, `RECT.left` =
        // LVIR_* (non il contrario).
        // SAFETY: `rc` è un buffer valido; il controllo è vivo.
        let rc = unsafe {
            let mut rc = RECT {
                left: LVIR_BOUNDS,
                top: column as i32,
                right: 0,
                bottom: 0,
            };
            let ok = SendMessageW(
                list_hwnd,
                LVM_GETSUBITEMRECT,
                display,
                &mut rc as *mut RECT as isize,
            );
            if ok == 0 {
                return false;
            }
            rc
        };

        // ListView-client → schermo → frame-client.
        // SAFETY: HWND validi; le POINT sono buffer out.
        let (p1, p2) = unsafe {
            let mut p1 = POINT {
                x: rc.left,
                y: rc.top,
            };
            let mut p2 = POINT {
                x: rc.right,
                y: rc.bottom,
            };
            ClientToScreen(list_hwnd, &mut p1);
            ClientToScreen(list_hwnd, &mut p2);
            ScreenToClient(frame_hwnd, &mut p1);
            ScreenToClient(frame_hwnd, &mut p2);
            (p1, p2)
        };

        // SAFETY: creazione finestra figlia con class/testo wide validi.
        let hedit = unsafe {
            let class = to_wide("EDIT");
            let title = to_wide(&text);
            CreateWindowExW(
                0,
                class.as_ptr(),
                title.as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_BORDER | ES_AUTOHSCROLL,
                p1.x,
                p1.y,
                (p2.x - p1.x).max(40),
                (p2.y - p1.y).max(18),
                frame_hwnd,
                edit_id as usize as HMENU,
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null(),
            )
        };
        if hedit.is_null() {
            return false;
        }

        // SAFETY: hedit/list_hwnd sono finestre vive di questo thread.
        unsafe {
            // Stessa font del ListView, per coerenza visiva.
            let font = SendMessageW(list_hwnd, WM_GETFONT, 0, 0);
            if font != 0 {
                SendMessageW(hedit, WM_SETFONT, font as usize, 1);
            }
            SendMessageW(hedit, EM_SETSEL, 0, -1);

            let original_wndproc = GetWindowLongPtrW(hedit, GWLP_WNDPROC);
            let session = Rc::new(RefCell::new(EditSession {
                edit_hwnd: hedit,
                source_row: source,
                display_row: display,
                column,
                done: false,
                original_wndproc,
                state: state.clone(),
                list: list.clone(),
            }));
            SESSIONS.with(|m| m.borrow_mut().insert(hedit as isize, session.clone()));
            state.borrow_mut().edit = Some(session);
            SetWindowLongPtrW(
                hedit,
                GWLP_WNDPROC,
                edit_wnd_proc as *const () as usize as isize,
            );
            SetFocus(hedit);
        }
        true
    }

    /// Chiude la sessione: con `save` scrive il valore parsato nel
    /// modello e ridisegna la riga; poi distrugge l'EDIT.
    pub fn commit(session: &Rc<RefCell<EditSession>>, save: bool) {
        let (hwnd, source, display, column, state, list) = {
            let mut s = session.borrow_mut();
            if s.done {
                return;
            }
            s.done = true;
            (
                s.edit_hwnd,
                s.source_row,
                s.display_row,
                s.column,
                s.state.clone(),
                s.list.clone(),
            )
        };

        if save {
            let mut buf = [0u16; 512];
            // SAFETY: buffer wide valido di 512 elementi.
            let len =
                unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) }.max(0) as usize;
            let text = String::from_utf16_lossy(&buf[..len]);
            let mut callback = None;
            {
                let mut st = state.borrow_mut();
                if let Some(col_meta) = st.data.columns.get(column) {
                    if let Some(value) = parse_cell_value(col_meta, &text) {
                        if let Some(row) = st.data.rows.get_mut(source) {
                            if column < row.len() {
                                row[column] = value.clone();
                                callback = st.on_cell_edited.take().map(|c| (c, value));
                            }
                        }
                    }
                }
            }
            list.redraw_items(display, display);
            if let Some((mut cb, value)) = callback {
                cb(source, column, value);
                let mut st = state.borrow_mut();
                if st.on_cell_edited.is_none() {
                    st.on_cell_edited = Some(cb);
                }
            }
        }

        state.borrow_mut().edit = None;
        // SAFETY: l'EDIT è una finestra figlia creata da `start`.
        unsafe {
            DestroyWindow(hwnd);
            // Rimanda il focus alla lista (Enter/killfocus lo avevano
            // spostato sull'EDIT).
            SetFocus(list.as_widget_ref().borrow().native_handle() as HWND);
        }
    }

    /// Wndproc di subclass dell'EDIT: Enter = commit, Esc = annulla;
    /// in `WM_NCDESTROY` ripristina la wndproc originale e rilascia la
    /// sessione dalla mappa thread-local.
    unsafe extern "system" fn edit_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_KEYDOWN {
            let key = wparam as u32;
            if key == VK_RETURN as u32 || key == VK_ESCAPE as u32 {
                let session = SESSIONS.with(|m| m.borrow().get(&(hwnd as isize)).cloned());
                if let Some(session) = session {
                    commit(&session, key == VK_RETURN as u32);
                }
                return 0;
            }
        }
        if msg == WM_NCDESTROY {
            let original = SESSIONS
                .with(|m| m.borrow_mut().remove(&(hwnd as isize)))
                .map(|s| s.borrow().original_wndproc);
            if let Some(original) = original {
                SetWindowLongPtrW(hwnd, GWLP_WNDPROC, original);
                // SAFETY: wndproc originale appena ripristinata.
                let original: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT =
                    std::mem::transmute(original);
                return CallWindowProcW(Some(original), hwnd, msg, wparam, lparam);
            }
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
        let original = SESSIONS.with(|m| {
            m.borrow()
                .get(&(hwnd as isize))
                .map(|s| s.borrow().original_wndproc)
        });
        match original {
            // SAFETY: wndproc originale catturata in `start`.
            Some(o) => {
                let o: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT =
                    std::mem::transmute(o);
                CallWindowProcW(Some(o), hwnd, msg, wparam, lparam)
            }
            None => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    use crate::data::DataFieldType;

    fn sample_table() -> DataTable {
        DataTable::from_rows(
            "prodotti",
            vec![
                DataColumn::new("nome", DataFieldType::Text),
                DataColumn::new("quantita", DataFieldType::Integer),
            ],
            vec![
                vec![DataValue::Text("alfa".into()), DataValue::Int(10)],
                vec![DataValue::Text("beta".into()), DataValue::Int(2)],
                vec![DataValue::Text("gamma".into()), DataValue::Int(30)],
            ],
        )
    }

    #[test]
    fn view_filters_case_insensitively() {
        let data = sample_table();
        assert_eq!(compute_view(&data, "", None, true), vec![0, 1, 2]);
        assert_eq!(compute_view(&data, "BET", None, true), vec![1]);
        assert_eq!(compute_view(&data, "zzz", None, true), Vec::<usize>::new());
    }

    #[test]
    fn view_sorts_numerically() {
        let data = sample_table();
        // Senso numerico: 2 < 10 < 30 (non ordine lessicografico).
        assert_eq!(compute_view(&data, "", Some(1), true), vec![1, 0, 2]);
        assert_eq!(compute_view(&data, "", Some(1), false), vec![2, 0, 1]);
        // Sort testuale sulla prima colonna.
        assert_eq!(compute_view(&data, "", Some(0), true), vec![0, 1, 2]);
    }

    #[test]
    fn grid_maps_display_rows_to_source_rows() {
        let frame = Frame::for_testing();
        let grid = DataGrid::new(&frame);
        grid.set_data(&frame, sample_table());
        assert_eq!(grid.row_count(), 3);
        assert_eq!(grid.column_count(), 2);

        grid.set_filter("beta");
        assert_eq!(grid.row_count(), 1);
        assert_eq!(grid.visible_row_indices(), vec![1]);
        assert_eq!(grid.display_to_source(0), Some(1));
        assert_eq!(grid.source_to_display(1), Some(0));
        assert_eq!(grid.source_to_display(0), None);
        assert_eq!(grid.cell(1, 0), Some(DataValue::Text("beta".into())));

        grid.set_filter("");
        grid.set_sort(Some(1), false);
        assert_eq!(grid.visible_row_indices(), vec![2, 0, 1]);
        assert_eq!(grid.sort_order(), Some((1, false)));
    }

    /// Simula end-to-end la richiesta `LVN_GETDISPINFOW` del ListView:
    /// la `WM_NOTIFY` entra nella wndproc del frame reale, il dispatch
    /// deve raggiungere l'handler della griglia e riempire il buffer
    /// con il testo della cella. È la catena che rende visibili le
    /// righe a runtime.
    #[test]
    fn disp_info_riempe_il_testo_della_cella() {
        use windows_sys::Win32::UI::Controls::{LVIF_TEXT, LVITEMW, NMLVDISPINFOW};
        use windows_sys::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_NOTIFY};

        let frame = Frame::builder()
            .with_title("data-grid-disp-test")
            .with_size(300, 200)
            .build();
        let grid = DataGrid::new(&frame);
        grid.set_data(&frame, sample_table());

        let mut buf = [0u16; 64];
        // SAFETY: strutture POD azzerate; `buf` vive quanto `nm`.
        let mut nm: NMLVDISPINFOW = unsafe { std::mem::zeroed() };
        nm.hdr.idFrom = grid.list().id() as usize;
        nm.hdr.code = crate::controls::list_ctrl::LVN_GETDISPINFOW;
        let mut item: LVITEMW = unsafe { std::mem::zeroed() };
        item.mask = LVIF_TEXT;
        item.iItem = 0;
        item.iSubItem = 0;
        item.pszText = buf.as_mut_ptr();
        item.cchTextMax = buf.len() as i32;
        nm.item = item;

        let frame_hwnd = crate::platform::window_handle_as_hwnd(&frame);
        // SAFETY: `nm` è una NMLVDISPINFOW valida per la durata della
        // chiamata sincrona; il frame è una finestra reale.
        unsafe {
            SendMessageW(
                frame_hwnd,
                WM_NOTIFY,
                grid.list().id() as usize,
                &mut nm as *mut _ as isize,
            );
        }

        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        let text = String::from_utf16_lossy(&buf[..len]);
        assert_eq!(text, "alfa");
    }

    /// Dopo `set_data` il controllo nativo deve essere in modalità
    /// `LVS_OWNERDATA` con il conteggio righe corretto: è la
    /// precondizione perché il paint chieda le celle.
    #[test]
    fn il_controllo_nativo_e_virtuale_con_il_count_giusto() {
        use windows_sys::Win32::UI::Controls::{LVM_GETITEMCOUNT, LVS_OWNERDATA};
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SendMessageW, GWL_STYLE,
        };

        let frame = Frame::builder()
            .with_title("data-grid-style-test")
            .with_size(300, 200)
            .build();
        let grid = DataGrid::new(&frame);
        grid.set_data(&frame, sample_table());

        let hwnd = grid.list().as_widget_ref().borrow().native_handle();
        // SAFETY: HWND del ListView appena creato, ancora vivo.
        let (style, count) = unsafe {
            (
                GetWindowLongPtrW(hwnd as _, GWL_STYLE) as u32,
                SendMessageW(hwnd as _, LVM_GETITEMCOUNT, 0, 0) as u32,
            )
        };
        assert_ne!(style & LVS_OWNERDATA, 0, "LVS_OWNERDATA non attivo");
        assert_eq!(count, 3, "LVM_GETITEMCOUNT");
    }

    /// Checkbox native: il toggle arriva via `LVN_ITEMCHANGED` con lo
    /// state image nel `NMLISTVIEW`, e lo stato viene ri-erogato a
    /// ogni paint via `LVIF_STATE`.
    #[test]
    fn checkbox_native_toggle_e_stato() {
        use windows_sys::Win32::UI::Controls::{LVITEMW, NMLISTVIEW};
        use windows_sys::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_NOTIFY};

        let frame = Frame::builder()
            .with_title("data-grid-check-test")
            .with_size(300, 200)
            .build();
        let grid = DataGrid::new(&frame);
        grid.set_data(&frame, sample_table());
        grid.set_checkboxes(true);
        assert!(grid.checkboxes_enabled());
        assert!(!grid.is_checked(0));

        let frame_hwnd = crate::platform::window_handle_as_hwnd(&frame);
        // Simula il click sulla checkbox della riga display 0
        // (sorgente 0): LVIF_STATE con state image 1 → 2.
        let mut nm: NMLISTVIEW = unsafe { std::mem::zeroed() };
        nm.hdr.idFrom = grid.list().id() as usize;
        nm.hdr.code = crate::controls::list_ctrl::LVN_ITEMCHANGED;
        nm.iItem = 0;
        nm.uChanged = 0x8; // LVIF_STATE
        nm.uOldState = 0x1000; // INDEXTOSTATEIMAGEMASK(1)
        nm.uNewState = 0x2000; // INDEXTOSTATEIMAGEMASK(2)
        unsafe {
            SendMessageW(
                frame_hwnd,
                WM_NOTIFY,
                grid.list().id() as usize,
                &mut nm as *mut _ as isize,
            );
        }
        assert!(grid.is_checked(0), "toggle non registrato");
        assert_eq!(grid.checked_rows(), vec![0]);

        // Il disp-info deve ri-erogare lo state per la riga spuntata.
        let mut item: LVITEMW = unsafe { std::mem::zeroed() };
        item.mask = 0x8; // LVIF_STATE
        item.iItem = 0;
        let mut disp: windows_sys::Win32::UI::Controls::NMLVDISPINFOW =
            unsafe { std::mem::zeroed() };
        disp.hdr.idFrom = grid.list().id() as usize;
        disp.hdr.code = crate::controls::list_ctrl::LVN_GETDISPINFOW;
        disp.item = item;
        unsafe {
            SendMessageW(
                frame_hwnd,
                WM_NOTIFY,
                grid.list().id() as usize,
                &mut disp as *mut _ as isize,
            );
        }
        assert_eq!(disp.item.state & 0x2000, 0x2000, "state image non erogato");

        // Toggle inverso via API.
        grid.set_checked(0, false);
        assert!(!grid.is_checked(0));
        assert!(grid.checked_rows().is_empty());
    }

    /// Edit in-place: apertura programmatica, commit con Enter (via la
    /// wndproc subclassata) e rifiuto di valori non parsabili.
    #[test]
    fn edit_in_place_commit_e_annulla() {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_RETURN;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SendMessageW, SetWindowTextW, WM_KEYDOWN,
        };

        let frame = Frame::builder()
            .with_title("data-grid-edit-test")
            .with_size(400, 300)
            .build();
        let grid = DataGrid::new(&frame);
        grid.set_data(&frame, sample_table());

        // Senza set_editable, begin_edit non fa nulla.
        assert!(!grid.begin_edit(&frame, 0, 0));
        grid.set_editable(true);
        assert!(grid.is_editable());

        // Apre l'editor sulla cella (sorgente 0, colonna "nome").
        assert!(grid.begin_edit(&frame, 0, 0));
        let session = grid.state.borrow().edit.clone().expect("sessione aperta");
        let edit_hwnd = session.borrow().edit_hwnd;

        // Scrive il nuovo valore e committa con Enter.
        let wide = crate::platform::win32::to_wide("omega");
        unsafe {
            SetWindowTextW(edit_hwnd, wide.as_ptr());
            SendMessageW(edit_hwnd, WM_KEYDOWN, VK_RETURN as usize, 0);
        }
        assert_eq!(grid.cell(0, 0), Some(DataValue::Text("omega".to_string())));
        assert!(grid.state.borrow().edit.is_none(), "sessione chiusa");

        // Valore non valido su colonna Integer ("quantita"): non salvato.
        assert!(grid.begin_edit(&frame, 0, 1));
        let session = grid.state.borrow().edit.clone().expect("sessione aperta");
        let edit_hwnd = session.borrow().edit_hwnd;
        let wide = crate::platform::win32::to_wide("abc");
        unsafe {
            SetWindowTextW(edit_hwnd, wide.as_ptr());
            SendMessageW(edit_hwnd, WM_KEYDOWN, VK_RETURN as usize, 0);
        }
        assert_eq!(
            grid.cell(0, 1),
            Some(DataValue::Int(10)),
            "parse fallito: valore invariato"
        );
    }

    /// Stili nativi: `LVS_EX_DOUBLEBUFFER` attivo e freccia di sort
    /// sull'header dopo `set_sort`.
    #[test]
    fn double_buffer_e_freccia_sort_nativi() {
        use windows_sys::Win32::UI::Controls::HDITEMW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;

        const LVM_GETEXTENDEDLISTVIEWSTYLE: u32 = 0x1000 + 55;
        const LVM_GETHEADER: u32 = 0x1000 + 31;
        const HDM_GETITEMW: u32 = 0x1200 + 11;
        const HDI_FORMAT: u32 = 0x0004;
        const HDF_SORTUP: i32 = 0x0400;
        const HDF_SORTDOWN: i32 = 0x0200;

        let frame = Frame::builder()
            .with_title("data-grid-native-test")
            .with_size(300, 200)
            .build();
        let grid = DataGrid::new(&frame);
        grid.set_data(&frame, sample_table());

        let hwnd_isize = grid.list().as_widget_ref().borrow().native_handle();
        // SAFETY: HWND del ListView appena creato, ancora vivo.
        let ex =
            unsafe { SendMessageW(hwnd_isize as _, LVM_GETEXTENDEDLISTVIEWSTYLE, 0, 0) as u32 };
        assert_ne!(ex & LVS_EX_DOUBLEBUFFER, 0, "double buffer non attivo");

        // Freccia crescente sulla colonna 1 dopo il sort.
        grid.set_sort(Some(1), true);
        let header_fmt = |index: usize| unsafe {
            let header = SendMessageW(hwnd_isize as _, LVM_GETHEADER, 0, 0);
            let mut item: HDITEMW = std::mem::zeroed();
            item.mask = HDI_FORMAT;
            SendMessageW(
                header as _,
                HDM_GETITEMW,
                index,
                &mut item as *mut _ as isize,
            );
            item.fmt
        };
        assert_ne!(header_fmt(1) & HDF_SORTUP, 0, "freccia su assente");
        assert_eq!(header_fmt(0) & (HDF_SORTUP | HDF_SORTDOWN), 0);

        // Invertendo il senso la freccia diventa discendente.
        grid.set_sort(Some(1), false);
        assert_ne!(header_fmt(1) & HDF_SORTDOWN, 0, "freccia giù assente");
        assert_eq!(header_fmt(1) & HDF_SORTUP, 0);
    }

    /// Icone di riga: la richiesta `LVIF_IMAGE` del ListView virtuale
    /// riceve l'indice associato con `set_row_image`.
    #[test]
    fn icone_riga_erogate_in_disp_info() {
        use windows_sys::Win32::UI::Controls::{LVIF_IMAGE, LVITEMW, NMLVDISPINFOW};
        use windows_sys::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_NOTIFY};

        let frame = Frame::builder()
            .with_title("data-grid-icons-test")
            .with_size(300, 200)
            .build();
        let grid = DataGrid::new(&frame);
        grid.set_data(&frame, sample_table());
        grid.set_image_list(crate::ImageList::new(16, 16));
        grid.set_row_image(1, Some(2));
        assert_eq!(grid.row_image(1), Some(2));
        assert_eq!(grid.row_image(0), None);

        // Simula LVN_GETDISPINFOW con LVIF_IMAGE per la riga display
        // che mappa la sorgente 1 (senza sort è la display 1).
        let mut disp: NMLVDISPINFOW = unsafe { std::mem::zeroed() };
        disp.hdr.idFrom = grid.list().id() as usize;
        disp.hdr.code = crate::controls::list_ctrl::LVN_GETDISPINFOW;
        let mut item: LVITEMW = unsafe { std::mem::zeroed() };
        item.mask = LVIF_IMAGE;
        item.iItem = 1;
        disp.item = item;

        let frame_hwnd = crate::platform::window_handle_as_hwnd(&frame);
        // SAFETY: `disp` è una NMLVDISPINFOW valida per la chiamata
        // sincrona; il frame è una finestra reale.
        unsafe {
            SendMessageW(
                frame_hwnd,
                WM_NOTIFY,
                grid.list().id() as usize,
                &mut disp as *mut _ as isize,
            );
        }
        assert_eq!(disp.item.iImage, 2, "indice icona non erogato");

        // Rimozione: la riga torna senza icona.
        grid.set_row_image(1, None);
        assert_eq!(grid.row_image(1), None);
    }
}
