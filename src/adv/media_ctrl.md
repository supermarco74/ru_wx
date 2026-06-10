# media_ctrl.rs

`wxMediaCtrl` analog — an audio / video playback control backed by the Windows MCI (Media Control Interface) string API.

## Purpose

`MediaCtrl` plays a media file (audio or video) using `mciSendStringW`. It exposes a coarse-grained state machine (`Stopped` / `Paused` / `Playing`) plus a few query methods (`position_ms`, `length_ms`, `seek_ms`).

## Supported formats

MCI is the simplest cross-format playback API on Windows and works for:

* audio: WAV, MP3 (with default codecs), MIDI,
* video: AVI, MPG (with default codecs),
* other: any format for which an `mciSendCommand`-style device is registered.

## Key Types

- **`MediaCtrl`** — public struct, wraps `Rc<RefCell<MediaCtrlInner>>`. Cloneable.
- **`MediaState`** — public enum: `Stopped` (no file loaded OR file is stopped), `Paused`, `Playing`.
- `MciState` (private) — finer-grained state (`Stopped` / `Playing` / `Paused`).
- `MediaCtrlInner` (private) — `alias`, `has_media`, `state`, `last_error`.

## Constructors

- `MediaCtrl::new<W: Window>(parent: &W) -> Self` — empty control in `MediaState::Stopped`.

## Key Methods

- `load(&self, path: &Path) -> Result<(), String>` — load a media file. Closes any previously-loaded file first. On success the control is in `MediaState::Stopped`. On failure returns the MCI error description.
- `play(&self) -> Result<(), String>` — start or resume playback. Sends `resume <alias>` if paused, `play <alias>` otherwise.
- `pause(&self) -> Result<(), String>` — pause. No-op if not playing.
- `stop(&self) -> Result<(), String>` — stop, reset position to 0.
- `close(&self) -> Result<(), String>` — close the current file, return to "no file loaded".
- `position_ms(&self) -> Option<u64>` — current playback position in milliseconds.
- `length_ms(&self) -> Option<u64>` — total length of the loaded file in milliseconds.
- `seek_ms(&self, ms: u64) -> Result<(), String>` — seek to a position.
- `state(&self) -> MediaState` — coarse playback state.
- `alias(&self) -> String` — the MCI alias used internally (`ruwx_media_<n>`); mostly for diagnostics and tests.

## Quick start

```rust,no_run
use std::path::Path;
use ru_wx::prelude::*;

// 1) Build the control parented on a frame. The control itself
//    does not own a child window on Windows — it is a thin wrapper
//    around mciSendStringW with a per-instance alias.
let media = MediaCtrl::new(&frame);

// 2) Load a media file. Any previously-loaded file is closed
//    first. On success the state is MediaState::Stopped.
media.load(Path::new("theme.mp3"))?;

// 3) Drive the state machine. play() is idempotent: it picks
//    `resume` if the file was paused, `play` otherwise.
media.play()?;
assert_eq!(media.state(), MediaState::Playing);

// 4) Query position / length. Both return None when no file is
//    loaded; both are expressed in milliseconds.
if let Some(pos) = media.position_ms() {
    let total = media.length_ms().unwrap_or(0);
    println!("{pos} / {total} ms");
}

// 5) Pause / resume / stop / seek are all one-liners.
media.pause()?;
assert_eq!(media.state(), MediaState::Paused);
media.play()?;
media.seek_ms(0)?;
media.stop()?;
assert_eq!(media.state(), MediaState::Stopped);

// 6) Close the file. After this the control is back in
//    MediaState::Stopped with no media loaded. Drop has the same
//    effect (sends `close <alias>` if a file was loaded).
media.close()?;
// drop(media);
```

MCI is a **synchronous** string-driven API; each call blocks the
GUI thread until MCI acknowledges. Use the control from the same
thread that owns the `Frame` — never from a worker thread, or the
device will deadlock on the cross-thread reply buffer. The
[`timer.md`](../core/timer.md) module is the right way to keep the
playhead UI in sync: tick every 250 ms, read `position_ms`, and
update a slider.

The control is **not** a child window on Windows — there is no
embedded video surface. If you need a "play / stop" video panel
on a specific HWND, open the file with `play ... window <hwnd>`
through the `OleDropTarget` / custom `Drop` integration; the
current API plays into the system-chosen default rendering target.

## Usage

```rust,no_run
use ru_wx::prelude::*;
use std::path::Path;

let media = MediaCtrl::new(&frame);
media.load(Path::new("theme.mp3"))?;
media.play();
assert_eq!(media.state(), MediaState::Playing);
```

## Win32 Notes

- The control is **not a child window** — it is a stateful object that forwards commands to `mciSendStringW`. It does, however, take a `parent` in the constructor so future implementations can anchor video to a window.
- Each `MediaCtrl` instance is given a unique alias (`ruwx_media_<n>`) at construction; the alias is used as the device handle in every command string.
- MCI is a string-driven, **synchronous** API: each command blocks the calling thread until the device has finished processing it. For UI responsiveness we therefore issue commands on the thread that owns the `MediaCtrl` (the GUI thread).
- The `open` command omits the `type` keyword so MCI picks the device by file extension.
- Path strings are passed through `escape_for_mci`, which doubles backslashes and escapes double-quotes — required by the MCI grammar inside a quoted string.
- MCI errors are decoded via `mciGetErrorStringW` into a human-readable `String`. The `Err(_)` variants of every method carry that string.
- The control does not own a child rendering window for video by default: MCI plays into a default rendering target chosen by the system. Video is therefore best treated as a "play / stop" surface. (For embedded video, MCI can be told to render to a parent `HWND` with the `play … window` variant; the `play_into_window` knob is not yet exposed.)
- `Drop` sends a `close <alias>` if a file was loaded, so dropping a `MediaCtrl` does not leave an orphan MCI device open.

## Cross-platform stub

On non-Windows targets `MediaCtrl` is constructible but every operation is a no-op and `state()` is always `MediaState::Stopped`. `load` and `seek_ms` return `Err("MediaCtrl: MCI is Windows-only")`; `play` / `pause` / `stop` / `close` return `Ok(())`; `position_ms` / `length_ms` return `None`. This is enough to keep code that embeds a `MediaCtrl` in a layout compiling.

## Tests

- `escape_preserves_plain_ascii` — `"hello.mp3"` round-trips unchanged.
- `escape_doubles_backslashes` — `"a\b\c"` → `"a\\b\\c"`.
- `escape_escapes_quotes` — `"a\"b"` → `"a\\\"b"`.
- `parse_decimal_u64` — wide-string reply `[b'1', b'2', b'3', 0]` parses to `Some(123)`.
- `parse_invalid_returns_none` — `[b'X', b'Y', b'Z', 0]` returns `None`.
- `alias_is_unique_per_instance` — `next_alias()` returns distinct names prefixed with `ruwx_media_`.

## See Also

- [`widget.md`](../core/widget.md) — `Window` trait, the bound on the `parent` parameter.
- [`message_box.md`](../dialogs/message_box.md) — `Err` strings follow the same "human-readable description" pattern.
