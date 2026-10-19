//! Accesso all'element tree di `windows-reactor` (cfr. `PIANO_DEBUG_INSPECTOR.md`
//! §10.1, voci **A1**/**B2**).
//!
//! Il backend `winui3_xaml` non crea **HWND**: i widget vivono in un
//! *element tree* costruito da `windows-reactor` a ogni render. Questo
//! modulo è il ponte che l'inspector usa per leggerlo con le API
//! pubbliche del reactor (`Element` / `Widget` / `PropValue` / `find_prop`).
//!
//! # Thread model
//!
//! [`Element`] non è `Send`: vive sul thread UI che esegue il render loop.
//! Il registro è quindi `thread_local` e va letto **sul thread UI**, che è
//! esattamente il thread su cui l'inspector dispatcha i job (§5).

use std::cell::RefCell;
use std::collections::HashMap;

use serde_json::{json, Map, Value};
use windows_reactor::{find_prop, Callback, Children, Element, Prop, PropValue};

use crate::platform::inspector::{source, Bounds, ComponentNode, InjectedEvent, WindowInfo};

thread_local! {
    /// Element tree corrente per ciascun frame XAML, indicizzato per `frame_id`.
    static TREES: RefCell<HashMap<u64, Element>> = RefCell::new(HashMap::new());
}

/// Registra l'element tree corrente del frame. Chiamata dal render loop.
pub(crate) fn register(frame_id: u64, root: Element) {
    TREES.with(|trees| {
        trees.borrow_mut().insert(frame_id, root);
    });
}

/// Rimuove l'element tree del frame (alla chiusura della finestra).
pub(crate) fn unregister(frame_id: u64) {
    TREES.with(|trees| {
        trees.borrow_mut().remove(&frame_id);
    });
}

/// `true` se il frame ha un element tree registrato.
pub fn is_registered(frame_id: u64) -> bool {
    TREES.with(|trees| trees.borrow().contains_key(&frame_id))
}

/// Copia dell'element tree del frame, se registrato.
///
/// È l'accesso pubblico richiesto da **A1**: il chiamante può poi usare
/// `Element::as_widget()`, `Widget::children()`, `find_prop(..)` ecc.
pub fn element_tree(frame_id: u64) -> Option<Element> {
    TREES.with(|trees| trees.borrow().get(&frame_id).cloned())
}

/// `dump_tree` del backend XAML (**B2**): cammina l'element tree del frame
/// e produce un [`ComponentNode`]. `None` se il frame non è registrato.
pub fn dump_tree(frame_id: u64) -> Option<ComponentNode> {
    let root = element_tree(frame_id)?;
    let mut node = node_from_element(&root, "0");
    // La radice rappresenta la finestra.
    node.id = format!("xaml:{frame_id}");
    node.role = "window".to_string();
    Some(node)
}

// ── GET /window (cfr. §10.1) ────────────────────────────────────────

/// `window_info` del backend XAML: metadati leggibili **dall'element tree**.
///
/// L'element tree non espone rect, DPI o monitor (servirebbe un layout
/// pass, §6.3): `bounds`, `client_*`, `visible`, `scale`, `dpi` e
/// `screen_*` restano ai valori default e vengono completati dal
/// chiamante (`inspector_windows::window_info`) leggendo la finestra
/// top-level Win32 che ospita la UI XAML. `None` se il frame non è
/// registrato.
pub fn window_info(frame_id: u64) -> Option<WindowInfo> {
    let root = element_tree(frame_id)?;
    let title = root
        .as_widget()
        .and_then(|widget| read_text(&widget.bindings()))
        .unwrap_or_default();
    Some(WindowInfo {
        id: format!("xaml:{frame_id}"),
        title,
        // "Classe" XAML: nome della variante reactor della radice.
        class: root.kind_name().to_string(),
        source: source::NATIVE.to_string(),
        ..Default::default()
    })
}

// ── POST /event (cfr. §11) ─────────────────────────────────────────

/// `inject_event` del backend XAML: risolve `event.id` (`xaml:<path>`, o
/// `xaml:<frame_id>` per la radice) nell'element tree del frame ed esegue
/// l'azione invocando l'handler `pub` del widget reactor equivalente.
///
/// Come sui backend nativi (§11) l'azione è una chiamata diretta
/// all'handler del widget, non un messaggio grezzo all'OS. `focus` e
/// `close` non sono supportati: il reactor non espone API di focus e la
/// chiusura finestra passa dal dispatcher UI.
pub fn inject_event(frame_id: u64, event: &InjectedEvent) -> Result<(), String> {
    let id = event
        .id
        .as_deref()
        .ok_or_else(|| "evento senza `id`: le coordinate non sono ancora supportate".to_string())?;
    let element = resolve(frame_id, id)?;
    let kind = element.kind_name();
    match event.action.as_str() {
        "click" => click(&element, event, kind),
        "set_text" => set_text(&element, event, kind),
        "select" => select(&element, event, kind),
        "focus" => Err(
            "focus non supportato sul registro XAML: il reactor non espone API di focus"
                .to_string(),
        ),
        "close" => Err(
            "close non supportato sul registro XAML: la chiusura finestra passa dal dispatcher UI"
                .to_string(),
        ),
        other => Err(format!("azione non supportata: `{other}`")),
    }
}

/// Risolve l'id di un nodo del registro nell'element tree del frame.
///
/// La radice ha id `xaml:<frame_id>` (assegnato da [`dump_tree`]) e path
/// `0`; i discendenti hanno id `xaml:0/…` costruiti da [`walk_children`].
fn resolve(frame_id: u64, id: &str) -> Result<Element, String> {
    let path = id.strip_prefix("xaml:").unwrap_or(id);
    let root =
        element_tree(frame_id).ok_or_else(|| format!("frame XAML {frame_id} non registrato"))?;
    if path == frame_id.to_string() || path == "0" {
        return Ok(root);
    }
    let mut segments = path.split('/');
    if segments.next() != Some("0") {
        return Err(format!("id non valido: `{id}`"));
    }
    let mut current = root;
    for segment in segments {
        let index: usize = segment
            .parse()
            .map_err(|_| format!("id non valido: `{id}`"))?;
        let child = {
            let Some(widget) = current.as_widget() else {
                return Err(format!("nessun nodo con id `{id}`"));
            };
            child_at(widget.children(), index)
        }
        .ok_or_else(|| format!("nessun nodo con id `{id}`"))?;
        current = child;
    }
    Ok(current)
}

/// Figlio n-esimo di `children`, con la stessa convenzione di
/// [`walk_children`] (l'indice è il segmento finale della path).
fn child_at(children: Children<'_>, index: usize) -> Option<Element> {
    match children {
        Children::None => None,
        Children::PositionalSingle(child) => (index == 0).then(|| child.clone()),
        Children::Keyed(items) => items.get(index).cloned(),
        Children::Tabs(tabs) => tabs.get(index).map(|tab| tab.content.clone()),
        Children::PivotItems(items) => items.get(index).map(|item| item.content.clone()),
    }
}

// ── Azioni (cfr. §11) ──────────────────────────────────────────────

/// `click`: attivazione del widget (per gli widget spuntabili inverte lo
/// stato, come `BM_CLICK` sui backend nativi).
fn click(element: &Element, event: &InjectedEvent, kind: &str) -> Result<(), String> {
    match element {
        Element::Button(button) => match (&button.on_click, &button.on_item_clicked) {
            (Some(on_click), _) => {
                on_click.invoke(());
                Ok(())
            }
            (None, Some(on_item_clicked)) => {
                on_item_clicked.invoke(arg_text(event)?);
                Ok(())
            }
            _ => Err(no_handler("click", kind)),
        },
        Element::HyperlinkButton(button) => call_unit(&button.on_click, "click", kind),
        Element::RepeatButton(button) => call_unit(&button.on_click, "click", kind),
        Element::SplitButton(button) => call_unit(&button.on_click, "click", kind),
        Element::DropDownButton(button) => match (&button.on_click, &button.on_item_clicked) {
            (Some(on_click), _) => {
                on_click.invoke(());
                Ok(())
            }
            (None, Some(on_item_clicked)) => {
                on_item_clicked.invoke(arg_text(event)?);
                Ok(())
            }
            _ => Err(no_handler("click", kind)),
        },
        Element::CommandBar(bar) => call_text(&bar.on_click, arg_text(event)?, "click", kind),
        Element::MenuBar(bar) => call_text(&bar.on_item_clicked, arg_text(event)?, "click", kind),
        Element::BreadcrumbBar(bar) => {
            call_index(&bar.on_item_clicked, arg_index(event)?, "click", kind)
        }
        // Lo stato resta governato dal render dell'app: qui si invoca
        // solo l'handler con il valore invertito.
        Element::CheckBox(check) => match &check.on_checked {
            Some(on_checked) => {
                on_checked.invoke(!check.is_checked);
                Ok(())
            }
            None => Err(no_handler("click", kind)),
        },
        Element::ToggleButton(toggle) => match &toggle.on_checked {
            Some(on_checked) => {
                on_checked.invoke(!toggle.is_checked);
                Ok(())
            }
            None => Err(no_handler("click", kind)),
        },
        Element::ToggleSwitch(toggle) => match &toggle.on_toggled {
            Some(on_toggled) => {
                on_toggled.invoke(!toggle.is_on);
                Ok(())
            }
            None => Err(no_handler("click", kind)),
        },
        Element::RadioButton(radio) => call_unit(&radio.on_checked, "click", kind),
        _ => Err(no_handler("click", kind)),
    }
}

/// `set_text`: modifica del testo del widget (handler `on_text_changed`,
/// o `on_password_changed` per la `PasswordBox`).
fn set_text(element: &Element, event: &InjectedEvent, kind: &str) -> Result<(), String> {
    let text = event.text.clone().unwrap_or_default();
    match element {
        Element::TextBox(edit) => call_text(&edit.on_text_changed, text, "set_text", kind),
        Element::RichEditBox(edit) => call_text(&edit.on_text_changed, text, "set_text", kind),
        Element::AutoSuggestBox(edit) => call_text(&edit.on_text_changed, text, "set_text", kind),
        Element::NavigationView(nav) => call_text(&nav.on_text_changed, text, "set_text", kind),
        Element::PasswordBox(edit) => call_text(&edit.on_password_changed, text, "set_text", kind),
        _ => Err(no_handler("set_text", kind)),
    }
}

/// `select`: selezione (indice in `key`) o, sugli widget spuntabili,
/// toggle (booleano in `key`; `RadioButton` espone `Callback<()>`).
fn select(element: &Element, event: &InjectedEvent, kind: &str) -> Result<(), String> {
    match element {
        Element::ComboBox(choice) => call_index(
            &choice.on_selection_changed,
            arg_index(event)?,
            "select",
            kind,
        ),
        Element::ListBox(list) => call_index(
            &list.on_selection_changed,
            arg_index(event)?,
            "select",
            kind,
        ),
        Element::RadioButtons(radio) => call_index(
            &radio.on_selection_changed,
            arg_index(event)?,
            "select",
            kind,
        ),
        Element::Pivot(pivot) => call_index(
            &pivot.on_selection_changed,
            arg_index(event)?,
            "select",
            kind,
        ),
        Element::TabView(tabs) => call_index(
            &tabs.on_selection_changed,
            arg_index(event)?,
            "select",
            kind,
        ),
        Element::TemplatedList(list) => {
            if !list.has_selection_handler() {
                return Err(no_handler("select", kind));
            }
            list.invoke_selection_changed(arg_index(event)?);
            Ok(())
        }
        Element::NavigationView(nav) => {
            call_text(&nav.on_selection_changed, arg_text(event)?, "select", kind)
        }
        Element::SelectorBar(selector) => call_text(
            &selector.on_selection_changed,
            arg_text(event)?,
            "select",
            kind,
        ),
        Element::CheckBox(check) => call_bool(&check.on_checked, arg_bool(event)?, "select", kind),
        Element::ToggleButton(toggle) => {
            call_bool(&toggle.on_checked, arg_bool(event)?, "select", kind)
        }
        Element::ToggleSwitch(toggle) => {
            call_bool(&toggle.on_toggled, arg_bool(event)?, "select", kind)
        }
        Element::RadioButton(radio) => call_unit(&radio.on_checked, "select", kind),
        _ => Err(no_handler("select", kind)),
    }
}

fn no_handler(action: &str, kind: &str) -> String {
    format!("`{action}` non supportato su `{kind}`: nessun handler pubblico")
}

/// Testo dell'evento (`key`, oppure `text`): obbligatorio.
fn arg_text(event: &InjectedEvent) -> Result<String, String> {
    event
        .key
        .clone()
        .or_else(|| event.text.clone())
        .ok_or_else(|| "l'azione richiede `key` (o `text`)".to_string())
}

/// Indice numerico in `key`.
fn arg_index(event: &InjectedEvent) -> Result<i32, String> {
    event
        .key
        .as_deref()
        .and_then(|key| key.parse::<i32>().ok())
        .ok_or_else(|| "l'azione richiede `key` con un indice numerico".to_string())
}

/// Booleano in `key`.
fn arg_bool(event: &InjectedEvent) -> Result<bool, String> {
    match event.key.as_deref() {
        Some("true") | Some("1") => Ok(true),
        Some("false") | Some("0") => Ok(false),
        _ => Err("l'azione richiede `key` con un booleano (`true`/`false`)".to_string()),
    }
}

fn call_unit(cb: &Option<Callback<()>>, action: &str, kind: &str) -> Result<(), String> {
    match cb {
        Some(cb) => {
            cb.invoke(());
            Ok(())
        }
        None => Err(no_handler(action, kind)),
    }
}

fn call_text(
    cb: &Option<Callback<String>>,
    text: String,
    action: &str,
    kind: &str,
) -> Result<(), String> {
    match cb {
        Some(cb) => {
            cb.invoke(text);
            Ok(())
        }
        None => Err(no_handler(action, kind)),
    }
}

fn call_index(
    cb: &Option<Callback<i32>>,
    index: i32,
    action: &str,
    kind: &str,
) -> Result<(), String> {
    match cb {
        Some(cb) => {
            cb.invoke(index);
            Ok(())
        }
        None => Err(no_handler(action, kind)),
    }
}

fn call_bool(
    cb: &Option<Callback<bool>>,
    value: bool,
    action: &str,
    kind: &str,
) -> Result<(), String> {
    match cb {
        Some(cb) => {
            cb.invoke(value);
            Ok(())
        }
        None => Err(no_handler(action, kind)),
    }
}

// ── Walk ────────────────────────────────────────────────────────────

fn node_from_element(element: &Element, path: &str) -> ComponentNode {
    let component_type = element.kind_name().to_string();
    // Ruolo dichiarato dal costruttore (marcatore `wx.role:*` su
    // `automation_id`) in precedenza sull'euristica del kind: è l'analogo
    // XAML del registro `declared` dei backend nativi (§6, `platform::declared`).
    let role = match declared_role(element) {
        Some(role) => role.to_string(),
        None => role_for(&component_type),
    };
    let mut node = ComponentNode::new(format!("xaml:{path}"), component_type.clone(), role);
    node.source = source::NATIVE.to_string();
    // Su XAML non esiste un rect per-elemento senza layout pass: il campo
    // resta invalido (`w == h == 0`), come da §6.3.
    node.bounds = Bounds::default();

    let Some(widget) = element.as_widget() else {
        return node;
    };

    let bindings = widget.bindings();
    node.text = read_text(&bindings);
    node.enabled = match find_prop(&bindings, Prop::IsEnabled) {
        Some(PropValue::Bool(enabled)) => *enabled,
        _ => true,
    };
    node.state = read_state(&bindings);

    let mut extra = Map::new();
    if let Some(key) = widget.key() {
        extra.insert("key".to_string(), json!(key));
    }
    if !extra.is_empty() {
        node.extra = Value::Object(extra);
    }

    node.children = walk_children(widget.children(), path);
    node
}

fn walk_children(children: Children<'_>, path: &str) -> Vec<ComponentNode> {
    match children {
        Children::None => Vec::new(),
        Children::PositionalSingle(child) => {
            vec![node_from_element(child, &format!("{path}/0"))]
        }
        Children::Keyed(items) => items
            .iter()
            .enumerate()
            .map(|(i, child)| node_from_element(child, &format!("{path}/{i}")))
            .collect(),
        Children::Tabs(tabs) => tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                let mut node = node_from_element(&tab.content, &format!("{path}/{i}"));
                node.text = Some(tab.header.clone());
                node
            })
            .collect(),
        Children::PivotItems(items) => items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let mut node = node_from_element(&item.content, &format!("{path}/{i}"));
                node.text = Some(item.header.clone());
                node
            })
            .collect(),
    }
}

/// Testo dedotto dalle prop più comuni (il primo valorizzato vince).
fn read_text(bindings: &[windows_reactor::Binding]) -> Option<String> {
    for prop in [
        Prop::Text,
        Prop::Content,
        Prop::Header,
        Prop::Title,
        Prop::PlaceholderText,
    ] {
        if let Some(PropValue::Str(text)) = find_prop(bindings, prop) {
            if !text.is_empty() {
                return Some(text.clone());
            }
        }
    }
    None
}

/// Stato osservabile (`checked`, `selected_index`, `value`) come JSON.
fn read_state(bindings: &[windows_reactor::Binding]) -> Value {
    let mut state = Map::new();

    if let Some(PropValue::Bool(checked)) = find_prop(bindings, Prop::IsChecked) {
        state.insert("checked".to_string(), json!(checked));
    }
    if let Some(PropValue::I32(index)) = find_prop(bindings, Prop::SelectedIndex) {
        state.insert("selected_index".to_string(), json!(index));
    }
    if let Some(PropValue::F64(value)) = find_prop(bindings, Prop::Value) {
        state.insert("value".to_string(), json!(value));
    }

    if state.is_empty() {
        Value::Null
    } else {
        Value::Object(state)
    }
}

/// Prefisso del marcatore di ruolo dichiarato dal costruttore XAML.
///
/// Il backend XAML non ha un HWND su cui pendere il registro `declared`
/// dei backend nativi: i widget che non si riconoscono dal `kind_name`
/// (sizer → `group`, griglie tabellari → `table`) marcano
/// `automation_id = "wx.role:<ruolo>"`, qui decodificato da
/// [`declared_role`].
pub(crate) const ROLE_MARKER_PREFIX: &str = "wx.role:";

/// Ruolo dichiarato dal costruttore, se il marcatore è presente.
fn declared_role(element: &Element) -> Option<&str> {
    element
        .accessibility()?
        .automation_id
        .as_deref()?
        .strip_prefix(ROLE_MARKER_PREFIX)
}

/// Ruolo logico normalizzato a partire dal nome dell'elemento reactor.
///
/// I valori devono stare nell'elenco dei ruoli normalizzati di
/// `PIANO_DEBUG_INSPECTOR.md` §7.1 e allinearsi ai backend nativi
/// (`inspector_windows::role_for_class` / `inspector_macos::role_for_class`,
/// tabella §6 di `PIANO_PORTABILITA_COMPONENTI.md`).
fn role_for(kind: &str) -> String {
    let role = match kind {
        "TextBlock" | "RichTextBlock" => "label",
        "Button" | "HyperlinkButton" | "RepeatButton" | "DropDownButton" | "SplitButton"
        | "ToggleButton" => "button",
        "CheckBox" | "ToggleSwitch" => "check",
        "RadioButton" | "RadioButtons" => "radio",
        "TextBox" | "PasswordBox" | "RichEditBox" | "NumberBox" | "AutoSuggestBox" => "input",
        // Simmetria con i nativi: `msctls_trackbar32`/`msctls_progress32`
        // (Win32) e `NSSlider`/`NSProgressIndicator` (AppKit) → `value`
        // (§6: SpinCtrlDouble → `value`).
        "Slider" | "RatingControl" | "ProgressBar" | "ProgressRing" => "value",
        "ComboBox" | "FlipView" | "SelectorBar" => "choice",
        "ListBox" | "TemplatedList" => "list",
        "ListView" | "GridView" => "table",
        "TabView" | "Pivot" => "tabs",
        "TreeView" => "tree",
        "Group" => "group",
        "Image" | "PersonPicture" => "image",
        "ColorPicker" | "DatePicker" | "TimePicker" | "CalendarDatePicker" | "CalendarView" => {
            "picker"
        }
        "ContentDialog" | "TeachingTip" | "InfoBar" => "dialog",
        // Simmetria: Win32 `ScrollBar` e AppKit `NSScrollView` → `scroll`.
        "ScrollViewer" | "ScrollView" | "ScrollBar" => "scroll",
        "WebView2" => "webview",
        // Contenitori/layout (e tutto il resto, compresi `MenuBar` e
        // `CommandBar`, fuori dall'elenco §7.1): il ruolo normalizzato di
        // riferimento è `view`, come il default dei backend nativi.
        _ => "view",
    };
    role.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use windows_reactor::{vstack, Button, CheckBox, ComboBox, TextBox};

    use crate::platform::inspector::{find_nodes, Selector};

    /// Allineamento §6/§8: stessi ruoli dei backend nativi Win32/AppKit.
    #[test]
    fn role_for_allinea_i_ruoli_chiave() {
        assert_eq!(role_for("TabView"), "tabs");
        assert_eq!(role_for("Pivot"), "tabs");
        assert_eq!(role_for("ListBox"), "list");
        assert_eq!(role_for("Slider"), "value");
        assert_eq!(role_for("ProgressBar"), "value");
        assert_eq!(role_for("TimePicker"), "picker");
        assert_eq!(role_for("CalendarView"), "picker");
        assert_eq!(role_for("ScrollViewer"), "scroll");
        assert_eq!(role_for("ContentDialog"), "dialog");
        assert_eq!(role_for("WebView2"), "webview");
        assert_eq!(role_for("Group"), "group");
        // Fallback sempre dentro i ruoli normalizzati §7.1.
        assert_eq!(role_for("Qualsiasi"), "view");
    }

    /// `GET /window` e `POST /event` sul registro XAML (§10.1, §11):
    /// metadati letti dall'element tree, azioni eseguite come invoke
    /// degli handler `pub` dei widget (nessun messaggio grezzo all'OS).
    #[test]
    fn window_info_e_inject_event_sul_registro() {
        let clicked = Rc::new(Cell::new(false));
        let typed = Rc::new(RefCell::new(String::new()));
        let selected = Rc::new(Cell::new(-1_i32));
        let checked = Rc::new(Cell::new(None::<bool>));

        let on_click = clicked.clone();
        let on_text = typed.clone();
        let on_selection = selected.clone();
        let on_checked = checked.clone();
        let frame = 900_001;
        register(
            frame,
            vstack((
                Button::new("Ok").on_click(move || on_click.set(true)),
                TextBox::new("")
                    .on_text_changed(move |text: String| on_text.borrow_mut().push_str(&text)),
                ComboBox::new(vec!["Uno".to_string(), "Due".to_string()])
                    .on_selection_changed(move |index| on_selection.set(index)),
                CheckBox::new(false).on_checked(move |value| on_checked.set(Some(value))),
            ))
            .into(),
        );

        // `GET /window`: id/classe/source dall'element tree; `bounds`
        // resta ai default (nessun rect per-elemento, §6.3) e viene
        // completato dal chiamante con la finestra top-level Win32.
        let info = window_info(frame).expect("frame registrato");
        assert_eq!(info.id, "xaml:900001");
        assert_eq!(info.class, "StackPanel");
        assert_eq!(info.source, source::NATIVE);
        assert_eq!(info.bounds, Bounds::default());

        let send = |action: &str, id: &str, key: Option<&str>, text: Option<&str>| {
            inject_event(
                frame,
                &InjectedEvent {
                    action: action.to_string(),
                    id: Some(id.to_string()),
                    key: key.map(str::to_string),
                    text: text.map(str::to_string),
                    ..Default::default()
                },
            )
        };

        // `click` sul Button (path `0/0`).
        send("click", "xaml:0/0", None, None).expect("click sul Button");
        assert!(clicked.get());

        // `set_text` sulla TextBox (path `0/1`).
        send("set_text", "xaml:0/1", None, Some("ciao")).expect("set_text sulla TextBox");
        assert_eq!(*typed.borrow(), "ciao");

        // `select` sulla ComboBox con l'indice in `key` (path `0/2`).
        send("select", "xaml:0/2", Some("1"), None).expect("select sulla ComboBox");
        assert_eq!(selected.get(), 1);

        // `click` sulla CheckBox (path `0/3`): toggle come `BM_CLICK`.
        send("click", "xaml:0/3", None, None).expect("click sulla CheckBox");
        assert_eq!(checked.get(), Some(true));

        // Errori: nodo inesistente, widget senza handler per l'azione,
        // azioni non disponibili sul registro e frame non registrato.
        assert!(send("click", "xaml:0/9", None, None).is_err());
        assert!(send("click", "xaml:0/1", None, None).is_err());
        assert!(send("focus", "xaml:0/0", None, None).is_err());
        let root_id = format!("xaml:{frame}");
        assert!(send("click", &root_id, None, None).is_err());

        // `/find?role=` (§8): ruoli identici ai backend nativi, letti
        // dallo stesso `dump_tree` che alimenta la route HTTP.
        let role = |role: &str| Selector {
            role: Some(role.to_string()),
            ..Default::default()
        };
        let hits = find_nodes(frame as isize, &role("button"));
        assert_eq!(hits.len(), 1, "un solo Button");
        assert_eq!(hits[0].path, "0/0");
        assert_eq!(find_nodes(frame as isize, &role("input")).len(), 1);
        assert_eq!(find_nodes(frame as isize, &role("choice")).len(), 1);
        assert_eq!(find_nodes(frame as isize, &role("check")).len(), 1);

        unregister(frame);
        assert!(window_info(frame).is_none());
        assert!(send("click", "xaml:0/0", None, None).is_err());
    }
}
