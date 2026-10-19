# Piano di Implementazione — Debug Inspector cross-platform (ru_wx)

> Obiettivo: dare a un agente IA (Cursor / Claude / VS Code) e a uno sviluppatore
> la capacità di **vedere** e **pilotare** la GUI di un'app costruita su `ru_wx`,
> in modo programmatico, **sia su Windows sia su macOS**.
>
> Questo documento è il **progetto** e insieme il **registro di avanzamento**:
> le fasi completate sono spuntate in §15, lo stato reale in §18.
> Le demo della libreria servono come banco di prova (cfr. §16).

---

## 1. Obiettivo e principi

Il pattern è lo stesso del *DevTools Protocol* di Chrome o dell'*Inspector* di
Flutter, adattato a una libreria di widget nativi:

| Capacità | Descrizione |
|---|---|
| **Stato** (semantica) | Albero dei componenti con tipo, contenuti, stato, colori e posizione reale — in JSON. L'IA analizza questo, **non** i pixel. |
| **Azione** | Iniezione di eventi nativi (click, move, key, text) verso un elemento identificato per `id`. |
| **Visione** (on-demand) | Screenshot PNG di tutta la finestra o di un singolo componente, richiesto **solo quando serve** (estetica/layout), per non sprecare token. |

Principi guida:

1. **Zero dipendenze pesanti.** Si riusano `std::net`, `serde_json` (già presente) e
   il meccanismo di marshaling già esistente in `ru_wx`. Niente `tokio`, niente
   `tungstenite`.
2. **Compilazione condizionale.** Il codice dell'inspector non entra nella build di
   release: `#[cfg(debug_assertions)]` + feature opzionale `inspector`.
3. **Stesso protocollo su Windows e macOS.** La logica (server, coda job, schema
   JSON) è *condivisa*; cambiano solo le 3 primitive native (`dump_tree`,
   `screenshot`, `inject_event`) e i getter/setter dei widget che le alimentano
   (vedi il principio a tre livelli, §2.4).
4. **Tutto dietro `ru_wx`.** Le chiamate AppKit / Win32 restano nei moduli
   `platform::*`, come da `DIRETTIVE_PROGETTO.md`.
5. **Tre livelli, una sola direzione.** Ogni lettura e ogni azione passa per
   `Inspector` → `Widget ru_wx` → `Backend nativo` → OS. L'inspector **non**
   importa mai `objc2`, `windows-sys` né i registri interni: chiede a `ru_wx`,
   che a sua volta chiede all'oggetto di sistema.
6. **Si scende di livello solo se il getter manca.** Se `ru_wx` non espone ancora
   ciò che serve (es. la posizione della finestra su macOS), si implementa la
   primitiva nel **backend**, si espone il **getter sul widget**, e solo dopo
   l'inspector può usarlo. L'inspector è il *volano* che completa l'API di
   `ru_wx`, non un suo sostituto.

---

## 2. Cosa esiste già e va riusato

Fondamentale: **il pezzo difficile esiste già ed è già cross-platform**.

### 2.1 Marshaling verso il main thread
`ru_wx` ha già un canale per far eseguire codice sul thread UI da un thread
qualsiasi (usato oggi dai worker async, es. la lettura cartelle in `rust_explorer`):

| | Windows | macOS |
|---|---|---|
| Invio | `Frame::post_raw_message()` → `PostMessageW(WM_USER+N)` | `Frame::post_raw_message()` → coda `RAW_QUEUE` (`Mutex<Vec<RawMessage>>`) |
| Esecuzione | `WndProc`, range `WM_USER+N` (`0x0400..=0x7FFF`) | `drain_raw_messages()` nel run loop, polling ogni 30 ms |
| API pubblica unica | `Frame::register_raw_message_handler(msg, f)` | idem |

File: `src/window/frame.rs` (`register_raw_message_handler`, `post_raw_message`,
`post_raw_message_to_hwnd`, `post_raw_message_ptr`) e
`src/platform/appkit.rs` (`RAW_HANDLERS`, `RAW_QUEUE`, `run_event_loop`).

> `post_raw_message_to_hwnd(hwnd, msg, w, l)` è **statico** e prende l'`hwnd` come
> `isize`: è esattamente ciò che serve al thread server, che non può possedere una
> `Frame` (`Frame` contiene `Rc<RefCell<…>>`, quindi non è `Send`).

### 2.2 Altro materiale riutilizzabile
- `src/core/app.rs` → `App::run(frame)` è il punto naturale di autostart in debug.
- `src/core/widget.rs` → trait `Widget` (`native_handle`, `rect`, `is_visible`,
  `is_enabled`) e tipo `WidgetRef`.
- `src/platform/mod.rs` → `NativeHandle = isize`, `as_hwnd()`.
- `src/platform/appkit.rs` → registri `thread_local` già esistenti:
  - `VIEWS: HashMap<NativeHandle, Retained<NSView>>`
  - `FRAME_CONTENT: HashMap<NativeHandle, Retained<NSView>>`
  - `FRAME_WINDOWS: HashMap<NativeHandle, Retained<NSWindow>>`
  - `VIEW_PARENT: HashMap<NativeHandle, NativeHandle>`, `POPUP_IS_COMBO`, …
- `serde_json` (dipendenza già presente) → serializzazione/parsing.
- `image` (dipendenza già presente, feature `png`) → codifica PNG su Windows.
- `src/net/socket_server.rs` → esempio di TCP server con `std::net` (non serve,
  usiamo `TcpListener` direttamente per avere `local_addr()`).

### 2.3 Dipendenza da aggiungere (una sola)
```toml
# ru_wx/Cargo.toml
serde = { version = "1", features = ["derive"] }
```
`serde_json` porta già `serde`, ma non le derive: servono per gli `struct` dello
schema (vedi §6) così come nel progetto originale.

### 2.4 Il principio a tre livelli (Inspector → Widget → Backend → OS)

Il flusso di una richiesta — esempio reale: *"quanto è grande la finestra?"*

```
[MCP / HTTP]  →  ru_wx::debug::Inspector
                    ↓  chiede a ru_wx (mai al sistema)
                 ru_wx::Frame::size()                       ← livello Widget
                    ↓  se il getter esiste
                 ru_wx::platform::mac_window_frame_size()    ← livello Backend (macOS)
                 ru_wx::platform::win_window_rect()          ← livello Backend (Windows)
                    ↓
                 NSWindow.frame.size / GetWindowRect         ← OS
                    ↓
                 (w, h) → JSON → risposta MCP
```

Regola operativa, da applicare a **ogni** campo e a **ogni** azione:

1. L'inspector chiama **solo** metodi `pub` di `ru_wx` (widget o `Frame`).
2. Se il metodo manca, si aggiunge al **widget** (getter/setter pubblico), e la
   sua implementazione chiama una primitiva di `platform::*`.
3. Se anche la primitiva manca, si aggiunge al **backend** per quel sistema
   operativo — è l'unico punto in cui si tocca `objc2` / `windows-sys`.
4. **Mai** il contrario: non si legge `FRAME_WINDOWS`/`VIEWS` da `debug/`, non si
   importa `objc2`/`windows-sys` da `debug/`.

Conseguenza importante: **un campo può restare `null` non perché l'OS non lo
sappia, ma perché `ru_wx` non ha ancora il getter**. Il piano quindi si estende
all'indietro: prima il backend, poi il widget, poi l'inspector.

### 2.5 Inventario delle primitive da implementare

Ricognizione del codice attuale (`ru_wx` v0.7.1). Tre gruppi.

#### A1 — Primitive backend (livello più basso)

Ricognizione del codice `ru_wx` v0.7.1 e stato dopo l'implementazione.

**macOS — `src/platform/appkit.rs`**

| Funzione | Stato | Note |
|---|---|---|
| `mac_window_frame_size(handle) -> (u32,u32)` | ✅ preesistente | `NSWindow.frame.size`, in punti |
| `mac_window_frame_rect(handle) -> Rect` | ✅ **implementata** | origine normalizzata top-left (`ns_rect_to_top_left`) |
| `mac_view_frame_rect(handle) -> Rect` | ✅ **implementata** | `convertRect_toView(None)` + `convertRectToScreen` |
| `mac_frame_get_title(handle) -> String` | ✅ preesistente (`pub`, già ri-esportata) | non serviva promuoverla |
| `mac_view_client_size(handle) -> (u32,u32)` | ✅ preesistente | |
| `frame_client_size(handle) -> (u32,u32)` | ✅ preesistente | |

**Windows — `src/platform/win32.rs`**

| Funzione | Stato | Note |
|---|---|---|
| `window_rect(hwnd) -> Rect` | ✅ **implementata** | `GetWindowRect`; vale anche per i controlli figli, quindi **non** serve una `win_view_rect` separata |
| `client_size(hwnd) -> (u32,u32)` | ✅ **implementata** | `GetClientRect`; unifica le copie locali in `winui3/app.rs`, `winui3/window.rs` |
| `read_window_text(hwnd) -> String` | ✅ preesistente (`pub`, ri-esportata) | `GetWindowTextW` |

**Dispatcher cross-platform — `src/platform/mod.rs`**

| Funzione | Stato | Note |
|---|---|---|
| `widget_screen_rect(handle) -> Rect` | ✅ **implementata** | `cfg` interno: `mac_view_frame_rect` / `win32::window_rect` / `gtk::widget_screen_rect`; usata dal default di `Widget::bounds_screen` |

#### A2 — Getter pubblici sul widget (livello `ru_wx`)

Prima dell'intervento `Frame` esponeva `hwnd()`, `set_size()`, `is_visible()` ma
**nessun getter simmetrico**.

| Getter | Stato | Implementazione |
|---|---|---|
| `Frame::size() -> (u32,u32)` | ✅ **implementato** | `win32::window_rect` / `mac_window_frame_size` |
| `Frame::position() -> (i32,i32)` | ✅ **implementato** | `win32::window_rect` / `mac_window_frame_rect` |
| `Frame::client_size() -> (u32,u32)` | ✅ **implementato** | `win32::client_size` / `frame_client_size` |
| `Frame::title() -> String` | ✅ **implementato** | `read_window_text` / `mac_frame_get_title` |
| `Widget::bounds_screen() -> Rect` | ✅ **implementato come metodo di default** | delega a `platform::widget_screen_rect` → **nessun widget concreto da toccare** |
| `TreeCtrl::get_item_text(item)` | ✅ **implementato** | getter su tutti i backend; su Linux delega a `gtk::tree_get_item_text` |
| `Button::text()`, `TextCtrl::value()` | ✅ **implementato** | alias simmetrici ai setter (`set_text`) su tutti i backend |
| `Grid::rows()`, `Grid::columns()`, `Grid::selected_rows()` | ✅ **implementato** | alias sui metodi esistenti; su Linux la selezione vive in `GridInner` (`last_selection` / `last_selection_rows`), aggiornata da `select_row` con dispatch dei callback |

> `Widget::rect()`, `is_visible()`, `is_enabled()` esistono già
> (`src/core/widget.rs`) e sono la base dei reader di §6.4.

#### A3 — Azioni sui controlli (stessa struttura a tre livelli)

Per "pilotare" un widget l'inspector deve invocare un **metodo del widget**, non
un messaggio grezzo all'OS:

| Livello | Esempio (testo pulsante) | Esempio (selezione riga) |
|---|---|---|
| Inspector (MCP) | `set_widget_text { id, text }` | `select_list_row { id, index }` |
| Widget `ru_wx` | `Button::set_title(&str)` | `ListCtrl::select(usize)` |
| Backend `platform::*` | `mac_button_set_title` / `SendMessageW(WM_SETTEXT)` | `listctrl_select` / `LBM_SETSEL` |
| OS | `NSButton.title =` / `SetWindowTextW` | `NSTableView.selectRowIndexes` / `LB_SETCURSEL` |

Vale la stessa regola: **se l'azione non esiste come metodo `pub` del widget, va
aggiunta al widget** prima di essere esposta via MCP. L'insieme delle azioni è
quindi, di fatto, l'insieme dei setter pubblici di `ru_wx` — che va completato
in parallelo ai getter.

---

## 3. Architettura

```
┌─────────────────────────── App Rust (ru_wx) ───────────────────────────┐
│                                                                        │
│  MAIN THREAD (UI)                          THREAD SERVER (worker)      │
│  ────────────────                          ──────────────────────      │
│  run loop / WndProc                        TcpListener 127.0.0.1:PORTA │
│      ▲                                          │                      │
│      │  WM_USER+INSPECTOR  ◄── marshaling ──    │  push job + oneshot  │
│      │  (post_raw_message)                      ▼                      │
│  drain_jobs():  ┌──────────────────────────────────────────┐           │
│   • dump_tree   │  CODA JOB  Mutex<VecDeque<Job>>          │           │
│   • screenshot  │  Job { req, resp_tx }                    │           │
│   • inject_evt  └──────────────────────────────────────────┘           │
│      │                                                                 │
│      ▼                                                                 │
│  ── LIVELLO 1: Inspector (debug/) ────────────────────────────────     │
│     compone la risposta JSON; chiama SOLO API pubbliche di ru_wx       │
│      │                                                                 │
│      ▼                                                                 │
│  ── LIVELLO 2: Widget ru_wx (Frame::size, Button::text, …) ───────     │
│     getter/setter pubblici; se mancano → vanno aggiunti qui            │
│      │                                                                 │
│      ▼                                                                 │
│  ── LIVELLO 3: Backend platform::* (cfg per OS) ──────────────────     │
│     primitive native: mac_window_frame_rect / win_window_rect / …      │
│      │                                                                 │
│      ▼                                                                 │
│     OS:  AppKit (NSWindow/NSView)   |   Win32 (HWND) / reactor (XAML)  │
└────────────────────────────────────────────────────────────────────────┘
                                   ▲
                    HTTP/JSON      │            MCP (stdio)
              (curl / script)      │      (Cursor / Claude / VS Code)
                                   │
                          ┌────────┴─────────┐
                          │  bridge MCP bin  │  ru_wx_mcp (processo separato)
                          └──────────────────┘
```

Due modi di accesso, **un solo motore**:

- **HTTP/JSON locale** (`127.0.0.1:PORTA`) — universale: testabile con `curl`,
  script Python, dashboard web.
- **Bridge MCP** (binario separato, trasporto stdio) — è ciò che rende i tool
  visibili automaticamente a Cursor/Claude. Non duplica logica: traduce
  JSON-RPC MCP ↔ HTTP verso l'app.

---

## 4. Attivazione e compilazione condizionale

Tre livelli, dal più comodo al più esplicito:

```rust
// 1) Autostart in debug: dentro App::run(), prima di frame.show()
#[cfg(debug_assertions)]
crate::debug::inspector::start(&frame);

// 2) Feature opt-in per build non-debug
#[cfg(feature = "inspector")]
...

// 3) API esplicita per l'app consumatrice
ru_wx::debug::inspector::start_with(&frame, InspectorConfig { port: 7373, ..Default::default() });
```

`Cargo.toml` di `ru_wx`:
```toml
[features]
inspector = []   # abilita il modulo anche in release (default: solo debug_assertions)
```

Configurazione:

| Parametro | Default | Note |
|---|---|---|
| `host` | `127.0.0.1` | mai `0.0.0.0` |
| `port` | `7373` | `0` = porta effimera, restituita da `start()` |
| `allow_events` | `true` in debug | inibisce `POST /event` (input injection) |
| `screenshot_max_px` | es. 4096 | limita dimensioni PNG |

`start()` ritorna `Option<u16>` (la porta effettivamente in ascolto).

---

## 5. Modello dei thread (marshaling) — dettaglio

1. `start(&frame)` è chiamata sul **main thread**. Registra l'handler:
   ```rust
   frame.register_raw_message_handler(RAW_MSG_INSPECTOR, move |_w, _l| drain_jobs());
   ```
2. Avvia il **thread server** (`std::thread::spawn`), passandogli solo:
   - `root: NativeHandle` (`frame.hwnd()`)
   - `port` (e la porta bindata via canale)
3. Il server, per ogni richiesta:
   - costruisce `Job { req, resp_tx: mpsc::Sender<Reply> }`, lo spinge nella
     `static JOBS: Mutex<VecDeque<Job>>`
   - chiama `Frame::post_raw_message_to_hwnd(root, RAW_MSG_INSPECTOR, 0, 0)`
   - attende la risposta con `recv_timeout(Duration::from_secs(5))`
4. Sul main thread l'handler svuota la coda ed esegue le query GUI
   (siamo sul thread giusto per AppKit/Win32) → `resp_tx.send(reply)`.

> Perché non passare il `Job` dentro `w`/`l`? Funziona (`post_raw_message_ptr` fa
> lo split 32/32 bit per non troncare il puntatore), ma la coda globale evita
> `Box::into_raw`/`from_raw` e il rischio di leak se il messaggio si perde.
> Si veda il commento in `frame.rs` sul crash `0x40000015` da puntatore troncato.

Codice messaggio: `RAW_MSG_INSPECTOR = 0x7FE0` (dentro `WM_USER..0x7FFF`, scelto
alto per non collidere con i messaggi dell'app; l'handler
`drain_jobs()` è idempotente).

**Variante XAML/reactor** (`winui3_xaml`). Il backend XAML non ha HWND: la finestra
è un *element tree* `windows-reactor` e il ciclo di messaggi è quello di WinUI
(`Application::Start`). Il passo 3 non può quindi usare
`post_raw_message_to_hwnd`. Al suo posto:

- il "handle" radice è il `frame_id` (u64) del frame;
- il thread server sveglia il thread UI accodando un job sul `DispatcherQueue`
  WinUI (`Microsoft.UI.Dispatching`), ottenuto dal reactor
  (`windows_reactor::WinUIDispatcher::for_current_thread()` → `UiMarshaller`,
  che è `Send + Sync` e accetta closure `Send`): `marshaller.dispatch(drain_jobs)`.
  `DispatcherQueue` è un oggetto agile, quindi la chiamata è lecita da un thread
  qualsiasi e `drain_jobs()` gira comunque sul thread UI.

L'inspector astrae questo in un'unica `static WAKE: Mutex<Option<Arc<dyn Fn() +
Send + Sync>>>`: `start_with` (Win32) vi installa il post del messaggio custom,
`start_xaml` (XAML) il dispatch sul `DispatcherQueue`. `dispatch()` chiama
`wake_ui()` senza sapere quale backend è attivo.

---

## 6. Modello dati (schema JSON)

### 6.1 Nodo componente

```rust
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct Bounds {          // coordinate SCHERMO, pixel reali
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Serialize, Clone)]
pub struct ColorInfo {       // RGBA 0..255, None se non determinabile
    pub text: Option<String>,       // "#RRGGBBAA"
    pub background: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct ComponentNode {
    pub id: String,                  // handle nativo in decimale, o "mac:0x.."
    pub component_type: String,      // classe nativa ("NSButton" / "Button")
    pub role: String,                // ruolo logico normalizzato: "button","label","table",…

    pub text: Option<String>,        // contenuto/testo visibile
    pub bounds: Bounds,              // posizione REALE su schermo
    pub visible: bool,
    pub enabled: bool,
    pub focused: bool,

    pub colors: ColorInfo,
    pub state: serde_json::Value,    // { checked, editable, selected_rows, … }
    pub extra: serde_json::Value,    // contenuti complessi (tabelle, liste, scroll)

    pub source: String,              // provenienza del dato: vedi sotto

    pub children: Vec<ComponentNode>,
}
```

Il campo `source` dice all'agente **quanto fidarsi** del nodo:

| Valore | Significato |
|---|---|
| `native` | letto dall'OS tramite il backend (AppKit / Win32 / reactor) |
| `registry` | fornito direttamente dal widget `ru_wx` |
| `inferred` | dedotto → **non** da considerare veritiero |
| `missing` | il getter di `ru_wx` non esiste ancora → campo `null` (§17.1) |

Note:

- `role` è **normalizzato** tra le piattaforme (mappa `NSButton`→`button`,
  `Button`→`button`, `NSTableView`/`SysListView32`→`table`, …) così l'IA non deve
  conoscere i nomi nativi.
- `component_type` resta la classe nativa, per debug a basso livello.
- `id` è stabile per la durata della finestra: su macOS è l'`NativeHandle` se il
  view è registrato in `VIEWS`/`CONTAINERS`, altrimenti `mac:<ptr>`; su Windows è
  l'`HWND` in decimale.
- `path` (indice posizionale, es. `"0/2/1"`) è utile per riferirsi a nodi non
  registrati; da valutare come campo aggiuntivo in `state`.

### 6.2 Contenuti dettagliati per widget (`extra` / `state`)

Requisito esplicito: **contenuti di form, label, tabelle, colori, posizioni reali**.

| Widget | Campi esposti |
|---|---|
| Label / StaticText | `text` (via `NSControl.stringValue` / `GetWindowTextW`) |
| TextField / Edit | `text`, `placeholder`, `editable`, `selectable`, `secure` |
| Button | `text`/`title`, `enabled`, `focused`, eventuale `image` |
| CheckBox / RadioButton | `checked`, `text` |
| Choice / ComboBox | `items: [..]`, `selected: [i]`, `editable` |
| ListBox | `items: [..]`, `selected: [i]`, `multi`, `count` |
| **Table / ListView** | `columns: [{title,width}]`, `rows: [[cell,..],..]`, `row_count`, `selected_rows: [i]`, `header_text: "col1 \| col2"` (fallback per la IA) |
| ScrollView | `scroll_x`, `scroll_y`, `content_w`, `content_h`, `viewport_w`, `viewport_h` |
| Slider / Spin / Gauge | `value`, `min`, `max` |
| Form (contenitore) | aggregazione dei figli `label`+`input` → comodo per l'IA |

Per le **tabelle** vale una nota di realismo:

- macOS: `NSTableView.numberOfRows()/numberOfColumns()/tableColumns()` +
  `viewAtColumn_row_makeIfNecessary()` per leggere il testo delle celle visibili
  (le righe fuori viewport non esistono come view → si leggono solo quelle
  visibili, oppure si passa dal `dataSource` registrato da `ru_wx`).
- Windows: `LVM_GETITEMCOUNT` + `LVM_GETITEMTEXTW` per riga/colonna. Il chiamante
  deve essere **il thread UI** (è il caso nostro).

### 6.3 Colori

| | macOS | Windows |
|---|---|---|
| testo | `NSTextField.textColor` → `NSColor` → `getRed_green_blue_alpha` (previa `colorUsingColorSpace: sRGB`) | `GetDC(hwnd)` + `GetTextColor(hdc)` + `ReleaseDC` |
| sfondo | `NSTextField.backgroundColor`, `NSView.layer.backgroundColor` se presente | `GetBkColor(hdc)` |
| note | molti view sono trasparenti → `null` è un valore legittimo | i controlli custom-drawn possono dare valori non significativi |

Su Windows i colori via `GetDC`/`GetTextColor` valgono per i controlli standard;
per owner-drawn/theme-drawn il valore può non riflettere il rendering reale →
`null`. È un limite noto e accettabile per uno strumento di debug.

### 6.4 Reader di interrogazione per-widget

Oltre ai campi comuni (§6.1), ogni widget è interrogato da un **reader** dedicato.
I nomi sono stabili e **identici sulle due piattaforme**: cambia solo l'API nativa
sotto. I reader vengono richiamati da `dump_tree` durante il walk dell'albero,
quindi ogni loro campo finisce nel JSON di `/tree` senza endpoint aggiuntivi.

| Reader | macOS | Windows | Campi prodotti |
|---|---|---|---|
| `read_text` | `NSControl.stringValue()`, `NSButton.title()` | `GetWindowTextW` | `text` |
| `read_input` | `NSTextField.isEditable()`, `isSelectable()`, `placeholderString()`, `NSSecureTextField` | `GetWindowLongPtrW(ES_*)` | `editable`, `selectable`, `placeholder`, `secure` |
| `read_check` | `NSButton.state()` | `BM_GETCHECK` | `checked` |
| `read_choice` | `NSPopUpButton` / `NSComboBox` items + selezione | `CB_GETCOUNT`, `CB_GETLBTEXT`, `CB_GETCURSEL` | `items[]`, `selected[]`, `editable` |
| `read_list` | `NSTableView` / `ListBoxView` items + `LISTBOX_SELECTED` | `LB_GETCOUNT`, `LB_GETTEXT`, `LB_GETCURSEL`, `LB_GETSELCOUNT` | `items[]`, `selected[]`, `multi`, `count` |
| `read_table` | `NSTableView.numberOfRows()/numberOfColumns()`, `tableColumns()`, `viewAtColumn_row_makeIfNecessary()`, `headerView()`, `selectedRowIndexes()` | `LVM_GETITEMCOUNT`, `LVM_GETITEMTEXTW`, `LVM_GETHEADER` + `HDM_GETITEMW`, `LVM_GETSELECTEDCOUNT` | `columns[{title,width}]`, `rows[[..]]`, `row_count`, `selected_rows[]`, `header_text`, `truncated` |
| `read_scroll` | `NSScrollView.contentView()` (`NSClipView`) → `bounds().origin`, `documentView()` | `GetScrollInfo(SB_HORZ/SB_VERT)` | `scroll_x`, `scroll_y`, `content_w/h`, `viewport_w/h` |
| `read_value` | `NSSlider` / `NSStepper.doubleValue` | `TBM_GETPOS`, `UDM_GETPOS` | `value`, `min`, `max` |
| `read_colors` | §6.3 | §6.3 | `colors.text`, `colors.background` |
| `read_state` | `isEnabled()`, `firstResponder`, `alignment()` | `IsWindowEnabled`, `GetFocus()`, class style | `enabled`, `focused`, `alignment` |

Nota tabelle: su macOS si leggono **solo le righe visibili** (le altre non esistono
come view) → `truncated: true`; su Windows `LVM_GETITEMTEXTW` le legge tutte.
Su macOS `read_table` è eseguibile **solo sul main thread** (è il caso garantito da §5).

---

## 7. Protocollo HTTP/JSON

Server minimale scritto a mano su `std::net::TcpListener` (una richiesta per
connessione; `Connection: close`). Parsing: request-line + header + body
`Content-Length`.

| Metodo | Path | Risposta |
|---|---|---|
| `GET` | `/health` | `{"ok":true,"platform":"macos","version":"0.7.1","pid":N}` |
| `GET` | `/tree` | albero completo (`ComponentNode` root) |
| `GET` | `/window` | metadati finestra: titolo, rect schermo, schermo/i, DPI |
| `GET` | `/find?text=..&role=..&id=..` | lista di nodi corrispondenti (id + bounds) |
| `GET` | `/screenshot` | PNG full-window (`Content-Type: image/png`) |
| `GET` | `/screenshot?id=<node>` | PNG ritagliato sul nodo (su XAML: finestra intera, vedi §10.1) |
| `POST` | `/event` | iniezione evento (§11) → `{"ok":true}` o errore |

Esempi:

```bash
curl -s localhost:7373/health
curl -s localhost:7373/tree | jq .
curl -s localhost:7373/screenshot?id=12345 -o button.png

curl -s -X POST localhost:7373/event \
  -d '{"action":"click","id":"12345"}'

curl -s -X POST localhost:7373/event \
  -d '{"action":"type_text","id":"12345","text":"admin"}'
```

Formato di errore uniforme:
```json
{ "ok": false, "error": { "code": "not_found", "message": "node 999 not found" } }
```

### 7.1 Selettori di `/find`

`/find` (e il tool MCP `find_element`) accetta questi parametri, combinabili in AND:

| Parametro | Semantica |
|---|---|
| `id` | handle esatto (`12345` su Windows, `12345`/`mac:0x…` su macOS) |
| `role` | ruolo normalizzato: `window`, `button`, `check`, `radio`, `group`, `label`, `input`, `choice`, `list`, `table`, `tree`, `tabs`, `scroll`, `value`, `image`, `container`, `webview`, `view` |
| `text` | substring sul testo visibile, case-insensitive |
| `path` | posizione nella gerarchia, es. `"0/2/1"` |

Risposta: lista di `{ id, role, component_type, text, bounds }`, ordine = profondità-first.
`find_element` è **derivato da `dump_tree`**, quindi è condiviso cross-platform senza
codice per-OS aggiuntivo.

---

## 8. Esposizione MCP

Binario separato `ru_wx_mcp` (nuovo `[[bin]]` o crate nel workspace) che parla
**MCP su stdio** e inoltra all'HTTP locale. Tool esposti:

| Tool MCP | Equivalente HTTP | Descrizione |
|---|---|---|
| `get_ui_tree` | `GET /tree` | albero componenti |
| `find_element` | `GET /find` | risolve un elemento per `id`/testo/ruolo |
| `get_window_info` | `GET /window` | titolo, rect, schermo, DPI |
| `interact_with_element` | `POST /event` | `click`, `move`, `key`, `type_text`, `focus` |
| `take_screenshot` | `GET /screenshot` | PNG **base64** (MCP è testuale) |
| `get_visual_snapshot` | `GET /screenshot?id=` | PNG ritagliato su un elemento |

Minimo indispensabile del protocollo MCP da implementare (JSON-RPC 2.0 su stdio,
righe o `Content-Length` frame):

- `initialize` → capabilities + `serverInfo`
- `tools/list` → schema JSON di ogni tool
- `tools/call` → esegue e ritorna `content: [{type:"text"|"image", …}]`
- `notifications/initialized` → no-op

Configurazione lato IDE (esempio Cursor/Claude Desktop):
```json
{
  "mcpServers": {
    "ru_wx_inspector": {
      "command": "/percorso/ru_wx_mcp",
      "args": ["--port", "7373"]
    }
  }
}
```

---

## 9. Backend macOS — mappatura API (Livello 3)

Quanto segue è **implementazione del livello Backend**: vive in
`src/platform/appkit*.rs`, gira **sul main thread** (garantito da §5) ed è
l'unico punto in cui si usa `objc2`. L'inspector (Livello 1) non lo chiama mai
direttamente: lo raggiunge tramite i metodi pubblici dei widget (Livello 2).

Requisito già accertato (§2.5/A1): su macOS **esiste** la dimensione finestra
(`mac_window_frame_size`) ma **manca la posizione** → va aggiunta
`mac_window_frame_rect`. Senza di essa `Frame::position()` non è implementabile.

### 9.1 Radice dell'albero
```rust
// da aggiungere in appkit.rs (accessor ai registri privati)
pub(crate) fn inspector_root(frame: NativeHandle) -> Option<Retained<NSView>>;   // FRAME_CONTENT, fallback FRAME_WINDOWS.contentView()
pub(crate) fn inspector_window(frame: NativeHandle) -> Option<Retained<NSWindow>>; // FRAME_WINDOWS
```

### 9.2 Walk + attributi (API verificate in `objc2-app-kit 0.3.2`)

| Dato | API |
|---|---|
| figli | `NSView::subviews()` → `NSArray<NSView>` |
| padre | `NSView::superview()`, `window()` |
| geometria | `NSView::frame()`, `bounds()`, `isFlipped()` |
| conversione | `NSView::convertRect_toView(rect, None)` → coord. finestra |
| **schermo** | `NSWindow::convertRectToScreen(rect)` |
| visibilità | `isHidden()`, `isHiddenOrHasHiddenAncestor()` |
| testo | `NSControl::stringValue()`, `NSButton::title()` |
| stato | `NSControl::isEnabled()`, `isHighlighted()`; `NSButton::state()` |
| font/align | `NSControl::font()`, `alignment()` |
| text field | `NSTextField::textColor()`, `placeholderString()`, `backgroundColor()`, `isEditable()`, `isSelectable()` |
| colori | `NSColor::colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())` + `getRed_green_blue_alpha` |
| tabella | `NSTableView::numberOfRows()`, `numberOfColumns()`, `tableColumns()`, `viewAtColumn_row_makeIfNecessary()`, `headerView()`, `selectedRowIndexes()` (iterazione con `NSIndexSet::firstIndex()` + `indexGreaterThanIndex()`) |
| colonne | `NSTableColumn::title()`, `width()`, `identifier()` |
| scroll | `NSScrollView::contentView()` (`NSClipView`) → `bounds().origin`; `documentView()`; `hasVerticalScroller()` |
| schermo | `NSScreen::mainScreen(mtm)`, `NSScreen::frame()` |
| nome classe | `AnyClass::name()` via `msg_send![obj, class]` (da confermare in fase di implementazione) |

### 9.3 Screenshot
```rust
// rect in coordinate view
let rep = view.bitmapImageRepForCachingDisplayInRect(rect);   // Option<Retained<NSBitmapImageRep>>
view.cacheDisplayInRect_toBitmapImageRep(rect, &rep);
let png = rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new());
let bytes = png.getBytes_length(...);                          // NSData → Vec<u8>
```
La libreria già usa `NSBitmapImageRep` (in `appkit.rs`), quindi la feature è attiva.
Alternativa senza AppKit: `CGWindowListCreateImage` (screenshot a livello sistema).

### 9.4 Iniezione eventi
- `CGEvent` (mouse move/down/up, key) + `CGEventPost(kCGHIDEventTap, ev)` da
  `objc2-core-graphics`.
- Requisito utente: l'app in debug è **già** quella che gira; i permessi
  *Accessibility* servono al processo che inietta (l'app stessa).

---

## 10. Backend Windows — mappatura API (Livello 3)

`windows-sys 0.59`, feature già abilitate in `ru_wx/Cargo.toml`
(`Win32_UI_WindowsAndMessaging`, `Win32_UI_Controls`, `Win32_Graphics_Gdi`,
`Win32_UI_Input_KeyboardAndMouse`, `Win32_UI_HiDpi`).

### 10.1 Tre backend Windows, tre sorgenti dell'albero

`ru_wx` non ha un solo backend Windows: ne ha **tre**, e la sorgente dei dati
cambia. Va deciso per backend, non per sistema operativo.

| Backend (`ru_wx`) | Fonte dell'albero | Lettura |
|---|---|---|
| `win32_aui`, `winui3` (Win32) | controlli **HWND** reali | `EnumChildWindows` + `GetWindowRect` + `GetClassNameW` (tabella sotto) |
| `winui3_xaml`, `winui3_reactor` | **element tree** di `windows-reactor` | `Element` / `PropValue` / `find_prop` / `Children` — **nessun HWND** |
| fallback (qualsiasi) | UIAutomation / MSAA | `AccessibleObjectFromWindow` (Oleacc) — da valutare |

> Su XAML `EnumChildWindows` **non trova nulla**: i widget non sono finestre.
> Il vantaggio è che l'albero `reactor` è già tipizzato (props con nome e valore
> tipizzati), quindi più ricco dell'introspezione HWND e senza permessi.
> Per questo motivo la strada XAML va considerata di pari rango, non un ripiego.

**Screenshot su XAML.** Il frame non ha un HWND proprio, quindi `PrintWindow`
non ha una finestra su cui operare. L'inspector ricade sull'HWND della finestra
**top-level WinUI3** del processo (`EnumWindows` + `GetWindowThreadProcessId`,
preferendo la prima finestra visibile con titolo) e la cattura con
`PrintWindow` + `PW_RENDERFULLCONTENT`. Di conseguenza `/screenshot` restituisce
sempre la **finestra intera**: il ritaglio per-nodo non è possibile senza un
layout pass (gli `bounds` XAML restano `w=0/h=0`, §6.3).

### 10.2 Mappatura API — backend Win32 (HWND)

| Dato | API |
|---|---|
| figli | `EnumChildWindows(parent, cb, lparam)` (chiude il `cb` con `true` per proseguire) |
| classe | `GetClassNameW` → `component_type` |
| testo | `GetWindowTextW` (+ `GetWindowTextLengthW`) |
| id | `GetDlgCtrlID` |
| padre | `GetParent` |
| **schermo** | `GetWindowRect` (rect già in coordinate schermo fisiche) |
| visibilità/stato | `IsWindowVisible`, `IsWindowEnabled`, `GetFocus()` per `focused` |
| colori | `GetDC` + `GetTextColor` / `GetBkColor` + `ReleaseDC` |
| scroll | `GetScrollInfo(hwnd, SB_HORZ/SB_VERT, &SCROLLINFO)` |
| lista | `LB_GETCOUNT`, `LB_GETTEXT`, `LB_GETCURSEL`, `LB_GETSELCOUNT` via `SendMessageW` |
| combo | `CB_GETCOUNT`, `CB_GETLBTEXT`, `CB_GETCURSEL` |
| **list view** | `LVM_GETITEMCOUNT`, `LVM_GETITEMTEXTW` (+ `LVITEMW`), `LVM_GETSELECTEDCOUNT`, header via `LVM_GETHEADER` + `HDM_GETITEMW` |
| DPI | `GetDpiForWindow` (da `core::dpi`) per normalizzare in pixel logici se serve |
| schermo | `GetSystemMetrics(SM_CXVIRTUALSCREEN, SM_XVIRTUALSCREEN, …)` |

Screenshot:
```rust
let hdc = GetWindowDC(hwnd);                 // o GetDC
let mem = CreateCompatibleDC(hdc);
let bmp = CreateCompatibleBitmap(hdc, w, h);
SelectObject(mem, bmp);
PrintWindow(hwnd, mem, PW_RENDERFULLCONTENT);  // cattura anche contenuti composited
// GetDIBits → buffer BGRA → image::RgbaImage → PNG
```
Si usa la crate `image` (già dipendenza) per la codifica PNG.

Iniezione eventi:
```rust
SetCursorPos(x, y);                          // assoluto, coordinate schermo
// oppure, meglio, SendInput con MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK
SendInput(1, &INPUT { type: INPUT_MOUSE, Anonymous: INPUT_0 { mi: MOUSEINPUT { .. } } }, size_of::<INPUT>() as i32);
// tastiera: INPUT_KEYBOARD + KEYBDINPUT (KEYEVENTF_UNICODE per il testo)
```

Nota `SendInput` è per input **a livello sistema** (muove il cursore vero). Per
un click "programmatico" senza spostare il puntatore si possono inviare
`WM_LBUTTONDOWN/WM_LBUTTONUP` con `SendMessageW(hwnd, …)` direttamente al
controllo mirato: più chirurgico, meno invasivo. **Da preferire** per
`interact_with_element`; `SendInput` come fallback per test end-to-end realistici.

---

## 11. Iniezione eventi — formato comune

```json
{ "action": "click",      "id": "12345" }
{ "action": "move",       "id": "12345" }
{ "action": "focus",      "id": "12345" }
{ "action": "type_text",  "id": "12345", "text": "admin" }
{ "action": "key",        "key": "enter", "modifiers": ["ctrl"] }
{ "action": "click",      "x": 100.0, "y": 200.0 }
```

Strategia `click` (identica concettualmente sui tre OS):

1. Risolvi `id` → nodo → `bounds` (coordinate schermo).
2. Calcola il centro (`x + w/2`, `y + h/2`).
3. macOS: `CGEvent` mouse-down/up in quel punto + post.
   Windows (preferito): `SendMessageW(hwnd, WM_LBUTTONDOWN/UP, …)` al controllo.
   Windows (fallback): `SetCursorPos` + `SendInput`.
   Linux: `Button::emit_clicked()` / `Widget::activate()` sul widget (§19.5) —
   nessun evento a livello X.
4. Attendi N ms il ciclo di eventi (per il repaint) e ritorna `{"ok":true}`.
   L'IA poi richiama `get_ui_tree` per verificare l'effetto **senza screenshot**.

Ogni azione è espressa come chiamata a un metodo `pub` del widget `ru_wx`
(Livello 2, cfr. §2.5/A3): l'inspector non invia messaggi grezzi all'OS. Se
l'azione richiesta non ha un corrispondente setter sul widget, va aggiunto lì
prima di esporla via MCP.

---

## 12. Schermate e coordinate — regole

> **Aggiornamento post-implementazione (Fase A).** Le regole iniziali
> prevedevano "pixel fisici" su entrambe le piattaforme. In implementazione si è
> scelta la **coerenza interna a ciascun OS**, perché è ciò che rende
> funzionante l'iniezione eventi: vedi sotto.

- Tutti i `bounds` hanno origine in **alto-sinistra** (convenzione uniforme) e
  usano l'**unità nativa dello spazio coordinate in cui verranno iniettati gli
  eventi**:
  - **macOS**: *punti* (`NSWindow.frame`, `convertRectToScreen`). `CGEvent`
    usa la stessa unità → nessuna conversione in fase di click.
  - **Windows**: *pixel fisici* (`GetWindowRect`). `SetCursorPos`/`SendInput`
    usano la stessa unità → nessuna conversione.
  - **Linux**: *pixel X11* (`XGetGeometry` sulla XID + `Widget::compute_bounds`
    §19.2). Le azioni non sono coordinate (§19.5) → nessuna conversione.
- Conversione di origine: macOS espone i frame con origine **bottom-left**
  rispetto allo schermo principale. La normalizzazione avviene nel backend
  (`ns_rect_to_top_left`, che sottrae `NSScreen.mainScreen.frame().height`), così
  l'IA vede sempre la stessa convenzione su entrambi i sistemi.
- Il fattore di scala (retina `backingScaleFactor` / DPI) è esposto **a parte**
  nei metadati finestra, non applicato ai `bounds`: l'agente lo usa solo se deve
  ragionare in pixel assoluti.
- Gli angoli dei rettangoli sono **arrotondati a interi** (macOS `CGEvent`
  accetta f64; Windows `SetCursorPos` è intero).
- Su schermi multipli: coordinate nello spazio virtuale unificato
  (`SM_XVIRTUALSCREEN/SM_YVIRTUALSCREEN` su Windows; griglia dei `NSScreen.frame()`
  su macOS).

---

## 13. Sicurezza

- Bind **solo** `127.0.0.1`; mai esposto in rete.
- Attivo **solo** in debug (`debug_assertions`) o con feature `inspector` esplicita.
- `POST /event` (input injection) disattivabile via `InspectorConfig::allow_events`.
- Nessun dato viene inviato all'esterno: il server è polled dall'agente locale.
- `human-panic` già presente: i panic del thread server non devono uccidere l'UI →
  il server cattura gli errori e risponde `{"ok":false,…}`.

---

## 14. Struttura file da creare

```
ru_wx/
├── src/
│   ├── lib.rs                        (+ pub mod debug;)
│   │
│   │   ── LIVELLO 2: getter/setter pubblici sui widget ────────────
│   ├── window/frame.rs               (+ size(), position(), client_size(), title())
│   ├── core/widget.rs                (+ bounds_screen(); rect/is_visible/is_enabled esistono)
│   ├── controls/<widget>.rs          (+ getter/setter mancanti, es. TreeCtrl::get_item_text)
│   │
│   │   ── LIVELLO 3: primitive native per-OS ──────────────────────
│   ├── platform/
│   │   ├── mod.rs                    (+ pub mod inspector;)
│   │   ├── inspector.rs              tipi comuni + dispatch cfg verso i backend
│   │   ├── inspector_macos.rs        dump_tree / screenshot / inject_event (AppKit)
│   │   ├── inspector_windows.rs      dump_tree / screenshot / inject_event (Win32 + reactor)
│   │   ├── inspector_gtk.rs          dump_tree / screenshot / inject_event (GTK4 + X11, §19)
│   │   ├── appkit.rs                 (+ mac_window_frame_rect, mac_view_frame_rect, accessor)
│   │   ├── appkit_frame.rs           (+ promozione pub di mac_frame_get_title)
│   │   ├── win32.rs                  (+ win_window_rect, win_view_rect, win_client_size, win_window_text)
│   │   └── winui3_xaml/              (+ dump_tree dall'element tree reactor)
│   │
│   │   ── LIVELLO 1: inspector ────────────────────────────────────
│   └── debug/
│       ├── mod.rs                    modulo + rustdoc
│       └── inspector.rs              CORE condiviso:
│                                     • InspectorConfig / start / stop
│                                     • HTTP server (std::net) + router
│                                     • coda job + protocollo reply
│                                     • RAW_MSG_INSPECTOR + drain_jobs()
│                                     • schema (ComponentNode, Bounds, ColorInfo)
└── src/bin/ru_wx_mcp.rs              bridge MCP (stdio ↔ HTTP locale)
```

Nuove dipendenze: solo `serde` (derive). Nessun runtime async.

### 14.1 Interfaccia interna (firme)

```rust
// ── Livello 2 — widget (getter/setter pubblici) ──────────────────────────
impl Frame {
    pub fn size(&self) -> (u32, u32);          // da aggiungere
    pub fn position(&self) -> (i32, i32);      // da aggiungere
    pub fn client_size(&self) -> (u32, u32);   // da aggiungere
    pub fn title(&self) -> String;             // da aggiungere
}
trait Widget {
    fn bounds_screen(&self) -> Rect;           // da aggiungere (pixel schermo)
    // rect(), is_visible(), is_enabled(), native_handle() esistono già
}

// ── Livello 1 — inspector (chiama SOLO il Livello 2) ─────────────────────
// core condiviso — src/debug/inspector.rs
pub struct InspectorConfig { pub host: String, pub port: u16, pub allow_events: bool }

pub fn start(frame: &Frame) -> Option<u16>;                    // autostart in debug
pub fn start_with(frame: &Frame, cfg: InspectorConfig) -> Option<u16>;
pub fn stop();
pub fn is_running() -> bool;

// ── Livello 3 — primitive native (chiamate DAL Livello 1, non dai widget) ─
// dispatch condiviso — src/platform/inspector.rs
pub fn dump_tree(root: NativeHandle) -> ComponentNode;
pub fn window_info(root: NativeHandle) -> WindowInfo;
pub fn find_nodes(root: NativeHandle, sel: &Selector) -> Vec<NodeRef>;        // derivato da dump_tree
pub fn node_screenshot(root: NativeHandle, sel: &Selector) -> Vec<u8>;

// implementazioni per-OS — src/platform/inspector_macos.rs / inspector_windows.rs
//                                 / inspector_gtk.rs (Linux)
//   dump_tree / window_info / node_screenshot  (+ i reader di §6.4)
```

**Vincolo di compilazione da far rispettare**: in `src/debug/**` non devono
comparire `use objc2…`, `use windows_sys…` né `use gtk4…`. È la traduzione in
codice della regola 4 di §2.4, e va verificata in review (o con un semplice
`grep` in CI).

---

## 15. Piano a fasi (checklist)

> Ordine obbligatorio: **prima il Livello 2/3 (API di `ru_wx`), poi il Livello 1
> (inspector)**. Se manca il getter o la primitiva, l'inspector non può
> funzionare: si implementa a monte.

**Fase A — API di introspezione di `ru_wx` (Livelli 2 e 3)**

*A1 — primitive backend*
- [x] macOS: `mac_window_frame_rect` (posizione; la dimensione esisteva già), `mac_view_frame_rect`
- [x] macOS: `mac_frame_get_title` — già `pub` e ri-esportata, nessuna promozione necessaria
- [x] Windows: `window_rect`, `client_size` in `win32.rs` (unificano le copie in `winui3/*`)
- [x] Windows: `read_window_text` — già `pub` e ri-esportata
- [x] Dispatcher cross-platform `platform::widget_screen_rect`
- [x] XAML: accesso all'element tree `reactor` (`Element` / `PropValue` / `find_prop`) — registro `thread_local` in `winui3_xaml/element_tree.rs` (popolato dal render loop, accessor `element_tree(frame_id)`)

*A2 — getter/setter widget*
- [x] `Frame::size() / position() / client_size() / title()`
- [x] `Widget::bounds_screen()` (metodo di default, nessun widget concreto da toccare)
- [x] `TreeCtrl::get_item_text()` — presente su entrambi gli OS (Windows `TVM_GETITEMW`, macOS `tree_get_item_text`)
- [x] Getter di introspezione presenti in stile wx: `Button::get_label`, `TextCtrl::get_value`, `Grid::row_count()/col_count()/selected_rows()` (nomi allineati a wxWidgets, non si aggiungono alias)
- [x] Verifica: test unitari sui getter dei widget (senza inspector) — 4 test
      `#[cfg(all(test, target_os = "windows"))]` su `Frame` reale non mostrato:
      `frame_getters_read_native_window_state` (frame.rs),
      `button_label_roundtrip_reads_native_text` (button.rs),
      `text_ctrl_value_roundtrip_reads_native_text` (text_ctrl.rs),
      `grid_count_and_selection_getters_read_native_state` (grid.rs);
      `cargo test` → **EXIT=0**, 441 lib + 54 doctest (log `a2_full_test.log`).
      Durante il test su `Grid` è emerso e corretto un bug preesistente:
      `select_display_row` passava un intero come `lParam` di
      `LVM_SETITEMSTATE`, che invece dereferenzia un `LVITEMW*` → access
      violation (0xc0000005); ora usa due `LVITEMW` zeroed (pattern già
      corretto di `list_ctrl.rs`)

**Fase B — Inspector (Livello 1)**

*B0 — fondamenta*
- [x] `platform/inspector.rs`: tipi `ComponentNode`, `Bounds`, `ColorInfo`, `InjectedEvent`
- [x] `platform/inspector_macos.rs` + accessor in `appkit.rs`
- [x] `platform/inspector_windows.rs` (+ variante XAML/reactor)

*B1 — stato (macOS)*
- [x] `dump_tree` macOS: walk `NSView`, tipo, testo, bounds schermo, visible/enabled
- [x] `debug/inspector.rs`: server HTTP + `/health` + `/tree`
- [x] autostart in `App::run` (debug) + verifica con `curl /tree`
- [x] Ruolo `check`/`radio` dichiarato dal widget (Livello 2): elimina la divergenza macOS↔Windows (cfr. §17.1)

*B2 — stato (Windows)*
- [x] `dump_tree` Windows Win32: `EnumChildWindows`, classe, testo, rect, stato
- [x] `dump_tree` Windows XAML: walk dell'element tree reactor (`winui3_xaml/element_tree::dump_tree`, dispatch in `inspector_windows::dump_tree`)
- [x] XAML end-to-end: `start_xaml(frame_id)` + wake-up via `DispatcherQueue` WinUI, `/tree` verificato via HTTP su `wx_step2_button` — esiti in §18
- [x] `cargo check --target x86_64-pc-windows-gnu` (cross-compile) del codice inspector
- [x] Verifica runtime su macchina Windows reale (porta 7373, form + tabella) — esiti in §18

*B3 — contenuti complessi*
- [x] Tabelle (macOS `NSTableView`, Windows `LVM_*`)
- [x] Liste/combo/checkbox (items, selezione)
- [x] Colori (testo/sfondo) su entrambi
- [x] Scroll position

*B4 — azione*
- [x] `POST /event` + `inject_event` macOS (`performClick:`/`setStringValue:`/…) e Windows (`SendMessageW`)
- [x] Azioni espresse come chiamate ai setter dei widget (§2.5/A3)
- [x] `find_element` (`GET /find`)

*B5 — visione*
- [x] `screenshot` macOS (`NSBitmapImageRep`: `bitmapImageRepForCachingDisplayInRect` + `cacheDisplayInRect:toBitmapImageRep:`) e Windows (`PrintWindow`+`image`)
- [x] `GET /screenshot?id=` (ritaglio su nodo; selettore vuoto = finestra intera)

*B6 — MCP*
- [x] `bin/ru_wx_mcp.rs`: `initialize`, `tools/list`, `tools/call` (JSON-RPC 2.0 su stdio, una riga per messaggio)
- [x] Tool: `get_ui_tree`, `find_element`, `get_window_info`, `interact_with_element`, `take_screenshot`, `get_visual_snapshot`
- [x] Config di esempio per Cursor / Claude Desktop (documentata nell'header del binario)

*B7 — demo e verifica*
- [x] Demo stabile `examples/inspector_demo.rs` (form: label + text/password + checkbox + choice; tabella `ListCtrl` report) come banco di prova
- [x] Verifica end-to-end (prompt → tool → verifica logica senza screenshot): comandi in §16.3
- [x] Controllo: `grep -r "objc2\|windows_sys" src/debug/` → solo commenti, **zero import**

---

## 16. Come testare sulle demo (workflow previsto)

Le demo in `examples/` saranno il banco di prova. Flusso:

1. La demo costruisce la UI con `ru_wx` e chiama `App::run(frame)`.
2. In debug l'inspector parte da solo e stampa:
   ```
   [ru_wx inspector] in ascolto su http://127.0.0.1:7373
   ```
3. Verifica manuale/script:
   ```bash
   curl -s localhost:7373/tree | jq '.. | objects | select(.role=="button")'
   ```
4. Verifica guidata dall'IA (MCP): prompt tipo
   *"Compila il form di login e verifica che il pannello utente appaia;
   controlla poi che il padding del pannello sia corretto"* →
   l'IA chiama `get_ui_tree`, `interact_with_element`, `get_ui_tree`,
   e solo alla fine `get_visual_snapshot`.

Per una demo generica riusabile si può aggiungere un esempio
`examples/inspector_demo.rs` che mostra un form + una tabella e serve da
"terreno di prova" stabile.

### 16.1 La demo stabile (`examples/inspector_demo.rs`)

Realizzata. Contiene:
- **form**: label, campo testo e password, `CheckBox`, `Choice`, pulsante *Login*
  che riscrive una label con i valori **letti dai widget** (stato → azione → stato);
- **tabella**: `ListCtrl` report con colonne *Name / Role / Status* e un pulsante
  *Add row* (per verificare la lettura dello stato dopo un'azione).

```bash
cargo run --example inspector_demo
```

### 16.2 Vincoli operativi
- La porta di default è `7373`: **una sola demo alla volta**. Se è già occupata,
  il secondo processo parte ma l'inspector non si avvia
  (`bind ... fallito: Address already in use`). Chiudere l'istanza precedente.
- Il selettore è il *handle* del nodo (es. `65545`): va riletto da `/tree` o
  `/find` dopo ogni azione, perché può cambiare tra esecuzioni.

### 16.3 Verifica end-to-end (riproducibile)

Stato (senza screenshot) — l'IA legge l'albero e verifica l'effetto logico:

```bash
curl -s localhost:7373/health
curl -s localhost:7373/window
curl -s 'localhost:7373/find?role=table'      # 1 nodo RuWxListTableView / GtkTreeView (Linux, ruolo da `declared`)
curl -s 'localhost:7373/find?role=choice'     # 1 nodo (NSPopUpButton)
curl -s 'localhost:7373/find?role=input'      # campi di testo
```

Azione → nuovo stato:

```bash
# click sul pulsante Login (id letto da /find?role=button)
curl -s -X POST localhost:7373/event -d '{"action":"click","id":"65545"}'
curl -s 'localhost:7373/find?text=Signed%20in'   # la label rispecchia i valori letti

# seleziona la riga 2 della tabella
curl -s -X POST localhost:7373/event -d '{"action":"select","id":"<table_id>","key":"2"}'
# → /tree mostra selected_rows:[2]
```

Visione (on-demand):

```bash
curl -s localhost:7373/screenshot -o full.png
curl -s 'localhost:7373/screenshot?id=<table_id>' -o table.png
```

Bridge MCP:

```bash
printf '%s\n' \
 '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}' \
 '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_window_info","arguments":{}}}' \
| ru_wx_mcp --port 7373
```

---

## 17. Rischi e limiti noti

| Rischio | Mitigazione |
|---|---|
| Il run loop macOS è bloccato fuori dal polling (30 ms) → latenza percepita | accettabile per debug; si può abbassare `RAW_POLL_INTERVAL` o postare un evento dummy per svegliare il loop |
| Righe di tabella fuori viewport non leggibili come view (macOS) | leggere dal `dataSource` di `ru_wx` invece che dalle view |
| Colori di controlli custom-drawn non affidabili | esporre `null` e documentarlo |
| `SendInput` muove il cursore reale | preferire `SendMessageW` diretto al controllo |
| Permessi Accessibility su macOS | sono già richiesti per i test; documentare in fase demo |
| Codice Windows non compilabile su macchina Apple | cross-check con `cargo check --target x86_64-pc-windows-gnu` (target già installato) |

### 17.1 Limiti dichiarati (cosa NON è coperto)

- **Nessuna analisi pixel / OCR**: la semantica arriva dall'albero JSON, non
  dall'immagine. Lo screenshot serve solo a valutazioni estetiche.
- **Menu aperti e popup transitori**: sono finestre separate, enumerabili solo
  parzialmente.
- **Righe di tabella fuori viewport su macOS** (possibile fallback leggendo dal
  `dataSource` registrato da `ru_wx`).
- **Colori di controlli custom-drawn / tematizzati**: possono risultare `null`.
- **Tooltip e stati hover**: non esposti dal sistema in modo affidabile.
- **Widget non-`NSView`** (es. contenuti disegnati in un unico canvas): invisibili
  all'albero, perché non esistono come view distinte.
- **Getter non ancora esposti da `ru_wx`**: un campo può risultare `null` **non
  perché l'OS non lo sappia**, ma perché il metodo pubblico che lo legge non
  esiste ancora (es. `TreeCtrl` non espone `get_item_text`; la posizione della
  finestra su macOS non era esposta). Non è un limite del sistema operativo: è
  debito interno di `ru_wx`, e si sanisce con la Fase A (§2.5, §15).
  Regola: il campo va emesso come `null` con `source: "missing"`, **mai**
  stimato o dedotto in modo silenzioso.
- **CheckBox / RadioButton su macOS** — *risolto*. AppKit moderno (macOS 26,
  pulsanti "SwiftUI-backed") **non espone** `NSButton.buttonType` (né sul `cell`)
  e `accessibilityRole` risponde `AXUnknown`, quindi il ruolo non è deducibile
  dalla classe nativa. Il ruolo ora è **dichiarato dal widget `ru_wx`**
  (Livello 2): `create_checkbox` / `create_radio_button` registrano la view in
  `DECLARED_ROLES` con `"check"` / `"radio"`, e `role_for_class` consulta quel
  registro **prima** dell'euristica sulla classe. Così macOS emette gli stessi
  ruoli di Windows (dagli stili `BS_*`), con `state.checked` invariato.

---

## 18. Stato attuale

- Documento di progetto: **questo file**.
- Ordine di lavoro confermato: **Fase A** (getter/setter + primitive backend di
  `ru_wx`) → **Fase B** (inspector). L'inspector è *consumatore* dell'API di
  introspezione, non un canale parallelo verso l'OS.

### Fase A — implementata (codice scritto)

| Livello | File | Contenuto |
|---|---|---|
| Backend macOS | `src/platform/appkit.rs` | `mac_window_frame_rect`, `mac_view_frame_rect`, helper `ns_rect_to_top_left`, `primary_screen_height_points` |
| Backend Windows | `src/platform/win32.rs` | `window_rect`, `client_size` |
| Backend Linux | `src/platform/gtk.rs` | posizionamento/visibilità/abilitazione widget GTK4 (`set_view_frame`, `set_view_visible`, `set_view_enabled`) + `widget_screen_rect` (origine schermo via `XGetGeometry` su connessione X11 dedicata + `compute_bounds` relativi alla finestra) |
| Dispatcher | `src/platform/mod.rs` | `widget_screen_rect(handle) -> Rect` |
| Widget | `src/window/frame.rs` | `Frame::size() / position() / client_size() / title()` |
| Widget | `src/core/widget.rs` | `Widget::bounds_screen()` (metodo di **default**) |

- Compilazione macOS (`cargo check --lib`): **ok**, nessun errore.
- Compilazione Windows (`cargo check --lib --target x86_64-pc-windows-gnu`):
  **ok**, nessun errore.
- Compilazione Linux (`cargo check --lib`, toolchain rustup + sysroot locale
  dei `.deb` GTK4): **ok**, nessun errore.
- Ancora da fare in Fase A: `TreeCtrl::get_item_text`, uniformazione
  `Button::text()` / `TextCtrl::value()` / getter di `Grid`, accesso all'element
  tree `reactor` (XAML), test unitari dei getter. ~~Ramo Linux di
  `widget_screen_rect`~~ → **fatto** (dispatcher + `gtk::widget_screen_rect`,
  minitest `mt_bounds_screen` PASS).

### Fase B — implementata (B0–B7)

| Livello | File | Contenuto |
|---|---|---|
| 3 — schema | `src/platform/inspector.rs` | tipi (`ComponentNode`, `Bounds`, `ColorInfo`, `Selector`, `InjectedEvent`), `dump_tree`/`window_info`/`find_nodes`/`inject_event`/`node_screenshot` |
| 3 — macOS | `src/platform/inspector_macos.rs` | walk `NSView`, reader per-widget, azioni (`performClick:`, `setStringValue:`, `makeFirstResponder:`, `select…`), screenshot (`NSBitmapImageRep`) |
| 3 — Windows | `src/platform/inspector_windows.rs` | walk `GetWindow`, letture `LVM_*`/`LB_*`/`CB_*`, azioni (`BM_CLICK`, `SetWindowTextW`, …), screenshot (`PrintWindow` + `image`; su XAML fallback sull'HWND top-level, §10.1) |
| 3 — Linux | `src/platform/inspector_gtk.rs` | walk GTK4 (`first_child`/`next_sibling`), reader per-widget (`role_of`/`text_of`/`state_of`/`extra_of`), azioni (`emit_clicked`, `grab_focus`, `set_text`, `select`, `close`), screenshot X11 (`XGetImage` + `image`) — §19 |
| 3 — accessor | `src/platform/appkit.rs` | `inspector_content_view/window`, `inspector_handle_for_view`, `inspector_view_screen_rect`, `inspector_class_name`, … |
| 1 — server | `src/debug/inspector.rs` | HTTP su `127.0.0.1` (`/health`, `/window`, `/tree`, `/find`, `/screenshot`, `POST /event`), coda job + marshaling via raw message |
| 1 — MCP | `src/bin/ru_wx_mcp.rs` | bridge stdio ↔ HTTP (JSON-RPC 2.0), 6 tool (`get_ui_tree`, `find_element`, `get_window_info`, `interact_with_element`, `take_screenshot`, `get_visual_snapshot`) |
| demo | `examples/inspector_demo.rs` | form (label, text, password, checkbox, choice) + tabella `ListCtrl`; banco di prova e2e |
| wiring | `src/lib.rs`, `src/platform/mod.rs`, `src/core/app.rs` | modulo `debug`, dispatch per-OS, autostart in `App::run` |

- Compilazione macOS (`cargo check --lib` + `cargo build --example inspector_demo`): **ok**, nessun errore.
- Verifica end-to-end su macOS (demo `inspector_demo`, porta 7373): **ok** —
  `/health`, `/window`, `/tree` (testo, bounds, colori, `extra.table`),
  `/find` per ruolo (`table`, `choice`, `input`, `label`, `button`),
  `POST /event` click → label aggiornata con i valori reali, `select` riga →
  `selected_rows:[2]`, `/screenshot` (full e su nodo), bridge MCP
  (`find_element`, `take_screenshot` base64).
- Bug corretti emersi dalla demo stabile:
  1. `read_extra` classificava le view interne di AppKit col nome del contenitore
     (es. `_NSScrollViewContentBackgroundView` ⊃ `"NSScrollView"`) → `msg_send!` su
     metodo inesistente e **panic**. Ora il test usa `respondsToSelector:`.
  2. `index_set_to_vec` non riconosceva `NSNotFound` (che è `NSIntegerMax`, non
     `usize::MAX`): una tabella **senza selezione** produceva
     `selected_rows:[9223372036854775807]` invece di `[]`.
  3. La tabella di `ru_wx` ha classe `RuWxListTableView`: il ruolo `table` ora la
     riconosce (prima cadeva in `view`).
- Azioni senza permessi di Accessibilità: su macOS `inject_event` usa i metodi dei
  controlli (`performClick:` ecc.), **non** `CGEvent`; su Windows `SendMessageW`,
  **non** `SendInput`.
- Vincolo rispettato: in `src/debug/**` nessun `use objc2…` / `use windows_sys…`
  (solo menzioni nei commenti).
- **Ruolo `check`/`radio` dichiarato dal widget (Livello 2)**: `appkit.rs` tiene
  un registro `DECLARED_ROLES` (view → ruolo) popolato da `create_checkbox`
  (`"check"`) e `create_radio_button` (`"radio"`); `role_for_class` lo consulta
  prima dell'euristica sulla classe. Verificato e2e su macOS (demo
  `inspector_demo`): `/find?role=check` → `Remember me`, `/find?role=button` →
  `Login` + `Add row` (la checkbox non è più inclusa), `state.checked` invariato.
  La divergenza macOS↔Windows sui checkbox/radio è così eliminata.
- Cross-check Windows (`cargo check --lib --target x86_64-pc-windows-gnu` del
  codice screenshot + azioni): **ok**. Bug corretti in `inspector_windows.rs`:
  1. `GetFocus`/`SetFocus`/`IsWindowEnabled` importati da `WindowsAndMessaging`
     ma in `windows-sys 0.59` stanno in `Win32::UI::Input::KeyboardAndMouse`.
  2. `SendMessageW` nel ramo `select`: `index` era `isize` (da `parse::<isize>()`)
     mentre `wparam` è `usize` → cast `index as usize`.
- **Verifica runtime su Windows: SUPERATA** (2026-09-28, toolchain nativa
  `stable-x86_64-pc-windows-gnu`, demo `examples/inspector_demo`, porta 7373).
  Esiti: `[ru_wx inspector] in ascolto su http://127.0.0.1:7373` da stdout;
  `/health` → `{ok:true, platform:"windows", version:"0.7.1", allow_events:true}`;
  `/window` → titolo, rect, `dpi:96`, `scale:1.0`, schermo virtuale
  (`screen_w/screen_h`); `/tree` con ruoli normalizzati
  (`label/button/input/check/choice/table/view`); `/find` per ruolo
  (`button: Login+Add row`, `check: Remember me`, `table`); click `Login` →
  label aggiornata (`Signed in as alice [Admin] — password 0 chars, remember=true`);
  `select` su `Choice` → `selected:[2]`; `select` su tabella → `selected_rows:[2]`;
  `/screenshot` full + nodo (PNG validi); bridge MCP (`get_window_info` +
  `tools/list` con i 6 tool).
- Fix di completezza Windows in `inspector_windows.rs`, emersi dalla verifica:
  1. `read_listview` emette ora `selected_rows` (`LVM_GETITEMSTATE`), a parità con
     macOS (`read_table`).
  2. `inject_event`/`select` supporta `SysListView32` (`LVM_SETITEMSTATE`): il test
     §16.3 (`select` riga → `selected_rows:[2]`) passa anche su Windows.
  3. `window_info` usa `GetDpiForWindow` (DPI per-finestra) e lo spazio virtuale
     unificato (`SM_CXVIRTUALSCREEN`/`SM_CYVIRTUALSCREEN`), come da §10.2/§12.
- Nota ambiente: il toolchain GNU richiede `dlltool.exe` (MinGW); su questa macchina
  è in `C:\msys64\mingw64\bin`, da aggiungere al `PATH` prima di `cargo build`
  (con `CARGO_TARGET_DIR` allineato a `%USERPROFILE%\target\ru_wx`).
- **XAML/reactor (A1 + B2) implementati** (2026-09-28). Il backend
  `winui3_xaml` non compilava più: `winui3_xaml/app.rs::run` restituiva
  `windows_reactor::Error`, ma `ru_wx` usa `windows-core` da crates.io mentre il
  reactor (path in `../windows-rs-study`) usa la copia in-tree → due crate
  `windows-result` distinte e tipi `Error` non unificabili (E0308). Risolto
  convertendo l'errore al confine via `HRESULT`
  (`windows_core::Error::from_hresult(..)`): la firma pubblica resta
  `Result<(), windows_core::Error>`.
  Nuovo modulo `winui3_xaml/element_tree.rs`:
  1. **A1** — registro `thread_local` dell'`Element` tree per `frame_id`,
     popolato dal render loop (`register`/`unregister`) ed esposto con
     `element_tree(frame_id)` / `is_registered(frame_id)`.
  2. **B2** — `dump_tree(frame_id)` cammina l'albero con le API reactor
     (`Element::kind_name` / `as_widget` / `Widget::children` / `bindings` /
     `find_prop`) e produce `ComponentNode` (testo da `Text`/`Content`/`Header`/…,
     stato da `IsChecked`/`SelectedIndex`/`Value`, figli da
     `Children::{PositionalSingle,Keyed,Tabs,PivotItems}`).
  Dispatch: `inspector_windows::dump_tree` prova prima il registro XAML
  (`root as u64` = `frame_id`) e ricade sul percorso Win32.
- Verifica: `cargo check --lib --features winui3_xaml`, i 3 example XAML
  (`winui3_xaml_basic`, `wx_step1_window`, `wx_step7_listctrl`) e il build di
  default: **ok**.
- **Esecuzione XAML resa possibile** (2026-09-28). Il blocco era doppio:
  1. `bootstrap.rs` pinnava il canale WinAppSDK **1.7** (non installato sulla
     macchina: presenti `Microsoft.WindowsAppRuntime.1.8` e `.2`/2.5.1.0), mentre
     i binding del reactor sono compilati per il canale **2.0**
     (`WINDOWSAPPSDK_RELEASE_MAJORMINOR = 0x00020000`, min **2.0.1.0**). Ora
     `bootstrap.rs` targetta **2.0**, allineato al reactor a monte
     (`windows-rs-study/.../reactor/src/bootstrap.rs`); è soddisfatto da
     qualunque runtime 2.x installato.
  2. Crash `0xC0000005` (access violation) **dentro**
     `microsoft.windowsappruntime.bootstrap.dll` all'avvio: la costante del
     version tag era `&[u16] = &[]`, il cui `.as_ptr()` è un puntatore dangling
     non nullo e **non NUL-terminato**, mentre la DLL legge una `PCWSTR`. Fix:
     `&[0]` (equivalente a `w!("")` del reactor).
  Verifica: `wx_step1_window.exe` parte e mostra la finestra
  (`MainWindowTitle: "Step 1 — Finestra base"`, `Responding: True`).
- **Wiring inspector su frame XAML + verifica end-to-end di A1/B2**
  (2026-09-28). Aggiunti:
  1. `debug/inspector.rs::start_xaml(frame_id)`: avvia il server con radice =
     `frame_id` e installa in `static WAKE` un dispatch sul `DispatcherQueue`
     WinUI (`windows_reactor::WinUIDispatcher::for_current_thread()`). Refactor
     comune in `start_common(root, cfg, wake)`; `start_with` (Win32) installa il
     wake-up via `post_raw_message_to_hwnd`. `dispatch()` ora usa `wake_ui()`.
  2. `winui3_xaml/app.rs`: nel render loop (thread UI), dopo
     `element_tree::register`, avvia l'inspector una sola volta
     (`Once`) con `inspector::start_xaml(frame_id)` (gated da
     `debug_assertions` / feature `inspector`, come `core/app.rs`).
  Esito e2e (example `wx_step2_button`, porta 7373):
  - stdout: `[ru_wx inspector] in ascolto su http://127.0.0.1:7373`;
  - `/health` → `{ok:true, platform:"windows", version:"0.7.1", allow_events:true}`;
  - `/tree` → radice `StackPanel` (`id:"xaml:1"`, `role:"window"`) con 5 figli
    camminati dall'element tree: `TextBlock` "Count: 0" (`role:"label"`) e 4
    `Button` ("Increment"/"Decrement"/"Reset" `enabled:true`, "Disabled"
    `enabled:false`). I testi provengono dai binding reactor (`Prop::Text` /
    `Prop::Content`), lo stato da `find_prop`.
  Il wake-up non usa alcun HWND: il thread server accoda sul `DispatcherQueue` e
  `drain_jobs()` gira sul thread UI, dove vivono le `Element` e il registro
  `thread_local` (§10.1).

- **Ottimizzazione grafica degli example XAML guidata da misure + screenshot**
  (2026-09-28). Dal confronto tra le immagini catturate (HWND top-level via
  `PrintWindow`/`PW_RENDERFULLCONTENT`, flag 2) e i dati di posizione
  (`516×339` per finestre `500×300`, `516×439` per `500×400`) è emerso che il
  contenuto era a filo del bordo, senza spaziatura tra i widget e con
  allineamento/larghezze automatiche. Aggiunte al backend XAML le primitive di
  layout mancanti e applicate nei tre example `wx_step2_*`:
  1. `window.rs` — `XamlFrameBuilder::with_padding(f64)` (padding uniforme
     sull'intero contenuto) e `with_spacing(f64)` (spaziatura verticale tra i
     widget di primo livello); propagati a `FrameDesc.padding`/`.spacing`.
  2. `element_bridge.rs` — campi `FrameDesc.padding`/`.spacing`,
     `WidgetDesc::Button.width`, `WidgetDesc::ComboBox.width` (0.0 = auto).
  3. `app.rs` — `build_element_tree(..., spacing, padding)` avvolge i figli in
     un `vstack` con `spacing` e `modifiers.padding` (`Thickness`) solo quando
     necessari (a 0 il comportamento resta invariato); i mapping `Button` e
     `ComboBox` applicano `modifiers.width` quando `width > 0.0`.
  4. `controls/button.rs`, `controls/combo_box.rs` — `set_width(f64)`.
  Esito (screenshot `_cap2_wx_step2_*.png`): bordo/padding visibile, widget
  distanziati, bottoni a larghezza fissa 180 px e ComboBox 260 px, titoli di
  gruppo a 16 pt bold. Nessuna modifica all'inspector.
- **(nota operativa)** La cattura con `PrintWindow` va eseguita dopo che la
  finestra ha completato il primo paint: con attese brevi (≤4 s) il PNG può
  risultare bianco anche se l'app è partita correttamente.
- **Screenshot XAML via `/screenshot`** (2026-09-28). Rimosso il workaround
  esterno: `inspector_windows::node_screenshot` ora riconosce il backend XAML
  (frame registrato in `element_tree` oppure `sel.id` con prefisso `xaml:`) e
  ricade sulla finestra top-level WinUI3 del processo
  (`find_own_top_level_window`: `EnumWindows` + `GetWindowThreadProcessId`,
  preferendo la prima finestra visibile con titolo). La cattura è stata
  estratta in `screenshot_hwnd` (condivisa col percorso HWND) e il fallback è
  `cfg`-gated su `winui3_xaml` per non introdurre warning nel build di default.
  Esito e2e (`wx_step2_checks`, porta 7373): `GET /screenshot` → `200`,
  `Content-Type: image/png`, PNG 25422 byte con l'intera UI XAML;
  `GET /screenshot?id=xaml:1` → stesso PNG (ritaglio per-nodo non disponibile).
  `cargo check` pulito con e senza la feature (restano solo i 4 warning
  preesistenti in `chrome/ribbon_bar.rs`).

- **Campagna di ottimizzazione guidata da screenshot — tutti gli example
  wx_step1…wx_step8** (2026-09-28). Per ogni esempio: build → avvio → click
  sulla **titlebar** (attiva la finestra senza alterare il contenuto) →
  `GET /screenshot` → analisi dell'immagine → fix del componente (quando
  l'incoerenza era nel backend) → ricattura di verifica.
  - **Workaround primo render**: le finestre XAML mostrano solo titlebar +
    corpo bianco finché non ricevono un evento di interazione; `SetForegroundWindow`
    + click simulato sulla titlebar (`SetCursorPos(l+120, t+12)`) sblocca il
    paint. `GET /window` su XAML torna ancora `bounds w=0/h=0` (nota §18
    "Prossimo passo") e `GetWindowRect` può tornare zeri: usare coordinate
    fisse (le finestre sono centrate).
  - **Fix componente ListCtrl** (`app.rs::widget_desc_to_element`): le righe
    erano `hstack` di `text_block` → colonne attaccate (`"IDNomePrezzoStock"`).
    Riscritto con una vera `grid` (colonne `GridLength::STAR`, righe `Auto`,
    `column_spacing(16)`/`row_spacing(4)`, cella con
    `Element::grid_row/grid_column` — **inherent su `Element`**, applicare
    `.grid_row` sul `TextBlock` richiede il trait `ElementExt` e crea
    ambiguità di inferenza con `.into()`: convertire prima in `let cell: Element`).
    Verifica: `_s7e_wx_step7_listctrl.png` (4 colonne allineate).
  - **Fix componente Grid** (`app.rs::build_grid_element`): stessa incoerenza
    (`"IDNome ↑CategoriaPrezzoStockRating"`) più colori calcolati e mai
    applicati (`_bg_color` scartato). Riscritto con `grid` + ogni cella in un
    `border` con `padding(6)`; header grigio `(240,240,245)` bold con indicatore
    di sort, riga selezionata con sfondo accent `(79,70,229)` + testo bianco,
    righe alternate con `even_color`/`odd_color`. Verifica:
    `_s8b_wx_step8_grid.png`.
  - **Fix componente Image** (`element_bridge.rs` + `controls/image.rs` +
    `app.rs`): `WidgetDesc::Image` ha ora `width: Option<f64>`/`height` e
    `XamlImage::set_size(w, h)`; il mapping li applica a `modifiers`. Senza
    size esplicita un'`Image` con `Stretch::Uniform` in uno `StackPanel`
    desidera tutta la larghezza e schiaccia i widget successivi fuori finestra
    (esempio `wx_step6_icon` con `set_size(96,96)` → `_s6j`).
  - **URI immagini**: i path Windows (`C:\...`) non sono URI WinRT validi →
    normalizzazione a `file:///C:/...` nel mapping; **SVG non supportato**
    (`SvgImageSource` inesistente, `BitmapImage` decodifica solo WIC) → gli
    esempi usano PNG.
  - **Dedup radice+container** in `build_element_tree` (evita il doppio
    padding del contenitore radice), **SplitView** con `DisplayMode` corretto,
    **TabView** con `closable`, ColorPicker fix — verificati fix13/fix14/fix15.
  - **Esempi step7/step8**: aggiunti `.with_padding(16.0).with_spacing(12.0)`
    a `wx_step7_listbox`/`_listctrl`/`_tree` e `wx_step8_grid`.
  - **ProgressBar indeterminata**: indagata con 6 frame consecutivi — **non è
    un bug**, l'animazione gira correttamente (todo chiuso).
  Esiti screenshot di riferimento (in `C:\Users\marco\target\ru_wx\debug\examples\`):
  `_s7c` (listbox), `_s7e` (listctrl), `_s7f` (tree), `_s8b` (grid), `_s6j` (icon).

- **Nuovi widget XAML + ruoli Win32** (allineamento AppKit ↔ Win32 ↔ XAML):
  - `XamlCheckListBox` (`controls/check_list_box.rs`, `WidgetDesc::CheckListBox`)
    — omologo di `create_check_list_box`: vstack di `CheckBox` con lo stato dal
    descrittore (`append`/`set_checked`/`is_checked`/`clear`).
  - `XamlHyperlink` (`controls/hyperlink.rs`, `WidgetDesc::Hyperlink`) —
    omologo di `create_hyperlink`: `HyperlinkButton` con `NavigateUri`
    (`new`/`update`).
  - `role_for_class` Win32: `SysDateTimePick32` → `picker`
    (**nota**: la classe registrata è `SysDateTimePick32`, *non*
    `msctls_datetimepick32`) e `Static` + `SS_ICON`/`SS_BITMAP`/`SS_ENHMETAFILE`
    → `image` (prima tutti i `Static` cadevano in `label`, come `NSImageView`
    su AppKit già faceva).
  - Verifiche: screenshot `_s9a_wx_step9_checklist.png` (checkbox con stati
    1/3 spuntati + 2 hyperlink blu) e `/find` su `mt_pickers` →
    `role=image` 1 nodo (`Static` swatch 64×28), `role=picker` 2 nodi
    (`SysDateTimePick32`), le altre `Static` restano `label`.
  - Esempio registrato: `wx_step9_checklist` (`[[example]]` in Cargo.toml).

- **Fase F1 — TimePickerCtrl + StdDialogButtonSizer** (esiti completi in
  `PIANO_PORTABILITA_COMPONENTI.md` §7):
  - `/find?role=picker` su `mt_time_sizer` → `count = 2` (`SysDateTimePick32`,
    text `14:30:00` e `09:05`); `/find?role=button` → `count = 3` in ordine
    Windows `OK → Cancel → Help` (x=483/588/693).
  - Screenshot `f1_mt_time_sizer.png` (root del progetto).
  - Bug risolto: `DTM_SETFORMAT` con stringa UTF-16 non veniva applicato
    (classe `SysDateTimePick32` **ANSI**) → fallback `SendMessageA` verificato
    col testo del controllo (`GetWindowTextW`), non col ritorno del messaggio.

- **Fase F2 — Simplebook (ruolo `tabs`)** (esiti completi in
  `PIANO_PORTABILITA_COMPONENTI.md` §7):
  - Nuovo registro **`src/platform/declared.rs`**, sempre compilato
    (`pub(crate)`): `mark(handle, role)` / `set_selection` /
    `set_selector` / `role` / `selection` / `select` su un `thread_local`
    `HashMap<NativeHandle, …>`. Serve ai container **senza classe nativa
    propria** (il guscio di `Simplebook` è un `RuWxPanel`), che
    compilano in tutte le cfg mentre `inspector_windows/macos` no: un
    solo registro, scritto dai primi e letto dai secondi, quindi niente
    `thread_local` divergenti.
  - `role_for_class` (Win32 e AppKit) consulta `declared::role` **prima**
    dell'euristica di classe (stesso ordine dell' AppKit
    `inspector_declared_role` §17.1); `read_state` emette
    `state.selection`; `inject_event` prova `declared::select` prima del
    dispatch nativo.
  - Verifiche su `mt_simplebook`: `/find?role=tabs` → `count = 1`
    (`RuWxPanel` 710×330); `/tree` → `state.selection: 0`;
    `POST /event {"action":"select","key":"1"}` → `ok:true` con
    `selection` → 1; screenshot `f2_mt_simplebook.png`; count = 1 anche
    con la build `winui3_xaml`.

- **Fase F8 — Backfill ruoli XAML (C9)** (esiti completi in
  `PIANO_PORTABILITA_COMPONENTI.md` §7):
  - `element_tree::role_for` riscritto sui ruoli normalizzati §7.1:
    `TabView|Pivot → tabs`, `ListBox → list`, `ListView|GridView → table`,
    `Slider|ProgressBar → value` (simmetria trackbar/progress nativi),
    `ScrollViewer → scroll`, `WebView2 → webview`, `Group → group`,
    fallback `view`.
  - Marcatore dichiarato XAML `wx.role:*` su `automation_id` dell'`Element`
    (letto da `declared_role()` prima dell'euristica) — analogo del
    registro `declared` §17: sizer → `wx.role:group`, `ListCtrl`/`Grid`
    tabellari → `wx.role:table`.
  - Verifiche `/find` su nodi `xaml:`: `group = 2` (`wx_step3_sizer`),
    `tabs = 1` (`wx_step3_tab`, risolta l'apertura di F2), `value = 3`
    (`wx_step2_slider`), `list = 1` (`wx_step7_listbox`),
    `scroll = 1` (`wx_step3_scroll`), `table = 1` (`wx_step7_listctrl`);
    `cargo test --lib` (feature) → 431 passed; 89 target senza errori.

- **Fase F3 — Book attivi C4 (ruoli `tabs`/`tree` sui 4 book)** (esiti
  completi in `PIANO_PORTABILITA_COMPONENTI.md` §7):
  - `bind` su `Listbook`/`Choicebook`/`Toolbook` → `declared::mark(handle,
    "tabs")`, `Treebook` → `mark(handle, "tree")`, più `set_selector`
    (indice → pagina) e `set_selection` iniziale sull'helper
    `register_strip` di `src/containers/book.rs`. `read_state.selection`
    e `inject_event: select` sono gli hook già installati in F2: nessuna
    modifica agli `inspector_*`.
  - Verifiche live (tutte `ok:true`):
    `/find?role=tabs` → `count = 1` su `mt_listbook` (`ListBox`),
    `mt_choicebook` (`ComboBox`) e `mt_toolbook` (`ToolbarWindow32`);
    `/find?role=tree` → `count = 1` su `mt_treebook`
    (`SysTreeView32`); `POST /event {"action":"select","key":"…"}` →
    `state.selection` portata a 1 / 2 / 1 / 2 rispettivamente.
  - Screenshot `f3_listbook_select.png`, `f3_choicebook_select.png`,
    `f3_treebook_select.png`, `f3_toolbook_select.png`; `cargo test
    --lib` → 433 passed (3 nuovi `containers::book::tests`); 89 target
    con `--features winui3_xaml` → 0 errori.

- **Fase F4 — RadioBox su macOS C5 (ruolo `radio` sul guscio +
  `checked_index`)** (esiti completi in
  `PIANO_PORTABILITA_COMPONENTI.md` §7):
  - `create_radio_box` AppKit in `platform/appkit_controls.rs` (f4b) e,
    in `src/controls/radio_box.rs`, registro `declared` sul guscio:
    `mark(shell, "radio")` (override di `group`/`container`),
    `set_selector` → `RadioBox::set_selection` (hook `inject_event:
    select`) e `set_selection` sincronizzato anche dai closure di
    `on_select`. `read_state` di `inspector_windows.rs` e
    `inspector_macos.rs` pubblica la selezione con la chiave
    **`checked_index`** quando `declared::role == Some("radio")`,
    `selection` per tutti gli altri nodi. XAML fuori scope (nessun
    guscio marcato).
  - Verifiche live su `mt_checkbox_radio` (tutte `ok:true`):
    `/find?role=radio` → `count = 8` (guscio `Priority` + 4 radio +
    3 `RadioButton`); `/tree` sul guscio → `state.checked_index = 1`;
    `POST /event {"action":"select","id":"<guscio>","key":"2"}` →
    `checked_index = 2`; `POST /event {"action":"click"}` su `Urgent`
    → `checked_index = 3` e callback `on_select` scattata.
  - Screenshot `f4_mt_checkbox_radio.png`, `f4_radiobox_shell.png`;
    `cargo test --lib` → 434 passed (1 nuovo
    `controls::radio_box::tests`); 89 target con
    `--features winui3_xaml` → 0 errori. Ramo mac **non** compilabile
    da Windows (criterio §8).

- **Fase F7 — TipProvider C8 (ruolo `dialog` + azione `close`)** (esiti
  completi in `PIANO_PORTABILITA_COMPONENTI.md` §7):
  - `src/adv/tip_provider.rs`: `trait TipProvider`,
    `show_tip<W: Window>(parent, title, provider)` e finestra
    `RuWxTipProviderWindowClass` marcati con
    `declared::mark(hwnd, "dialog")` → il ruolo §6 arriva dal registro
    `declared`, nessuna modifica ai reader. Export da `lib.rs` e
    `prelude.rs`; esempio `mt_tip_window` con il bottone
    "Show tips series (TipProvider)".
  - **Fix di rientranza del dispatcher** (problema emerso in live): il
    dispatch dei raw message del frame (`frame.rs`) *rimuove* il `Vec`
    degli handler dalla mappa mentre li esegue; un job che ri-entra in
    un loop modale (il `show_tip` scatena `GetMessageW`) continuava a
    pompare messaggi e i `RAW_MSG_INSPECTOR` postati durante quella
    finestra cadevano nel vuoto → `drain_jobs` non girava più e **tutte**
    le richieste successive rispondevano 504. Soluzione:
    `Frame::inspector_wake_window()` crea una *message-only window*
    dedicata (`RuWxInspectorWakeWindowClass`) il cui `wnd_proc` chiama
    `inspector::drain_jobs()` direttamente (fuori dalla mappa handler) e
    `start_with` la usa come destinazione del wake; fallback sulla
    finestra radice dove la message-only non esiste (non-Windows).
  - Verifiche live su `mt_tip_window`:
    `GET /find?role=dialog` → `count = 1`
    (`RuWxTipProviderWindowClass`, `text:"Tip series"`, `path:"0/5"`);
    `GET /health` → `ok:true` **con il tip aperto**;
    `POST /event {"action":"close","id":"<tip>"}` → `ok:true` e
    `count = 0` dopo la chiusura; status bar letta da `/tree`
    (`Tip series: at least one tip read`).
  - Limite noto: il `POST /event click` che **apre** il tip risponde 504
    perché il job è dentro il loop modale e risponde solo a tip chiuso;
    le richieste successive sono tutte sane.
  - Screenshot `f7_tip_dialog.png`, `f7_frame_con_tip.png`;
    `cargo test --lib` → **437 passed** (3 nuovi
    `adv::tip_provider::tests`); `cargo check --features winui3_xaml
    --example wx_step9_checklist` → 0 errori.

- **Backend Linux/GTK4 — verifica end-to-end su Xvfb** (2026-10-02;
  `examples/inspector_demo` in debug su `DISPLAY=:99`, porta 7373; backend
  descritto in §19):
  - stdout: `[ru_wx inspector] in ascolto su http://127.0.0.1:7373`;
    `/health` → `{"ok":true,"platform":"linux","version":"0.7.1","allow_events":true}`.
  - `/window` → `title:"ru_wx Inspector — Demo"`, `bounds 640×560` (da
    `XGetGeometry`), `client 640×560`, `dpi:96`, `scale:1.0`,
    `screen 1280×800`, `source:native`.
  - `/tree` → ruoli normalizzati (`window`, `container`, `label`, `input`,
    `check`, `choice`, `button`, `table`, `scroll`, `list`, `image`), testi
    reali (`Sign in`, `alice`, `Remember me`, `Login`), `state.checked:true`
    su `GtkCheckButton`, `state.selected:0` su `GtkDropDown`; i widget
    interni non registrati hanno id sintetico `gtk:0x…`, i registrati l'handle
    ru_wx (`65537`, `65545`, …).
  - `/find` per ruolo → `button: 6`, `check: 1` (`Remember me`),
    `choice: 1` (tutti con `bounds` in coordinate schermo).
  - `POST /event {"action":"click"}` su `Login` → `ok:true` e label
    aggiornata (`Signed in as alice [Admin] — password 0 chars,
    remember=true`); `{"action":"select","key":"1"}` su `Choice` → `ok:true`.
  - `/screenshot` → `200`, `Content-Type: image/png`, 39586 byte,
    PNG 640×560 RGBA valido.
  - **Divergenze §19.6 chiuse** (2026-10-02, verifica successiva):
    `/find?role=table` → `count: 1` (nodo `65548`, `GtkTreeView`, grazie a
    `declared::mark(handle, "table")` in `create_list_ctrl`);
    `POST /event {"action":"select","key":"2"}` sulla tabella → `ok:true`
    e `/tree` → `state.selected_rows:[2]` + `extra` con
    `columns[3]`/`rows[4]`/`row_count:4`/`selected_count:1`/`header_text`/
    `truncated:false`; `key:"99"` → `400 riga 99 fuori range (0..4)`.
    `cargo test --lib` → 168 passed; screenshot finestra (640×560) e nodo
    tabella (640×184) PNG validi.

### Prossimo passo

- [x] *(opzionale)* XAML: `/window` e `POST /event` ora coperti dal registro
  XAML (§10.1, §11):
  - `element_tree::window_info` legge id/title/class dall'element tree;
    `bounds`, `client_*`, `visible`, `scale` e `dpi` della **radice** sono
    completati in `inspector_windows::window_info` dalla top-level HWND
    WinUI3 (`find_own_top_level_window`), `screen_*` da `GetSystemMetrics`
    (spazio virtuale). I rect dei **figli** restano indisponibili: reactor
    non espone geometria per-elemento senza layout pass (§6.3).
  - `element_tree::inject_event` risolve l'id `xaml:<frame>/…` nell'element
    tree e esegue `click`/`set_text`/`select` invocando gli handler `pub`
    dei widget (nessun messaggio grezzo all'OS, §11); `focus` e `close`
    restano non supportati sul registro (reactor senza API di focus;
    chiusura finestra via dispatcher UI) e le azioni per coordinate `x/y`
    restano non supportate.
  - Verifiche: `cargo check --lib` e `cargo check --lib --features
    winui3_xaml` → 0 errori (4 warning preesistenti in `ribbon_bar.rs`);
    `cargo test --lib --features winui3_xaml` → **443 passed** (nuovo
    `element_tree::tests::window_info_e_inject_event_sul_registro`);
    `cargo test` completo → **441 lib + 54 doctest** (5 ignored), EXIT=0.

---

## 19. Backend Linux/GTK4 — mappatura API (Livello 3)

Pendant Linux di §9/§10, implementato in `src/platform/inspector_gtk.rs`
durante il porting GTK4 della libreria. Per questo è numerato 19 (dopo lo
stato): le sezioni 11–18 hanno riferimenti incrociati in questo documento e
in `PIANO_PORTABILITA_COMPONENTI.md`, quindi non sono state rinumerate.

Gira sul thread UI GTK (§5) ed è l'unico punto che combina `gtk4`,
`gdk4_x11` e FFI manuale su `libX11`. Come sugli altri OS, il Livello 1 lo
raggiunge solo tramite il dispatch di `src/platform/inspector.rs`.

### 19.1 Identità dei nodi

- Gli id sono gli **handle runtime** ru_wx (`alloc_widget_handle`, registro
  `gtk::VIEWS`). La mappa inversa widget → handle si costruisce da
  `gtk::registered_views()` chiaveando sul puntatore GObject (`widget_key`).
- I widget interni non registrati usano un id sintetico `gtk:0x…` con
  `source: inferred` (analogo di `mac:0x…`), la radice è l'handle del frame.
- Budget di walk come su Windows: `MAX_NODES = 4000`, `MAX_DEPTH = 24`; al
  esaurimento il nodo riceve `source: missing`.

### 19.2 Walk + attributi

| Dato | API |
|---|---|
| radice | `gtk::frame_window_of(root)` → `gtk4::Window` |
| figli | `Widget::first_child()` / `next_sibling()` (walk ricorsivo) |
| classe | `Widget::type_().name()` (`GtkButton`, …) |
| ruolo | `declared::role(handle)` (precedenza) → mapping tipo GTK→ruolo (`role_of`, §6.4) |
| geometria | `Widget::compute_bounds(&window)` + origine finestra (`window_origin`) → coordinate schermo |
| visibilità/stato | `is_visible()`, `is_sensitive()`, `has_focus()` |
| testo | `Label::label()`, `Entry::text()`, `Button::label()`, `TextView::buffer()`… (`text_of`) |
| stato | `CheckButton/Switch::is_active()`, `SpinButton/Scale::value()`, selezione `ListBox`/`DropDown`/`Notebook`, `selected_rows` della tabella (`state_of`) |
| extra | `extra_of`: tabella `table_extra` (`columns`/`rows`/`row_count`/`selected_count`/`selected_rows`/`header_text`/`truncated` da `listctrl_column_info` + `listctrl_get_item_text`, budget 200×32), conteggi pagine/righe |
| finestra | `window.title()`, `window.type_().name()`, `scale_factor()`, `surface.width()/height()` |

### 19.3 Geometria e finestra (X11)

- XID della finestra: `window.surface()` downcast `gdk4_x11::X11Surface::xid()`
  — **solo backend X11**.
- `window_info`: bounds via `XGetGeometry` sulla XID (coordinate schermo),
  `screen_w/h` da `XDisplayWidth`/`XDisplayHeight`, `dpi = 96 × scale_factor`;
  senza server X ricade sull'area della superficie GTK (`source` invariato).
- I bounds dei nodi sono `compute_bounds` ( finestra ) + origine X11 della
  finestra: nessuna conversione di unità (§12).

### 19.4 Screenshot

```rust
// connessione X11 DEDICATA (non quella di GDK) + error handler che ignora
// gli errori di protocollo: XGetImage fuori dalla root non deve abortire
let dpy = XOpenDisplay(ptr::null());
XSetErrorHandler(Some(ignore_errors));
let img = XGetImage(dpy, drawable, x, y, w, h, AllPlanes, ZPixmap);
// maschere red/green/blue + byte_order → image::RgbaImage → PNG (crate image)
```

Finestra intera (selettore vuoto o id della radice) = geometria della XID;
nodo = `screen_bounds` del widget. Il rettangolo è arrotondato
(`floor`/`ceil`) perché X11 lavora su interi.

### 19.5 Iniezione eventi

Azioni sui widget, **nessun evento a livello X** (nessun XTest, nessun
permesso di Accessibilità):

| Azione | Implementazione |
|---|---|
| `click` | `Button::emit_clicked()`, altrimenti `Widget::activate()` |
| `focus` | `Widget::grab_focus()` |
| `set_text` | `backend::set_widget_text` (solo widget con testo) |
| `select` | registro `declared` → `ComboBoxText::set_active` / `DropDown::set_selected` / `Notebook::set_current_page` / `ListBox::select_row` / tabella `GtkTreeView` (`listctrl_clear_selection` + `listctrl_select`, range validato come `select_listview_row` su Windows) |
| `close` | `Window::close()` (radice, dialogo o top-level del widget) |

Convenzioni allineate a `inspector_windows.rs`: niente `id` → errore
"le coordinate non sono ancora supportate"; id non numerico → `id non
valido`; handle sconosciuto → `nessuna finestra con id`.

### 19.6 Vincoli noti

- In `src/debug/**` nessun `use gtk4…` (vincolo §14.1, come `objc2`/`windows_sys`).
- Solo X11: con Wayland `window_xid()` è `None` → `/window` degrada
  (bounds dall'area della superficie) e `/screenshot` risponde errore.
- **`Widget::bounds_screen()` (Livello 2)** — **risolto**: il dispatcher
  `widget_screen_rect` ha il ramo Linux → `gtk::widget_screen_rect`
  (origine schermo X11 via `XGetGeometry` + `compute_bounds`); verificato
  col minitest `mt_bounds_screen` su Xvfb (`BOUNDS_MINITEST_OK`).
- **Ruolo `table`** — **risolto**: `role_of` mappa `GtkTreeView` → `tree`
  (§19.2), ma il `ListCtrl` di `ru_wx` su GTK *è* un `GtkTreeView`; il
  registro `declared` ha la precedenza e `create_list_ctrl` chiama
  `mark(handle, "table")`, così `/find?role=table` trova le tabelle come
  su macOS/Windows (`RuWxListTableView`/`SysListView32`). Di conseguenza
  `state_of` **non** espone il `selection: 0` implicito di `mark` per il
  ruolo `table` (la selezione è `selected_rows`).
- **`select` su tabella** — **risolto**: `inject_event` gestisce
  `GtkTreeView` (valida il range → `listctrl_clear_selection` →
  `listctrl_select`), `state_of` legge `selected_rows` da
  `listctrl_get_selected_items` e `extra_of` espone la tabella completa
  (`table_extra`: `columns`/`rows`/`row_count`/`selected_count`/
  `selected_rows`/`header_text`/`truncated`, budget 200×32 come
  `read_listview` su Windows). Test §16.3 verificato su Linux.
- `state`/`extra` dei widget GTK non registrati sono limitati: senza handle
  ru_wx non c'è corrispondenza col registro `declared` né coi getter del
  Livello 2.
