# ru_wx

**Official site:** [ru_wx.easytaskflow.app](https://ru_wx.easytaskflow.app/) ·
**Repository:** [github.com/supermarco74/ru_wx](https://github.com/supermarco74/ru_wx)

**ru_wx** is a cross-platform GUI library written in **pure Rust**, with a [wxWidgets](https://www.wxwidgets.org/)-inspired API and native platform controls.

Unlike Rust bindings to wxWidgets, **ru_wx has no C++ dependency**: it talks directly to the OS APIs — Win32 on Windows, AppKit on macOS, GTK4 on Linux — while keeping a familiar API for wxWidgets users.

## Features

- wxWidgets-like API (`Frame`, `Button`, `Grid`, `TreeCtrl`, `AuiToolBar`, ...)
- Native widgets on all three platforms: HWND-based Win32, `NSView`/`NSWindow` AppKit, `gtk4::Widget` GTK4
- No C++ or wxWidgets bindings, no FFI shim
- Modular layout: `core`, `controls`, `containers`, `window`, `dialogs`, `chrome`, `dc`, `data`, `net`, `io`, `dnd`, `adv`, `debug`
- 135 examples plus focused minitests per component
- CI on Windows, macOS, and Linux
- 475 unit tests + 25 integration tests

## GitHub Releases

Pre-built **Windows x64** binaries (examples and minitests) are published on
[GitHub Releases](https://github.com/supermarco74/ru_wx/releases).

Download `ru_wx-*-win64.zip`, extract it, and run any `.exe` under
`examples_win32/` (e.g. `showcase_all.exe`, `music_player.exe`). No extra runtime
is required.

### Publish a new release (maintainers)

1. Bump `version` in `Cargo.toml` and commit.
2. Tag and push:

```bash
git tag v0.7.1
git push origin main --tags
```

The [Release workflow](.github/workflows/release.yml) builds all examples
(with the Win11 manifest linked from `build.rs`), packages three `.zip`
archives, and creates the GitHub Release automatically. On tag push it also runs
`cargo publish` to [crates.io](https://crates.io/crates/ru_wx) (requires the
`CARGO_REGISTRY_TOKEN` repository secret).

### crates.io

Documentation and guides: [ru_wx.easytaskflow.app](https://ru_wx.easytaskflow.app/)

```toml
[dependencies]
ru_wx = "0.7.1"
```

Publish manually (after `cargo login`):

```bash
cargo publish
```

Local packaging (same output as CI):

```powershell
cargo build --release --examples
.\scripts\package_github_release.ps1 -Version 0.7.1
# -> dist\ru_wx-0.7.1-win64.zip (+ examples-only and minitests-only zips)
```

### Library bundle for other projects

To ship the **crate source** (for `path` dependencies, vendoring, or offline copy)
into the `release/` folder:

```powershell
.\package_library.bat
# or: .\scripts\package_library_release.ps1 -Version 0.7.1
```

This produces `release/ru_wx-0.7.1/` with the `.crate` archive, extracted `ru_wx/`
source tree, `INTEGRATION.md`, and a `dependency.snippet.toml`. See
[`release/README.md`](release/README.md).

The Common Controls v6 + PerMonitorV2 manifest and the **ru_wx application
icon** (title bar + Explorer `.exe` icon) are embedded automatically by
`build.rs` — no separate manifest or icon tool is required.

## Platform support

| Platform | Backend    | Status                                    |
|----------|------------|-------------------------------------------|
| Windows  | Win32 API  | Active — reference implementation        |
| Windows  | WinUI3     | Experimental (`winui3`): Win32-style surface |
| Windows  | WinUI3/XAML| Experimental (`winui3_xaml`): real `Microsoft.UI.Xaml` |
| macOS    | AppKit     | Active — 20.5k lines across 39 modules   |
| Linux    | GTK4       | Active — 5.3k lines, 27 controls ported  |

### Component coverage

| Component group | Windows | macOS | Linux |
|-----------------|---------|-------|-------|
| Frame / Panel / Button / StaticText / TextCtrl | ✅ | ✅ | ✅ |
| CheckBox / RadioButton / ComboBox / ListBox / Choice | ✅ | ✅ | ✅ |
| Slider / Gauge / SpinCtrl / ToggleButton | ✅ | ✅ | ✅ |
| ListCtrl / TreeCtrl / Grid | ✅ | ✅ | ✅ |
| MenuBar / ToolBar / AuiManager / AuiToolBar | ✅ | ✅ | ✅ |
| Notebook (AuiNotebook) | ✅ | ✅ | ✅ |
| MessageBox / TextDialog / DirDialog / FontDialog | ✅ | ✅ | ⚠️ MessageBox only |
| Timer / Accelerator / ToolTip | ✅ | ✅ | ✅ |
| Splitter / HeaderCtrl | ✅ | ✅ | ✅ |
| StatusBar | ✅ | ✅ | ❌ |
| Clipboard / Caret | ✅ | ✅ | ❌ |
| ScrolledWindow | ✅ | ❌ | ❌ |
| Dialog / TextDialog / FileDialog / ColourDialog | ✅ | ⚠️ partial | ❌ |
| PopupMenu / ContextMenu | ✅ | ✅ | ❌ |
| BitmapButton / HyperlinkCtrl / DatePicker | ✅ | ✅ | ❌ |
| DC painting / Icon / Bitmap | ✅ | ✅ | ❌ |
| Drag & Drop | ✅ | ✅ | ❌ |

✅ native implementation · ⚠️ partial · ❌ falls back to a stub that degrades
gracefully (no panic, inert widget).

The Linux backend targets **GTK 4.10** (via the `v4_10` feature, required by
`PopoverMenuBar`). `ListCtrl` and `Grid` are built on `TreeView`/`ListStore`,
deprecated in GTK 4.10 in favour of `ColumnView`/`GridView`; the module opts
into them explicitly with `#![allow(deprecated)]`.

macOS uses `objc2` / `objc2-app-kit` and calls AppKit directly — no Objective-C
shim layer. On Linux the backend degrades to `stub_backend` when GTK cannot
initialise (headless CI), so the library stays testable without a display.

## Cargo features

| Feature       | Description                                                        |
|---------------|--------------------------------------------------------------------|
| *(default)*   | Production backend for the host platform (Win32 / AppKit / GTK4)   |
| `winui3`      | Parallel WinUI3-style API surface on top of Win32 controls (`ru_wx::platform::winui3`) |
| `winui3_xaml` | Experimental native WinRT/XAML backend (`Microsoft.UI.Xaml` through the `windows-reactor` bridge) |
| `winui3_reactor` | Fluent UI component model on `windows-reactor` (experimental)   |
| `inspector`   | Enables the `debug` inspector module in non-debug builds           |
| `data-sqlite` | SQLite driver for the `data` layer                                 |
| `data-mysql`  | MySQL / MariaDB driver for the `data` layer                       |
| `data-db`     | Both database drivers at once                                      |

The crate root (`ru_wx::Button`, `ru_wx::Frame`) always resolves to the
production backend for the platform you are compiling for; feature-gated
backends are parallel types under `ru_wx::platform::*`, so do not mix them in
the same `use` list.

## WinUI3 / WinAppSDK backend (experimental)

Enable the backend with the `winui3_xaml` feature:

```bash
cargo build --features winui3_xaml
cargo test  --lib --features winui3_xaml
```

```rust
use ru_wx::platform::winui3_xaml::*;

let app = XamlApp::new();
let frame = XamlFrame::builder()
    .with_title("My App")
    .with_size(600, 400)
    .build();
app.run(frame)?;
```

Requirements:

- **Windows App SDK runtime, 2.0 channel (>= 2.0.1.0)**. Startup calls
  `MddBootstrapInitialize2`, which loads
  `microsoft.windowsappruntime.bootstrap.dll` from the installed runtime —
  install it from <https://aka.ms/windowsappsdk/> (any
  `Microsoft.WindowsAppRuntime.2.x` package satisfies the check). Without it
  the backend cannot initialize.
- **A local `windows-rs-study` checkout next to this repository's folder**.
  The backend depends on the unpublished crates `windows-reactor`,
  `windows-numerics`, `windows-collections` and `windows-future`, which are
  resolved as path dependencies from `../windows-rs-study/crates/libs/*`.
  They are therefore **not declared in the published `Cargo.toml`**: Cargo
  resolves `path` dependencies even when they are `optional`, so declaring them
  would break `cargo build` for anyone cloning the crate without that sibling
  checkout. To build this backend, clone `windows-rs-study` alongside this
  repository and restore the four entries under
  `[target.'cfg(target_os = "windows")'.dependencies]` (see the comment in
  `Cargo.toml`). The sources live in `src/platform/winui3_xaml/`.
- Toolchain: the same Rust/Windows toolchain used for the default build.

The backend covers the widget surface listed in
[`PIANO_SVILUPPO_WINUI3_XAML.md`](PIANO_SVILUPPO_WINUI3_XAML.md); it is not
yet a drop-in replacement for Win32 (e.g. per-child layout metrics, focus and
coordinate-driven actions are still Win32-only).

## Requirements

- [Rust](https://rustup.rs/) 1.70+ (edition 2021)
- Windows: MSVC toolchain
- macOS: Xcode command line tools (`objc2` builds against the AppKit SDK)
- Linux: GTK **4.10+** and its development headers, plus `libx11` for the
  inspector (`gdk4-x11`). On Debian/Ubuntu:
  `sudo apt install libgtk-4-dev libx11-dev`

## Quick start

Add to your `Cargo.toml`:

```toml
[dependencies]
ru_wx = "0.7.1"
```

Minimal example:

```rust
use ru_wx::*;

let app = App::new();
let frame = Frame::builder()
    .with_title("Hello ru_wx")
    .with_size(400, 300)
    .build();

let button = Button::new(&frame, "Click me!");
button.on_click(&frame, || {
    println!("Clicked!");
});

app.run(frame);
```

## Examples

```bash
# Window with a button
cargo run --example window_with_button

# Dockable toolbar (wxAuiToolBar-style)
cargo run --example aui_toolbar_demo

# Full control showcase
cargo run --example showcase_all

# Focused minitest for a single component
cargo run --example mt_button
```

## Migration status

Historical migration and upgrade documents are archived in
[`vecchie_elaborazioni/`](../vecchie_elaborazioni/):

- `MIGRATION_STATUS.md` - component-by-component matrix
- `MIGRATION_SPRINT.md` - migration sprint (19 rounds, 135 components)
- `upgrade.md` / `upgrade_report_*.md` - per-version logs and reports

## Development

```bash
cargo build
cargo test --lib
cargo clippy --all-targets -- -D warnings
cargo doc --no-deps
```

## License

This project is licensed under the [GNU General Public License v3.0 or later](LICENSE) (GPL-3.0-or-later).
