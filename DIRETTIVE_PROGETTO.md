# Direttive del Progetto ru_wx - Libreria Widget Cross-Platform

## Visione Generale

**Obiettivo**: Creare una libreria Rust per UI cross-platform che fornisce API uniformi per widget native su Windows, macOS e Linux, utilizzando chiamate native Rust per ogni piattaforma senza passare per librerie intermedie di porting.

---

## Architettura Multi-Backend

```
ru_wx (API Unificata)
├── ru_wx::platform::win32  → Chiamate native Windows API (CreateWindowExW, GDI, etc.)
├── ru_wx::platform::winui3 → Chiamate native WinUI3 (XAML, Composition)  
├── ru_wx::platform::macos  → Chiamate native macOS (AppKit, Objective-C)
└── ru_wx::platform::gtk    → Chiamate native GTK (per Linux)
```

**Principio Fondamentale**: Ogni backend chiama direttamente le API native della piattaforma senza bridge intermedi come wxWidgets o wrapper C++. Le stesse API logiche ma implementazioni native separate.

---

## Backend Attuali e Obiettivi

### Backend `win32` (già presente)
- **Tecnologia**: Win32 classico via `windows-sys`
- **API**: CreateWindowExW, GDI, standard controls (Button, Edit, ListBox, etc.)
- **Stato**: Funzionante, maturo

### Backend `winui3` (da convertire)
- **名前目標**: WinUI3/XAML reale via `windows-core` + WinRT
- **Non è**: Win32 classico (attuale implementazione errata!)
- **Obiettivo**: Microsoft.UI.Xaml.Controls.* con Composition API

### Backend `macos` (da implementare)
- **Tecnologia**: Objective-C runtime via `objc2` crate
- **API**: AppKit native (NSWindow, NSView, NSControl)
- **Bridge**: Chiamate dirette a framework Apple

### Backend `gtk` (futuro)
- **Tecnologia**: GTK4 via `gtk4-rs`
- **API**: Widget nativi GTK

---

## Direttive Tecniche Fondamentali

### 1. Chiamate Native Dirette
```rust
// ❌ NON fare così (bridge intermedio)
impl_button_from_wxwdigets(config)

// ✅ FARE così (chiamata nativa)
windows_core::implement::<Button>()  // WinUI3
objc_msg_send![btn, setTitle:@"Click"]  // macOS
```

### 2. Nessuna Dipendenza da wxWidgets
- **No**: `wx` crate, binding wxWidgets
- **Sì**: API native per ogni piattaforma

### 3. API Unificata
- Stessa API logica per tutti i backends
- Backend selezionato a compile-time con feature flags
- Possibile override runtime per multi-window

### 4. Feature Flags per Selezione Backend
```toml
[features]
default = ["win32"]
win32 = ["dep:windows-sys"]
winui3 = ["dep:windows-core"]
macos = ["dep:objc2"]
```

### 5. Windows: sempre controlli adattati a Windows 11
Ogni binario che linka `ru_wx` deve aprirsi con i **visual styles
Windows 11** in *tutti* i controlli, indipendentemente dal fatto che il
consumatore incorpori risorse o meno. Garanzie obbligatorie:

- **Manifest nel binario** — `build.rs` incorpora via `embed-resource`
  `app.manifest` (dipendenza `Microsoft.Windows.Common-Controls 6.0.0.0`,
  `supportedOS` Win10/11, `dpiAwareness=PerMonitorV2`). Verifica: le
  stringhe `PerMonitorV2` / `Microsoft.Windows.Common-Controls` devono
  essere presenti nell'`.exe` (job CI `smoke_launch_windows`).
  Il consumatore **non** deve aggiungere un secondo `RT_MANIFEST`/`1 ICON`
  (altrimenti ".rsrc merge failure: duplicate leaf").
- **Fallback runtime ACTCTX v6** — `core::app::App::new()` e
  `platform::winui3::ensure_class_registered()` attivano a runtime un
  contesto `CreateActCtxW`/`ActivateActCtx` con le Common-Controls
  6.0.0.0 e riprovano `InitCommonControlsEx` se il primo tentativo
  fallisce: copre i binari senza manifest (es. test/esempi sfusi).
  **Ogni nuovo backend/init deve ripetere questo schema.**
- **DPI PerMonitorV2** — dichiarato nel manifest + re-assert runtime
  `SetProcessDpiAwarenessContext` (`core/dpi.rs`, `winui3/window.rs`)
  prima della prima finestra.
- **Finestre top-level Win11** — usare `dwm_style.rs`/`top_level_window.rs`:
  angoli arrotondati (`WindowCornerPreference`), backdrop Mica/Acrylic
  (`DWMWA_SYSTEMBACKDROP_TYPE`), title bar scura
  (`DWMWA_USE_IMMERSIVE_DARK_MODE`).
- **Token grafici condivisi** — spacing/radius/altezza/font/accent da
  `core::tokens.rs` (§5 `PIANO_PORTABILITA_COMPONENTI.md`), non valori
  magici locali.
- **Nuovi controlli Win32** — registrare le classi con le `ICC_*`
  corrette in `InitCommonControlsEx`, non disabilitare il tema di
  sistema (`SetWindowTheme`), owner-draw solo se il tema nativo non
  basta.

### 6. Controlli complessi e layer dati

Moduli aggiunti al di sopra dei controlli base:

- **Controlli Win11-style** (`controls/`): `TaskDialog`
  (`TaskDialogIndirect`), `BreadcrumbBar`, `NumberBox`,
  `RatingControl`, `TeachingTip`. Stesse regole dei controlli base:
  nessun `SetWindowTheme`, token da `core::tokens.rs`.
- **Layer dati** (`data/`): trait `DataProvider` + `DataTable` +
  backend SQLite / MySQL / MariaDB / memoria (feature `data-db`).
  `DataValue` è il tipo cella comune (`Null`, `Int`, `Real`, `Text`,
  `Bool`, `Bytes`).
- **`DataGrid`** (`containers/data_grid.rs`): `ListCtrl` in modalità
  `LVS_OWNERDATA` (virtualizzata), sort per colonna, filtro testuale.
  Gli handler pubblici lavorano su indici **sorgente** della tabella,
  mai su quelli di vista. È il controllo **standard per la gestione
  dati**: tutto nativo Win32, niente owner-draw.
  - Checkbox native (`set_checkboxes`): `LVS_EX_CHECKBOXES`; in modo
    virtuale lo stato va ri-erogato a ogni paint via `LVIF_STATE`
    (`ListItem::set_state_image_checked`) e il toggle si legge da
    `LVN_ITEMCHANGED` (`uNewState & 0x2000`).
  - Edit in-place (`set_editable`): doppio click → `EDIT` nativo
    figlio del frame sopra la cella (`LVM_GETSUBITEMRECT`: `RECT.top`
    = subitem, `RECT.left` = `LVIR_*`), commit su Enter/`EN_KILLFOCUS`,
    annulla con Esc, parsing per tipo colonna.
  - Grafica nativa: `LVS_EX_DOUBLEBUFFER` sempre attivo (niente
    sfarfallio) e freccia di sort nativa sull'header
    (`HDF_SORTUP`/`HDF_SORTDOWN` via `HDM_SETITEMW`, aggiornata a ogni
    `apply_view`).
  - Icone di riga (`set_image_list` + `set_row_image`): `ImageList`
    nativa (slot `LVSIL_SMALL`) erogata in virtuale via `LVIF_IMAGE`
    (`ListItem::set_image`); l'indice vive nel modello e segue la riga
    sorgente attraverso sort e filtri. Solo prima colonna (report
    mode standard).
- **`Grid`** (`containers/grid.rs`): griglia owner-draw complessa —
  **non usarla per la gestione dati** (resta per showcase/demos);
  per i dati usare `DataGrid` + `AutoForm` + `GridFormLink`.
- **`AutoForm`** (`containers/auto_form.rs`): form generato da
  `TableSchema`; mapping tipo→controllo (Text→`TextCtrl`,
  Integer→`SpinCtrl`, Real→`SpinCtrlDouble`, Boolean→`CheckBox`,
  Date/DateTime→`DatePickerCtrl`, FK+lookup→`ComboBox`). Il layout è
  **manuale** (`AutoForm::layout`): `FlexGridSizer` non si annida nei
  `BoxSizer`. `validate()` segnala i campi NOT NULL vuoti.
- **`GridFormLink`** (`containers/grid_form_link.rs`): ponte
  griglia↔form (selezione → caricamento riga, `apply` → riscrittura
  nella `DataTable` + `refresh`). `apply_to_provider` /
  `insert_to_provider` persistono nel DB con SQL generato da
  `data::sql_gen` (`build_update` per PK originale, `build_insert`
  senza colonne auto-incremento).

Regole apprese sui controlli Win32:

- **`LVS_OWNERDATA` solo alla creazione** — il ListView ignora
  silenziosamente `SetWindowLongPtrW(GWL_STYLE)` su quel bit: liste
  virtuali si creano con `ListCtrlStyle::ReportVirtual`
  (`DataGrid`, `VListBox`). Senza, la griglia resta vuota a runtime
  anche se il modello è corretto.

- `UDM_SETRANGE`: **LOWORD = minimo, HIWORD = massimo** (interi con
  segno a 16 bit, range utile 0..0x7FFF). Non usare range più ampi:
  le posizioni oltre i 16 bit non sono rappresentabili.
- `UDM_SETPOS`: la posizione va nella **LOWORD** di `lParam` con
  HIWORD a zero.
- Con `UDS_SETBUDDYINT` il buddy viene riletto dal controllo:
  `SpinCtrlDouble::get_value` parsa il testo del buddy (fonte di
  verità visibile) e usa `UDM_GETPOS` solo come fallback.
- Nei test i controlli che usano `SendMessage`/`SetWindowTextW`
  richiedono un **frame reale** (`Frame::builder()...build()`);
  `Frame::for_testing()` ha HWND nullo e i round-trip falliscono.

---

## Backend WinUI3 - Implementazione

### Fonti di Studio
Il progetto **windows-rs** (microsoft/windows-rs su GitHub) fornisce:
- `windows-reactor`: Pattern React-style per WinUI3
- `windows-core`: Binding WinRT per chiamate native
- `bindings`: 126 controlli WinUI3 definiti

### Dipendenze Necessarie
```toml
[dependencies.windows-core]
version = "0.62"
features = ["UI_Xaml_Controls", "UI_Xaml", "Foundation"]
```

### Pattern di Implementazione (da windows-rs)
```rust
// Creazione controllo WinUI3
use windows_core::Interface;

// Pattern da windows-reactor
bindings::Button::new().unwrap()      // Crea Button
bindings::Grid::new().unwrap()        // Crea Grid  
bindings::TextBlock::new().unwrap()   // Crea TextBlock

// Impostazione proprietà
btn.SetContent(&content.into())?;
btn.SetIsEnabled(true)?;

// Eventi
btn.Click(move |s, a| { /* handler */ })?;

// XAML Loading
bindings::XamlReader::Load(&xaml)?;
```

### Controlli Prioritari da Implementare
1. **Window/Frame**: ApplicationWindow, Frame
2. **Layout**: Grid, StackPanel, Canvas, Border
3. **Base Controls**: Button, TextBlock, TextBox, CheckBox, RadioButton
4. **Containers**: ScrollViewer, ListView, TreeView
5. **Navigation**: NavigationView, Pivot

---

## Differenze conAltri Backend

### vs Win32 Classico
| Aspetto | Win32 | WinUI3 |
|---------|-------|--------|
| Graphics | GDI/GDI+ | DirectX + Composition |
| Controls | Classic controls | XAML controls |
| DPI | Per-monitor manuale | Nativo |
| Animations | Manuali | Integrate |
| XAML | No | Sì |

### vs wxWidgets (altri progetti)
| Aspetto | wxWidgets wrapper | ru_wx nativo |
|---------|-------------------|--------------|
| Performance | Bridge overhead | Zero overhead |
| Manutenzione | Dipende da upstream | Controllo totale |
| Customizzazione | Limitata | Completa |
| Feature parity | wxWidgets features | Solo quello che serve |

### vs web-based (Electron/etc)
| Aspetto | Electron | Native Rust |
|---------|----------|-------------|
| Performance | JS overhead | Nativo |
| Memory | Pesante | Leggero |
| UI fidelity | HTML/CSS | Platform-native |

---

## Roadmap Implementazione

*Stato aggiornato a MP18 (2026-09-29) — verifiche mac saltate su richiesta
dell'utente; dettagli in `PIANO_SVILUPPO_WINUI3_XAML.md`,
`PIANO_PORTABILITA_COMPONENTI.md`, `PIANO_DEBUG_INSPECTOR.md`.*

### Fase 1: Convertire winui3 attuale
- [x] Analizzare windows-rs per pattern WinUI3 — backend `platform/winui3/`
      (19 moduli) + backend XAML `platform/winui3_xaml/` (29 moduli) documentati
      in `src/platform/winui3/mod.rs`
- [x] Convertire da `windows-sys` a `windows-core` + WinRT — `windows-core`
      0.62 usato dal bootstrap XAML (`bootstrap.rs`, `windows_core::link!`);
      `windows-sys` resta per le API classiche Win32 (scelta §9 del piano
      portabilità), `windows-reactor`/WinRT per il tree XAML
- [x] Implementare WindowBuilder con WinUI3 reale —
      `platform/winui3/window.rs` (`WindowBuilder` con chaining) +
      `XamlFrameBuilder` (`with_padding`/`with_spacing`) nel backend XAML
- [x] Implementare controlli base (Button, TextBlock, Grid) —
      `winui3/widgets.rs` e `winui3_xaml/controls/` (button, text_block,
      text_box, check_box, radio_button, combo_box, list_box, list_ctrl,
      tree_view, grid, slider, progress_bar, hyperlink, image, check_list_box)
- [x] Testare con esempio funzionante — 33 demo `wx_step1…wx_step9`
      avviati senza crash (MP10) + confronto visivo (MP11) + screenshot per
      step (MP12) + 6 demo mancanti Fasi 5-7 creati e verificati (MP14) +
      3 demo Grid Fase 8 creati e verificati + fix sort fisico in
      `grid.rs` (MP15); residui 4 demo Grid 8.4-8.7 per API mancanti.
      Verifica §8.8 (MP16): `cust_table_grid`/`grid_demo` OK su backend
      XAML + confronto win32 ≈ XAML; ⚠️ `winui3_grid_showcase` rende vuoto
      sul backend `winui3` (difetto da diagnosticare).
      Verifica §9.1/§9.2 (MP17): 12 esempi esistenti buildati e verificati con
      `--features winui3_xaml` (screenshot `mp17_*.png`) + **fix crash
      `music_player`** (panic `RefCell already mutably borrowed` in
      `panel_wnd_proc`/`WM_ERASEBKGND`, preesistente → `try_borrow`); 20
      minitest buildati e verificati (20/20 vivi, screenshot `mp17_mt_*.png`);
      ⚠️ residuo: `aui_manager_demo` label sovrapposte. **Fix MP18**: campi
      status bar multipli (`Frame::show()` ora fa il dispatch iniziale dei
      resize handler — Win32 non emette `WM_SIZE` alla prima show) e testo
      SysLink non renderizzato (ACTCTX runtime con manifest Common-Controls
      v6 in `App::new()` perché i binari esempio non incorporano il manifest);
      verificati con `mp18_final_sb.png` / `mp18_final_hl.png`, test 446/444
      ok; residui minori: label bottoni troncate in 6 minitest

### Fase 2: Backend macOS
- [x] Setup objc2 con Objective-C runtime — dipendenze `objc2-*` in `Cargo.toml`
- [x] Implementare NSApplication/NSWindow — `platform/appkit_frame.rs`,
      `appkit_controls.rs`, ~40 moduli `appkit_*`
- [x] Implementare controlli base AppKit — `appkit_controls.rs` + moduli dedicati
- [x] Event handling con closures — callback registrate sul frame (§F3)
      — *build/verifiche mac saltate su richiesta dell'utente*

### Fase 3: API Unificata
- [x] Definire trait comuni per Widget — API astratta `ru_wx::{App, Frame,
      Button, …}` con dispatch `#[cfg(target_os)]` in `platform/mod.rs`
- [x] Backend switching mechanism — dispatch per OS + feature
      `winui3_xaml` (abilita solo il modulo XAML; l'API astratta resta su
      Win32), fallback `stub_backend.rs`/`gtk_stubs.rs`
- [x] Esempi multi-piattaforma — `examples/` (API astratta) e
      `examples/winui3new/` (demo XAML), build via `build_all_examples.ps1` /
      `build_all.bat`

---

## Note Tecniche

### Windows App SDK
Per WinUI3 serve Windows App SDK runtime. Il setup può essere:
- **Framework-dependent**: Richiede runtime installato
- **Self-contained**: Bundle include runtime

### Binding Generation
I binding WinUI3 sono generati da windows-bindgen basato su .winmd metadata. Il pattern `bindings::Control::new()` è fornito da questi binding generati.

### Thread Model
- WinUI3: UI thread separato, marshalling necessario
- Win32: Message loop nel thread UI
- macOS: Main thread requirement

---

## Riferimenti

- [windows-rs](https://github.com/microsoft/windows-rs) - Repository principale per Rust Windows API
- [windows-reactor](https://github.com/microsoft/windows-rs/tree/master/crates/libs/reactor) - Pattern React per WinUI3
- [WinUI3 Docs](https://docs.microsoft.com/en-us/windows/apps/winui3/) - Documentazione Microsoft
- [windows-bindgen](https://github.com/microsoft/windows-rs/tree/master/crates/libs/bindgen) - Generatore binding
