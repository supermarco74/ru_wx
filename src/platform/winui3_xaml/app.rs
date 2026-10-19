//! XamlApp — reactor-based application entry point.
//!
//! Replaces the previous Win32-only `XamlApp` with a real WinUI3/XAML
//! backend built on `windows-reactor`. The app collects widget descriptors
//! during the imperative phase, then converts them to a reactor `Element`
//! tree inside the render closure.
//!
//! # Thread model
//!
//! Reactor requires `Send` closures. The bridge extracts `Send`-safe data
//! (cloned `WidgetDesc` tree + `Arc<Mutex<EventRegistry>>`) before creating
//! the render closure, avoiding `Rc` in the reactor boundary.
//!
//! # Usage
//!
//! ```ignore
//! let app = XamlApp::new();
//! let frame = XamlFrame::builder()
//!     .with_title("My App")
//!     .with_size(600, 400)
//!     .build();
//! app.run(frame)?;
//! ```

use windows_reactor::{App as ReactorApp, Element, ElementExt, RenderCx};

use super::element_bridge::{GridLengthDesc, SharedRegistry, WidgetDesc};
use super::element_tree::ROLE_MARKER_PREFIX;
use super::window::XamlFrame;

/// Application entry point for the WinUI3/XAML backend.
///
/// Wraps `windows_reactor::App` and converts the imperative `ru_wx` widget
/// model into reactor's declarative element tree.
/// Boxed handler for reactor callback faults.
type FaultHandler = Box<dyn Fn(&windows_reactor::Fault) + Send>;

pub struct XamlApp {
    /// Optional global fault handler (reactor panics in callbacks).
    on_fault: Option<FaultHandler>,
}

impl XamlApp {
    pub fn new() -> Self {
        Self { on_fault: None }
    }

    /// Register a handler for reactor callback faults.
    pub fn on_fault<F>(mut self, f: F) -> Self
    where
        F: Fn(&windows_reactor::Fault) + Send + 'static,
    {
        self.on_fault = Some(Box::new(f));
        self
    }

    /// Run the application with the given frame.
    ///
    /// Converts the frame's widget descriptors into a reactor element tree
    /// and starts the reactor event loop. This call blocks until the last
    /// window closes.
    pub fn run(self, frame: XamlFrame) -> Result<(), windows_core::Error> {
        // Initialize the Windows App SDK runtime BEFORE touching any WinRT
        // type. The reactor's own `init_app_platform` only sets DPI awareness
        // and COM — it does NOT call MddBootstrapInitialize2, so we must do
        // it here. Without this, `Application::Start` fails with
        // E_ACCESSDENIED / "runtime not found".
        crate::platform::winui3_xaml::bootstrap::init_platform()?;

        // Extract Send-safe data from the descriptor BEFORE creating the
        // closure. `WidgetDesc` is all-`Send` (String, Vec, f64, bool);
        // `SharedRegistry` is `Arc<Mutex>` which is `Send`.
        let frame_id = frame.frame_id;
        let (title, width, height, children, registry, padding, spacing) = {
            let d = frame.desc.borrow();
            (
                d.title.clone(),
                d.width,
                d.height,
                d.children.clone(),
                d.registry.clone(),
                d.padding,
                d.spacing,
            )
        };

        let mut reactor_app = ReactorApp::new().title(title).inner_size(width, height);

        if let Some(on_fault) = self.on_fault {
            reactor_app = reactor_app.on_fault(on_fault);
        }

        // Track previous size for resize-event detection.
        let prev_size = std::sync::Arc::new(std::sync::Mutex::new((width, height)));
        let prev_size_render = prev_size.clone();

        // Clone the Arc for the closure; keep the original for the close handler.
        let registry_render = registry.clone();

        // The render closure converts the widget descriptor tree into
        // a reactor Element tree. It re-runs whenever reactor invalidates
        // the UI (e.g. after state changes, which includes resize).
        let result = reactor_app.render(move |cx| {
            // Fire resize handler if the size changed since last render.
            if let Ok(mut prev) = prev_size_render.lock() {
                let current = (cx.use_inner_size().width, cx.use_inner_size().height);
                if current != *prev {
                    *prev = current;
                    if let Ok(mut reg) = registry_render.lock() {
                        if let Some(handler) = reg.resize_handlers.get_mut(&frame_id) {
                            handler(current.0 as i32, current.1 as i32);
                        }
                    }
                }
            }
            let root = build_element_tree(&children, &registry_render, cx, spacing, padding);
            // Register the freshly built element tree so the debug inspector
            // can read it (`Element` / `PropValue` / `find_prop`). Runs on the
            // UI thread, which is also where `dump_tree` is dispatched (§5).
            super::element_tree::register(frame_id, root.clone());
            // Avvia l'inspector (una sola volta) sul frame XAML: il "handle"
            // radice è il `frame_id` e il thread UI viene svegliato tramite il
            // `DispatcherQueue` WinUI, non con un messaggio HWND (§5, §10.1).
            #[cfg(any(debug_assertions, feature = "inspector"))]
            {
                static INSPECTOR_STARTED: std::sync::Once = std::sync::Once::new();
                INSPECTOR_STARTED.call_once(|| {
                    crate::debug::inspector::start_xaml(frame_id);
                });
            }
            root
        });

        // Fire close handler after the window closes.
        if let Ok(mut reg) = registry.lock() {
            if let Some(handler) = reg.close_handlers.get_mut(&frame_id) {
                handler();
            }
        }

        super::element_tree::unregister(frame_id);

        // Reactor is built against the in-tree `windows-core` while `ru_wx`
        // uses the crates.io release: the two `Error` types are distinct even
        // though they expose the same API. Bridge them through the raw
        // `HRESULT` so the public signature stays `windows_core::Error`.
        result.map_err(|e| windows_core::Error::from_hresult(windows_core::HRESULT(e.code().0)))
    }
}

impl Default for XamlApp {
    fn default() -> Self {
        Self::new()
    }
}

// ── Element tree builder ─────────────────────────────────────────────────────

/// Convert a list of widget descriptors into a single reactor `Element`.
///
/// Multiple children are wrapped in a vertical `StackPanel`. A single child
/// is returned directly. An empty list returns `Element::Empty`.
///
/// `spacing` e `padding` (dal frame) vengono applicati al container radice:
/// con entrambi a 0 il comportamento resta identico a prima (nessun wrapper
/// aggiunto nei casi a 0/1 figlio).
fn build_element_tree(
    children: &[WidgetDesc],
    registry: &SharedRegistry,
    _cx: &mut RenderCx,
    spacing: f64,
    padding: f64,
) -> Element {
    // Dedup radice: i widget creati con `XamlXxx::new(&frame, …)` finiscono
    // tra i figli del frame; se vengono poi referenziati anche dentro un
    // container (Panel, BoxSizer, Tab, Splitter, ScrollViewer) comparirebbero
    // due volte (root + container). Escludi dal root gli id già presenti in
    // un subtree, così ogni widget è renderizzato una sola volta.
    let mut nested: std::collections::HashSet<u64> = std::collections::HashSet::new();
    for c in children {
        for sub in child_descendants(c) {
            collect_tree_ids(sub, &mut nested);
        }
    }
    let top: Vec<&WidgetDesc> = children
        .iter()
        .filter(|c| !c.id().is_some_and(|id| nested.contains(&id)))
        .collect();

    let needs_container = top.len() != 1 || spacing > 0.0 || padding > 0.0;

    // Status bar: la si ancora in basso con una grid a 2 righe
    // (contenuto = Star, barra = Pixel 24). Con una semplice vstack la
    // barra restava "appesa" subito dopo il contenuto.
    if top
        .iter()
        .any(|d| matches!(d, WidgetDesc::StatusBar { .. }))
    {
        let mut content: Vec<Element> = Vec::new();
        let mut grid_children: Vec<Element> = Vec::new();
        for d in &top {
            let e = widget_desc_to_element(d, registry);
            if matches!(d, WidgetDesc::StatusBar { .. }) {
                grid_children.push(e.grid_row(1));
            } else {
                content.push(e);
            }
        }
        if !content.is_empty() {
            let wrap = content.len() != 1 || spacing > 0.0 || padding > 0.0;
            let el = if wrap {
                let mut stack = windows_reactor::vstack(content);
                if spacing > 0.0 {
                    stack = stack.spacing(spacing);
                }
                if padding > 0.0 {
                    stack.modifiers.padding = Some(windows_reactor::Thickness {
                        left: padding,
                        top: padding,
                        right: padding,
                        bottom: padding,
                    });
                }
                stack.into()
            } else {
                content.into_iter().next().unwrap_or(Element::Empty)
            };
            grid_children.insert(0, el);
        }
        return windows_reactor::grid(grid_children)
            .rows([
                windows_reactor::GridLength::STAR,
                windows_reactor::GridLength::Pixel(24.0),
            ])
            .into();
    }

    if !needs_container {
        return top
            .first()
            .map(|d| widget_desc_to_element(d, registry))
            .unwrap_or(Element::Empty);
    }

    let elements: Vec<Element> = top
        .iter()
        .map(|d| widget_desc_to_element(d, registry))
        .collect();
    let mut stack = windows_reactor::vstack(elements);
    if spacing > 0.0 {
        stack = stack.spacing(spacing);
    }
    if padding > 0.0 {
        stack.modifiers.padding = Some(windows_reactor::Thickness {
            left: padding,
            top: padding,
            right: padding,
            bottom: padding,
        });
    }
    stack.into()
}

/// Figli diretti di un descrittore container (vuoto per i widget foglia).
fn child_descendants(d: &WidgetDesc) -> Vec<&WidgetDesc> {
    match d {
        WidgetDesc::VStack { children, .. }
        | WidgetDesc::HStack { children, .. }
        | WidgetDesc::Panel { children, .. }
        | WidgetDesc::GridLayout { children, .. } => children.iter().collect(),
        WidgetDesc::TabView { tabs, .. } => tabs.iter().map(|t| &t.content).collect(),
        WidgetDesc::SplitView { pane, content, .. } => vec![pane.as_ref(), content.as_ref()],
        WidgetDesc::ScrollViewer { child, .. } => vec![child.as_ref()],
        _ => Vec::new(),
    }
}

/// Inserisce in `out` l'id di `d` e di tutti i suoi discendenti.
fn collect_tree_ids(d: &WidgetDesc, out: &mut std::collections::HashSet<u64>) {
    if let Some(id) = d.id() {
        out.insert(id);
    }
    for c in child_descendants(d) {
        collect_tree_ids(c, out);
    }
}

/// Convert a single `WidgetDesc` into a reactor `Element`.
///
/// This is the core mapping from the ru_wx imperative model to the reactor
/// declarative model. Each variant creates the corresponding reactor widget
/// and wires up event handlers from the shared registry.
///
/// NOTE: reactor widget constructors use builder pattern with specific
/// signatures — see `windows-reactor/src/widgets/*.rs` for the canonical
/// API. The mapping below follows the real reactor API (verified against
/// `windows-reactor` v0.0.0 in this workspace).
fn widget_desc_to_element(desc: &WidgetDesc, _registry: &SharedRegistry) -> Element {
    match desc {
        WidgetDesc::TextBlock {
            text,
            font_size,
            bold,
            ..
        } => {
            let mut tb = windows_reactor::text_block(text.clone());
            if *font_size > 0.0 {
                tb = tb.font_size(*font_size);
            }
            if *bold {
                tb = tb.bold();
            }
            tb.into()
        }
        WidgetDesc::Button {
            label,
            enabled,
            width,
            ..
        } => {
            let mut btn = windows_reactor::button(label.clone()).enabled(*enabled);
            if *width > 0.0 {
                btn.modifiers.width = Some(*width);
            }
            btn.into()
        }
        WidgetDesc::TextBox {
            text,
            placeholder,
            read_only,
            ..
        } => {
            let mut tb = windows_reactor::text_box(text.clone());
            if !placeholder.is_empty() {
                tb = tb.placeholder_text(placeholder.clone());
            }
            if *read_only {
                // reactor TextBox has no read_only; use enabled=false as
                // the closest approximation for now.
                tb = tb.enabled(false);
            }
            tb.into()
        }
        WidgetDesc::TextArea {
            text, read_only, ..
        } => {
            let mut tb = windows_reactor::text_box(text.clone()).multiline();
            if *read_only {
                tb = tb.enabled(false);
            }
            tb.into()
        }
        WidgetDesc::CheckBox { label, checked, .. } => {
            // reactor check_box takes only `is_checked`; label via .content()
            let cb = windows_reactor::check_box(*checked).content(label.clone());
            cb.into()
        }
        WidgetDesc::RadioButton {
            label,
            group,
            checked,
            ..
        } => {
            // reactor has no free `radio_button()` fn; use struct directly
            let rb = windows_reactor::RadioButton::new(label.clone())
                .group(group.clone())
                .checked(*checked);
            rb.into()
        }
        WidgetDesc::Slider {
            min, max, value, ..
        } => {
            // reactor Slider::new(value), then .range(min, max)
            let s = windows_reactor::Slider::new(*value).range(*min, *max);
            s.into()
        }
        WidgetDesc::ProgressBar {
            max,
            value,
            indeterminate,
            ..
        } => {
            // reactor ProgressBar::new(value) or ProgressBar::indeterminate()
            let pb = if *indeterminate {
                windows_reactor::ProgressBar::indeterminate()
            } else {
                windows_reactor::ProgressBar::new(*value).range(0.0, *max)
            };
            pb.into()
        }
        WidgetDesc::ComboBox {
            items,
            selected,
            editable,
            width,
            ..
        } => {
            // reactor ComboBox::new(items), selected_index is i32 (-1 = none)
            let mut cb = windows_reactor::ComboBox::new(items.clone());
            if let Some(sel) = selected {
                cb = cb.selected_index(*sel as i32);
            }
            if *editable {
                cb = cb.editable(true);
            }
            if *width > 0.0 {
                cb.modifiers.width = Some(*width);
            }
            cb.into()
        }
        WidgetDesc::VStack {
            children,
            spacing,
            group,
            ..
        } => {
            let elements: Vec<Element> = children
                .iter()
                .map(|d| widget_desc_to_element(d, _registry))
                .collect();
            let mut sp = windows_reactor::vstack(elements);
            if *spacing > 0.0 {
                sp = sp.spacing(*spacing);
            }
            let el: Element = sp.into();
            // Sizer dichiarato → ruolo `group` nell'inspector (§6 C3).
            if *group {
                return el.automation_id(format!("{ROLE_MARKER_PREFIX}group"));
            }
            el
        }
        WidgetDesc::HStack {
            children,
            spacing,
            group,
            ..
        } => {
            let elements: Vec<Element> = children
                .iter()
                .map(|d| widget_desc_to_element(d, _registry))
                .collect();
            let mut sp = windows_reactor::hstack(elements);
            if *spacing > 0.0 {
                sp = sp.spacing(*spacing);
            }
            let el: Element = sp.into();
            if *group {
                return el.automation_id(format!("{ROLE_MARKER_PREFIX}group"));
            }
            el
        }
        WidgetDesc::Panel { children, .. } => {
            // Panel = plain container; children stacked vertically by default.
            let elements: Vec<Element> = children
                .iter()
                .map(|d| widget_desc_to_element(d, _registry))
                .collect();
            windows_reactor::vstack(elements).into()
        }
        WidgetDesc::TabView { tabs, selected, .. } => {
            let tab_items: Vec<windows_reactor::TabItem> = tabs
                .iter()
                .map(|t| {
                    let content = widget_desc_to_element(&t.content, _registry);
                    // Imposta IsClosable sempre: il default della piattaforma
                    // è `true`, quindi un tab non-closable deve dichiarare
                    // esplicitamente `false` altrimenti mostra la X lo stesso.
                    windows_reactor::TabItem::new(t.header.clone(), content).closable(t.closable)
                })
                .collect();
            let tv = windows_reactor::TabView::new(tab_items).selected_index(*selected);
            tv.into()
        }
        WidgetDesc::SplitView {
            pane,
            content,
            pane_open,
            pane_length,
            ..
        } => {
            let pane_el = widget_desc_to_element(pane, _registry);
            let content_el = widget_desc_to_element(content, _registry);
            let sv = windows_reactor::SplitView::new(content_el)
                .pane(pane_el)
                .is_pane_open(*pane_open)
                .open_pane_length(*pane_length);
            sv.into()
        }
        WidgetDesc::ScrollViewer {
            child,
            horizontal_scroll,
            vertical_scroll,
            ..
        } => {
            let child_el = widget_desc_to_element(child, _registry);
            let mut sv = windows_reactor::scroll_viewer(child_el);
            if *horizontal_scroll {
                sv =
                    sv.horizontal_scroll_bar_visibility(windows_reactor::ScrollBarVisibility::Auto);
            }
            if *vertical_scroll {
                sv = sv.vertical_scroll_bar_visibility(windows_reactor::ScrollBarVisibility::Auto);
            }
            sv.into()
        }
        WidgetDesc::GridLayout {
            rows,
            columns,
            children,
            row_spacing,
            column_spacing,
            ..
        } => {
            let elements: Vec<Element> = children
                .iter()
                .map(|d| widget_desc_to_element(d, _registry))
                .collect();
            let mut g = windows_reactor::grid(elements);
            if !rows.is_empty() {
                g = g.rows(rows.iter().map(|r| grid_length_desc_to_reactor(*r)));
            }
            if !columns.is_empty() {
                g = g.columns(columns.iter().map(|c| grid_length_desc_to_reactor(*c)));
            }
            if *row_spacing > 0.0 {
                g = g.row_spacing(*row_spacing);
            }
            if *column_spacing > 0.0 {
                g = g.column_spacing(*column_spacing);
            }
            g.into()
        }
        WidgetDesc::MenuBar { menus, .. } => {
            let reactor_menus: Vec<windows_reactor::MenuBarItemDef> = menus
                .iter()
                .map(|m| {
                    let items = m.items.iter().map(menu_item_desc_to_reactor).collect();
                    windows_reactor::MenuBarItemDef::new(m.title.clone(), items)
                })
                .collect();
            windows_reactor::menu_bar(reactor_menus).into()
        }
        WidgetDesc::ToolBar { items, .. } => {
            let reactor_items: Vec<windows_reactor::CommandBarCommandDef> = items
                .iter()
                .map(|item| match item {
                    super::element_bridge::ToolBarItemDesc::Button { label } => {
                        windows_reactor::app_bar_button(label.clone())
                    }
                    super::element_bridge::ToolBarItemDesc::Toggle { label } => {
                        windows_reactor::app_bar_toggle(label.clone())
                    }
                    super::element_bridge::ToolBarItemDesc::Separator => {
                        windows_reactor::app_bar_separator()
                    }
                })
                .collect();
            // Etichette affiancate: con il default (Bottom) e senza icone i label
            // degli AppBarButton non vengono renderizzati.
            windows_reactor::command_bar(reactor_items)
                .default_label_position(windows_reactor::CommandBarDefaultLabelPosition(1))
                .into()
        }
        WidgetDesc::StatusBar { fields, .. } => {
            // StatusBar = barra orizzontale con campi di testo. L'ancoraggio
            // in basso lo fa la grid del root (vedi `build_element_tree`);
            // qui si gestiscono solo spaziatura, padding e width dei campi.
            let field_elements: Vec<Element> = fields
                .iter()
                .map(|f| {
                    let mut tb = windows_reactor::text_block(f.text.clone());
                    if let Some(w) = f.width {
                        tb.modifiers.width = Some(w);
                    }
                    tb.into()
                })
                .collect();
            let mut bar = windows_reactor::hstack(field_elements).spacing(16.0);
            bar.modifiers.padding = Some(windows_reactor::Thickness {
                left: 8.0,
                top: 0.0,
                right: 8.0,
                bottom: 0.0,
            });
            bar.into()
        }
        WidgetDesc::ContentDialog {
            title,
            content,
            primary_text,
            secondary_text,
            close_text,
            is_open,
            ..
        } => {
            let mut dlg = windows_reactor::ContentDialog::new(title.clone())
                .content(content.clone())
                .is_open(*is_open);
            if !primary_text.is_empty() {
                dlg = dlg.primary_button_text(primary_text.clone());
            }
            if !secondary_text.is_empty() {
                dlg = dlg.secondary_button_text(secondary_text.clone());
            }
            if !close_text.is_empty() {
                dlg = dlg.close_button_text(close_text.clone());
            }
            dlg.into()
        }
        WidgetDesc::ColorPicker { color, is_open, .. } => {
            // Chiuso = nessuno spazio in layout (altrimenti il picker
            // "entra" nella finestra alla creazione e butta fuori gli altri
            // widget). Vedi `XamlColorDialog::show/close`.
            if !*is_open {
                return Element::Empty;
            }
            let cp = windows_reactor::color_picker(windows_reactor::ColorArgb::with_alpha(
                color.0, color.1, color.2, color.3,
            ));
            cp.into()
        }
        WidgetDesc::Image {
            id: _,
            uri,
            stretch,
            width,
            height,
        } => {
            // `BitmapImage.SetUriSource` riceve un `Uri` WinRT: un path
            // Windows assoluto ("C:\a\b.png") non è un URI valido e il
            // caricamento fallisce silenziosamente. Normalizzo a file:///.
            let norm = if (uri.contains('\\') || uri.starts_with('/'))
                && !uri.contains("://")
                && !uri.starts_with("ms-appx:")
            {
                format!("file:///{}", uri.replace('\\', "/").trim_start_matches('/'))
            } else {
                uri.clone()
            };
            let mut img = windows_reactor::Image::new_with_uri(norm);
            img = img.stretch(match stretch {
                super::element_bridge::ImageStretch::None => windows_reactor::Stretch::None,
                super::element_bridge::ImageStretch::Fill => windows_reactor::Stretch::Fill,
                super::element_bridge::ImageStretch::Uniform => windows_reactor::Stretch::Uniform,
                super::element_bridge::ImageStretch::UniformToFill => {
                    windows_reactor::Stretch::UniformToFill
                }
            });
            // Size esplicita: senza, l'Image desidera tutta la larghezza
            // disponibile e schiaccia i widget successivi fuori finestra.
            img.modifiers.width = *width;
            img.modifiers.height = *height;
            img.into()
        }
        WidgetDesc::TreeView { nodes, .. } => {
            let reactor_nodes = nodes.iter().map(tree_node_desc_to_reactor).collect();
            windows_reactor::tree_view(reactor_nodes).into()
        }
        WidgetDesc::ListBox {
            items, selected, ..
        } => {
            let lb = windows_reactor::list_box()
                .items(items.clone())
                .selected_index(*selected);
            lb.into()
        }
        WidgetDesc::CheckListBox { items, checked, .. } => {
            // Lista con checkbox per riga: reactor non espone ListView,
            // quindi una vstack di CheckBox con lo stato dal descrittore.
            let rows: Vec<Element> = items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let is_on = checked.get(i).copied().unwrap_or(false);
                    let cell: Element = windows_reactor::check_box(is_on)
                        .content(item.clone())
                        .into();
                    cell
                })
                .collect();
            windows_reactor::vstack(rows).into()
        }
        WidgetDesc::Hyperlink { label, url, .. } => {
            // Omologo XAML dell'HyperlinkCtrl di AppKit.
            let link =
                windows_reactor::HyperlinkButton::new(label.clone()).navigate_uri(url.clone());
            link.into()
        }
        WidgetDesc::ListCtrl { columns, rows, .. } => {
            // ListCtrl = griglia vera con righe/colonne: le hstack di
            // text_block rendevano i testi delle colonne attaccati
            // ("IDNomePrezzoStock") e non allineati tra loro.
            let n_cols = columns.len();
            let mut cells: Vec<Element> = Vec::new();
            // Header row
            for (c, col) in columns.iter().enumerate() {
                let cell: Element = windows_reactor::text_block(col.clone()).bold().into();
                cells.push(cell.grid_row(0).grid_column(c as i32));
            }
            // Data rows
            for (r, row) in rows.iter().enumerate() {
                for (c, val) in row.iter().enumerate() {
                    let cell: Element = windows_reactor::text_block(val.clone()).into();
                    cells.push(cell.grid_row((r + 1) as i32).grid_column(c as i32));
                }
            }
            // Vista tabellare → ruolo `table` (simmetria `SysListView32`).
            let table: Element = windows_reactor::grid(cells)
                .columns(std::iter::repeat_n(
                    windows_reactor::GridLength::STAR,
                    n_cols,
                ))
                .rows(std::iter::repeat_n(
                    windows_reactor::GridLength::Auto,
                    rows.len() + 1,
                ))
                .column_spacing(16.0)
                .row_spacing(4.0)
                .into();
            table.automation_id(format!("{ROLE_MARKER_PREFIX}table"))
        }
        WidgetDesc::Grid {
            columns,
            rows,
            sort_column,
            sort_ascending,
            selected_row,
            alternating_colors,
            even_color,
            odd_color,
            ..
        } => {
            // Anche la vista dettagliata è tabellare (ruolo `table`).
            build_grid_element(
                columns,
                rows,
                *sort_column,
                *sort_ascending,
                *selected_row,
                *alternating_colors,
                *even_color,
                *odd_color,
            )
            .automation_id(format!("{ROLE_MARKER_PREFIX}table"))
        }
        WidgetDesc::Spacer => Element::Empty,
    }
}

/// Build the grid element from descriptors.
///
/// The grid is a complex composite: header row with sort indicators,
/// data rows with cell rendering, alternating colors, and selection.
#[allow(clippy::too_many_arguments)] // grid builder mirrors the element descriptor shape
fn build_grid_element(
    columns: &[super::element_bridge::GridColumnDesc],
    rows: &[Vec<super::element_bridge::GridCellDesc>],
    sort_column: Option<usize>,
    sort_ascending: bool,
    selected_row: Option<usize>,
    alternating_colors: bool,
    even_color: (u8, u8, u8),
    odd_color: (u8, u8, u8),
) -> Element {
    let n_cols = columns.len();
    let mut cells: Vec<Element> = Vec::with_capacity((rows.len() + 1) * n_cols);

    // ── Header row ──
    // Le hstack rendevano i titoli delle colonne attaccati
    // ("IDNome ↑CategoriaPrezzoStockRating"): serve una vera grid.
    for (i, col) in columns.iter().enumerate() {
        let mut text = col.title.clone();
        if sort_column == Some(i) {
            text.push_str(if sort_ascending { " ↑" } else { " ↓" });
        }
        let header: Element = windows_reactor::border(windows_reactor::text_block(text).bold())
            .background(windows_reactor::Color::rgb(240, 240, 245))
            .padding(windows_reactor::Thickness::uniform(6.0))
            .into();
        cells.push(header.grid_row(0).grid_column(i as i32));
    }

    // ── Data rows ──
    for (row_idx, row) in rows.iter().enumerate() {
        let is_selected = selected_row == Some(row_idx);
        // Sfondo: selezione (accent) > righe alternate > nessuno.
        let bg: Option<(u8, u8, u8)> = if is_selected {
            Some((79, 70, 229)) // accent
        } else if alternating_colors {
            Some(if row_idx % 2 == 0 {
                even_color
            } else {
                odd_color
            })
        } else {
            None
        };

        for (col_idx, cell_desc) in row.iter().enumerate() {
            let content: Element = grid_cell_to_element(cell_desc);
            let content = if is_selected {
                content.foreground(windows_reactor::Color::rgb(255, 255, 255))
            } else {
                content
            };
            let cell: Element = match bg {
                Some((r, g, b)) => windows_reactor::border(content)
                    .background(windows_reactor::Color::rgb(r, g, b))
                    .padding(windows_reactor::Thickness::uniform(6.0))
                    .into(),
                // Nessuno sfondo: padding su un contenitore trasparente
                // mantiene le celle allineate e distanziate.
                None => windows_reactor::border(content)
                    .padding(windows_reactor::Thickness::uniform(6.0))
                    .into(),
            };
            cells.push(
                cell.grid_row((row_idx + 1) as i32)
                    .grid_column(col_idx as i32),
            );
        }
    }

    let grid: Element = windows_reactor::grid(cells)
        .columns(std::iter::repeat_n(
            windows_reactor::GridLength::STAR,
            n_cols,
        ))
        .rows(std::iter::repeat_n(
            windows_reactor::GridLength::Auto,
            rows.len() + 1,
        ))
        .into();

    // Wrap in a scrollable vertical stack
    windows_reactor::scroll_viewer(windows_reactor::vstack(vec![grid])).into()
}

/// Convert a grid cell descriptor to a reactor element.
fn grid_cell_to_element(cell: &super::element_bridge::GridCellDesc) -> Element {
    match cell {
        super::element_bridge::GridCellDesc::Text(text) => {
            windows_reactor::text_block(text.clone()).into()
        }
        super::element_bridge::GridCellDesc::Empty => windows_reactor::text_block("").into(),
        super::element_bridge::GridCellDesc::Progress { value, max } => {
            let pct = if *max > 0 {
                (*value as f64 / *max as f64 * 100.0) as u32
            } else {
                0
            };
            let bar = progress_bar_unicode(*value, *max, 10);
            windows_reactor::text_block(format!("{} {}%", bar, pct)).into()
        }
        super::element_bridge::GridCellDesc::Badge { text, kind } => {
            let icon = match kind {
                super::element_bridge::GridBadgeKind::Good => "● ",
                super::element_bridge::GridBadgeKind::Warn => "▲ ",
                super::element_bridge::GridBadgeKind::Bad => "✕ ",
                super::element_bridge::GridBadgeKind::Info => "○ ",
            };
            windows_reactor::text_block(format!("{}{}", icon, text)).into()
        }
        super::element_bridge::GridCellDesc::Bool(checked) => {
            windows_reactor::check_box(*checked).into()
        }
    }
}

/// Render a progress bar using Unicode block characters.
fn progress_bar_unicode(value: u32, max: u32, width: usize) -> String {
    let filled = if max > 0 {
        (value as f64 / max as f64 * width as f64) as usize
    } else {
        0
    };
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// Convert a `TreeNodeDesc` to reactor's `TreeNodeDef`.
fn tree_node_desc_to_reactor(
    desc: &super::element_bridge::TreeNodeDesc,
) -> windows_reactor::TreeNodeDef {
    let mut node = windows_reactor::TreeNodeDef::new(desc.text.clone());
    if desc.expanded {
        node = node.expanded();
    }
    if !desc.children.is_empty() {
        let children = desc
            .children
            .iter()
            .map(tree_node_desc_to_reactor)
            .collect();
        node = node.children(children);
    }
    node
}

/// Convert a `MenuItemDesc` to reactor's `MenuItemDef`.
fn menu_item_desc_to_reactor(
    desc: &super::element_bridge::MenuItemDesc,
) -> windows_reactor::MenuItemDef {
    match desc {
        super::element_bridge::MenuItemDesc::Item { text, enabled } => {
            if *enabled {
                windows_reactor::menu_item(text.clone())
            } else {
                windows_reactor::menu_item_disabled(text.clone())
            }
        }
        super::element_bridge::MenuItemDesc::Separator => windows_reactor::menu_separator(),
        super::element_bridge::MenuItemDesc::SubMenu { text, children } => {
            let child_items = children.iter().map(menu_item_desc_to_reactor).collect();
            windows_reactor::menu_sub_item(text.clone(), child_items)
        }
    }
}

/// Convert our `GridLengthDesc` to reactor's `style::GridLength` enum.
fn grid_length_desc_to_reactor(desc: GridLengthDesc) -> windows_reactor::GridLength {
    match desc {
        GridLengthDesc::Absolute(v) => windows_reactor::GridLength::Pixel(v),
        GridLengthDesc::Star(v) => windows_reactor::GridLength::Star(v),
        GridLengthDesc::Auto => windows_reactor::GridLength::Auto,
    }
}
