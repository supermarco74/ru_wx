# mt_tab.rs

Minitest for [`Tab`](file:///f:/code/ru_wx/ru_wx/src/tab.rs) — a notebook with three pages, each populated with buttons that report into a shared status bar.

**Run:** `cargo run --example mt_tab`

## Purpose
1. Creating a tab control as a child of the frame
2. Building `Panel` pages and adding child widgets to them
3. Adding multiple pages to the notebook with `add_page`
4. Reacting to page-selection changes via `on_selection_change`
5. Sharing a single `StatusBar` across all pages (status messages from any page land in the same field 0)

## Top-level flow
1. Frame 540×360.
2. 1-field `StatusBar` with hint text `"Switch tabs and click any button."`.
3. `Tab::new(&frame)` — empty notebook.
4. **Page 1 — Alpha**
   - `Panel::new(&frame)` (note: parent is the frame, not the notebook; the notebook hosts the panel as a child window)
   - `StaticText` label "Page 1 — alpha"
   - `Button` "Alpha 1" → `set_status_text("Page1 → Alpha 1", 0)`
   - `Button` "Alpha 2" → `set_status_text("Page1 → Alpha 2", 0)`
   - Vertical `BoxSizer`; `page1.set_sizer(sz1)`
5. **Page 2 — Beta**
   - `Panel` + label "Page 2 — beta" + 3 buttons "Beta 1/2/3"
   - Each button writes `Page2 → Beta N` into the shared status field
   - Vertical sizer; `page2.set_sizer(sz2)`
6. **Page 3 — Gamma**
   - `Panel` + label "Page 3 — gamma" + 2 buttons "Gamma A/B"
   - Each button writes `Page3 → Gamma N` into the shared status field
   - Vertical sizer; `page3.set_sizer(sz3)`
7. `notebook.add_page("Alpha", &page1)`, `add_page("Beta", &page2)`, `add_page("Gamma", &page3)`.
8. `notebook.on_selection_change(&frame, |idx| set_status_text(&format!("Selected tab: {idx}"), 0))` — the callback fires when the user clicks a tab header.
9. `app.run(frame)`.

> **Parent of the page `Panel` is the frame, not the notebook.** The notebook is a *host* control, not a parent window — it manages the page HWNDs internally, but the page is parented to the frame so its own `WM_SIZE` / sizer logic still runs.

## Key APIs exercised
- [`Tab::new(&frame)`](file:///f:/code/ru_wx/ru_wx/src/tab.rs)
- `Tab::add_page(&str title, &Panel page)`
- `Tab::on_selection_change(&frame, FnMut(usize))`
- [`Panel::new(parent)`](file:///f:/code/ru_wx/ru_wx/src/panel.rs)
- `Panel::set_sizer(sizer)`
- [`Button::new(parent, &str)`](file:///f:/code/ru_wx/ru_wx/src/button.rs)
- `Button::on_click(&frame, FnClosure)`
- [`StatusBar::new(&frame, n)`](file:///f:/code/ru_wx/ru_wx/src/status_bar.rs)
- `StatusBar::set_status_text(&str, field_idx)`
- [`BoxSizer::vertical()`](file:///f:/code/ru_wx/ru_wx/src/sizer.rs)
- [`Frame::builder()…build()`](file:///f:/code/ru_wx/ru_wx/src/frame.rs)

## Patterns worth noting
- **One status bar, many pages** — the status bar lives on the frame, not on any page, so all pages can write into the same field 0 by cloning `status` and capturing it in their button closures. This is the standard "shared sink" pattern in `ru_wx`.
- **Pages are full Panels** — each `Panel` gets its own sizer, its own children, and its own paint/erase background. There is no need to call any notebook-specific "add child" method; `add_page` does the host-side wiring automatically.
- **`on_selection_change` is a `FnMut(usize)`** — it is called with the **newly selected page index** (0-based) every time the user clicks a tab. The closure captures `status` by `move` and writes the formatted message into the bar.
- **The "parent is the frame" rule for pages** — passing `&page1` to a sizer on `page1` would be a no-op; passing `&frame` to `add_page` would not register the page with the notebook. The asymmetry is intentional: the notebook manages the page as a *tab*, the frame parents it as a *window*.

## Win32 notes
- `Tab` is a native `SysTabControl32` (`WC_TABCONTROL`) with `TCS_TABS | TCS_MULTILINE`-style behavior.
- `add_page` calls `TCM_INSERTITEMW` with a `TCITEMW` whose `pszText` is the title (wide-encoded) and whose `lParam` is the page's HWND; the page HWND is also `ShowWindow`'d / `MoveWindow`'d to match the tab control's display area.
- `on_selection_change` registers a `WM_NOTIFY` / `TCN_SELCHANGE` filter on the parent frame and dispatches the new selection index back to the closure.
- Each page is a child `BUTTON`/`STATIC`-class window; its own `WM_ERASEBKGND` and `WM_PAINT` flow normally, including the sizer-driven layout.

## Cross-references
- [`tab.md`](file:///f:/code/ru_wx/ru_wx/src/tab.md)
- [`panel.md`](file:///f:/code/ru_wx/ru_wx/src/panel.md)
- [`button.md`](file:///f:/code/ru_wx/ru_wx/src/button.md)
- [`status_bar.md`](file:///f:/code/ru_wx/ru_wx/src/status_bar.md)
- [`sizer.md`](file:///f:/code/ru_wx/ru_wx/src/sizer.md)
- [`frame.md`](file:///f:/code/ru_wx/ru_wx/src/frame.md)
