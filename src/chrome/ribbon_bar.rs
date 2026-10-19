//! Office ribbon (`wxRibbonBar`) — tab strip plus per-page panel groups.
//!
//! Each page gets a real `Panel` container. Within each page panel, for
//! each `RibbonPanel` (data struct), buttons are laid out vertically with
//! a group label (`StaticText`) displayed below them.  Vertical 1px
//! separator panels divide groups — Office style has NO bordered boxes.

use std::cell::RefCell;
use std::rc::Rc;

use crate::chrome::ribbon_page::RibbonPage;
#[cfg(target_os = "windows")]
use crate::containers::tab::Tab;
use crate::controls::button::Button;
use crate::controls::static_text::StaticText;
use crate::core::geometry::Colour;
#[cfg(target_os = "macos")]
use crate::core::geometry::Rect;
#[cfg(target_os = "macos")]
use crate::core::widget::Widget;
use crate::core::widget::WidgetRef;
use crate::platform::next_control_id;
#[cfg(target_os = "macos")]
use crate::platform::NativeHandle;
use crate::window::frame::Frame;
use crate::window::panel::Panel;

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOP, SWP_NOACTIVATE};

// ── Windows 11 / Office 365 colour constants ──────────────────────────────────────────

/// Height of the ribbon tab strip (page labels) in pixels.
const RIBBON_TAB_STRIP_H: u32 = 28;

/// Fixed ribbon content area height (for buttons panel).
const RIBBON_CONTENT_H: u32 = 80;

/// Content area background: #F9F9F9 (Windows 11 style)
const CONTENT_BG: (u8, u8, u8) = (249, 249, 249);

/// Vertical separator colour: #E5E5E5 (Windows 11 style, lighter)
const SEPARATOR_COLOR: (u8, u8, u8) = (229, 229, 229);

/// Group label text colour: #404040 (darker for better contrast)
const GROUP_LABEL_COLOR: (u8, u8, u8) = (64, 64, 64);

/// Inset from group edge to first button (px)
const GROUP_INSET: i32 = crate::core::tokens::SPACING_S;

/// Space between the last button and the group label below (px)
const GROUP_LABEL_GAP: i32 = crate::core::tokens::SPACING_XS;

/// Default button height inside a ribbon panel group.
const BUTTON_H: u32 = 26;

/// Vertical gap between consecutive buttons inside a panel group.
const BUTTON_GAP: i32 = 3;

/// Group label height (px)
const GROUP_LABEL_H: u32 = 14;

/// Vertical separator width (px)
const SEPARATOR_W: u32 = 1;

/// Horizontal padding between groups (px)
const GROUP_PAD: i32 = 12;

// ── macOS native tab strip ─────────────────────────────────────────────

#[cfg(target_os = "macos")]
struct RibbonTabStripInner {
    handle: crate::platform::NativeHandle,
    parent_handle: crate::platform::NativeHandle,
    rect: Rect,
    visible: bool,
    enabled: bool,
}

#[cfg(target_os = "macos")]
impl Widget for RibbonTabStripInner {
    fn native_handle(&self) -> isize {
        self.handle as isize
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.rect.x = x;
        self.rect.y = y;
        crate::platform::appkit::ribbon_strip_set_frame(self.parent_handle, self.handle, self.rect);
    }

    fn set_size(&mut self, w: u32, h: u32) {
        self.rect.width = w;
        self.rect.height = h;
        crate::platform::appkit::ribbon_strip_set_frame(self.parent_handle, self.handle, self.rect);
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::macos_widget_set_visible(self.handle, visible);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = visible;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::macos_widget_set_enabled(self.handle, enabled);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = enabled;
    }
}

// ── RibbonBar ──────────────────────────────────────────────────────────

/// Layout state for a ribbon group (per Microsoft Fluent UX).
/// Groups reduce their layout when the window narrows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RibbonLayoutState {
    /// Full-size icons (32x32) with labels below.
    Large,
    /// Medium icons (24x24) with labels below.
    Medium,
    /// Small icons (16x16) without labels.
    Small,
    /// Only the dropdown arrow is visible.
    Collapsed,
}

impl RibbonLayoutState {
    /// Next smaller layout state.
    pub fn smaller(self) -> Self {
        match self {
            RibbonLayoutState::Large => RibbonLayoutState::Medium,
            RibbonLayoutState::Medium => RibbonLayoutState::Small,
            RibbonLayoutState::Small => RibbonLayoutState::Collapsed,
            RibbonLayoutState::Collapsed => RibbonLayoutState::Collapsed,
        }
    }

    /// Icon size for this state.
    pub fn icon_size(self) -> u32 {
        match self {
            RibbonLayoutState::Large => 32,
            RibbonLayoutState::Medium => 24,
            RibbonLayoutState::Small => 16,
            RibbonLayoutState::Collapsed => 0,
        }
    }

    /// Whether to show the label.
    pub fn show_label(self) -> bool {
        matches!(self, RibbonLayoutState::Large | RibbonLayoutState::Medium)
    }
}

/// Per-page group data: for each `RibbonPanel` on a page we keep the
/// list of `Button` widgets (used on non-macOS), the group label,
/// the number of buttons created on macOS, and the current layout state.
#[derive(Clone)]
struct PanelGroup {
    /// Button wrappers (used on Windows and other platforms).
    #[cfg(not(target_os = "macos"))]
    buttons: Vec<Button>,
    /// Number of native 3D buttons created for this group on macOS.
    #[cfg(target_os = "macos")]
    btn_count: usize,
    label: StaticText,
    /// Current layout state for responsive resizing.
    layout_state: RibbonLayoutState,
}

/// Shared callback invoked by every ribbon button (`None` until `on_tool`).
type ToolCallback = Rc<RefCell<Option<Box<dyn FnMut(u16)>>>>;
/// Last computed layout rectangle `(x, y, w, h)` of the ribbon.
type LastLayout = Rc<RefCell<Option<(i32, i32, u32, u32)>>>;

#[derive(Clone)]
pub struct RibbonBar {
    frame: Frame,
    #[cfg(target_os = "windows")]
    tab: Tab,
    #[cfg(target_os = "macos")]
    tab_strip: Rc<RefCell<RibbonTabStripInner>>,
    pages: Rc<RefCell<Vec<RibbonPage>>>,
    page_panels: Rc<RefCell<Vec<Panel>>>,
    page_groups: Rc<RefCell<Vec<Vec<PanelGroup>>>>,
    /// Button handles created via `create_ribbon_button` on macOS.
    /// Format: `(page_index, button_handle)`. Used by `layout_page_groups`.
    #[cfg(target_os = "macos")]
    page_button_handles: Rc<RefCell<Vec<(usize, NativeHandle)>>>,
    active_index: Rc<RefCell<usize>>,
    /// Shared callback invoked by every ribbon button.  The outer
    /// `Option` is `None` until [`on_tool`] is called.
    tool_callback: ToolCallback,
    last_layout: LastLayout,
    /// Current context state (for contextual tabs).
    current_context: Rc<RefCell<crate::chrome::ribbon_page::RibbonContextType>>,
}

impl RibbonBar {
    pub fn new(frame: &Frame) -> Self {
        #[cfg(target_os = "windows")]
        let tab = Tab::new(frame);
        #[cfg(target_os = "macos")]
        let parent_handle = crate::platform::window_handle(frame);
        #[cfg(target_os = "macos")]
        let tab_strip = {
            let handle = crate::platform::appkit::create_ribbon_tab_strip(
                parent_handle,
                Rect::new(0, 0, 400, RIBBON_TAB_STRIP_H),
            );
            Rc::new(RefCell::new(RibbonTabStripInner {
                handle,
                parent_handle,
                rect: Rect::new(0, 0, 400, RIBBON_TAB_STRIP_H),
                visible: true,
                enabled: true,
            }))
        };
        Self {
            frame: frame.clone(),
            #[cfg(target_os = "windows")]
            tab,
            #[cfg(target_os = "macos")]
            tab_strip,
            pages: Rc::new(RefCell::new(Vec::new())),
            page_panels: Rc::new(RefCell::new(Vec::new())),
            page_groups: Rc::new(RefCell::new(Vec::new())),
            active_index: Rc::new(RefCell::new(0)),
            tool_callback: Rc::new(RefCell::new(None)),
            last_layout: Rc::new(RefCell::new(None)),
            #[cfg(target_os = "macos")]
            page_button_handles: Rc::new(RefCell::new(Vec::new())),
            current_context: Rc::new(RefCell::new(
                crate::chrome::ribbon_page::RibbonContextType::None,
            )),
        }
    }

    /// Tab control used as the ribbon page strip (`Home`, `Insert`, …).
    pub fn tab_widget(&self) -> WidgetRef {
        #[cfg(target_os = "windows")]
        {
            self.tab.as_widget_ref()
        }
        #[cfg(target_os = "macos")]
        {
            self.tab_strip.clone()
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            self.frame.as_widget_ref()
        }
    }

    /// Add a new ribbon page and return its zero-based index.
    ///
    /// The page panel is created with a horizontal sizer that arranges
    /// one button group per `RibbonPanel`.  Buttons are wired to the
    /// shared `tool_callback` so all clicks route through
    /// [`on_tool`](crate::RibbonBar::on_tool) /
    /// [`on_ribbon_event`](crate::RibbonBar::on_ribbon_event).
    pub fn add_page(&self, page: RibbonPage) -> usize {
        // Compute page_index before the loop so it can be used inside.
        let page_index = self.pages.borrow().len();

        let page_panel = Panel::new(&self.frame);
        page_panel.set_background_colour(Colour::new(
            CONTENT_BG.0,
            CONTENT_BG.1,
            CONTENT_BG.2,
            255,
        ));

        let mut groups: Vec<PanelGroup> = Vec::new();

        let tool_cb = self.tool_callback.clone();
        let frame_ref = self.frame.clone();

        for panel_data in page.panels() {
            // Create buttons for this group.
            let mut buttons: Vec<Button> = Vec::new();
            #[cfg(target_os = "macos")]
            let mut macos_btn_count: usize = 0;

            for bar in panel_data.bars() {
                for btn in bar.buttons() {
                    let cb = tool_cb.clone();

                    #[cfg(target_os = "macos")]
                    {
                        // Use 3D buttons with optional RGBA image on macOS.
                        let image_data = btn
                            .image_data
                            .as_ref()
                            .map(|(w, h, rgba)| (*w, *h, rgba.as_slice()));
                        let hover_image_data = btn
                            .hover_image_data
                            .as_ref()
                            .map(|(w, h, rgba)| (*w, *h, rgba.as_slice()));
                        let image_position = btn.image_position.to_ns_position();
                        let _btn_handle = crate::platform::appkit::create_ribbon_button(
                            page_panel.hwnd(),
                            image_data,
                            hover_image_data,
                            Some(&btn.label),
                            image_position,
                            Rect::new(0, 0, 70, BUTTON_H),
                        );
                        // Wire click via the stub's dispatch table.
                        crate::platform::appkit::set_button_handler(
                            _btn_handle,
                            self.frame.hwnd(),
                            btn.id,
                        );
                        // Store the button handle for repositioning in layout_page_groups.
                        self.page_button_handles
                            .borrow_mut()
                            .push((page_index, _btn_handle));
                        macos_btn_count += 1;
                    }

                    #[cfg(not(target_os = "macos"))]
                    {
                        let btn_widget = Button::new(&page_panel, &btn.label);
                        let id = btn.id;
                        btn_widget.on_click(&frame_ref, move || {
                            if let Some(ref mut f) = *cb.borrow_mut() {
                                f(id);
                            }
                        });
                        buttons.push(btn_widget);
                    }
                }
            }

            // Create the group label (displayed below buttons).
            let label = StaticText::new(&page_panel, &panel_data.label);
            label.set_foreground_colour(Colour::new(
                GROUP_LABEL_COLOR.0,
                GROUP_LABEL_COLOR.1,
                GROUP_LABEL_COLOR.2,
                255,
            ));

            #[cfg(target_os = "macos")]
            groups.push(PanelGroup {
                btn_count: macos_btn_count,
                label,
                layout_state: RibbonLayoutState::Large,
            });
            #[cfg(not(target_os = "macos"))]
            groups.push(PanelGroup {
                buttons,
                label,
                layout_state: RibbonLayoutState::Large,
            });
        }

        // Add to the tab strip (Win32 tab control / macOS ribbon strip).
        #[cfg(target_os = "windows")]
        self.tab.add_page(&page.label, &page_panel);
        #[cfg(target_os = "macos")]
        {
            crate::platform::appkit::ribbon_strip_add_page(
                self.tab_strip.borrow().handle,
                &page.label,
            );
        }

        self.pages.borrow_mut().push(page);
        self.page_panels.borrow_mut().push(page_panel.clone());
        self.page_groups.borrow_mut().push(groups);

        if page_index == 0 {
            self.show_page(0);
        } else {
            self.set_page_visible(page_index, false);
        }
        #[cfg(target_os = "windows")]
        self.tab.hide_all_pages();

        page_index
    }

    // ── page visibility helpers ──────────────────────────────────────

    fn set_page_visible(&self, index: usize, visible: bool) {
        if let Some(panel) = self.page_panels.borrow().get(index) {
            panel.as_widget_ref().borrow_mut().set_visible(visible);
        }
    }

    fn show_page(&self, index: usize) {
        let count = self.page_panels.borrow().len();
        for i in 0..count {
            self.set_page_visible(i, i == index);
        }
        *self.active_index.borrow_mut() = index;
        #[cfg(target_os = "macos")]
        crate::platform::appkit::ribbon_strip_set_selection(self.tab_strip.borrow().handle, index);

        // Re-layout the active page (the panel may have been hidden
        // and then shown at a potentially different size).
        if let Some((x, y, w, h)) = *self.last_layout.borrow() {
            self.layout_active_page(x, y, w, h);
        }
    }

    // ── layout ───────────────────────────────────────────────────────

    /// Position the ribbon tab strip and the active page panel, then
    /// lay out buttons within each group.
    pub fn layout(&self, x: i32, y: i32, w: u32, h: u32) {
        *self.last_layout.borrow_mut() = Some((x, y, w, h));

        // Position the tab strip at the top.
        self.tab_widget().borrow_mut().set_position(x, y);
        self.tab_widget()
            .borrow_mut()
            .set_size(w, RIBBON_TAB_STRIP_H);

        #[cfg(target_os = "windows")]
        self.tab.hide_all_pages();

        // Position and size the active page panel below the strip.
        self.layout_active_page(x, y, w, h);

        #[cfg(target_os = "windows")]
        {
            let tab_hwnd = self.tab.as_widget_ref().borrow().native_handle()
                as windows_sys::Win32::Foundation::HWND;
            // SAFETY: ribbon tab HWND is live during layout.
            unsafe {
                SetWindowPos(tab_hwnd, HWND_TOP, x, y, w as i32, h as i32, SWP_NOACTIVATE);
            }
        }
    }

    /// Position the active page panel and lay out its button groups.
    fn layout_active_page(&self, x: i32, y: i32, w: u32, _h: u32) {
        let content_y = y + RIBBON_TAB_STRIP_H as i32;
        // Use fixed height for ribbon content area
        let content_h = RIBBON_CONTENT_H;
        let active = *self.active_index.borrow();

        if let Some(panel) = self.page_panels.borrow().get(active) {
            panel.set_position(x, content_y);
            panel.set_size(w, content_h);

            // Now position buttons and labels within each group.
            self.layout_page_groups(active, x, content_y, w, content_h);
        }
    }

    /// For the given page, lay out each group's buttons horizontally
    /// with the group label below. Groups are arranged horizontally
    /// with vertical separators between them.
    fn layout_page_groups(
        &self,
        page_index: usize,
        page_x: i32,
        page_y: i32,
        page_w: u32,
        _page_h: u32,
    ) {
        let groups = self.page_groups.borrow();
        if let Some(page_groups) = groups.get(page_index) {
            if page_groups.is_empty() {
                return;
            }

            // On macOS: collect ALL button handles for this page.
            #[cfg(target_os = "macos")]
            let page_handles: Vec<NativeHandle> = self
                .page_button_handles
                .borrow()
                .iter()
                .filter(|(pi, _)| *pi == page_index)
                .map(|(_, h)| *h)
                .collect();

            #[cfg(target_os = "macos")]
            eprintln!("[DEBUG] layout_page_groups: page={}, groups={}, handles={}, page_x={}, page_y={}, page_w={}",
                page_index, page_groups.len(), page_handles.len(), page_x, page_y, page_w);

            let count = page_groups.len();
            // Calculate available width for groups (minus separators)
            let total_sep_w = SEPARATOR_W as i32 * (count - 1) as i32;
            let total_pad = GROUP_PAD * (count + 1) as i32;
            let available_w = (page_w as i32 - total_sep_w - total_pad).max(0);
            let group_w = (available_w / count as i32).max(80);

            let mut x_pos = page_x + GROUP_PAD;
            // Running index into page_handles (shared across groups).
            #[cfg(target_os = "macos")]
            let mut handle_idx: usize = 0;

            for (gi, group) in page_groups.iter().enumerate() {
                let group_inner_x = x_pos + GROUP_INSET;
                let group_inner_y = page_y + GROUP_INSET;

                // Position buttons horizontally within the group.
                #[cfg(target_os = "macos")]
                {
                    let btn_count = group.btn_count;
                    // Auto-size buttons: divide available group width by button count
                    let btn_w = if btn_count > 0 {
                        (((group_w - GROUP_INSET * 2) as i32 - (btn_count as i32 - 1) * BUTTON_GAP)
                            / btn_count as i32)
                            .max(40) as u32
                    } else {
                        50
                    };
                    let parent_handle = self
                        .page_panels
                        .borrow()
                        .get(page_index)
                        .map(|p| p.hwnd())
                        .unwrap_or(0);
                    let mut bx = group_inner_x;
                    for _ in 0..btn_count {
                        if handle_idx < page_handles.len() {
                            crate::platform::appkit::ribbon_button_set_frame(
                                parent_handle,
                                page_handles[handle_idx],
                                Rect::new(bx, group_inner_y, btn_w, BUTTON_H),
                            );
                            handle_idx += 1;
                        }
                        bx += btn_w as i32 + BUTTON_GAP;
                    }
                }

                #[cfg(not(target_os = "macos"))]
                {
                    let btn_w = if !group.buttons.is_empty() {
                        (((group_w - GROUP_INSET * 2)
                            - (group.buttons.len() as i32 - 1) * BUTTON_GAP)
                            / group.buttons.len() as i32)
                            .max(40) as u32
                    } else {
                        50
                    };
                    let mut bx = group_inner_x;
                    for btn in &group.buttons {
                        btn.as_widget_ref()
                            .borrow_mut()
                            .set_position(bx, group_inner_y);
                        btn.as_widget_ref().borrow_mut().set_size(btn_w, BUTTON_H);
                        bx += btn_w as i32 + BUTTON_GAP;
                    }
                }

                // Position group label below the buttons (centered in group)
                let label_y = page_y + BUTTON_H as i32 + GROUP_INSET + GROUP_LABEL_GAP;
                let label_w = (group_w - GROUP_INSET * 2).max(40) as u32;
                group
                    .label
                    .as_widget_ref()
                    .borrow_mut()
                    .set_position(group_inner_x, label_y);
                group
                    .label
                    .as_widget_ref()
                    .borrow_mut()
                    .set_size(label_w, GROUP_LABEL_H);

                x_pos += group_w + GROUP_PAD;

                // Draw vertical separator (except after the last group)
                if gi < count - 1 {
                    x_pos += SEPARATOR_W as i32;
                }
            }
        }
    }

    /// Number of pages in the ribbon.
    pub fn page_count(&self) -> usize {
        self.pages.borrow().len()
    }

    /// Add a tool to the *active* page (legacy API).
    ///
    /// In the new panel-group architecture this creates a standalone
    /// button positioned at a fixed location.  Prefer populating pages
    /// via `RibbonPage::add_panel` + `RibbonPanel::add_bar` before
    /// calling `RibbonBar::add_page`.
    pub fn add_tool(&self, label: &str) -> u16 {
        let id = next_control_id();
        let active = *self.active_index.borrow();
        if let Some(panel) = self.page_panels.borrow().get(active) {
            let btn = Button::new(panel, label);
            let cb = self.tool_callback.clone();
            btn.on_click(&self.frame, move || {
                if let Some(ref mut f) = *cb.borrow_mut() {
                    f(id);
                }
            });
            // Position the button at a default location.
            btn.as_widget_ref().borrow_mut().set_position(10, 10);
            btn.as_widget_ref().borrow_mut().set_size(100, BUTTON_H);
        }
        id
    }

    /// Finalise the ribbon bar layout and hook up tab-selection events.
    pub fn realize(&self) {
        if !self.pages.borrow().is_empty() {
            self.show_page(0);
        }
        let outer = self.clone();
        #[cfg(target_os = "windows")]
        self.tab.on_selection_change(&self.frame, move |index| {
            outer.show_page(index);
            outer.tab.hide_all_pages();
        });
        #[cfg(target_os = "macos")]
        {
            let strip_handle = self.tab_strip.borrow().handle;
            crate::platform::appkit::ribbon_strip_set_selection_handler(
                strip_handle,
                Box::new(move |index| {
                    outer.show_page(index);
                }),
            );
            // Start the hover timer to enable hover effect tracking
            crate::platform::appkit::start_hover_timer();
        }
    }

    /// Register a click handler for all ribbon buttons.
    ///
    /// The callback receives the tool ID of the clicked button.
    pub fn on_tool<F: FnMut(u16) + 'static>(&self, _frame: &Frame, f: F) {
        *self.tool_callback.borrow_mut() = Some(Box::new(f));
    }

    /// Tool click with [`RibbonBarEvent`](crate::RibbonBarEvent) payload (`wxRibbonBarEvent`).
    pub fn on_ribbon_event<F: FnMut(&crate::chrome::ribbon_bar_event::RibbonBarEvent) + 'static>(
        &self,
        frame: &Frame,
        mut f: F,
    ) {
        self.on_tool(frame, move |id| {
            f(&crate::chrome::ribbon_bar_event::RibbonBarEvent::new(
                crate::chrome::ribbon_bar_event::RibbonBarEventKind::ToolClick,
                id,
            ));
        });
    }

    /// Update the ribbon context state (per Microsoft Fluent UX).
    ///
    /// When the application state changes (e.g., user selects an image),
    /// call this method to show/hide contextual tabs and enable/disable
    /// context-specific buttons.
    ///
    /// Example:
    /// ```ignore
    /// ribbon.update_context(RibbonContextType::ImageSelected);
    /// ```
    pub fn update_context(&self, context: crate::chrome::ribbon_page::RibbonContextType) {
        *self.current_context.borrow_mut() = context;

        // Show/hide contextual tabs based on context.
        for (i, page) in self.pages.borrow().iter().enumerate() {
            if page.is_contextual {
                let visible = page.context_type == context;
                self.set_page_visible(i, visible);
            }
        }

        // If a contextual tab is now active, show it.
        if context != crate::chrome::ribbon_page::RibbonContextType::None {
            for (i, page) in self.pages.borrow().iter().enumerate() {
                if page.is_contextual && page.context_type == context {
                    self.show_page(i);
                    break;
                }
            }
        } else {
            // Show the first non-contextual tab.
            for (i, page) in self.pages.borrow().iter().enumerate() {
                if !page.is_contextual {
                    self.show_page(i);
                    break;
                }
            }
        }
    }

    /// Resize the ribbon groups based on available width (per Microsoft Fluent UX).
    ///
    /// This implements the `resizeRibbon` / `reduceLayout` behavior:
    /// when the window narrows, groups reduce their layout state from
    /// Large → Medium → Small → Collapsed, starting from the rightmost group.
    ///
    /// Call this method when the window is resized.
    pub fn resize_ribbon(&self, available_width: u32) {
        let active = *self.active_index.borrow();
        let mut groups = self.page_groups.borrow_mut();

        if let Some(page_groups) = groups.get_mut(active) {
            // Calculate minimum width needed if all groups are Large.
            let group_count = page_groups.len() as i32;
            let total_sep_w = SEPARATOR_W as i32 * (group_count - 1).max(0);
            let total_pad = GROUP_PAD * (group_count + 1);
            let min_group_w = 80;

            // Calculate how much space we have vs. need.
            let min_needed = group_count * min_group_w + total_sep_w + total_pad;

            if (available_width as i32) < min_needed {
                // Not enough space: reduce layout states from right to left.
                let deficit = min_needed - (available_width as i32);
                let mut reduction = deficit;

                // Walk groups from right to left, reducing layout state.
                for group in page_groups.iter_mut().rev() {
                    if reduction <= 0 {
                        break;
                    }
                    // Reduce this group's layout state.
                    while group.layout_state != RibbonLayoutState::Collapsed && reduction > 0 {
                        group.layout_state = group.layout_state.smaller();
                        // Estimate width reduction from this change.
                        let prev_icon = group.layout_state.smaller().icon_size();
                        let curr_icon = group.layout_state.icon_size();
                        reduction -= (prev_icon - curr_icon) as i32 * 2;
                    }
                }
            } else {
                // Enough space: restore all groups to Large.
                for group in page_groups.iter_mut() {
                    group.layout_state = RibbonLayoutState::Large;
                }
            }
        }

        // Re-layout with new states.
        if let Some((x, y, w, h)) = *self.last_layout.borrow() {
            self.layout_active_page(x, y, w, h);
        }
    }

    /// Get the current layout state for a group on the active page.
    pub fn group_layout_state(&self, group_index: usize) -> Option<RibbonLayoutState> {
        let active = *self.active_index.borrow();
        self.page_groups
            .borrow()
            .get(active)
            .and_then(|g| g.get(group_index))
            .map(|g| g.layout_state)
    }
}
