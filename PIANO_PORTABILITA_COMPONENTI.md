# Piano di portabilità componenti — ru_wx (macOS ↔ Windows)

> **Scopo**: portare i componenti avanzati ereditati da wxWidgets (e i gap rilevati) a
> parità di API su **macOS (AppKit)** e **Windows (Win32 / WinUI3-XAML)**, usando
> **una sola definizione "in mezzo"** sul crate `ru_wx`, con **grafica moderna**
> su entrambe le piattaforme e **ispettore di debug** previsto fin dall'inizio.
>
> **Stato**: proposto — in attesa di approvazione prima dell'implementazione.
>
> **Riferimenti collegati**: `PIANO_DEBUG_INSPECTOR.md`, `PIANO_SVILUPPO_WINUI3_XAML.md`,
> `DIRETTIVE_PROGETTO.md`.

---

## 1. Principi guida

1. **Definizione unica in mezzo** — ogni componente ha *una* sola firma pubblicata da
   `ru_wx` (`src/controls/*.rs`, `src/containers/*.rs`). Il backend è un dettaglio
   interno raggiunto tramite `#[cfg(target_os)]`. Nessuna API duplicata per OS,
   nessun tipo pubblico esposto diverso tra mac e Windows.
2. **Nessuna FFI nuova non necessaria** — si riusano le primitive esistenti
   (`src/platform/win32.rs`, `src/platform/appkit*.rs`). Una nuova FFI si aggiunge
   solo se il controllo nativo non è già raggiungibile.
3. **Simmetria dei ruoli inspector** — lo stesso componente produce lo **stesso ruolo
   logico** (`button|check|radio|label|input|choice|list|table|tree|tabs|value|scroll|
   picker|image|view|dialog|menu|toolbar|status|group|webview`) su Win32, AppKit e
   XAML, così `/find?role=` e `/tree` rendono i test trasversali automatizzabili.
4. **Grafica moderna** — niente imitazioni XP: Fluent/WinUI3 su Windows
   (Mica, corner preference, accent di sistema), AppKit moderno su macOS
   (vibrancy, SF Symbols, accent di sistema). I valori condivisi sono i **token §5**.
5. **`cfg` pulito** — tre rami ovunque, nel pattern già consolidato:
   `#[cfg(target_os = "windows")]` / `#[cfg(target_os = "macos")]` /
   `#[cfg(all(not(windows), not(macos)))]` (stub no-op con `let _ = ...`).
6. **Inspector come volano** — l'inspector osserva, non invade: nessun costo sul
   percorso di rendering in release, primitive isolate in
   `inspector_{macos,windows}.rs`, schema condiviso senza FFI in `inspector.rs`.

---

## 2. Architettura a 4 livelli

```
App utente
   │  API canonica (una sola definizione "in mezzo")
   ▼
ru_wx  — src/controls/*.rs · src/containers/*.rs · src/adv/*.rs · src/chrome/*.rs
   │  dispatch #[cfg(target_os)]
   ├──────────────► src/platform/win32.rs            (Windows, nativo)
   ├──────────────► src/platform/appkit*.rs          (macOS, nativo)
   └──────────────► src/platform/winui3_xaml/        (Windows, feature winui3_xaml)

Inspector (volano, stesso pattern):
   src/platform/inspector.rs          → schema condiviso, zero FFI
   src/platform/inspector_windows.rs  → primitive Win32 (EnumChildWindows, GetWindowText…)
   src/platform/inspector_macos.rs    → primitive AppKit (accessibility, NSView walk)
```

| Ruolo | File | OS |
|---|---|---|
| API canonica | `src/controls/*.rs`, `src/containers/*.rs` | entrambi |
| Backend Win32 | `src/platform/win32.rs` | Windows |
| Backend AppKit | `src/platform/appkit.rs`, `appkit_controls.rs`, `appkit_list_table.rs`, `appkit_splitter.rs` | macOS |
| Backend XAML (feature) | `src/platform/winui3_xaml/` (`element_bridge.rs`, `app.rs`, `element_tree.rs`) | Windows |
| Inspector | `src/platform/inspector*.rs` | entrambi |

### Regole per il backend XAML (5 passi, da `PIANO_SVILUPPO_WINUI3_XAML.md`)

1. `WidgetDesc` in `element_bridge.rs`;
2. wrapper in `src/controls/*.rs`;
3. mapping in `app.rs::widget_desc_to_element`;
4. re-export in `src/lib.rs` / `mod.rs`;
5. voce `[[example]]` in `Cargo.toml` (senza `required-features` per gli esempi base).

### Regole reactor (errori già incontrati ed evitati)

- `Element::grid_row` / `grid_column` sono **inherent** su `Element`, non trait method:
  tipizzare esplicitamente `let cell: Element = ...into();` per evitare **E0282**
  quando `ElementExt` è in scope (**E0599** se si chiama su `TextBlock`).
- Regole rustc da leggere con `Out-File` + `Select-String -SimpleMatch "error"`.

---

## 3. Tabella componenti target (C1–C9)

Analisi alla base (metodo: grep `crate::platform::appkit::` vs `windows_sys::` in
`src/controls/*.rs`): **27 controlli dual-backend nativi + 21 compositi puri Rust =
48/51 portabili**; **3 solo Windows**: `CalendarCtrl`, `RadioBox`, `SpinCtrlDouble`.

| # | Componente | wxWidgets | Difficoltà | Backend oggi | Fase |
|---|---|---|---|---|---|
| C1 | **TimePickerCtrl** | `wxTimePickerCtrl` | banale | Win32 già pronto (`DTS_TIMEFORMAT`), mac da estendere | F1 |
| C2 | **Simplebook** | `wxSimplebook` | banale | puro Rust (mostra/nascondi pagine) | F2 |
| C3 | **StdDialogButtonSizer** | `wxStdDialogButtonSizer` | banale | puro Rust (sizer esistente) | F1 |
| C4 | **Book attivi** (Listbook/Choicebook/Treebook/Toolbook con eventi) | `wxBookCtrlBase` | medio | eventi non cablati oggi (`containers/book.rs` passivo) | F3 |
| C5 | **RadioBox su macOS** | `wxRadioBox` | medio | solo Windows (`BS_GROUPBOX`+radio) | F4 |
| C6 | **SpinCtrlDouble su macOS** | `wxSpinCtrlDouble` | medio | solo Windows (`msctls_updown32`) | F5 |
| C7 | **CalendarCtrl su macOS** | `wxCalendarCtrl` | medio-alto | solo Windows (`SysMonthCal32`); AppKit ha già `NSDatePicker` ClockAndCalendar | F6 |
| C8 | **TipProvider / ShowTip** | `wxTipProvider` | banale | assente; esiste già `mt_tip_window.rs` | F7 |
| C9 | **Backfill ruoli XAML** | — | banale | `winui3_xaml/element_tree.rs` non copre i nuovi widget | F8 |

Assenti confermati nel crate: `TimePickerCtrl`, `Simplebook`, `StdDialogButtonSizer`,
`TipProvider`. Già presenti (non reimplementare): `MultiChoiceDialog`,
`PropertySheetDialog`, `FindReplaceDialog`, `BannerWindow`, `RichToolTip`,
`BitmapComboBox`, `TreeListCtrl`.

---

## 4. Schede operative (firme Rust proposte)

### C1 — TimePickerCtrl → `src/controls/time_picker_ctrl.rs`

```rust
/// Ora del giorno usata dal TimePickerCtrl.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Time {
    pub hour: u32,   // 0..=23
    pub minute: u32, // 0..=59
    pub second: u32, // 0..=59
}

pub struct TimePickerCtrl { inner: Rc<RefCell<TimePickerInner>> }

impl TimePickerCtrl {
    /// Ore:minuti:secondi (DTS_TIMEFORMAT / NSDatePicker HourMinuteSecond).
    pub fn new<W: Window>(parent: &W) -> Self;
    /// Solo ore:minuti.
    pub fn new_hm<W: Window>(parent: &W) -> Self;
    pub fn get_value(&self) -> Option<Time>;
    pub fn set_value(&self, value: Option<Time>);
    pub fn on_time_change<F: FnMut(Option<Time>) + 'static>(&self, frame: &Frame, cb: F);
    pub fn id(&self) -> u16;
    pub fn as_widget_ref(&self) -> WidgetRef;
}
```

- **Win32**: riusa la meccanica di [date_picker_ctrl.rs](src/controls/date_picker_ctrl.rs) —
  classe `SysDateTimePick32`, stile `DTS_TIMEFORMAT = 0x0009` (già gestito da
  `DateFormat::Time`), `DTM_GETSYSTEMTIME`/`DTM_SETSYSTEMTIME`, notifica
  `DTN_DATETIMECHANGE`. Basta estendere le conversioni `SystemTime` a h/m/s
  (oggi `from_date` azzera `hour/minute/second`).
- **AppKit**: variante di `create_date_picker` (`src/platform/appkit_controls.rs`)
  con `NSDatePickerElementPickerHourAndMinuteSecond` (maschera elementi) e
  formato `%H:%M:%S`.
- **XAML**: `TimePicker` → ruolo `picker` (già mappato).
- **Ruolo inspector**: `picker` (stesso ruolo già assegnato al `DatePickerCtrl`).

### C2 — Simplebook → `src/containers/simplebook.rs`

```rust
/// Contenitore di pagine senza tab: solo la pagina corrente è visibile.
pub struct Simplebook { /* pagine: RefCell<Vec<(String, WidgetRef)>>, indice corrente */ }

impl Simplebook {
    pub fn new<W: Window>(parent: &W) -> Self;
    pub fn add_page(&self, name: &str, page: WidgetRef);
    pub fn show_page(&self, name: &str);
    pub fn set_selection(&self, index: usize);
    pub fn selection(&self) -> usize;
}
```

- **Puro Rust**: nessun FFI — `set_visible(true/false)` delle pagine, come i book
  già presenti in `src/containers/book.rs` (architettura ereditata).
- **Ruolo inspector**: `tabs` (famiglia book) — vedi §6.

### C3 — StdDialogButtonSizer → `src/containers/std_dialog_button_sizer.rs`

```rust
pub enum StdDialogButton { Ok, Yes, No, Save, Apply, Cancel, Close, Help, Retry }

pub struct StdDialogButtonSizer { /* pulsanti + ruolo */ }

impl StdDialogButtonSizer {
    pub fn new() -> Self;
    pub fn add_button(&mut self, btn: &Button, which: StdDialogButton);
    /// Applica l'ordine per-OS e inserisce i pulsanti nel layout del parent.
    pub fn realize(&self, parent: &dyn Window);
}
```

- **Puro Rust**: composto su `FlexGridSizer`/`Sizer` esistenti.
- **Ordine per-OS** (unica differenza reale): **Windows** → affirmativo a sinistra
  (`[OK] [Cancel]`); **macOS** → negativi a sinistra, affirmativo a destra
  (`[Cancel] [OK]`), come da HIG.
- **Ruolo inspector**: il sizer non genera nodo (o `group`); i pulsanti restano `button`.

### C4 — Book attivi → `src/containers/book.rs` (+ `tab.rs`)

Oggi Listbook/Choicebook/Treebook/Toolbook sono **passivi**: le pagine ci sono, ma
gli eventi strip non sono cablati (il chiamante li collega a mano).

- Aggiungere `on_page_changed(&self, frame, cb)` / `bind(...)` sul modello già usato
  da `Tab::on_selection_change` (mappa `notify_handlers` interna).
- **Win32**: `WM_NOTIFY` (`LVN_SELCHANGE`, `TCN_SELCHANGE`, `TVN_SELCHANGED…`).
- **AppKit**: callback `onPageChanged`/selection dei rispettivi NSView
  (`NSTabView`, `NSOutlineView`, `NSPopUpButton`).
- **Ruolo inspector**: `tabs` (Tab/Listbook/Choicebook/Toolbook), `tree` (Treebook).

### C5 — RadioBox su macOS → `src/platform/appkit_controls.rs::create_radio_box`

```rust
pub(crate) fn create_radio_box(parent: NativeHandle, id: u16, rect: Rect,
                               items: &[String], major_dim: u32) -> NativeHandle;
```

- **AppKit**: stack/`NSMatrix` di `NSButton` con `NSButtonRadioType`, raggruppati
  (un solo stato selezionato), `major_dim` = numero di colonne.
- **Win32**: invariato (`BS_GROUPBOX` + radio buttons, `src/controls/radio_box.rs`).
- **Ruolo inspector**: `radio` (nodi figli) — oggi su mac cade in `view`.

### C6 — SpinCtrlDouble su macOS

- **AppKit**: coppia `NSStepper` + `NSTextField` (read-only) sincronizzati;
  `min/max/increment` dello stepper; notifica `NSControlStateDidChange`.
- **Win32**: invariato (`msctls_updown32`, `src/controls/spin_ctrl_double.rs`).
- **Ruolo inspector**: `value` (oggi solo Windows).

### C7 — CalendarCtrl su macOS

- **AppKit**: `NSDatePicker` con stile **`ClockAndCalendar`** e maschera data
  (la costante esiste già in `appkit_controls.rs`) oppure `NSCalendarView` in un
  `NSScrollView` — si scelga il primo: meno FFI, coerente con C1.
- **Win32**: invariato (`SysMonthCal32`, `src/controls/calendar_ctrl.rs`).
- **Ruolo inspector**: `picker` (entrambe le piattaforme).

### C8 — TipProvider → `src/adv/` (o `src/controls/`)

```rust
/// Fornitore delle successivi suggerimenti mostrati dalla finestra tip.
pub trait TipProvider {
    fn next_tip(&mut self, max_length: usize) -> Option<String>;
}

/// Mostra il tip; restituisce true se l'utente ne ha letto almeno uno.
pub fn show_tip<W: Window>(parent: &W, title: &str, provider: &mut dyn TipProvider) -> bool;
```

- **Puro Rust** su `PopupWindow`/`LayerWindow` già esposti da `lib.rs`; posizione
  accanto al rettangolo del chiamante, wrap del testo, scorciatoia chiave per chiudere.
- **Ruolo inspector**: `dialog`.

### C9 — Backfill ruoli XAML

Estendere `winui3_xaml/element_tree.rs` perché i nuovi widget producano lo stesso
ruolo dei nativi: `Simplebook` → `tabs`, `TimePicker` → `picker` (verifica),
`RadioBox` → `radio`, `SpinCtrlDouble`/`Gauge` → `value`, `Calendar` → `picker`,
finestre tip/dialog → `dialog`, sizer/raggruppamenti → `group`.

---

## 5. Token grafici condivisi (grafica moderna, standard attuali)

| Token | Valore | Windows (Fluent/WinUI3) | macOS (AppKit) |
|---|---|---|---|
| Spacing xs/s/m/l | **4 / 8 / 12 / 16 px** | `StackPanel.Spacing`, margini risorse | `NSStackView.spacing`, padding |
| Radius s / m | **4 px / 6–8 px** | `CornerRadius` (controllo 4, card 8) | `wantsLayer.cornerRadius` |
| Altezza controllo | **32 px** (compat 28) | `MinHeight` del control theme | frame `NSButton`/campo 28–32 |
| Font | 13 pt testo, 12 pt caption | `Segoe UI Variable` | `SF Pro` / `NSFont.system` |
| Accent | **di sistema** | `DwmGetColorizationColor` | `NSColor.controlAccentColor` |
| Superficie | opaca/a strati | **Mica** + corner preference (`dwm_style.rs`) | **vibrancy** (`NSVisualEffectView`) |
| Icone | lineari 16 px | lucide (assets/icons/lucide) | **SF Symbols** |

Regola: i token vivono **una sola volta** (header condiviso/const del crate) e i due
backend li consumano; nessun colore/spacing scritto a mano nei singoli controlli.

---

## 6. Integrazione inspector (prevista dal primo giorno)

Server `127.0.0.1:7373` — `GET /health /window /tree /find /screenshot`, `POST /event`.

| Componente | role Win32 | role AppKit | role XAML | `extra`/`read_state` | `inject_event` |
|---|---|---|---|---|---|
| TimePicker (C1) | `picker` (`SysDateTimePick32`) | `picker` (`NSDatePicker`) | `picker` | `value: "HH:MM:SS"` | set time + change |
| Simplebook (C2) | `tabs` | `tabs` | `tabs` | `selection` | select page |
| StdDialogButtonSizer (C3) | — (figli `button`) | — | `group` (opz.) | — | — |
| Book attivi (C4) | `tabs`/`tree` | `tabs`/`tree` | `tabs` | `selection` | select |
| RadioBox (C5) | `radio` | `radio` | `radio` | `checked_index` | click radio |
| SpinCtrlDouble (C6) | `value` | `value` | `value` | `value` | set value |
| CalendarCtrl (C7) | `picker` | `picker` | `picker` | `date` | set date |
| Tip window (C8) | `dialog` | `dialog` | `dialog` | — | close |

Obblighi:

- **Simmetria**: per ogni componente, `/find?role=X` deve dare `count ≥ 1` su
  *entrambi* i backend con lo stesso X (test trasversale automatico).
- **`read_state`/`extra`**: campi del §6 popolati dai backend nativi
  (`GetWindowText`/messaggi Win32; accessibility/AppKit su mac; `element_tree` su XAML).
- **`inject_event`**: `POST /event` deve poter pilotare il componente
  (utile per screenshot deterministici, workaround click titlebar §10.1 di
  `PIANO_DEBUG_INSPECTOR.md`).
- **Classi Win32 reali, non inventate**: usare i nomi registrati veri
  (`SysDateTimePick32`, non `msctls_datetimepick32`; i `msctls_*` reali sono
  trackbar/updown/progress) — lezione del fix §18 di `PIANO_DEBUG_INSPECTOR.md`.

---

## 7. Fasi di lavoro (F0–F8)

| Fase | Contenuto | Consegnabile |
|---|---|---|
| **F0** | **Baseline**: snapshot build Windows (`cargo build` + `--features winui3_xaml`), lista ruoli attuali (`/tree` su `showcase_all`, `mt_pickers`), eventuale minitest di riferimento per i ruoli | note di baseline nel piano inspector |
| **F1** ✅ | **TimePickerCtrl (C1) + StdDialogButtonSizer (C3)** — il primo riusa quasi tutto da `date_picker_ctrl.rs`, il secondo è puro Rust | 2 nuovi moduli + export + minitest + screenshot + `/find?role=picker` — **esiti vedi §7** |
| **F2** ✅ | **Simplebook (C2)** — puro Rust | modulo + export + minitest + screenshot + `/find?role=tabs` — **esiti vedi §7** |
| **F3** ✅ | **Book attivi (C4)** — cablaggio eventi Listbook/Choicebook/Treebook/Toolbook | `bind` sui 4 book + test + `/find?role=tabs|tree` — **esiti vedi §7** |
| **F4** ✅ | **RadioBox su macOS (C5)** | `create_radio_box` AppKit + test — **esiti vedi §7** |
| **F5** | **SpinCtrlDouble su macOS (C6)** | NSStepper+NSField + test |
| **F6** | **CalendarCtrl su macOS (C7)** | `NSDatePicker` ClockAndCalendar + test |
| **F7** ✅ | **TipProvider (C8)** | trait + `show_tip` + uso in `mt_tip_window.rs` — **esiti vedi §7** |
| **F8** ✅ | **Backfill ruoli XAML (C9)** | `element_tree.rs` allineato + `/find` su esempi XAML — **esiti vedi §7** |

Ordine consigliato: **F1 → F2 → F8** (valore immediato + verifica ruoli), poi
F3, poi i backend mac **F4–F6**, infine F7. Ogni fase è autonomamente
compilabile e verificabile.

**Procedura per fase** (identica sempre):

1. definizione "in mezzo" (wrapper in `src/controls|containers`) — una volta sola;
2. backend Win32 (o riuso di quello esistente);
3. backend AppKit (o stub `cfg` se la fase è Windows-only);
4. export in `mod.rs`/`lib.rs` + mapping XAML se serve;
5. ruolo inspector + `read_state`/`inject_event`;
6. minitest (`examples/minitest/mt_*.rs`) + screenshot + verifica `/find?role=`;
7. aggiornamento di questo MD e di `PIANO_DEBUG_INSPECTOR.md` (§ esiti).

### Esiti F1 (completata)

**Consegnato**
- `src/controls/time_picker_ctrl.rs` — `Time` + `TimePickerCtrl`
  (`new` HH:mm:ss, `new_hm` HH:mm, `get_value`/`set_value`,
  `on_time_change`, `id`, `as_widget_ref`). Win32 su `SysDateTimePick32`
  con `DTS_TIMEFORMAT|DTS_UPDOWN`; ABI (`DTM_*`, `GDT_*`, `SystemTime`,
  `NmDateTimeChange`) riusata da `date_picker_ctrl.rs` (definizione unica
  in mezzo). AppKit: `create_time_picker` (`NSDatePicker`,
  HourMinuteSecond/HourMinute, formatter `HH:mm:ss`, riuso
  `DATE_PICKERS`/`DatePickerTarget`). Stub per gli altri OS.
- `src/containers/std_dialog_button_sizer.rs` — `StdDialogButton` (9 ruoli)
  + `StdDialogButtonSizer` (`add_button`, `realize() -> BoxSizer`).
  Ordinamento stabile su `group_index(role, os)`: Windows →
  `[affirmativi] [negativi] [Help]`, macOS → `[negativi] [affirmativi] [Help]`;
  Help sempre ultimo; ordine d'inserimento preservato dentro il gruppo.
- Export in `src/containers/mod.rs`, `src/controls/mod.rs` (via `lib.rs`:
  `StdDialogButton`, `StdDialogButtonSizer`, `Time`, `TimePickerCtrl`).
- Minitest `examples/minitest/mt_time_sizer.rs` (registrato in `Cargo.toml`).

**Verifiche eseguite (Windows)**
- `cargo build` (default) e `cargo build --features winui3_xaml` → 0 errori.
- `cargo test --lib` → **426 passed / 0 failed** (di cui 6
  `std_dialog_button_sizer` + 4 `time_picker`).
- Inspector su `mt_time_sizer`: `/find?role=picker` → `count = 2`
  (`SysDateTimePick32`, text `14:30:00` e `09:05`);
  `/find?role=button` → `count = 3` in ordine Windows
  **OK (x=483) → Cancel (x=588) → Help (x=693)**.
- Screenshot: `f1_mt_time_sizer.png` (root del progetto).

**Bug risolto durante F1 — display del time picker**
- Sintomo: il control mostrava la sola ora (`14`, `9`) invece di
  `14:30:00` / `09:05`.
- Causa: la classe `SysDateTimePick32` risulta **ANSI**
  (`IsWindowUnicode == 0`); `DTM_SETFORMAT` con stringa UTF-16 veniva
  letta come stretta e fermata al primo NUL → formato `"HH"`.
- Fix in `time_picker_ctrl.rs`: si invia prima la variante wide
  (`SendMessageW` + `to_wide`); se il testo del controllo non contiene
  i separatori si riprova con `SendMessageA` + stringa a byte.
- **Note per le fasi successive**: `DTM_SETFORMAT` va sempre verificato
  col testo del controllo (`GetWindowTextW`), non col valore di ritorno
  del messaggio; vale anche per `DatePickerCtrl` se si dovesse impostare
  un formato esplicito.

**Ancora aperto da F1**
- La parte macOS (`create_time_picker`, ramo `cfg(target_os = "macos")`)
  **non viene type-checkata dal build Windows** → da compilare e
  screenshottare su mac (criterio §8).
- `realize()` restituisce un `BoxSizer` da annidare con `add_sizer`
  (la scheda C3 proponeva `realize(&self, parent)`; scelta più flessibile).

### Esiti F2 (completata)

**Consegnato**
- `src/containers/simplebook.rs` — `Simplebook` puro Rust (nessun FFI):
  `new(parent)`, `add_page(name, page)` (la prima pagina diventa la
  corrente, le successive restano nascoste), `show_page(name)`,
  `set_selection(index)`, `selection()`, `page_count()`. Logica separata
  in `SimplebookCore` (pagine + selezione) così i test girano senza
  widget nativi; identico su Windows e macOS.
- **Guscio con ruolo dichiarato**: `Simplebook::new` crea un
  `Panel::new_child` (`RuWxPanel`, nessun fondo → non copre le pagine),
  ne segue l'area con la prima pagina e lo registra come nodo **`tabs`**
  (§6). Senza classe Win32/AppKit propria il ruolo, la `selection` e il
  selettore di `inject_event` vivono nel nuovo registro
  `src/platform/declared.rs` — modulo **sempre compilato**
  (`pub(crate)`), scritto dai container in tutte le cfg e letto dagli
  `inspector_*` (compilati solo in debug/feature): nessun rischio di
  `thread_local` divergenti tra cfg.
- Hook inspector (procedura step 5):
  - Windows `inspector_windows.rs`: `role_for_class` consulta
    `declared::role(hwnd)` **prima** dell'euristica di classe;
    `read_state` emette `state.selection` dal registro; `inject_event`
    (`action:"select"`) prova `declared::select` prima dei messaggi nativi.
  - macOS `inspector_macos.rs`: stesso ordine (dopo
    `appkit::inspector_declared_role`, chiave = handle da
    `inspector_handle_for_view`), `read_state.selection` e
    `select()` via registro.
- Export: `src/containers/mod.rs` (`pub mod simplebook;` + voce di doc),
  `src/lib.rs` (`pub use …Simplebook`).
- Minitest `examples/minitest/mt_simplebook.rs` (registrato in
  `Cargo.toml`): 4 pagine (Dashboard/Tasks/Options/Notes), navigazione
  Prev/Next, guscio creato prima delle pagine.

**Verifiche eseguite (Windows)**
- `cargo check --lib --tests --example mt_simplebook` e
  `cargo build --example mt_simplebook` → 0 errori;
  `cargo build --features winui3_xaml --example mt_simplebook` → 0 errori.
- `cargo test --lib` → **430 passed / 0 failed** (di cui 4 nuovi
  `simplebook`); `cargo test --doc simplebook` → 1 passed.
- Regressione: 89 target (`wx_step1…wx_step9` + tutti i minitest) con
  `--features winui3_xaml` → `cargo check` 0 errori.
- Inspector su `mt_simplebook`: `/find?role=tabs` → `count = 1`
  (`RuWxPanel`, bounds 710×330 = area della pagina);
  `/tree` → `state: {"selection": 0}`;
  `POST /event {"action":"select","id":"…","key":"1"}` → `ok:true` e
  `selection` portata a **1**. Ripetuto con la build `winui3_xaml`
  (count = 1).
- Screenshot: `f2_mt_simplebook.png` (root del progetto) — pagina 2
  (Tasks) visibile dopo l'iniezione `select`.

**Ancora aperto da F2**
- Verifica su **macOS** (build AppKit + `/find?role=tabs` + screenshot):
  il lato mac di `declared`/`inspector_macos` non è compilato dal build
  Windows (criterio §8).
- Il guscio è un nodo `tabs` *senza* figli nell'albero: le pagine sono
  sorelle sotto il parent Win32 (z-order sotto il guscio). Se servisse
  un albero gerarchico reale va ripensato l'aggancio delle pagine.

### Esiti F8 (completata)

**Consegnato**
- `src/platform/winui3_xaml/element_tree.rs` — `role_for(kind)` riscritto
  con i soli ruoli normalizzati §7.1 del piano inspector:
  `TabView|Pivot → tabs`, `ListBox|TemplatedList → list`,
  `ListView|GridView → table`, `Slider|RatingControl|ProgressBar|ProgressRing → value`
  (simmetria `msctls_trackbar32`/`progress32` Win32 e `NSSlider`/
  `NSProgressIndicator` macOS), `ComboBox|FlipView|SelectorBar → choice`,
  `ScrollViewer|ScrollView|ScrollBar → scroll`, `WebView2 → webview`,
  `Group → group`, `picker` per `ColorPicker|DatePicker|TimePicker|
  CalendarDatePicker|CalendarView`, `dialog` per `ContentDialog|TeachingTip|
  InfoBar`, fallback `_ → view` (assorbe `MenuBar`, `CommandBar`, `Empty`,
  `widget` e i pannelli di layout).
- Marcatore di ruolo dichiarato XAML: costante `ROLE_MARKER_PREFIX =
  "wx.role:"` scritto su `automation_id` dell'`Element` reactor e letto da
  `declared_role()` **prima** di `role_for(kind)` — l'analogo XAML del
  registro `declared` dei backend nativi (F2).
- `src/platform/winui3_xaml/element_bridge.rs` — nuovo campo `group: bool`
  su `WidgetDesc::VStack/HStack`; `src/platform/winui3_xaml/layout/sizer.rs`
  — `XamlBoxSizer` imposta `group: true`; `src/platform/winui3_xaml/app.rs`
  — a `group` corrisponde il marcatore `wx.role:group` (ruolo `group` per
  gli sizer, §6 C3 StdDialogButtonSizer "opz."). Anche `ListCtrl` e
  `Grid` tabellari ricevono `wx.role:table` (simmetria `SysListView32`);
  la `Grid` di puro layout resta `view`.
- Nuovo unit test `role_for_allinea_i_ruoli_chiave` (12 asserzioni).

**Verifiche eseguite (Windows)**
- `cargo check --features winui3_xaml --tests` e `cargo check --tests`
  (default) → 0 errori.
- `cargo test --features winui3_xaml --lib` → **431 passed / 0 failed**
  (di cui il nuovo `role_for_allinea_i_ruoli_chiave`).
- Inspector `/find` su nodi reali `xaml:` (build `winui3_xaml`):
  - `wx_step3_sizer` → `role=group` **count = 2** (`xaml:0/0`,
    `xaml:0/0/2`, type `StackPanel`);
  - `wx_step3_tab` → `role=tabs` **count = 1** (`xaml:0/0`, `TabView`) —
    risolta l'apertura di F2 (count=1 veniva dal guscio HWND Win32);
  - `wx_step2_slider` → `role=value` **count = 3** (`xaml:0/1`, `Slider`);
  - `wx_step7_listbox` → `role=list` **count = 1** (`xaml:0/1`, `ListBox`);
  - `wx_step3_scroll` → `role=scroll` **count = 1** (`xaml:0/0`, `ScrollViewer`);
  - `wx_step7_listctrl` → `role=table` **count = 1** (`xaml:0/1`, `Grid`).
- Regressione §8: 89 target (`wx_step1…wx_step9` + tutti i minitest) con
  `--features winui3_xaml` → `cargo check` 0 errori.

**Ancora aperto da F8**
- Screenshot di verifica §8 non previsti (F8 non aggiunge componenti);
  le verifiche sono le risposte `/find` sopra.
- `examples/music_player.rs` aveva E0599 preesistenti sotto la feature
  `winui3_xaml` (`set_row_height`, `on_item_double_click`,
  `on_row_reordered`, `last_hit_cell` su `ListCtrl`) — non causati da
  F8 e **risolti dalla milestone MP** (voce "Esiti MP" §7).
- Build macOS non verificabile da Windows (§8); `bounds` resta `0×0`
  senza un layout pass (§6.3).

### Esiti F3 (completata)

**Consegnato**
- `src/containers/book.rs` — `bind` sui 4 book (forma scelta rispetto a
  `on_page_changed`, sul modello di `Tab::on_selection_change`):
  - `Listbook::bind(&Frame, &ListBox)` e `Choicebook::bind(&Frame, &Choice)`;
  - `Treebook::bind(&Frame, &TreeCtrl, page_of: impl Fn(&TreeItem) ->
    Option<usize>, item_of: impl Fn(usize) -> Option<TreeItem>)`;
  - `Toolbook::bind<F: Fn(u16) -> Option<usize>>(&Frame, &ToolBar, F)`.
- `bind` fa tre cose in un colpo solo (doc modulo, §Design):
  1. wiring strip → book (`on_selection_change` / `on_tool_clicked` →
     `core.select(index)`);
  2. registrazione all'inspector tramite l'helper `register_strip` →
     `declared::mark(handle, role)` + `declared::set_selector(…)` +
     `strip_handle` sul `BookCore`: ruolo **`tabs`** per
     Listbook/Choicebook/Toolbook e **`tree`** per Treebook (§6 C4),
     con `read_state: selection` e `inject_event: select` già
     funzionanti grazie agli hook di F2;
  3. allineamento strip ↔ pagina corrente (`declared::set_selection`).
- 3 unit test nel modulo `tests` di `book.rs`: `declared::mark` +
  `select` risveglia il selettore, allineamento della `selection`
  iniziale, guard di `select` (fuori range / invariato → nessuna
  callback) con `Frame`+`Panel` reali.
- Minitest allineati a `bind` (al posto del wiring manuale):
  `mt_listbook`, `mt_choicebook`, `mt_treebook` (mapping node→pagina
  spostato nei closure `page_of`/`item_of`), `mt_toolbook` (match sugli
  id delle icone). Nessun export nuovo: `Choicebook`, `Listbook`,
  `Toolbook`, `Treebook` erano già in `src/lib.rs`.
- Doc modulo `book.rs` aggiornata (design `bind`, note d'uso: chiamarlo
  dopo `add_page`, handler registrato *dopo* `bind` sostituisce il
  cablaggio, callback invocata sotto `borrow_mut` → non rientrare nel
  core).

**Bug risolto durante F3 — E0505 nei closure di `bind`**
- Sintomo: `cannot move out of 'strip' because it is borrowed`
  (Listbook riga ~233, Choicebook ~315): il `let strip = strip.clone()`
  usato come receiver di `on_selection_change` veniva anche spostato
  nel closure `move`.
- Fix: clone dedicato `strip_for_cb` usato **solo** dentro il closure
  (`Rc::clone(&self.core)` per il book). Rimosso anche l'import
  inutilizzato `crate::core::widget::Widget`.

**Verifiche eseguite (Windows)**
- `cargo check` (lib) → 0 errori, 0 warning su `book.rs`;
  check mirato `--example mt_listbook --example mt_choicebook
  --example mt_treebook --example mt_toolbook` → 0 errori.
- `cargo build` (default) e `cargo build --features winui3_xaml` dei
  4 minitest → 0 errori.
- `cargo test --lib` → **433 passed / 0 failed** (di cui 3 nuovi
  `containers::book::tests`; 7 tra book e simplebook).
- Inspector live sui 4 minitest (endpoint `GET /find`, `GET /tree`,
  `POST /event`, tutti `ok:true`):
  - `mt_listbook`: `/find?role=tabs` → `count = 1` (`ListBox`,
    item Dashboard/Tasks/Options/Notes), `state.selection: 0`;
    `POST /event {"action":"select","key":"1"}` → `selection` **1**
    (`selected:[1]`, pagina 2 visibile).
  - `mt_choicebook`: `/find?role=tabs` → `count = 1` (`ComboBox`,
    Red/Green/Blue); `select "key":"2"` → `selection` **2**.
  - `mt_treebook`: `/find?role=tree` → `count = 1` (`SysTreeView32`);
    `select "key":"1"` → `state.selection` **1** (la pagina della
    riga selezionata).
  - `mt_toolbook`: `/find?role=tabs` → `count = 1`
    (`ToolbarWindow32`); `select "key":"2"` → `state.selection` **2**.
- Screenshot: `f3_listbook_select.png`, `f3_choicebook_select.png`,
  `f3_treebook_select.png`, `f3_toolbook_select.png` (root del
  progetto).
- Regressione §8: 89 target (`wx_step1…wx_step9` + tutti i minitest)
  con `--features winui3_xaml` → `cargo check` 0 errori.

**Ancora aperto da F3**
- Verifica su **macOS**: il wiring passa dagli handler registrati sul
  `Frame` (stessa API, dispatcher diversi) ma il lato mac non è
  compilato dal build Windows (criterio §8) → da testare con
  `/find?role=tabs|tree` su mac.
- `examples/music_player.rs` aveva E0599 preesistenti (`set_row_height`,
  `on_item_double_click`, `on_row_reordered`, `last_hit_cell` su
  `ListCtrl`): usciva 101 per questo, non per F3 (la regressione §8
  sui 89 target era verde) — **risolti dalla milestone MP** (voce
  "Esiti MP" §7).
- `bind` registra il *suo* handler sulla strip: un handler registrato
  dopo `bind` lo sostituisce (documentato nella doc modulo); per
  l'evento di cambio pagina va usato `book.on_selection_change`.

### Esiti F4 (completata)

**Consegnato**
- `src/platform/appkit_controls.rs` — `create_radio_box` (già
  implementato in f4b: groupbox + radio `NSButton` via
  `create_radio_button`, che marca ogni figlio `radio`) invariato in
  questa sessione.
- `src/controls/radio_box.rs` — registro `declared` per il guscio
  (§6 C5):
  - `with_selection`: `declared::mark(shell, "radio")` **prima** di
    ogni altra operazione (ruolo del guscio overrideato rispetto a
    `group`/`container`) + `declared::set_selector(shell, …)` che
    richiama `radio_box.set_selection(index)` → `inject_event "select"`
    funziona su Win32 e AppKit senza toccare gli injector (già provano
    `declared::select` per primi);
  - `set_selection`: `declared::set_selection(shell, index)` alla fine
    (entrambi i rami) → `read_state` legge sempre lo stato aggiornato;
  - `on_select`: i closure (click utente) sincronizzano la selezione
    dichiarata prima di invocare la callback dell'utente;
  - test unit `radio_box_registra_ruolo_selezione_e_selettore`
    (role/selection/select su handle reale, `cargo test --lib`).
- `src/platform/inspector_windows.rs` e
  `src/platform/inspector_macos.rs` — `read_state`: se
  `declared::role == Some("radio")` la chiave della selezione diventa
  **`checked_index`** (§6), altrimenti resta `selection` (book,
  simplebook). Su mac il handle arriva da
  `appkit::inspector_handle_for_view`.
- Nessun export nuovo: `RadioBox` già in `src/lib.rs` e `prelude.rs`.
- Backend XAML **fuori scope** per F4: il guscio XAML non è marcato
  `radio` e `SelectedIndex` dei `RadioButton` è già letto da
  `read_state`; il secondo `RadioBox` in `platform/winui3/app.rs` non è
  usato da `default_api` → non toccato.

**Verifiche eseguite (Windows)**
- `cargo check --lib` → 0 errori (4 warning preesistenti in
  `chrome/ribbon_bar.rs`).
- `cargo test --lib radio_box` → 1 passed;
  `cargo test --lib` → **434 passed / 0 failed** (433 + il nuovo test).
- Regressione §8: 89 target (`wx_step1…wx_step9` + tutti i minitest)
  con `--features winui3_xaml` → `cargo check` 0 errori.
- Run live `mt_checkbox_radio` + inspector `127.0.0.1:7373`:
  - `GET /find?role=radio` → `count = 8`: guscio `Priority`
    (id 85331690) + 4 radio della RadioBox + 3 RadioButton liberi;
  - `GET /tree` sul guscio → `state.checked_index = 1` (selezione
    iniziale), figli con `checked` coerente;
  - `POST /event {"action":"select","id":"<guscio>","key":"2"}` →
    `ok: true` e `checked_index = 2` (`High` → `checked: true`);
  - `POST /event {"action":"click","id":"<Urgent>"}` → `checked_index
    = 3` **e** callback `on_select` scattata (label
    `Current priority: Urgent`) → `declared::set_selection` nei closure
    è coerente con il dispatch nativo.
- Screenshot: `f4_mt_checkbox_radio.png`,
  `f4_radiobox_shell.png` (root del progetto).

**Ancora aperto da F4**
- Verifica su **macOS**: il ramo AppKit non è compilato/type-checkabile
  dalla macchina Windows (toolchain solo `x86_64-pc-windows-gnu`,
  criterio §8) → da testare con `/find?role=radio` + `select` su mac.
- `checked_index` nel backend XAML (`winui3_xaml/element_tree.rs`):
  deciso fuori scope per F4 (nessun guscio marcato `radio`); se in
  futuro il guscio XAML verrà marcato, basta leggere la stessa
  `declared::selection` con la stessa chiave condizionata.
- `set_selection` programmatico (non tramite click) non dispaccia
  `on_select`: comportamento preesistente del controllo, non una
  regressione di F4.

### Esiti F7 (completata)

**Consegnato**
- `src/adv/tip_provider.rs` — card C8:
  - `pub trait TipProvider { fn next_tip(&mut self, max_length: usize) -> Option<String>; }`
    e `pub fn show_tip<W: Window>(parent, title, provider: &mut dyn TipProvider) -> bool`
    esattamente come da scheda §4 C8;
  - `TipSession` (`pull` + `clip`): conta i tip letti, ritaglia sul
    confine UTF-8 e restituisce `true` se l'utente ne ha letto almeno
    uno (ossia la condizione d'uscita di `show_tip`);
  - `run_tip_win32`: finestra classe `RuWxTipProviderWindowClass`
    (registrazione once), wrap del testo via `measure_tip`, posizionata
    accanto al rettangolo del chiamante, loop modale `GetMessageW` con
    scorciatoie (Enter/Space → prossimo tip, Esc/chiudi → uscita) e
    **`declared::mark(hwnd, "dialog")`** → ruolo §6/§4 C8;
  - `run_tip_macos`: ramo AppKit scritto ma **non compilabile** dalla
    macchina Windows (criterio §8).
- Export `TipProvider`/`show_tip` da `src/lib.rs` e `src/prelude.rs`.
- `examples/mt_tip_window.rs`: quarto bottone
  **"Show tips series (TipProvider)"** che itera un provider fino a
  esaurimento e scrive lo stato nella status bar
  (`Tip series: at least one tip read`).
- 3 test unit nuovi in `tip_provider::tests`:
  `clip_taglia_su_confine_utf8`, `session_pull_ritaglia_e_marca_letto`,
  `session_con_provider_vuoto_non_leta`.
- **Fix infrastruttura inspector** (emerso durante la verifica live):
  `Frame::inspector_wake_window()` + `inspector_wake_wnd_proc` in
  `src/window/frame.rs` — finestra *message-only* dedicata che chiama
  `inspector::drain_jobs()` direttamente; `Inspector::start_with` la
  usa come destinazione del wake. Serve perché il dispatch dei raw
  message del frame **rimuove il `Vec` degli handler mentre li esegue**
  e un job che ri-entra in un loop modale (come il `show_tip` stesso)
  avrebbe divorato i wake successivi → tutti i request successivi in
  504. Con la finestra dedicata la rientranza funziona.

**Verifiche eseguite (Windows)**
- `cargo test --lib` → **437 passed / 0 failed** (434 + i 3 nuovi test).
- `cargo check --example mt_tip_window` → 0 errori (4 warning
  preesistenti in `chrome/ribbon_bar.rs`).
- Regressione XAML: `cargo check --features winui3_xaml --example
  wx_step9_checklist` → 0 errori.
- Run live `mt_tip_window` + inspector `127.0.0.1:7373`:
  - `POST /event {"action":"click","id":"<Show tips series>"}` → il job
    resta in attesa (504 atteso: il job gira `show_tip` dentro il loop
    modale e risponde solo a tip chiuso);
  - `GET /find?role=dialog` → `count = 1`,
    `{"role":"dialog","component_type":"RuWxTipProviderWindowClass",
    "text":"Tip series","path":"0/5","bounds":{"x":465,"y":621,"w":360,"h":48}}`
    → obbligo §6 rispettato (finestra di proprietà sotto il frame);
  - `GET /health` → `ok: true` anche con il tip aperto (la rientranza
    non blocca più l'inspector);
  - `POST /event {"action":"close","id":"<tip>"}` → `{"ok":true}`
    (nuova azione `close` §6) e `GET /find?role=dialog` → `count = 0`;
  - status bar verificata via `/tree`: testo
    `Tip series: at least one tip read`.
- Screenshot: `f7_tip_dialog.png` (nodo dialog, testo
  "Press Enter or Space for the next tip.") e `f7_frame_con_tip.png`
  (frame + status bar) — root del progetto.

**Ancora aperto da F7**
- `POST /event click` sul bottone che **apre** il tip restituisce 504:
  limite di progettazione del loop modale (la risposta arriva a tip
  chiuso); le richieste successive funzionano tutte.
- Verifica su **macOS**: `run_tip_macos` non compilabile da Windows →
  da provare con `/find?role=dialog` + `close` su mac.
- `show_tip` non è ancorato al rettangolo di un widget generico ma alla
  finestra parent (posizionamento "accanto al chiamante" solo
  approssimato).

### Esiti MP (completata) — `examples/music_player.rs`

**Problema**: l'example non compilava (E0599) perché `ListCtrl`
mancava delle 4 API usate dall'esempio.

**Consegnato** — `src/controls/list_ctrl.rs`:
- `set_row_height(f64)` / `row_height()`: clamp 16–96; su Windows
  converte in punti (`(h-8)*0.72` clamp 8–28, formula di
  `Grid::win_apply_row_height`), rigenera il `Font` e fa `WM_SETFONT`
  su lista + header (`LVM_GETHEADER`); su mac `listctrl_set_row_height`.
- `on_item_double_click(&Frame, FnMut(usize))`: hub
  `register_lv_item_activate_handler` + `platform::shell::
  list_activate_item_index(lparam)`.
- `on_row_reordered(&Frame, FnMut(usize, usize))`: su mac abilita
  `listctrl_set_row_reorder_enabled` + consuma
  `listctrl_take_row_reordered` nell'hub notify; su Windows il
  callback è registrato ma il back-end non lo emette.
- `last_hit_cell() -> Option<(usize, usize)>`: su Windows subclass
  della list-view (`LVM_SUBITEMHITTEST = LVM_FIRST+57`, corretto a
  differenza di `grid.rs` che usa `+87`) che aggiorna la cella su
  `WM_LBUTTONDOWN`/`WM_LBUTTONDBLCLK`; su mac
  `listctrl_last_hit_cell`.
- **Hub notify unico** `ensure_notify_hub` (modello della `Grid`):
  `notify_handlers` è un `HashMap` a owner singolo → l'ultima
  registrazione per lo stesso id sovrascribe; ora `on_item_selected`
  e `on_row_reordered` condividono la stessa closure che fa dispatch
  sul codice (`LVN_ITEMCHANGED` / `LISTCTRL_NOTIFY_ITEM_CHANGED` /
  `LISTCTRL_NOTIFY_ROW_REORDERED`).

**Fix doctest** (ultimi 2 fallimenti di `cargo test`):
- `adv/tip_provider.rs`: `Frame::new().unwrap()` →
  `Frame::builder().with_title(…).build()`.
- `dnd/ole_dnd.rs`: `frame.hwnd()` (`isize`) convertito in
  `HWND = *mut c_void` per `do_drag_drop`.

**Verifiche eseguite (Windows)**
- `cargo check --all-targets` → 0 errori (l'example compila).
- `cargo test` → **EXIT=0**: lib **437 passed**, unit 2/25/1/0,
  doctest **54 passed / 0 failed / 5 ignored** (log `mp5_test.log`).
- Verifiche mac saltate su richiesta dell'utente.

---

## 8. Criteri di accettazione

- [x] `cargo build` (Windows, default) e `cargo build --features winui3_xaml` senza errori
      (PATH con `C:\msys64\mingw64\bin` in testa; build su cwd `ru_wx\ru_wx`,
      target dir dedicata, processi sulla porta 7373 killati prima del build)
      — **EXIT=0** entrambi i build (log `mp6_build_default.log`,
      `mp6_build_xaml.log`); 0 errori anche con `cargo check --all-targets`
      in entrambe le configurazioni (log `mp6_check_all_default.log`,
      `mp6_check_all_xaml.log`).
- [ ] Build macOS (AppKit) senza nuovi warning; stub per gli altri OS.
      *Saltata su richiesta dell'utente.*
- [x] `cargo test` verde (i test pinned degli snapshot, es. costanti Win32, invariati)
      — **EXIT=0** da milestone MP6: 441 lib + 28 unit + 54 doctest
      (5 ignored; log `mp5_test_full_default.log`); con
      `--features winui3_xaml` → 443 lib (log `mp5_test_full_xaml.log`).
- [x] Per ogni componente: minitest dedicato + screenshot di verifica
      (carpeta screenshot esistente) su **entrambe** le piattaforme.
      — **MP13**: *Windows* — **65/65 minitest** (`examples/minitest/mt_*.rs`)
      con screenshot in `img/v3_win32_minitests/`: erano presenti 62 PNG, i 3
      mancanti catturati ora via `PrintWindow`+`PW_RENDERFULLCONTENT`
      (`mt_listbox_winui3`, `mt_simplebook`, `mt_time_sizer` → OK 980×480 /
      760×440 / 520×240, tutti con contenuto renderizzato); copertura anche
      degli altri set: `img/v3_win32_examples/` (11), `img/v3_winui3_examples/`
      (43), `img/v3_winui3_minitests/` (19). *macOS* — verifiche **saltate su
      richiesta dell'utente**.
- [x] `/find?role=` con lo stesso ruolo su Win32, AppKit e XAML → `count ≥ 1`
      — **XAML**: coperto dal test automatico
      `element_tree::tests::window_info_e_inject_event_sul_registro`
      (`role=button/input/choice/check` → `count = 1`, stessi ruoli della
      tabella §6); **Win32**: verifiche live in `PIANO_DEBUG_INSPECTOR.md`;
      **AppKit**: saltato su richiesta dell'utente.
- [x] Nessun regressione: `wx_step1…wx_step9` e minitest esistenti compilano e
      rendono come prima. — *compilazione verificata*: `cargo check
      --all-targets` (default e `--features winui3_xaml`) → 0 errori;
      **verifica visiva eseguita (MP12)**: screenshot via inspector
      (`GET /screenshot`, workaround titlebar §10.1) di un demo per ogni step —
      `mp12_wx_step1_window.png` (finestra base, vuota per design — il sorgente
      non aggiunge widget), `mp12_wx_step3_sizer.png` (header + stretch +
      bottoni OK/Annulla spaziati), `mp12_wx_step4_menu.png` (MenuBar 3 menu +
      label), `mp12_wx_step5_msgbox.png` (5 bottoni dialogo), `mp12_wx_step6_icon.png`
      (immagine SVG renderizzata), `mp12_wx_step7_listctrl.png` (tabella a
      colonne distinte — fix griglia confermato), `mp12_wx_step8_grid.png`
      (10 righe, header, riga selezionata accent, sorting ↑); step 2 e 9 già
      coperti da MP11 (`cmp_xaml_wx_step2_*.png`, `cmp_xaml_wx_step9_checklist.png`).
      Nessuna regressione visiva rilevata; **verifica mac saltata su richiesta
      dell'utente**.
- [x] Grafica conforme ai token §5 (accent di sistema, altezza 32, radius, spacing)
      — **MP9**: token centralizzati nel nuovo `src/core/tokens.rs` (costanti §5:
      spacing 4/8/12/16, radius 4/8, altezza 32 / compat 28, font 13/12 pt,
      `system_accent_colour()` = `DwmGetColorizationColor` in 0xAARRGGBB con
      fallback #0078D7). Consumati da: `RibbonArtProvider::default` (accent),
      `Grid` (`selection_background` in `win11()`/`classic()`), `BannerWindow`,
      `Wizard` e `PropertySheetDialog` (altezza 28 → `CONTROL_HEIGHT_COMPAT`,
      gap 8 → `SPACING_S`), `ProgressDialog` (altezza compat), `AuiToolBar`
      (altezza compat), `ScrollablePanel` (padding 12 → `SPACING_M`),
      `RibbonBar` (inset 8 → `SPACING_S`, label gap 4 → `SPACING_XS`) — valori
      numerici invariati, nessun cambiamento visivo. 3 test pin dei valori §5
      in `tokens::tests`. Verifiche: `cargo check --lib` EXIT=0,
      `cargo test` EXIT=0 (**444 lib** +3 token, 2+25+1 unit, 54 doctest,
      5 ignored), `cargo clippy --features winui3_xaml -- -W clippy::all`
      EXIT=0 (log `mp9_check.log`, `mp9_test.log`, `mp9_clippy.log`).
      Residui dichiarati: radius su Win32 = DWM corner preference (le costanti
      `RADIUS_*` sono per il backend XAML); `FONT_SIZE*` dichiarati ma non
      ancora consumati; accent hardcoded in `appkit.rs` (`cfg(target_os = "macos")`,
      non compilabile qui) lasciato invariato.
- [x] Documentazione: questo piano aggiornato con gli esiti; voci in
      `PIANO_DEBUG_INSPECTOR.md` per i nuovi ruoli (§7.1 allineati, test
      `role_for` su entrambi i backend; XAML `/window` + `POST /event`
      registrati in §15 "Prossimo passo").

---

## 9. Rischi e mitigazioni

| Rischio | Mitigazione |
|---|---|
| Differenze di layout nativo (ordine pulsanti, densità) | token §5 + ordine per-OS esplicito in C3, screenshot affiancati |
| Formati ora/data locale (Win32 `DTS_TIMEFORMAT` segue la locale) | test con locale esplicita; `read_state` legge il testo mostrato |
| `NSMatrix` deprecato su mac | usare stack di `NSButton` radio con layout moderno |
| Eventi book su mac non omnogenei (stub storici) | F3 con verifica per-OS; fallback a callback registrate sul frame |
| Deriva dei ruoli inspector (bug `msctls_datetimepick32`) | tabella §6 come fonte unica + test `/find` automatici in F8 |
| Non è un repo git | snapshot manuali prima di ogni fase; attenzione a `_conflited_quarantine/` (file Dropbox da **non** compilare) |
| Finestre XAML bianche negli screenshot | workaround consolidato `SetForegroundWindow` + click titlebar (300,12), §10.1 del piano inspector |

---

## 10. Appendice

### A. Controlli già portabili oggi (48/51)

- **Dual-backend nativi (27)** con `crate::platform::appkit::` + `windows_sys::`:
  button, checkbox, radio button, text ctrl, static text, static bitmap, list box,
  check list box, choice/combo, list ctrl, tree ctrl, slider, gauge, spin button,
  spin ctrl, toggle, bitmap button, hyperlink, date picker, colour/font/file/dir
  picker, search ctrl, static box/line, scrollbar, …
- **Compositi puri Rust (21)** (portabilità ereditata dai mattoni):
  activity indicator, animated button, bitmap toggle, button variants,
  collapsible pane/header, combo/popup ctrl, editable list box, add/remove ctrl,
  rearrange list, simple HTML list box, v-list box, file ctrl, IP address ctrl,
  file/dir/font picker, generic dir ctrl, tree list ctrl, menu button.
- **Solo Windows (3)** → target di F4–F6: `CalendarCtrl`, `RadioBox`, `SpinCtrlDouble`.

### B. Compositi avanzati wxWidgets presenti (verifica "cosa mancava")

`adv/`: animation, media, property grid (+manager), rich text (+buffer/attr/style),
web view, html window, wizard, log window/gui, help controller.
`containers/`: book, data view, grid (+editor/renderer), grid bag sizer, wrap sizer,
static box sizer, flex grid sizer, sizer base, splitter, tab, scrolled window.
`chrome/`: AUI manager (+AuiNotebook dual-backend), AUI toolbar, ribbon (6 moduli),
info bar, header ctrl, icon tray, notification, status bar, toolbar.
`window/`: `MDIParentFrame`/`MDIChildFrame` dual-backend.

### C. Metodo di analisi portabilità (ripetibile)

```powershell
# controlli con backend AppKit
Grep "crate::platform::appkit::" src/controls/*.rs
# controlli con backend Win32
Grep "windows_sys::" src/controls/*.rs
# stub = nessuno dei due (cfg all(not(windows), not(macos)))
```

### D. Comandi di verifica

```powershell
# build Windows + XAML (cwd ru_wx\ru_wx, PATH msys64 in testa)
cargo build --features winui3_xaml --example wx_step9_checklist
# ispettore
Invoke-WebRequest -UseBasicParsing http://127.0.0.1:7373/tree
Invoke-WebRequest -UseBasicParsing "http://127.0.0.1:7373/find?role=picker"
```
