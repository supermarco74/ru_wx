# ru_wx

**Official site:** [ru_wx.easytaskflow.app](https://ru_wx.easytaskflow.app/) ·
**Repository:** [github.com/supermarco74/ru_wx](https://github.com/supermarco74/ru_wx)

**ru_wx** is a cross-platform GUI library written in **pure Rust**, with a [wxWidgets](https://www.wxwidgets.org/)-inspired API and native platform controls.

Unlike Rust bindings to wxWidgets, **ru_wx has no C++ dependency**: it talks directly to the OS APIs (Win32 on Windows) while keeping a familiar API for wxWidgets users.

## Features

- wxWidgets-like API (`Frame`, `Button`, `Grid`, `TreeCtrl`, `AuiToolBar`, ...)
- Native Win32 widgets (HWND-based) on Windows
- No C++ or wxWidgets bindings
- Modular layout: controls, windows, containers, drawing, dialogs, drag-and-drop
- 40+ examples and focused minitests per migrated component
- CI on Windows, macOS, and Linux
- ~90% coverage of the common wxWidgets surface on Win32

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
git tag v0.6.4
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
ru_wx = "0.6.4"
```

Publish manually (after `cargo login`):

```bash
cargo publish
```

Local packaging (same output as CI):

```powershell
cargo build --release --examples
.\scripts\package_github_release.ps1 -Version 0.6.4
# -> dist\ru_wx-0.6.4-win64.zip (+ examples-only and minitests-only zips)
```

### Library bundle for other projects

To ship the **crate source** (for `path` dependencies, vendoring, or offline copy)
into the `release/` folder:

```powershell
.\package_library.bat
# or: .\scripts\package_library_release.ps1 -Version 0.6.4
```

This produces `release/ru_wx-0.6.4/` with the `.crate` archive, extracted `ru_wx/`
source tree, `INTEGRATION.md`, and a `dependency.snippet.toml`. See
[`release/README.md`](release/README.md).

The Common Controls v6 + PerMonitorV2 manifest and the **ru_wx application
icon** (title bar + Explorer `.exe` icon) are embedded automatically by
`build.rs` — no separate manifest or icon tool is required.

## Platform support

| Platform | Backend      | Status                  |
|----------|--------------|-------------------------|
| Windows  | Win32 API    | Active                  |
| Windows  | WinUI3 / XAML| Experimental (feature `winui3_xaml`) |
| macOS    | AppKit       | Planned                 |
| Linux    | GTK          | Planned                 |

## Cargo features

| Feature       | Description                                                        |
|---------------|--------------------------------------------------------------------|
| *(default)*   | Production Win32 backend (HWND-based native controls)              |
| `winui3`      | Parallel WinUI3-style API surface on top of Win32 controls (`ru_wx::platform::winui3`) |
| `winui3_xaml` | Experimental native WinRT/XAML backend (`Microsoft.UI.Xaml` through the `windows-reactor` bridge) |
| `winui3_reactor` | Fluent UI component model on `windows-reactor` (experimental)   |
| `inspector`   | Enables the `debug` inspector module in non-debug builds           |

The crate root (`ru_wx::Button`, `ru_wx::Frame`) always resolves to the
production Win32 implementation; feature-gated backends are parallel types
under `ru_wx::platform::*`, so do not mix them in the same `use` list.

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
- **A local `windows-rs-study` checkout next to this repository's folder**:
  `Cargo.toml` resolves `windows-reactor`, `windows-numerics`,
  `windows-collections` and `windows-future` from `../windows-rs-study/crates/libs/*`,
  so the build fails if that sibling directory is missing.
- Toolchain: the same Rust/Windows toolchain used for the default build.

The backend covers the widget surface listed in
[`PIANO_SVILUPPO_WINUI3_XAML.md`](PIANO_SVILUPPO_WINUI3_XAML.md); it is not
yet a drop-in replacement for Win32 (e.g. per-child layout metrics, focus and
coordinate-driven actions are still Win32-only).

## Requirements

- [Rust](https://rustup.rs/) 1.70+ (edition 2021)
- Windows: MSVC toolchain

## Quick start

Add to your `Cargo.toml`:

```toml
[dependencies]
ru_wx = "0.6.4"
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
