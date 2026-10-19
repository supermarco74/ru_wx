//! Debug inspector — schema condiviso e dispatch cross-platform.
//!
//! Questo modulo è il **Livello 3** della regola a tre livelli descritta
//! in `PIANO_DEBUG_INSPECTOR.md` §2.4: definisce i tipi del modello dati
//! (§6) e smista le tre primitive native (`dump_tree`, `window_info`,
//! `node_screenshot`) verso l'implementazione per-OS.
//!
//! Qui **non** si usa `objc2` né `windows-sys`: le chiamate native vivono
//! in `inspector_macos.rs` / `inspector_windows.rs`.

use serde::{Deserialize, Serialize};

use crate::platform::NativeHandle;

/// Rettangolo in coordinate schermo, origine **alto-sinistra**.
///
/// L'unità è quella nativa dello spazio in cui vengono iniettati gli
/// eventi: *punti* su macOS, *pixel fisici* su Windows (cfr. §12).
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Bounds {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    /// Centro del rettangolo: il punto su cui va iniettato un click.
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    /// `true` se il rettangolo ha area non nulla.
    pub fn is_valid(&self) -> bool {
        self.w > 0.0 && self.h > 0.0
    }
}

/// Colori di primo piano/sfondo in formato `#RRGGBBAA`.
///
/// `None` è un valore legittimo: molti view sono trasparenti e alcuni
/// controlli custom-drawn non espongono un colore affidabile (§6.3).
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct ColorInfo {
    pub text: Option<String>,
    pub background: Option<String>,
}

/// Provenienza di un dato, per dire all'agente **quanto fidarsi** (§6.1).
pub mod source {
    /// Letto dall'OS tramite il backend.
    pub const NATIVE: &str = "native";
    /// Fornito direttamente dal widget `ru_wx`.
    pub const REGISTRY: &str = "registry";
    /// Dedotto: non da considerare veritiero.
    pub const INFERRED: &str = "inferred";
    /// Il getter di `ru_wx` non esiste ancora → campo `null`.
    pub const MISSING: &str = "missing";
}

/// Un nodo dell'albero dei componenti.
#[derive(Serialize, Clone, Debug)]
pub struct ComponentNode {
    /// Handle nativo in decimale, oppure `mac:0x..` per view non registrate.
    pub id: String,
    /// Classe nativa (`"NSButton"` / `"Button"`).
    pub component_type: String,
    /// Ruolo logico normalizzato (`"button"`, `"label"`, `"table"`, …).
    pub role: String,

    pub text: Option<String>,
    pub bounds: Bounds,
    pub visible: bool,
    pub enabled: bool,
    pub focused: bool,

    pub colors: ColorInfo,
    /// Stato specifico del widget (`checked`, `selected_rows`, …).
    pub state: serde_json::Value,
    /// Contenuti complessi (`columns`, `items`, `rows`, …).
    pub extra: serde_json::Value,

    /// Provenienza del dato (vedi [`source`]).
    pub source: String,

    pub children: Vec<ComponentNode>,
}

impl ComponentNode {
    /// Crea un nodo con i campi comuni valorizzati e stato/extra vuoti.
    pub fn new(
        id: impl Into<String>,
        component_type: impl Into<String>,
        role: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            component_type: component_type.into(),
            role: role.into(),
            text: None,
            bounds: Bounds::default(),
            visible: true,
            enabled: true,
            focused: false,
            colors: ColorInfo::default(),
            state: serde_json::Value::Null,
            extra: serde_json::Value::Null,
            source: source::NATIVE.to_string(),
            children: Vec::new(),
        }
    }
}

/// Metadati della finestra radice (`GET /window`).
#[derive(Serialize, Clone, Debug, Default)]
pub struct WindowInfo {
    pub id: String,
    pub title: String,
    /// Classe nativa della finestra (`NSWindow` / HWND class).
    pub class: String,
    /// Rect della finestra in coordinate schermo (include il chrome).
    pub bounds: Bounds,
    /// Area utile (client) in unità native.
    pub client_w: u32,
    pub client_h: u32,
    pub visible: bool,
    /// Fattore di scala (retina `backingScaleFactor`; DPI/96 su Windows).
    pub scale: f64,
    /// DPI effettivo (96 su macOS, `GetDpiForWindow` su Windows).
    pub dpi: u32,
    pub screen_w: u32,
    pub screen_h: u32,
    pub source: String,
}

/// Riferimento compatto a un nodo, usato da `/find` e `find_element` (§7.1).
#[derive(Serialize, Clone, Debug)]
pub struct NodeRef {
    pub id: String,
    pub path: String,
    pub role: String,
    pub component_type: String,
    pub text: Option<String>,
    pub bounds: Bounds,
}

/// Selettore per `/find`: i campi valorizzati sono combinati in AND.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct Selector {
    pub id: Option<String>,
    pub role: Option<String>,
    pub text: Option<String>,
    pub path: Option<String>,
}

impl Selector {
    pub fn is_empty(&self) -> bool {
        self.id.is_none() && self.role.is_none() && self.text.is_none() && self.path.is_none()
    }

    fn matches(&self, node: &ComponentNode, path: &str) -> bool {
        if let Some(id) = &self.id {
            if &node.id != id {
                return false;
            }
        }
        if let Some(role) = &self.role {
            if !node.role.eq_ignore_ascii_case(role) {
                return false;
            }
        }
        if let Some(path_pat) = &self.path {
            if path != path_pat {
                return false;
            }
        }
        if let Some(needle) = &self.text {
            let needle = needle.to_lowercase();
            let hit = node
                .text
                .as_deref()
                .map(|t| t.to_lowercase().contains(&needle))
                .unwrap_or(false);
            if !hit {
                return false;
            }
        }
        true
    }
}

/// Evento di input da iniettare (`POST /event`, §11).
#[derive(Deserialize, Clone, Debug, Default)]
pub struct InjectedEvent {
    pub action: String,
    /// Nodo bersaglio (alternativo a `x`/`y`).
    pub id: Option<String>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub text: Option<String>,
    pub key: Option<String>,
    #[serde(default)]
    pub modifiers: Vec<String>,
}

// ── Dispatch cross-platform ─────────────────────────────────────────

/// Costruisce l'albero dei componenti a partire dalla finestra radice.
pub fn dump_tree(root: NativeHandle) -> ComponentNode {
    #[cfg(target_os = "macos")]
    {
        super::inspector_macos::dump_tree(root)
    }
    #[cfg(target_os = "windows")]
    {
        super::inspector_windows::dump_tree(root)
    }
    #[cfg(target_os = "linux")]
    {
        super::inspector_gtk::dump_tree(root)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = root;
        let mut n = ComponentNode::new("0", "stub", "window");
        n.source = source::MISSING.to_string();
        n
    }
}

/// Legge i metadati della finestra radice.
pub fn window_info(root: NativeHandle) -> WindowInfo {
    #[cfg(target_os = "macos")]
    {
        super::inspector_macos::window_info(root)
    }
    #[cfg(target_os = "windows")]
    {
        super::inspector_windows::window_info(root)
    }
    #[cfg(target_os = "linux")]
    {
        super::inspector_gtk::window_info(root)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = root;
        WindowInfo {
            source: source::MISSING.to_string(),
            ..Default::default()
        }
    }
}

/// Cerca i nodi che soddisfano il selettore. Derivato da [`dump_tree`],
/// quindi condiviso cross-platform senza codice per-OS aggiuntivo.
pub fn find_nodes(root: NativeHandle, sel: &Selector) -> Vec<NodeRef> {
    let tree = dump_tree(root);
    let mut out = Vec::new();
    collect_matches(&tree, sel, "0", &mut out);
    out
}

fn collect_matches(node: &ComponentNode, sel: &Selector, path: &str, out: &mut Vec<NodeRef>) {
    if sel.matches(node, path) {
        out.push(NodeRef {
            id: node.id.clone(),
            path: path.to_string(),
            role: node.role.clone(),
            component_type: node.component_type.clone(),
            text: node.text.clone(),
            bounds: node.bounds.clone(),
        });
    }
    for (i, child) in node.children.iter().enumerate() {
        collect_matches(child, sel, &format!("{path}/{i}"), out);
    }
}

/// Risolve il primo nodo che soddisfa il selettore, in profondità-first.
pub fn find_first(root: NativeHandle, sel: &Selector) -> Option<NodeRef> {
    find_nodes(root, sel).into_iter().next()
}

/// Inietta un'azione su un nodo identificato da `event.id` (cfr. §11).
pub fn inject_event(root: NativeHandle, event: &InjectedEvent) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        super::inspector_macos::inject_event(root, event)
    }
    #[cfg(target_os = "windows")]
    {
        super::inspector_windows::inject_event(root, event)
    }
    #[cfg(target_os = "linux")]
    {
        super::inspector_gtk::inject_event(root, event)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = (root, event);
        Err("piattaforma non supportata".to_string())
    }
}

/// PNG della finestra (`sel` vuoto) o del nodo indicato (`GET /screenshot`).
pub fn node_screenshot(root: NativeHandle, sel: &Selector) -> Result<Vec<u8>, String> {
    #[cfg(target_os = "macos")]
    {
        super::inspector_macos::node_screenshot(root, sel)
    }
    #[cfg(target_os = "windows")]
    {
        super::inspector_windows::node_screenshot(root, sel)
    }
    #[cfg(target_os = "linux")]
    {
        super::inspector_gtk::node_screenshot(root, sel)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = (root, sel);
        Err("piattaforma non supportata".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tree() -> ComponentNode {
        let mut root = ComponentNode::new("1", "NSWindow", "window");
        root.text = Some("Login".into());
        let mut btn = ComponentNode::new("2", "NSButton", "button");
        btn.text = Some("Accedi".into());
        let mut label = ComponentNode::new("3", "NSTextField", "label");
        label.text = Some("Utente".into());
        root.children.push(btn);
        root.children.push(label);
        root
    }

    fn find_in(tree: &ComponentNode, sel: &Selector) -> Vec<NodeRef> {
        let mut out = Vec::new();
        collect_matches(tree, sel, "0", &mut out);
        out
    }

    #[test]
    fn selector_matches_role_and_text() {
        let tree = sample_tree();
        let sel = Selector {
            role: Some("button".into()),
            text: Some("acce".into()),
            ..Default::default()
        };
        let hits = find_in(&tree, &sel);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "2");
    }

    #[test]
    fn selector_path_is_positional() {
        let tree = sample_tree();
        let sel = Selector {
            path: Some("0/1".into()),
            ..Default::default()
        };
        let hits = find_in(&tree, &sel);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "3");
    }

    #[test]
    fn text_match_is_case_insensitive() {
        let tree = sample_tree();
        let sel = Selector {
            text: Some("UTENTE".into()),
            ..Default::default()
        };
        let hits = find_in(&tree, &sel);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].role, "label");
    }

    #[test]
    fn bounds_center_and_validity() {
        let b = Bounds::new(10.0, 20.0, 100.0, 40.0);
        assert_eq!(b.center(), (60.0, 40.0));
        assert!(b.is_valid());
        assert!(!Bounds::default().is_valid());
    }
}
