# file_dialog.rs

`wxFileDialog` analog. Wraps Win32 `GetOpenFileNameW` / `GetSaveFileNameW` to present the standard Windows Open / Save As dialog. Supports single-select, multi-select, wildcards, and default directory / filename.

## Purpose
The canonical "let the user pick a file" surface. The caller constructs a `FileDialog` with a `&Frame` parent and a `FileDialogStyle` (`Open` or `Save`), configures it (title, wildcard, default path, multi-select), and calls `show_modal` (or `show_modal_multi`).

## Key Types
- `FileDialogStyle` — `Open` or `Save`. Determines which `OFN_*` flags are set and which Win32 entry point is used.
- `FileDialog` — public struct. Fields: `parent_hwnd: HWND`, `style: FileDialogStyle`, `title`, `default_dir`, `default_file`, `wildcard`, `multi_select: bool`.
- `pub(crate) fn new_for_test(multi_select: bool) -> Self` — Test-only constructor that uses a null `HWND` and a fixed wildcard so unit tests can exercise the buffer parser / wildcard converter without an actual GUI.

## Key Methods
- `FileDialog::new(frame: &Frame, style: FileDialogStyle) -> Self` — Build a dialog. Defaults: empty title, no default directory, no default file, no wildcard, single-select.
- `set_title(&mut self, title: &str)`, `set_wildcard(&mut self, wildcard: &str)`, `set_directory(&mut self, dir: &str)`, `set_filename(&mut self, name: &str)`.
- `set_multi_select(&mut self, enabled: bool) -> &mut Self` — Builder-style (returns `&mut Self` for chaining). Enables `OFN_ALLOWMULTISELECT` and routes to `show_modal_multi`.
- `is_multi_select(&self) -> bool`.
- `show_modal(&mut self) -> Option<String>` — Single-select. Returns `None` on cancel. Internally allocates a 4096-u16 buffer.
- `show_modal_multi(&mut self) -> Vec<String>` — Multi-select. Returns the chosen paths, or an empty `Vec` on cancel. Internally allocates a 32 KiB buffer.
- `pub(crate) fn parse_multiselect_buffer(buf: &[u16], file_offset: usize) -> Vec<String>` — Pure function. Decodes the Win32 multi-select buffer (directory in the first slot, filenames in the rest) into Rust `String`s. The `file_offset` argument is a legacy parameter; the buffer is decoded by scanning for null terminators.
- `fn wildcard_to_win32_filter(&self) -> Vec<u16>` — Converts the wxWidgets-style `"Description|*.ext|Description2|*.ext2"` string into the Win32 `OPENFILENAMEW::lpstrFilter` format `"Description\0*.ext\0Description2\0*.ext2\0\0"`. The trailing double-null is the Win32 sentinel for "end of filter list".

## Win32 Notes
- Entry points: `GetOpenFileNameW` (for `FileDialogStyle::Open`) and `GetSaveFileNameW` (for `FileDialogStyle::Save`).
- `OPENFILENAMEW` struct fields used: `lStructSize`, `hwndOwner`, `lpstrFilter`, `lpstrFile` (the 4096 or 32768 wide buffer), `nMaxFile`, `lpstrInitialDir`, `lpstrTitle`, `Flags`.
- `OFN_*` flags: `OFN_FILEMUSTEXIST = 0x00001000`, `OFN_PATHMUSTEXIST = 0x00000800`, `OFN_NOCHANGEDIR = 0x00000008`, `OFN_ALLOWMULTISELECT = 0x00000200`, `OFN_EXPLORER = 0x00080000`.
- The `multi_select` field adds `OFN_ALLOWMULTISELECT | OFN_EXPLORER` to the flags, and the parser uses the `lpstrFile` buffer format: directory path (without trailing backslash for the root case, with trailing backslash otherwise), then a sequence of file names separated by null terminators, ending in a double-null.
- The wildcard filter is a **double-null-terminated** `Vec<u16>`; the function builds it with the right padding to satisfy `GetOpenFileNameW`.

## Tests
- Buffer parsing: empty buffer, all-zero buffer, single file, two files, three files, trailing backslash, file offset ignored, absolute paths, UNC paths, empty filenames, forward slashes, mixed relative/absolute.
- Wildcard conversion: empty input, single pair, two pairs, odd number of pipe-separated parts.
- State: `multi_select` round-trip via `set_multi_select` / `is_multi_select`.
- Constants: `OFN_FILEMUSTEXIST`, `OFN_PATHMUSTEXIST`, `OFN_NOCHANGEDIR`, `OFN_ALLOWMULTISELECT`, `OFN_EXPLORER` values match the documented headers; the low-12 bits of `OFN_EXPLORER` are zero.
- Enum: `FileDialogStyle::Open != FileDialogStyle::Save`; `Debug` and `Clone` are derivable.

## Quick start

```rust
use ru_wx::prelude::*;

// frame is the owning Frame.

// --- Open a single file ---------------------------------------------
let mut dlg = FileDialog::new(&frame, FileDialogStyle::Open);
dlg.set_title("Open a text file");
dlg.set_wildcard("Text files (*.txt)|*.txt|All files (*.*)|*.*");
dlg.set_directory("C:/Users/me/Documents");
if let Some(path) = dlg.show_modal() {
    println!("Picked: {path}");
}

// --- Save As --------------------------------------------------------
let mut save = FileDialog::new(&frame, FileDialogStyle::Save);
save.set_wildcard("Rust source (*.rs)|*.rs");
save.set_filename("untitled.rs");
if let Some(path) = save.show_modal() {
    std::fs::write(path, b"// hello").unwrap();
}

// --- Open multiple files -------------------------------------------
let mut multi = FileDialog::new(&frame, FileDialogStyle::Open);
multi.set_multi_select(true);                // builder-style chain
multi.set_wildcard("Images (*.png;*.jpg)|*.png;*.jpg");
let paths = multi.show_modal_multi();
for p in paths { println!("{p}"); }
```

The wildcard is the wxWidgets `"Description|*.ext|Description2|*.ext2"`
form. `show_modal` returns `None` on cancel; `show_modal_multi` returns
an empty `Vec` on cancel.

## See Also
- [`dialog.rs`](./dialog.md) — generic dialog for non-file use cases
- [`message_box.rs`](./message_box.md) — quick OK/Cancel prompt
- [`drop_target.rs`](./drop_target.md) — drag-and-drop file paths is the alternative entry point for files
