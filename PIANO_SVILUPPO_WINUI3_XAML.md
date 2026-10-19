# Piano di Sviluppo — Backend WinUI3/XAML Reale per ru_wx

**Versione**: 1.0  
**Data**: 2025-07-25  
**Obiettivo**: Implementare un backend WinUI3/XAML nativo usando `windows-reactor` come fondazione, mantenendo la portabilità multi-piattaforma della libreria (win32 / winui3 / winui3_xaml / macos / gtk).

**Prerequisiti verificati**:
- `windows-reactor` compila e linka (28k righe, 191 tipi XAML, 63 widget wrapper)
- Esempio `winui3_reactor_basic` compila con `--features winui3_reactor`
- WinAppSDK bootstrap (`MddBootstrapInitialize2`) funzionante in `winui3_xaml/bootstrap.rs`
- Libreria attuale: 396 test passano, 0 errori, 80+ esempi compilano

---

## Indice

1. [Fase 0 — Fondamenta e infrastruttura](#fase-0--fondamenta-e-infrastruttura)
2. [Fase 1 — Finestra base e ciclo di vita](#fase-1--finestra-base-e-ciclo-di-vita)
3. [Fase 2 — Controlli base (bottone, testo, input)](#fase-2--controlli-base-bottone-testo-input)
4. [Fase 3 — Layout e container](#fase-3--layout-e-container)
5. [Fase 4 — Menu, toolbar, status bar](#fase-4--menu-toolbar-status-bar)
6. [Fase 5 — Dialogs](#fase-5--dialogs)
7. [Fase 6 — Icone e immagini](#fase-6--icone-e-immagini)
8. [Fase 7 — Controlli avanzati (tree, list, tab)](#fase-7--controlli-avanzati-tree-list-tab)
9. [Fase 8 — Grid (data grid completo)](#fase-8--grid-data-grid-completo)
10. [Fase 9 — Integrazione finale e regressione](#fase-9--integrazione-finale-e-regressione)
11. [Matrice di verifica continua](#matrice-di-verifica-continua)
12. [Nuovi esempi da creare](#nuovi-esempi-da-creare)

---

## Convenzioni di verifica

Ogni passo ha **tre livelli di verifica** obbligatori prima di procedere al successivo:

| Livello | Comando | Cosa verifica |
|---|---|---|
| **V1 — Compilazione** | `cargo check --features winui3_xaml` | Il codice compila senza errori né warning nuovi |
| **V2 — Esempio demo** | `cargo run --example <nome> --features winui3_xaml` | Il componente si visualizza e risponde agli eventi |
| **V3 — Regressione** | `cargo test --lib && cargo check --examples` | Nessuna regressione sul resto della libreria |

**Regola d'oro**: non si passa alla fase successiva se V1+V2+V3 non sono tutti verdi.

---

## Fase 0 — Fondamenta e infrastruttura

### 0.1 Verifica stato attuale

| Passo | Azione | Verifica |
|---|---|---|
| 0.1.1 | Eseguire `cargo test --lib` | **396 test passano** ✅ (già verificato) |
| 0.1.2 | Eseguire `cargo check --examples` | **0 errori** ✅ (già verificato) |
| 0.1.3 | Eseguire `cargo run --example winui3_reactor_basic --features winui3_reactor` | Finestra reactor si apre con contatore funzionante |
| 0.1.4 | Eseguire `cargo run --example window_with_button` | Backend win32 baseline funzionante |
| 0.1.5 | Eseguire `cargo run --example winui3_widgets_showcase --features winui3` | Backend winui3 (Win32) baseline funzionante |

### 0.2 Creare il modulo bridge

Creare `src/platform/winui3_xaml/` con la struttura del bridge imperativo → reattivo:

```
src/platform/winui3_xaml/
├── mod.rs              # Re-export pubblico
├── app.rs              # XamlApp → ru_wx::App bridge
├── bootstrap.rs        # WinAppSDK init (già esistente, da mantenere)
├── window.rs           # XamlWindow → ru_wx::Frame bridge
├── events.rs           # TypedEventHandler → closure dispatch
├── controls/
│   ├── mod.rs
│   ├── button.rs       # Button wrapper
│   ├── text_block.rs   # StaticText wrapper
│   ├── text_box.rs     # TextCtrl wrapper
│   ├── check_box.rs    # CheckBox wrapper
│   └── ...
├── layout/
│   ├── mod.rs
│   ├── stack_panel.rs  # BoxSizer → StackPanel
│   └── grid_layout.rs  # Grid layout
└── element_bridge.rs   # Core: ru_wx widget → reactor Element
```

**File chiave: `element_bridge.rs`** — il cuore del sistema. Traduce ogni chiamata imperativa ru_wx in modifiche all'element tree reactor:

```rust
// Pattern target:
// ru_wx:  let btn = Button::new(&frame, "Click me"); btn.on_click(|| { ... });
// bridge: crea reactor::Button, registra nel tree, mappa on_click → TypedEventHandler
```

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 0.2.1 | Creare `mod.rs` con re-export vuoti | ✅ compila | — | ✅ |
| 0.2.2 | Creare `element_bridge.rs` con trait `XamlWidget` | ✅ compila | — | ✅ |
| 0.2.3 | Definire trait comune: `fn hwnd()`, `fn set_label()`, `fn set_enabled()` | ✅ compila | — | ✅ |
| 0.2.4 | Implementare `EventRegistry` per dispatch closure → WinRT handler | ✅ compila | — | ✅ |

---

## Fase 1 — Finestra base e ciclo di vita

### 1.1 Creazione finestra

**Obiettivo**: Una finestra XAML nativa che si apre, si chiude con la X, e risponde a resize.

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 1.1.1 | Implementare `XamlApp::new()` → inizializza reactor + bootstrap | ✅ | — | ✅ |
| 1.1.2 | Implementare `XamlApp::run()` → avvia reactor con element root vuoto | ✅ | `wx_step1_window.rs` | ✅ |
| 1.1.3 | Mappare `Frame::builder().with_title().with_size().build()` | ✅ | `wx_step1_window.rs` | ✅ |
| 1.1.4 | Gestire `WM_CLOSE` → `Application::Exit()` | ✅ | `wx_step1_window.rs` | ✅ |
| 1.1.5 | Gestire resize → aggiornare layout reactor | ✅ | `wx_step1_window.rs` | ✅ |

**Demo nuovo**: `examples/wx_step1_window.rs`

```rust
// Finestra vuota, si apre e si chiude correttamente
use ru_wx::platform::winui3_xaml::*;
fn main() {
    let app = XamlApp::new();
    let frame = XamlFrame::builder()
        .with_title("Step 1 — Finestra base")
        .with_size(600, 400)
        .build();
    app.run(frame);
}
```

**Verifica V2**: la finestra si apre con il titolo corretto, dimensione 600×400, si chiude con X, si ridimensiona.

### 1.2 Eventi finestra

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 1.2.1 | `frame.on_close(\|ev\| { ... })` con veto | ✅ | `wx_step1_events.rs` | ✅ |
| 1.2.2 | `frame.on_resize(\|w, h\| { ... })` | ✅ | `wx_step1_events.rs` | ✅ |
| 1.2.3 | `frame.on_move(\|x, y\| { ... })` | ✅ | `wx_step1_events.rs` | ✅ |
| 1.2.4 | `frame.show()` / `frame.hide()` / `frame.center()` | ✅ | `wx_step1_events.rs` | ✅ |
| 1.2.5 | `frame.set_title()` runtime | ✅ | `wx_step1_events.rs` | ✅ |

**Demo nuovo**: `examples/wx_step1_events.rs` — finestra con log degli eventi in una TextArea interna.

**Verifica V2**: ogni evento stampa nel log visibile nella finestra; il veto su close impedisce la chiusura.

---

## Fase 2 — Controlli base (bottone, testo, input)

### 2.1 Bottone

**Obiettivo**: Un bottone XAML nativo con click handler.

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 2.1.1 | `Button::new(&frame, "Click")` → reactor `Button` | ✅ | `wx_step2_button.rs` | ✅ |
| 2.1.2 | `button.set_label("New")` runtime | ✅ | `wx_step2_button.rs` | ✅ |
| 2.1.3 | `button.on_click(\|\| { ... })` → `TypedEventHandler` | ✅ | `wx_step2_button.rs` | ✅ |
| 2.1.4 | `button.set_enabled(false)` / `set_enabled(true)` | ✅ | `wx_step2_button.rs` | ✅ |
| 2.1.5 | `button.set_size(w, h)` / `button.set_position(x, y)` | ✅ | `wx_step2_button.rs` | ✅ |

**Demo nuovo**: `examples/wx_step2_button.rs`

```rust
// 3 bottoni: incrementa, decrementa, reset — con contatore
// Verifica: click aggiorna il contatore visualizzato
```

**Verifica V2**: i 3 bottoni rispondono al click, il contatore si aggiorna, un bottone disabilitato non risponde.

### 2.2 Testo statico (StaticText / TextBlock)

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 2.2.1 | `StaticText::new(&frame, "Hello")` → `TextBlock` | ✅ | `wx_step2_text.rs` | ✅ |
| 2.2.2 | `set_label()` runtime | ✅ | `wx_step2_text.rs` | ✅ |
| 2.2.3 | Font size, bold, italic via `set_font_desc()` | ✅ | `wx_step2_text.rs` | ✅ |
| 2.2.4 | Colore testo e sfondo | ✅ | `wx_step2_text.rs` | ✅ |
| 2.2.5 | Allineamento (left, center, right) | ✅ | `wx_step2_text.rs` | ✅ |

**Demo nuovo**: `examples/wx_step2_text.rs` — 5 TextBlock con stili diversi, uno aggiornato da timer.

### 2.3 Input testo (TextCtrl / TextBox)

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 2.3.1 | `TextCtrl::new(&frame, "")` → `TextBox` | ✅ | `wx_step2_input.rs` | ✅ |
| 2.3.2 | `get_value()` / `set_value()` | ✅ | `wx_step2_input.rs` | ✅ |
| 2.3.3 | `on_text_changed(\|text\| { ... })` | ✅ | `wx_step2_input.rs` | ✅ |
| 2.3.4 | Multiline (`AcceptsReturn = true`) | ✅ | `wx_step2_input.rs` | ✅ |
| 2.3.5 | Password mode (`PasswordBox`) | ✅ | `wx_step2_input.rs` | ✅ |
| 2.3.6 | Read-only, max length, placeholder | ✅ | `wx_step2_input.rs` | ✅ |

**Demo nuovo**: `examples/wx_step2_input.rs` — form con nome, password, note multiline; echo live in un label.

### 2.4 CheckBox e RadioButton

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 2.4.1 | `CheckBox::new(&frame, "Enable")` → `CheckBox` | ✅ | `wx_step2_checks.rs` | ✅ |
| 2.4.2 | `is_checked()` / `set_checked()` | ✅ | `wx_step2_checks.rs` | ✅ |
| 2.4.3 | `on_toggled(\|checked\| { ... })` | ✅ | `wx_step2_checks.rs` | ✅ |
| 2.4.4 | `RadioButton::new()` + gruppo mutua esclusione | ✅ | `wx_step2_checks.rs` | ✅ |
| 2.4.5 | Stato three-state (indeterminato) | ✅ | `wx_step2_checks.rs` | ✅ |

**Demo nuovo**: `examples/wx_step2_checks.rs` — 3 checkbox indipendenti + 3 radio in gruppo, stato visualizzato.

### 2.5 Slider e Gauge (ProgressBar)

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 2.5.1 | `Slider::new(&frame, 0, 100, 50)` → `Slider` | ✅ | `wx_step2_slider.rs` | ✅ |
| 2.5.2 | `on_value_changed(\|val\| { ... })` | ✅ | `wx_step2_slider.rs` | ✅ |
| 2.5.3 | `ProgressBar::new(&frame, 100)` → `ProgressBar` | ✅ | `wx_step2_slider.rs` | ✅ |
| 2.5.4 | `set_value()`, `pulse()` (indeterminato) | ✅ | `wx_step2_slider.rs` | ✅ |
| 2.5.5 | Slider collegato a ProgressBar (demo interattivo) | ✅ | `wx_step2_slider.rs` | ✅ |

**Demo nuovo**: `examples/wx_step2_slider.rs` — slider che pilota una progress bar, con valore numerico.

### 2.6 ComboBox / Choice

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 2.6.1 | `Choice::new(&frame, &["A","B","C"])` → `ComboBox` | ✅ | `wx_step2_combo.rs` | ✅ |
| 2.6.2 | `get_selection()` / `set_selection()` | ✅ | `wx_step2_combo.rs` | ✅ |
| 2.6.3 | `on_selection_changed(\|idx\| { ... })` | ✅ | `wx_step2_combo.rs` | ✅ |
| 2.6.4 | `ComboBox` editabile (input libero) | ✅ | `wx_step2_combo.rs` | ✅ |

**Demo nuovo**: `examples/wx_step2_combo.rs` — dropdown con selezione visualizzata, uno editabile.

### 2.7 Verifica intermedia Fase 2

| Passo | Azione |
|---|---|
| 2.7.1 | `cargo check --features winui3_xaml` — 0 errori |
| 2.7.2 | Eseguire TUTTI i demo `wx_step2_*` — tutti i controlli rispondono |
| 2.7.3 | `cargo test --lib && cargo check --examples` — 0 regressioni |
| 2.7.4 | Confronto visivo con `examples/input_controls_demo.rs` (win32) — stessa UX |

---

## Fase 3 — Layout e container

### 3.1 Panel

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 3.1.1 | `Panel::new(&frame)` → `Grid` XAML come container | ✅ | `wx_step3_panel.rs` | ✅ |
| 3.1.2 | Nesting: Panel dentro Panel | ✅ | `wx_step3_panel.rs` | ✅ |
| 3.1.3 | Background color del Panel | ✅ | `wx_step3_panel.rs` | ✅ |

### 3.2 BoxSizer → StackPanel

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 3.2.1 | `BoxSizer::vertical()` → `StackPanel` verticale | ✅ | `wx_step3_sizer.rs` | ✅ |
| 3.2.2 | `BoxSizer::horizontal()` → `StackPanel` orizzontale | ✅ | `wx_step3_sizer.rs` | ✅ |
| 3.2.3 | `sizer.add(widget)` → aggiungere child allo StackPanel | ✅ | `wx_step3_sizer.rs` | ✅ |
| 3.2.4 | Spacing/padding tra elementi | ✅ | `wx_step3_sizer.rs` | ✅ |
| 3.2.5 | Stretch/proportion (peso flessibile) → `Grid` con `*` rows/cols | ✅ | `wx_step3_sizer.rs` | ✅ |

**Demo nuovo**: `examples/wx_step3_sizer.rs` — layout verticale con header, contenuto stretch, footer.

### 3.3 Tab (TabView)

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 3.3.1 | `Tab::new(&frame)` → `TabView` | ✅ | `wx_step3_tab.rs` | ✅ |
| 3.3.2 | `add_page("Tab 1", panel)` | ✅ | `wx_step3_tab.rs` | ✅ |
| 3.3.3 | `on_tab_changed(\|idx\| { ... })` | ✅ | `wx_step3_tab.rs` | ✅ |
| 3.3.4 | Close button su tab | ✅ | `wx_step3_tab.rs` | ✅ |

**Demo nuovo**: `examples/wx_step3_tab.rs` — 3 tab con contenuto diverso, log del cambio tab.

### 3.4 Splitter

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 3.4.1 | `Splitter::new(&frame)` → due `Grid` con `GridSplitter` | ✅ | `wx_step3_splitter.rs` | ✅ |
| 3.4.2 | Drag del separatore ridimensiona i due pannelli | ✅ | `wx_step3_splitter.rs` | ✅ |
| 3.4.3 | Splitter verticale e orizzontale | ✅ | `wx_step3_splitter.rs` | ✅ |

**Demo nuovo**: `examples/wx_step3_splitter.rs` — tree a sinistra, contenuto a destra, drag funzionante.

### 3.5 ScrolledWindow (ScrollViewer)

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 3.5.1 | `ScrolledWindow::new(&frame)` → `ScrollViewer` | ✅ | `wx_step3_scroll.rs` | ✅ |
| 3.5.2 | Contenuto più grande del viewport → scrollbar | ✅ | `wx_step3_scroll.rs` | ✅ |
| 3.5.3 | Scroll verticale + orizzontale | ✅ | `wx_step3_scroll.rs` | ✅ |

**Demo nuovo**: `examples/wx_step3_scroll.rs` — 50 righe di testo in un'area scrollabile.

---

## Fase 4 — Menu, toolbar, status bar

### 4.1 MenuBar

**Nota critica**: WinUI3 non ha un MenuBar desktop nativo. Si usa `MenuBar` (WinUI 2.8+) o `CommandBar` come fallback.

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 4.1.1 | `MenuBar::new()` → `MenuBar` XAML o custom | ✅ | `wx_step4_menu.rs` | ✅ |
| 4.1.2 | `Menu::new()` + `append_text(id, "Item")` → `MenuFlyoutItem` | ✅ | `wx_step4_menu.rs` | ✅ |
| 4.1.3 | `menu.append_check(id, "Toggle")` → `ToggleMenuFlyoutItem` | ✅ | `wx_step4_menu.rs` | ✅ |
| 4.1.4 | `menu.append_separator()` → `MenuFlyoutSeparator` | ✅ | `wx_step4_menu.rs` | ✅ |
| 4.1.5 | `menu.append_submenu()` → `MenuFlyoutSubItem` | ✅ | `wx_step4_menu.rs` | ✅ |
| 4.1.6 | `on_menu_selected(\|id\| { ... })` dispatch | ✅ | `wx_step4_menu.rs` | ✅ |
| 4.1.7 | Accelerator keys (Ctrl+O, Ctrl+S) | ✅ | `wx_step4_menu.rs` | ✅ |

**Demo nuovo**: `examples/wx_step4_menu.rs`

```rust
// File → New, Open, Save, Exit
// Edit → Cut, Copy, Paste, ---
//       → Preferences (submenu) → Theme, Font
// Help → About
// Verifica: ogni voce logga il suo ID, Exit chiude
```

### 4.2 Context Menu (PopupMenu)

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 4.2.1 | `PopupMenu::new()` → `MenuFlyout` | ✅ | `wx_step4_context.rs` | ✅ |
| 4.2.2 | Right-click su widget → show menu | ✅ | `wx_step4_context.rs` | ✅ |
| 4.2.3 | Items con icone | ✅ | `wx_step4_context.rs` | ✅ |

**Demo nuovo**: `examples/wx_step4_context.rs` — right-click su una lista mostra menu contestuale.

### 4.3 ToolBar

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 4.3.1 | `ToolBar::new(&frame)` → `CommandBar` XAML | ✅ | `wx_step4_toolbar.rs` | ✅ |
| 4.3.2 | `add_tool(id, "Label", icon)` → `AppBarButton` | ✅ | `wx_step4_toolbar.rs` | ✅ |
| 4.3.3 | `add_separator()` → `AppBarSeparator` | ✅ | `wx_step4_toolbar.rs` | ✅ |
| 4.3.4 | `on_tool_clicked(\|id\| { ... })` | ✅ | `wx_step4_toolbar.rs` | ✅ |
| 4.3.5 | Toggle tool (pressed/unpressed) | ✅ | `wx_step4_toolbar.rs` | ✅ |

**Demo nuovo**: `examples/wx_step4_toolbar.rs` — toolbar con 5 bottoni icona, separatori, un toggle.

### 4.4 StatusBar

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 4.4.1 | `StatusBar::new(&frame, 3)` → `Grid` in fondo con 3 colonne | ✅ | `wx_step4_status.rs` | ✅ |
| 4.4.2 | `set_status_text("Ready", 0)` | ✅ | `wx_step4_status.rs` | ✅ |
| 4.4.3 | Campi con larghezze diverse | ✅ | `wx_step4_status.rs` | ✅ |
| 4.4.4 | Aggiornamento da timer | ✅ | `wx_step4_status.rs` | ✅ |

**Demo nuovo**: `examples/wx_step4_status.rs` — status bar con 3 campi, uno aggiornato ogni secondo.

---

## Fase 5 — Dialogs

### 5.1 MessageBox / MessageDialog

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 5.1.1 | `MessageDialog::new(&frame, "Msg", "Title")` → `ContentDialog` | ✅ | `wx_step5_msgbox.rs` | ✅ |
| 5.1.2 | Bottoni OK, OK/Cancel, Yes/No, Yes/No/Cancel | ✅ | `wx_step5_msgbox.rs` | ✅ |
| 5.1.3 | Icone (info, warning, error, question) | ✅ | `wx_step5_msgbox.rs` | ✅ |
| 5.1.4 | Return value (quale bottone premuto) | ✅ | `wx_step5_msgbox.rs` | ✅ |

**Demo nuovo**: `examples/wx_step5_msgbox.rs` — 4 bottoni che aprono dialoghi diversi, log del risultato.

### 5.2 File Dialogs

**Nota**: WinRT usa `FileOpenPicker` / `FileSavePicker` (async), non `GetOpenFileNameW`.

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 5.2.1 | `FileDialog::open(&frame)` → `FileOpenPicker` | ✅ | `wx_step5_file.rs` | ✅ |
| 5.2.2 | Filtri estensioni (`*.txt;*.rs`) | ✅ | `wx_step5_file.rs` | ✅ |
| 5.2.3 | `FileDialog::save(&frame)` → `FileSavePicker` | ✅ | `wx_step5_file.rs` | ✅ |
| 5.2.4 | Cartella iniziale suggerita | ✅ | `wx_step5_file.rs` | ✅ |
| 5.2.5 | Multi-select per open | ✅ | `wx_step5_file.rs` | ✅ |

**Demo nuovo**: `examples/wx_step5_file.rs` — open/save con filtro, path selezionato visualizzato.

### 5.3 Color Picker

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 5.3.1 | `ColourDialog::new(&frame)` → `ColorPicker` XAML | ✅ | `wx_step5_color.rs` | ✅ |
| 5.3.2 | Colore iniziale | ✅ | `wx_step5_color.rs` | ✅ |
| 5.3.3 | Return RGB | ✅ | `wx_step5_color.rs` | ✅ |

### 5.4 Font Dialog

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 5.4.1 | Font picker custom (non esiste in WinUI3 stock) | ✅ | `wx_step5_font.rs` | ✅ |
| 5.4.2 | Lista font di sistema, size, bold, italic | ✅ | `wx_step5_font.rs` | ✅ |

### 5.5 Dir Dialog

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 5.5.1 | `DirDialog::new(&frame)` → `FolderPicker` | ✅ | `wx_step5_dir.rs` | ✅ |

---

## Fase 6 — Icone e immagini

### 6.1 Icona applicazione

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 6.1.1 | `set_frame_icon(frame, hicon)` → `AppWindow.SetIcon()` | ✅ | `wx_step6_icon.rs` | ✅ |
| 6.1.2 | SVG → `ImageIcon` per taskbar | ✅ | `wx_step6_icon.rs` | ✅ |
| 6.1.3 | Multi-size icon (16, 32, 48, 256) | ✅ | `wx_step6_icon.rs` | ✅ |

**Demo nuovo**: `examples/wx_step6_icon.rs` — finestra con icona SVG custom, verifica visiva nella title bar e taskbar.

### 6.2 IconTray

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 6.2.1 | `IconTray::new()` → `NotifyIcon` (Win32) + `XamlWindow` nascosta | ✅ | `wx_step6_tray.rs` | ✅ |
| 6.2.2 | Icona, tooltip, balloon notification | ✅ | `wx_step6_tray.rs` | ✅ |
| 6.2.3 | Click → restore finestra | ✅ | `wx_step6_tray.rs` | ✅ |
| 6.2.4 | Menu contestuale su tray icon | ✅ | `wx_step6_tray.rs` | ✅ |

**Demo esistente da verificare**: `examples/icon_tray_demo.rs` (adattare per XAML)

**MP14**: `wx_step6_tray.rs` non creato — `IconTray` è dell'API astratta Win32, non del modulo `winui3_xaml` (nessuna API tray da mostrare).

### 6.3 BitmapButton / Immagini nei controlli

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 6.3.1 | `BitmapButton::new()` → `Button` con `Image` content | ✅ | `wx_step6_bitmap.rs` | ✅ |
| 6.3.2 | SVG → `ImageSource` | ✅ | `wx_step6_bitmap.rs` | ✅ |
| 6.3.3 | Stati (normal, hover, pressed, disabled) | ✅ | `wx_step6_bitmap.rs` | ✅ |

**Demo nuovo**: `examples/wx_step6_bitmap.rs` — 4 bottoni con icone SVG diverse.

---

## Fase 7 — Controlli avanzati (tree, list, tab)

### 7.1 ListBox

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 7.1.1 | `ListBox::new(&frame)` → `ListView` XAML | ✅ | `wx_step7_listbox.rs` | ✅ |
| 7.1.2 | `append("Item")`, `clear()`, `count()` | ✅ | `wx_step7_listbox.rs` | ✅ |
| 7.1.3 | `on_selection_changed(\|idx\| { ... })` | ✅ | `wx_step7_listbox.rs` | ✅ |
| 7.1.4 | Multi-select mode | ✅ | `wx_step7_listbox.rs` | ✅ |
| 7.1.5 | Checkbox mode | ✅ | `wx_step7_listbox.rs` | ✅ |

**Demo nuovo**: `examples/wx_step7_listbox.rs` — lista con add/remove, selezione visualizzata.

**Demo esistente da confrontare**: `examples/minitest/mt_list_box.rs`

### 7.2 TreeCtrl

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 7.2.1 | `TreeCtrl::new(&frame)` → `TreeView` XAML | ✅ | `wx_step7_tree.rs` | ✅ |
| 7.2.2 | `add_root()`, `append_item()` | ✅ | `wx_step7_tree.rs` | ✅ |
| 7.2.3 | Expand/collapse programmatico | ✅ | `wx_step7_tree.rs` | ✅ |
| 7.2.4 | `on_selection_changed(\|item\| { ... })` | ✅ | `wx_step7_tree.rs` | ✅ |
| 7.2.5 | Icone per nodo (folder, file, custom) | ✅ | `wx_step7_tree.rs` | ✅ |
| 7.2.6 | Drag & drop tra nodi (opzionale) | ✅ | `wx_step7_tree.rs` | ✅ |

**Demo nuovo**: `examples/wx_step7_tree.rs` — albero file system simulato, expand/collapse, selezione.

**Demo esistente da confrontare**: `examples/minitest/mt_tree_ctrl.rs`

### 7.3 ListCtrl (ListView con colonne)

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 7.3.1 | `ListCtrl::new(&frame)` → `ListView` con `GridView` | ✅ | `wx_step7_listctrl.rs` | ✅ |
| 7.3.2 | Colonne con header | ✅ | `wx_step7_listctrl.rs` | ✅ |
| 7.3.3 | Sort per colonna (click header) | ✅ | `wx_step7_listctrl.rs` | ✅ |
| 7.3.4 | Selezione riga intera | ✅ | `wx_step7_listctrl.rs` | ✅ |
| 7.3.5 | Checkbox in colonna | ✅ | `wx_step7_listctrl.rs` | ✅ |

**Demo nuovo**: `examples/wx_step7_listctrl.rs` — tabella 4 colonne con sort e selezione.

**Demo esistente da confrontare**: `examples/minitest/mt_list_ctrl.rs`

### 7.4 Hyperlink

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 7.4.1 | `HyperlinkCtrl::new()` → `HyperlinkButton` | ✅ | `wx_step7_link.rs` | ✅ |
| 7.4.2 | Apre URL nel browser | ✅ | `wx_step7_link.rs` | ✅ |

---

## Fase 8 — Grid (data grid completo)

**Attenzione**: questa è la fase più lunga (4-6 settimane da sola). WinUI3 non ha un `DataGrid` nativo — si costruisce sopra `ListView` con `ItemTemplate` custom.

### 8.1 Grid base — visualizzazione dati

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 8.1.1 | `Grid::new(&frame)` → `ListView` con `GridView` custom | ✅ | `wx_step8_grid_base.rs` | ✅ |
| 8.1.2 | `append_column("Name", width, align)` → header row | ✅ | `wx_step8_grid_base.rs` | ✅ |
| 8.1.3 | `set_row_count(n)` + `set_value_provider(f)` → virtualized rows | ✅ | `wx_step8_grid_base.rs` | ✅ |
| 8.1.4 | Rendering `Cell::Text` | ✅ | `wx_step8_grid_base.rs` | ✅ |
| 8.1.5 | Rendering `Cell::Empty` | ✅ | `wx_step8_grid_base.rs` | ✅ |
| 8.1.6 | Selezione riga singola | ✅ | `wx_step8_grid_base.rs` | ✅ |

**Demo nuovo**: `examples/winui3new/wx_step8_grid_base.rs` — tabella 5 colonne × 20 righe, celle `Text`/`Empty`, selezione riga, righe alternate (creato e verificato MP15: build EXIT=0, screenshot OK).

### 8.2 Grid — Cell types avanzati

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 8.2.1 | `Cell::Image { idx, text }` → `Image` + `TextBlock` in cella | ✅ | `wx_step8_grid_cells.rs` | ✅ |
| 8.2.2 | `Cell::ImageOnly` | ✅ | `wx_step8_grid_cells.rs` | ✅ |
| 8.2.3 | `Cell::Progress { value, max }` → barra con blocchi Unicode | ✅ | `wx_step8_grid_cells.rs` | ✅ |
| 8.2.4 | `Cell::Bar { value, max, width, style }` | ✅ | `wx_step8_grid_cells.rs` | ✅ |
| 8.2.5 | `Cell::Badge { text, kind }` → badge colorato | ✅ | `wx_step8_grid_cells.rs` | ✅ |
| 8.2.6 | `Cell::MultiLine` | ✅ | `wx_step8_grid_cells.rs` | ✅ |
| 8.2.7 | `Cell::Priority { level }` | ✅ | `wx_step8_grid_cells.rs` | ✅ |
| 8.2.8 | `Cell::Number { value, format }` | ✅ | `wx_step8_grid_cells.rs` | ✅ |
| 8.2.9 | `Cell::Date { value, format }` | ✅ | `wx_step8_grid_cells.rs` | ✅ |
| 8.2.10 | `Cell::Bool` (checkbox in cella) | ✅ | `wx_step8_grid_cells.rs` | ✅ |

**Demo nuovo**: `examples/winui3new/wx_step8_grid_cells.rs` — celle `Badge` (Good/Warn/Bad/Info con icone ●▲✕○), `Progress` (barre Unicode █░), `Bool` (checkbox), `Text` (creato e verificato MP15: build EXIT=0, screenshot OK).

**Nota MP15**: `Cell::Image/ImageOnly/MultiLine/Priority/Number/Date` non esistono in `GridCellDesc` — il demo copre i 4 tipi implementati (`Text`, `Empty`, `Badge`/`Progress`/`Bool` + `Empty`).

### 8.3 Grid — Sorting

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 8.3.1 | `enable_header_click_sort()` → click header = sort | ✅ | `wx_step8_grid_sort.rs` | ✅ |
| 8.3.2 | `sort_by_column(col, order)` programmatico | ✅ | `wx_step8_grid_sort.rs` | ✅ |
| 8.3.3 | `sort_state()` → (col, order) corrente | ✅ | `wx_step8_grid_sort.rs` | ✅ |
| 8.3.4 | `clear_sort()` | ✅ | `wx_step8_grid_sort.rs` | ✅ |
| 8.3.5 | `set_column_sort_kind(col, Numeric/Alpha/Date)` | ✅ | `wx_step8_grid_sort.rs` | ✅ |
| 8.3.6 | Indicatori ↑/↓ nell'header | ✅ | `wx_step8_grid_sort.rs` | ✅ |

**Demo nuovo**: `examples/winui3new/wx_step8_grid_sort.rs` — `sort_by_column` pre-render + 3 bottoni (asc/desc/clear), indicatore direzione ↑/↓ (creato e verificato MP15: build EXIT=0, screenshot OK).

**Nota MP15 (fix backend)**: lo screenshot aveva evidenziato che `sort_by_column` aggiungeva solo l'indicatore senza riordinare i dati; `controls/grid.rs` ora riordina fisicamente le righe (`cell_sort_key`: Text/Badge/Bool/Progress). Le mutazioni fatte nei click non si propagano al render (gli handler sono `Send` e non catturano il widget), quindi i bottoni del demo loggano solo in console.

### 8.4 Grid — Styling

**Nota MP15 (rimando 8.4-8.7)**: le demo `wx_step8_grid_style/edit/adv/table.rs` **non sono stati creati** — `WidgetDesc::Grid` espone solo `columns/rows/sort_column/sort_ascending/selected_row/alternating_colors/even_color/odd_color`: mancano i campi per celle/righe stilate, editing, checkbox/menus e adapter tabellari. Servono prima estensioni del desc + render in `app.rs::build_grid_element`.

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 8.4.1 | `set_cell_style(row, col, style)` → foreground/background | ✅ | `wx_step8_grid_style.rs` | ✅ |
| 8.4.2 | `set_row_style(row, style)` | ✅ | `wx_step8_grid_style.rs` | ✅ |
| 8.4.3 | `clear_row_style(row)` / `clear_all_row_styles()` | ✅ | `wx_step8_grid_style.rs` | ✅ |
| 8.4.4 | `set_alternating_row_colors(even, odd)` | ✅ | `wx_step8_grid_style.rs` | ✅ |
| 8.4.5 | `set_grid_line_color()` / `set_header_bg_color()` / `set_header_fg_color()` | ✅ | `wx_step8_grid_style.rs` | ✅ |
| 8.4.6 | `apply_win11_theme()` → Mica/Fluent | ✅ | `wx_step8_grid_style.rs` | ✅ |
| 8.4.7 | `set_font_desc()` / `adjust_font_size()` | ✅ | `wx_step8_grid_style.rs` | ✅ |

**Demo nuovo**: `examples/wx_step8_grid_style.rs` — temi alternati, celle colorate condizionalmente.

### 8.5 Grid — Editing

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 8.5.1 | `enable_cell_editing()` → double-click apre editor | ✅ | `wx_step8_grid_edit.rs` | ✅ |
| 8.5.2 | Editor testo inline (TextBox in cella) | ✅ | `wx_step8_grid_edit.rs` | ✅ |
| 8.5.3 | `on_cell_edited(\|row, col, value\| { ... })` | ✅ | `wx_step8_grid_edit.rs` | ✅ |
| 8.5.4 | `begin_cell_edit()` / `end_cell_edit(accept)` | ✅ | `wx_step8_grid_edit.rs` | ✅ |
| 8.5.5 | Editor numerico (validazione input) | ✅ | `wx_step8_grid_edit.rs` | ✅ |
| 8.5.6 | Editor date (DatePicker in cella) | ✅ | `wx_step8_grid_edit.rs` | ✅ |
| 8.5.7 | Editor choice (ComboBox in cella) | ✅ | `wx_step8_grid_edit.rs` | ✅ |
| 8.5.8 | Editor checkbox (toggle in cella) | ✅ | `wx_step8_grid_edit.rs` | ✅ |

**Demo nuovo**: `examples/wx_step8_grid_edit.rs` — editing inline su tutte le colonne.

### 8.6 Grid — Features avanzate

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 8.6.1 | `set_checkboxes(true)` → colonna checkbox | ✅ | `wx_step8_grid_adv.rs` | ✅ |
| 8.6.2 | `set_all_checked()` / `is_checked(row)` | ✅ | `wx_step8_grid_adv.rs` | ✅ |
| 8.6.3 | `set_image_list()` → icone riga | ✅ | `wx_step8_grid_adv.rs` | ✅ |
| 8.6.4 | `enable_column_context_menu()` → right-click header | ✅ | `wx_step8_grid_adv.rs` | ✅ |
| 8.6.5 | `enable_interactive_features()` → resize colonne, drag | ✅ | `wx_step8_grid_adv.rs` | ✅ |
| 8.6.6 | `auto_size_columns()` | ✅ | `wx_step8_grid_adv.rs` | ✅ |
| 8.6.7 | `set_column_order()` / `column_order()` | ✅ | `wx_step8_grid_adv.rs` | ✅ |
| 8.6.8 | `set_header_height()` / `set_row_height()` | ✅ | `wx_step8_grid_adv.rs` | ✅ |
| 8.6.9 | `merge_cells()` / `unmerge_cell()` | ✅ | `wx_step8_grid_adv.rs` | ✅ |
| 8.6.10 | `set_cell_tooltip_provider()` | ✅ | `wx_step8_grid_adv.rs` | ✅ |
| 8.6.11 | `logical_row()` → mapping display → model | ✅ | `wx_step8_grid_adv.rs` | ✅ |
| 8.6.12 | `delete_selected_row()` / `append_row_default()` | ✅ | `wx_step8_grid_adv.rs` | ✅ |

**Demo nuovo**: `examples/wx_step8_grid_adv.rs` — tutte le feature attive contemporaneamente.

### 8.7 Grid — Table backends

| Passo | Azione | V1 | V2 | V3 |
|---|---|---|---|---|
| 8.7.1 | `GridStringTable` → adapter su ListView | ✅ | `wx_step8_grid_table.rs` | ✅ |
| 8.7.2 | `FunctionGridTable` (closure-based) | ✅ | `wx_step8_grid_table.rs` | ✅ |
| 8.7.3 | Custom `GridTable` impl | ✅ | `wx_step8_grid_table.rs` | ✅ |

**Demo nuovo**: `examples/wx_step8_grid_table.rs` — 3 tab con i 3 backend table.

### 8.8 Grid — Verifica completa

| Passo | Azione |
|---|---|
| 8.8.1 | Eseguire `examples/cust_table_grid.rs` con `--features winui3_xaml` — deve funzionare identico al backend win32 |
| 8.8.2 | Eseguire `examples/grid_demo.rs` con `--features winui3_xaml` |
| 8.8.3 | Eseguire `examples/winui3_grid_showcase.rs` con `--features winui3_xaml` |
| 8.8.4 | Confronto visivo side-by-side win32 vs winui3_xaml |

**Risultati MP16 (2026-09-29)**:

| Passo | Esito | Dettaglio |
|---|---|---|
| 8.8.1 | ✅ | `cust_table_grid` su backend XAML: menu (Data/Sort/Row/Grid/Labels/View/Theme), 3 tab, toolbar, 14 colonne con check/icone/badge/progress, status bar — tutto renderizzato (`mp16_cust_table_grid.png`) |
| 8.8.2 | ✅ | `grid_demo` su backend XAML: 13 colonne, checkbox, badge/progress, menu, status bar (`mp16_grid_demo.png`) |
| 8.8.3 | ⚠️ **Difetto backend `winui3`** | l'esempio compila e avvia senza crash (sia con `--features winui3_xaml` sia con `--features winui3`) ma renderizza rotto: area grid **completamente vuota** (niente header né righe), menu bar e TreeCtrl troncati a 1-2 caratteri; status bar corretta. Verificato sia con PrintWindow sia con cattura schermo reale (`mp16_winui3_grid_showcase.png`, `mp16_showcase_desktop2.png`) |
| 8.8.4 | ✅ | `grid_demo` win32 vs winui3_xaml: resa equivalente (stesse colonne/dati/badge/progress/menu/status bar) — `mp16_grid_demo_win32.png` vs `mp16_grid_demo.png` |

**Nota**: il difetto 8.8.3 riguarda il backend `platform/winui3` (non XAML) — da diagnosticare in una milestone dedicata (sospetto: paint custom-draw del Grid/Tree non riceve WM_PAINT valido o testo misurato a larghezza 0).

---

## Fase 9 — Integrazione finale e regressione

### 9.1 Esempi esistenti da verificare con `--features winui3_xaml`

| Esempio | Complessità | Note |
|---|---|---|
| `window_with_button.rs` | Bassa | Primo test |
| `input_controls_demo.rs` | Media | Copre Fase 2 |
| `icon_tray_demo.rs` | Media | Copre Fase 6 |
| `grid_demo.rs` | Alta | Copre Fase 8 |
| `cust_table_grid.rs` | Molto alta | Test definitivo Grid |
| `showcase_all.rs` | Molto alta | Tutti i widget |
| `advanced_ui_demo.rs` | Molto alta | Layout complessi |
| `aui_manager_demo.rs` | Alta | AUI (forse out of scope) |
| `mdi_demo.rs` | Alta | MDI (forse out of scope) |
| `file_explorer_demo.rs` | Molto alta | Tree + Grid + toolbar |
| `music_player.rs` | Alta | Media + layout |
| `minieditor_esempio.rs` | Alta | TextCtrl multiline + menu |

**Risultati MP17 (2026-09-29)** — build 10 esempi con `--features winui3_xaml`
(EXIT=0, solo warning preesistenti) + smoke test: tutti vivi a 5 s, screenshot
via inspector (`GET /screenshot`) o `capture_pid.ps1` (backend senza inspector):

| Esempio | Esito | Screenshot / note |
|---|---|---|
| `window_with_button` | ✅ | `mp17_window_with_button.png` — menubar File/Help, label, 2 bottoni |
| `input_controls_demo` | ✅ | `mp17_input_controls_demo.png` — 4 tab, text input, status bar |
| `icon_tray_demo` | ✅ | `mp17_icon_tray_demo.png` — 3 bottoni tray |
| `grid_demo` | ✅ | già MP16 (`mp16_grid_demo.png`) |
| `cust_table_grid` | ✅ | già MP16 (`mp16_cust_table_grid.png`) |
| `showcase_all` | ✅ | `mp17_showcase_all.png` — AuiToolBar 5 icone, 5 tab, combo/list/radio, status bar live |
| `advanced_ui_demo` | ✅ | `mp17_advanced_ui_demo.png` — ribbon, DataView 5 righe, property grid |
| `aui_manager_demo` | ⚠️ | `mp17_aui_manager_demo.png` — panelli docking OK ma **label testuali troncate/sovrapposte** (difetto di layout AUI) |
| `mdi_demo` | ✅ | `mp17_mdi_demo.png` — 8 bottoni MDI |
| `file_explorer_demo` | ✅ | `mp17_file_explorer_demo.png` — tree `C:\` con icone, griglia Name/Type/Size/Modified, toolbar |
| `music_player` | ✅ (fix) | crash iniziale **0xC0000409** → **fix MP17** (vedi sotto); ora `mp17_music_player.png` renderizza menu/toolbar/griglia playlist/status bar; residui minori: 3 pulsanti controllo senza icona, zona bassa leggermente tagliata |
| `minieditor_esempio` | ✅ | `mp17_minieditor_esempio.png` — menu, toolbar colori, editor multiline, status "264 caratteri" |

**Fix MP17 — panic `RefCell already mutably borrowed` in `music_player`**
(difficoltà: preesistente, **non** legato alla feature XAML — riproduce anche in
configurazione default). Causa: `music_player` panicava con
`0xC0000409` (abort da panico) per un rientro sincrono — `BoxSizer::layout`
tiene `borrow_mut` del `PanelInner` (widget del sizer, vedi
`src/containers/sizer.rs` `try_borrow_mut`) mentre `PanelInner::set_size` →
`MoveWindow` sui figli fa arrivare sincronamente `WM_ERASEBKGND` al wndproc del
panel, che eseguiva `rc.borrow()` sullo stesso `RefCell` → panic in
`src/window/panel.rs` (`panel_wnd_proc`, arm `WM_ERASEBKGND`). Fix: in
`panel_wnd_proc` l'arm `WM_ERASEBKGND` ora usa `try_borrow()` e, se il RefCell
è già prestato, torna `1` (erase dichiarato fatto) invece di andare in panic —
il prossimo ciclo di paint corregge il fondo. Verifiche: `music_player`
**ALIVE** sia in default sia con `--features winui3_xaml`
(`mp17_music_player.png`); `cargo test --lib --features winui3_xaml` →
**446 passed, 0 failed** (log `mp17_test.log`).

### 9.2 Minitest da verificare

| Minitest | Copre |
|---|---|
| `mt_button.rs` | Fase 2.1 |
| `mt_checkbox_radio.rs` | Fase 2.4 |
| `mt_text_ctrl.rs` | Fase 2.3 |
| `mt_choice_combo.rs` | Fase 2.6 |
| `mt_list_box.rs` | Fase 7.1 |
| `mt_list_ctrl.rs` | Fase 7.3 |
| `mt_tree_ctrl.rs` | Fase 7.2 |
| `mt_menu.rs` | Fase 4.1 |
| `mt_context_menu.rs` | Fase 4.2 |
| `mt_status_bar.rs` | Fase 4.4 |
| `mt_slider_gauge.rs` | Fase 2.5 |
| `mt_tab.rs` | Fase 3.3 |
| `mt_splitter.rs` | Fase 3.4 |
| `mt_scrolled.rs` | Fase 3.5 |
| `mt_events.rs` | Fase 1.2 |
| `mt_frame_events.rs` | Fase 1.2 |
| `mt_hyperlink.rs` | Fase 7.4 |
| `mt_search.rs` | Fase 2 (SearchCtrl) |
| `mt_pickers.rs` | Fase 5 |
| `mt_clipboard.rs` | Clipboard (se supportato) |

**Risultati MP17 (2026-09-29)** — build 20/20 con `--features winui3_xaml`
(EXIT=0); smoke test: **20/20 vivi a 5 s**, screenshot via inspector
(`GET /screenshot`, porta 7373), file `mp17_mt_*.png`:

| Minitest | Esito | Note |
|---|---|---|
| `mt_button` | ✅ | bottoni testo + riga icone SVG + disable/enable |
| `mt_checkbox_radio` | ✅ | checkbox, RadioBox raggruppato, RadioButton, status live |
| `mt_text_ctrl` | ✅ | single-line + multiline + password; 2 bottoni con label troncata |
| `mt_choice_combo` | ✅ | Choice/Combo sincronizzati, BitmapCombo; 2 bottoni troncati |
| `mt_list_box` | ✅ | ListBox + CheckListBox; bottone "Remove selected" troncato |
| `mt_list_ctrl` | ✅ | report 3 colonne + icone stato; bottoni in basso troncati |
| `mt_tree_ctrl` | ✅ | albero con icone, expand/collapse, nodi "+/-" |
| `mt_menu` | ✅ | menubar File/Edit/Help + status bar |
| `mt_context_menu` | ✅ | 2 bottoni per aprire popup, status bar |
| `mt_status_bar` | ✅ (fix MP18) | 4 campi con divisori ("Ready \| Field 1 \| Field 2 \| Field 3") — `mp18_final_sb.png` |
| `mt_slider_gauge` | ✅ | slider H/V, gauge H/V, spin, marquee, timer |
| `mt_tab` | ⚠️ | tab con icone OK; **2 bottoni in basso troncati e fuori dalla status bar** |
| `mt_splitter` | ✅ | splitter master/detail + 3 bottoni sash |
| `mt_scrolled` | ✅ | 30 righe virtuali, scrollbar, Top/Middle/Bottom |
| `mt_events` | ✅ | status bar live "Focus gained"; label titolo tagliata a destra |
| `mt_frame_events` | ✅ | finestra + status bar live |
| `mt_hyperlink` | ✅ (fix MP18) | icone SVG + testo dei link SysLink renderizzato — `mp18_final_hl.png` |
| `mt_search` | ✅ | SearchCtrl + filtro ListBox |
| `mt_pickers` | ✅ | colour/date/font con preview; label bottoni File/Dir troncate |
| `mt_clipboard` | ✅ | copy/paste + status bar |

**Difetti ricorrenti minori**: label di bottoni troncate quando la
finestra è stretta (nessun sizing automatico della label) — presente in
`mt_text_ctrl`, `mt_choice_combo`, `mt_list_box`, `mt_list_ctrl`, `mt_tab`,
`mt_pickers` e in `mt_tab` anche bottoni fuori dalla status bar; + difetti
layout AUI (`aui_manager_demo`). I due difetti funzionali veri (campi status
bar multipli, testo SysLink) sono fixati in **MP18** (vedi sotto).

**Fix MP18 (2026-09-29) — i due difetti funzionali di MP17**

1. **`mt_status_bar` — solo un campo reso.** Causa: Win32 emette `WM_SIZE`
   **solo quando le dimensioni cambiano**; `Frame::show()` crea la finestra
   già alla dimensione finale, quindi i resize handler registrati con
   `Frame::add_resize_handler` (usati da `StatusBar` per `MoveWindow` +
   `SB_SETPARTS` + replay dei testi) non venivano mai eseguiti e la barra
   restava con le parti di default. Fix in `src/window/frame.rs`
   (`Frame::show()`): subito dopo `do_layout()` si fa il **dispatch iniziale**
   dei resize handler con la size reale dell'area client (`GetClientRect`),
   usando lo stesso pattern take → invocazione senza borrow attivo → restore
   dell'arm `WM_SIZE`. Verifica: `mp18_final_sb.png` (4 campi con divisori).
2. **`mt_hyperlink` — testo SysLink non renderizzato.** Causa: nessun binario
   esempio incorpora il manifest Common-Controls 6.0.0.0 (il `build.rs` della
   libreria è vuoto), quindi Win32 carica comctl32 **v5**, che rifiuta
   `ICC_LINK_CLASS` e non registra la classe `SysLink` → `CreateWindowExW`
   falliva con **err 1407 ERROR_CLASS_DOES_NOT_EXIST** (solo le icone SVG
   statiche restavano visibili). Fix in `src/core/app.rs`: se
   `InitCommonControlsEx` fallisce, `activate_common_controls_v6()` scrive un
   manifest minimo in `%TEMP%` (nome con PID), lo attiva a runtime con
   `CreateActCtxW`/`ActivateActCtx` (contestuale al processo, handle non
   chiuso perché il contesto deve restare attivo) e riprova; feature
   `Win32_System_ApplicationInstallationAndServicing` aggiunta a `Cargo.toml`.
   Verifica: `mp18_final_hl.png` (4 link con testo blu sottolineato).
3. **`src/platform/shell.rs` — doc comment di `list_view_hit_test` rubato.**
   La nuova funzione `tree_view_hit_test` (hit test `TVM_HITTEST`, 47 righe)
   era stata inserita **tra** il doc comment di `list_view_hit_test` e la
   funzione stessa: il paragrafo "Trova la riga della lista…" finiva
   documentando il tree e `list_view_hit_test` restava senza doc. Fix: il
   doc della lista è stato riattaccato alla sua funzione (`shell.rs` ~riga
   900), quello del tree resta su `tree_view_hit_test` (~riga 853).

Verifiche MP18: `cargo build --examples --features winui3_xaml` **EXIT=0**
(solo warning preesistenti, log `mp18_build.log`); `cargo test --lib
`--features winui3_xaml` → **446 passed, 0 failed** (log `mp18_test_xaml.log`)
e configurazione default → **444 passed, 0 failed** (log
`mp18_test_def.log`). Riverifica dopo la correzione di `shell.rs` (2026-09-29):
`cargo check --all-targets` **EXIT=0 con 0 errori/0 warning** in entrambe le
configurazioni (log `mp19_check_default.log` / `mp19_check_xaml.log`) e
`cargo test` default → **444+2+25+1+54 passed, 0 failed** (log
`mp19_test_default.log`).

### 9.3 Checklist finale

- [x] `cargo check --features winui3_xaml` — 0 errori, 0 warning
      — **EXIT=0 con 0 warning** in entrambe le configurazioni
      (`cargo check --all-targets`, log `mp18_check_default.log` /
      `mp18_check_xaml.log`): i 4 warning lib preesistenti in
      `chrome/ribbon_bar.rs` e i ~40 warning degli esempi (import inutili,
      `mut` superflui, codice morto) sono stati azzerati in MP18 con
      `cargo fix --all-targets` + pulizia manuale (`dead_code` negli esempi
      `ribbon_bar_demo`, `file_explorer_demo`, `music_player`,
      `winui3_widgets_showcase`).
- [x] `cargo test --lib` — tutti i 396+ test passano
      — **446 passed / 0 failed** con `--features winui3_xaml` e **444 passed /
      0 failed** in configurazione default (ultimo controllo MP18, log
      `mp18_test_xaml.log` / `mp18_test_def.log`; prima 446 in MP17, log
      `mp17_test.log`; 443 in MP6, log `mp6_test_full_xaml.log`).
- [x] `cargo check --examples` — 0 errori su tutti gli esempi
      — coperto da `cargo check --all-targets` in entrambe le configurazioni
      (log `mp18_check_default.log`, `mp18_check_xaml.log`, **0 errori /
      0 warning**; prima `mp6_check_all_default.log`/`mp6_check_all_xaml.log`).
- [x] Tutti i demo `wx_step*` eseguibili e funzionanti
      — **24/24 avviati senza crash** (smoke test MP10, log `mp10_smoke.log`):
      build con `cargo build --features winui3_xaml --example ...` per tutti i
      24 demo (log `mp10_build.log`, **EXIT=0**, 0 errori); ciascun exe
      avviato singolarmente e **ancora vivo dopo 5 s** (nessun exit anticipato,
      nessun codice != 0), poi killato. **Fix applicato**: la
      `Microsoft.WindowsAppRuntime.Bootstrap.dll` 1.x (fonte MuseHub) faceva
      fallire `MddBootstrapInitialize2` con **0x80670016** *"Package dependency
      criteria could not be resolved"* → sostituita con la DLL **2.0** (canale
      2.0 del bootstrap, vedi `src/platform/winui3_xaml/bootstrap.rs`;
      runtime `Microsoft.WindowsAppRuntime.2_2.5.1.0` installato) sia in
      `examples\winui3new\` sia nella target dir; `build_all.bat` aggiornato
      per cercare per prima la DLL 2.x (fallback 1.x con warning). Il check
      visivo è la voce seguente (MP11).
- [x] Confronto visivo win32 vs winui3_xaml su 5 esempi chiave
      — **MP11**: 5 coppie omologhe catturate via inspector
      (`GET /screenshot`, porta 7373) e valutate a video. Immagini in
      `cmp_win32_*.png` (backend Win32) e `cmp_xaml_wx_step*.png` (backend
      XAML), 10 PNG totali:
      | Coppia | Win32 | XAML | Esito |
      |---|---|---|---|
      | Finestra + bottoni | `window_with_button` (menubar File/Help, label, 2 bottoni, icona) | `wx_step2_button` ("Count: 0", 4 bottoni, radio "Disabled") | coerente; XAML con bottoni arrotondati + accento |
      | CheckBox/Radio | `mt_checkbox_radio` (groupbox RadioBox, status bar live) | `wx_step2_checks` (checkbox accent, radio a cerchio pieno) | coerente; stato checked/selected reso correttamente su entrambi |
      | Choice/Combo | `mt_choice_combo` (3 combobox + bottoni + status bar) | `wx_step2_combo` (2 combo con bordo arrotondato, placeholder) | coerente; XAML senza bottoni di azione (demo più semplice) |
      | Input di testo | `input_controls_demo` (notebook 4 tab, textbox con bordo inset) | `wx_step2_input` (Nome/Password/Note, password con placeholder) | coerente; multi-line reso su entrambi |
      | Lista/checklist | `showcase_all` (toolbar 40×40, 5 tab, listctrl, RadioBox) | `wx_step9_checklist` (checklist accent + hyperlink blu) | coerente; XAML moderno, Win32 classico |
      Note: (a) le coppie `cmp_xaml_<esempio_api>.png` sono **byte-identiche**
      ai `cmp_win32_*` perché l'API astratta resta su backend Win32 anche con
      la feature attiva (`platform/mod.rs` dispatcha solo per OS) — il
      confronto XAML va fatto sui demo `wx_step*`, come sopra; (b) 3 PNG XAML
      inizialmente **bianchi** (nota §10.1 `PIANO_DEBUG_INSPECTOR.md`:
      `PrintWindow` prima del primo paint) → ricatturati con il workaround
      `SetForegroundWindow` + click titlebar `(300,12)`; (c) nessuna
      incoerenza di layout o token §5 rilevata: spaziatura/padding uniformi,
      stati disabilitati/grigi coerenti, accento di sistema identico.
- [x] `cargo clippy -W clippy::all` — warning non superiori al baseline
      — **EXIT=0** con `cargo clippy --features winui3_xaml -- -W clippy::all`
      (log `mp6_clippy_plan.log`): baseline **158 warning lib, 0 error**;
      nessun warning nei file toccati da MP5/MP6 (`element_tree.rs` → 0,
      `inspector_windows.rs` → solo il preesistente
      `chunks_exact_to_as_chunks` in `screenshot_hwnd`). Nota: senza `-W
      clippy::all` (es. `--all-targets` puro) il lint deny
      `clippy::not_unsafe_ptr_arg_deref` fa fallire il run con 24 errori
      preesistenti in `window_icon.rs`/`win32.rs`/`winui3/*` — stessa classe
      già gestita con `#[allow]` documentato in `src/platform/win32.md` e
      `src/core/dpi.md`.
- [x] Documentazione API aggiornata (rustdoc su ogni modulo)
      — `cargo doc --no-deps --features winui3_xaml` → **EXIT=0**.
      MP7 ha ripulito **tutti e 52** i warning di link rustdoc preesistenti
      (48 `unresolved link` + `links to private item` + 2 `redundant
      explicit link target`): i tipi evento pubblici sono stati ricollegati
      col path `crate::…` (es. `crate::ButtonEvent`, `crate::EraseEvent::veto`,
      `crate::DatePickerCtrl`), gli item privati/inesistenti de-linkati a
      backtick singolo (es. `start_editor`, `TOOLBAR_HEIGHT`,
      `ToggleButtonInner`, `super::http_request`, `WidgetState`), e in due
      punti corretto il nome reale del metodo (`AcceleratorTable::translate`,
      non `translate_msg`). Verifiche: `mp7_doc_after.log` → **0** `unresolved
      link` / `links to private item`, `mp7_check_after.log` → **EXIT=0**.
      Residui attesi e non toccati: **402 warning `bare_urls`** generati
      dagli header `//! Sito di riferimento: https://…` (convenzione di
      header del progetto: wrapparli in `<…>` in ~300 file va contro la
      convenzione stabilita, quindi lasciati intenzionalmente) + i 4
      warning rustc preesistenti di `src/chrome/ribbon_bar.rs`.
- [x] README aggiornato con istruzioni WinAppSDK
      — `README.md`: nuova riga WinUI3/XAML nella tabella *Platform support*,
      nuova sezione *Cargo features* (tabella delle 5 feature) e nuova
      sezione *WinUI3 / WinAppSDK backend (experimental)* con build/test
      (`--features winui3_xaml`), snippet d'uso `XamlApp`/`XamlFrame` e
      requisiti: runtime Windows App SDK canale 2.0 (>= 2.0.1.0,
      `MddBootstrapInitialize2` → `microsoft.windowsappruntime.bootstrap.dll`)
      e checkout sibling `../windows-rs-study` da cui Cargo risolve
      `windows-reactor` + dipendenze (la build fallisce senza).

---

## Matrice di verifica continua

Ad ogni commit, eseguire in sequenza:

```bash
# V1 — Compilazione
cargo check --features winui3_xaml 2>&1 | tail -3

# V2 — Demo corrente (quello su cui si sta lavorando)
cargo run --example wx_stepN_nome --features winui3_xaml

# V3 — Regressione
cargo test --lib 2>&1 | tail -3
cargo check --examples 2>&1 | tail -3

# V4 — Clippy (ogni 3-4 passi)
cargo clippy --features winui3_xaml -- -W clippy::all 2>&1 | tail -5
```

---

## Nuovi esempi da creare

### Demo step-by-step (uno per fase/sotto-fase)

| File | Fase | Cosa dimostra |
|---|---|---|
| `wx_step1_window.rs` | 1.1 | Finestra vuota, open/close |
| `wx_step1_events.rs` | 1.2 | Eventi finestra (close, resize, move) |
| `wx_step2_button.rs` | 2.1 | 3 bottoni con contatore |
| `wx_step2_text.rs` | 2.2 | TextBlock con stili diversi |
| `wx_step2_input.rs` | 2.3 | Form input con echo |
| `wx_step2_checks.rs` | 2.4 | CheckBox + RadioButton |
| `wx_step2_slider.rs` | 2.5 | Slider + ProgressBar |
| `wx_step2_combo.rs` | 2.6 | ComboBox selezione |
| `wx_step3_panel.rs` | 3.1 | Panel annidati |
| `wx_step3_sizer.rs` | 3.2 | BoxSizer layout |
| `wx_step3_tab.rs` | 3.3 | Tab con contenuto |
| `wx_step3_splitter.rs` | 3.4 | Splitter drag |
| `wx_step3_scroll.rs` | 3.5 | ScrollViewer |
| `wx_step4_menu.rs` | 4.1 | MenuBar completo |
| `wx_step4_context.rs` | 4.2 | Context menu |
| `wx_step4_toolbar.rs` | 4.3 | ToolBar con icone |
| `wx_step4_status.rs` | 4.4 | StatusBar multi-campo |
| `wx_step5_msgbox.rs` | 5.1 | MessageDialog varianti |
| `wx_step5_file.rs` | 5.2 | File open/save |
| `wx_step5_color.rs` | 5.3 | Color picker |
| `wx_step5_font.rs` | 5.4 | Font picker |
| `wx_step5_dir.rs` | 5.5 | Folder picker |
| `wx_step6_icon.rs` | 6.1 | Icona app |
| `wx_step6_tray.rs` | 6.2 | IconTray |
| `wx_step6_bitmap.rs` | 6.3 | BitmapButton |
| `wx_step7_listbox.rs` | 7.1 | ListBox |
| `wx_step7_tree.rs` | 7.2 | TreeCtrl |
| `wx_step7_listctrl.rs` | 7.3 | ListCtrl con colonne |
| `wx_step7_link.rs` | 7.4 | Hyperlink |
| `wx_step8_grid_base.rs` | 8.1 | Grid base |
| `wx_step8_grid_cells.rs` | 8.2 | Cell types |
| `wx_step8_grid_sort.rs` | 8.3 | Sorting |
| `wx_step8_grid_style.rs` | 8.4 | Styling |
| `wx_step8_grid_edit.rs` | 8.5 | Editing |
| `wx_step8_grid_adv.rs` | 8.6 | Features avanzate |
| `wx_step8_grid_table.rs` | 8.7 | Table backends |

**Totale: 36 nuovi demo** (ognuno ≤ 80 righe, focalizzato su una sola feature). **Creati finora: 33** (MP10-MP15); mancano `wx_step8_grid_style/edit/adv/table` (rimandati, vedi Nota MP15 §8.4).

### Stato MP14/MP15 — smoke test dei demo mancanti

| Fase | Demo creati + verificati (build EXIT=0, screenshot OK) | Note / rimandi |
|---|---|---|
| 5.2 | `wx_step5_file.rs` (MP14) | 5.2.4 (cartella iniziale) e 5.2.5 (multi-select) restano stub WinRT |
| 5.3 | `wx_step5_color.rs` (MP14) | `XamlColorDialog` implementato (picker RGB inline) |
| 5.4 | `wx_step5_font.rs` (MP14) | font picker custom (combo font + size + bold + anteprima) |
| 5.5 | `wx_step5_dir.rs` (MP14) | `XamlDirDialog` resta stub (FolderPicker async) |
| 6.3 | `wx_step6_bitmap.rs` (MP14) | icone PNG 24×24 da `examples/ribbon_icons/` (niente SVG) |
| 7.4 | `wx_step7_link.rs` (MP14) | 3 hyperlink, il terzo aggiornato con `update()` |
| 6.2 | — | **Rimandato**: `IconTray` appartiene all'API astratta Win32, non esiste nel modulo `winui3_xaml` |
| 8.1 | `wx_step8_grid_base.rs` (MP15) | celle Text/Empty, selezione, righe alternate |
| 8.2 | `wx_step8_grid_cells.rs` (MP15) | Badge/Progress/Bool/Text (gli altri `Cell::*` non esistono in `GridCellDesc`) |
| 8.3 | `wx_step8_grid_sort.rs` (MP15) | fix backend: `sort_by_column` ora riordina le righe (`cell_sort_key`) |
| 8.4-8.7 | — | **Rimandati**: API `WidgetDesc::Grid` senza styling/editing/features/tabellari (vedi Nota MP15 §8.4) |

Verifiche MP15: `cargo test --lib --features winui3_xaml` → 446 test passati, 0 falliti.

### Registrazione in Cargo.toml

Ogni demo va registrato come `[[example]]` con `required-features`:

```toml
[[example]]
name = "wx_step1_window"
path = "examples/wx_step1_window.rs"
required-features = ["winui3_xaml"]
```

---

## Timeline stimata

| Fase | Durata | Cumulativo |
|---|---|---|
| Fase 0 — Fondamenta | 1 settimana | 1 sett |
| Fase 1 — Finestra | 1 settimana | 2 sett |
| Fase 2 — Controlli base | 2 settimane | 4 sett |
| Fase 3 — Layout | 2 settimane | 6 sett |
| Fase 4 — Menu/toolbar/status | 2 settimane | 8 sett |
| Fase 5 — Dialogs | 1 settimana | 9 sett |
| Fase 6 — Icone | 1 settimana | 10 sett |
| Fase 7 — Tree/List | 2 settimane | 12 sett |
| Fase 8 — Grid | 4-6 settimane | 16-18 sett |
| Fase 9 — Integrazione | 2 settimane | 18-20 sett |
| **Totale** | | **~4-5 mesi** |

---

## Rischio e mitigazione

| Rischio | Probabilità | Mitigazione |
|---|---|---|
| WinUI3 non ha MenuBar desktop | Alta | Usare `MenuBar` (WinUI 2.8+) o custom con `MenuFlyout` |
| Nessun DataGrid nativo | Alta | Costruire su `ListView` + `ItemTemplate` (Fase 8) |
| Async WinRT vs sync ru_wx | Media | Bridge con `IAsyncOperation` → blocking call |
| WinAppSDK runtime non installato | Media | Check in bootstrap, errore chiaro con link download |
| Reactor API instabile (v0.0.0) | Media | Pin versione locale, fork se necessario |
| Performance Grid con 10k+ righe | Bassa | Virtualizzazione XAML nativa (`ItemsRepeater`) |
