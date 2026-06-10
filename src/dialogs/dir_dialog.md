# dir_dialog.rs

Directory-picker dialog (`wxDirDialog`).

## Purpose
Modal dialog that lets the user pick a directory. On Windows it wraps the standard `SHBrowseForFolderW` shell API.

## Key Types
- `DirDialog` — owns the dialog state and the picked path.

## Key Functions/Methods
- `DirDialog::new<W: Window>(parent: &W, message: &str, default_path: Option<&str>, style: DirDialogStyle)` — constructs the dialog. `style` is a bit-set of `DirDialogStyle` (e.g. `DIR_DIALOG_DEFAULT_STYLE`, `DIR_DIALOG_CHANGE_DIR`).
- `DirDialog::show_modal() -> Option<PathBuf>` — runs the dialog. Returns `Some(path)` on OK, `None` on cancel.
- `message`, `default_path`, `style` — query / set the corresponding field.

## Win32 Notes
- Built on top of the `SHBrowseForFolderW` shell API.
- Uses a `BROWSEINFOW` struct on the stack; the picked path is returned via `SHGetPathFromIDListW`.
- The dialog runs a modal `PeekMessageW` loop on the calling thread.
- All FFI calls wrapped in `// SAFETY:` comments documenting validated arguments.

## Quick start

```rust
use ru_wx::prelude::*;

if let Some(dir) = DirDialog::new(&frame, "Pick a folder", None, DIR_DIALOG_DEFAULT_STYLE)
    .show_modal()
{
    println!("Picked: {}", dir.display());
}

// Pre-select a starting path:
let dlg = DirDialog::new(&frame, "Pick a folder", Some("C:\\Users"), DIR_DIALOG_DEFAULT_STYLE);
let _ = dlg.show_modal();
```

## See Also
- [`file_dialog.rs`](file_dialog.md) — sibling dialog for picking files (uses `GetOpenFileNameW` / `GetSaveFileNameW`).
- [`message_dialog.rs`](message_dialog.md) — sibling modal dialog pattern.
