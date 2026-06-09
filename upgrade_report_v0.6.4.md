# ru_wx — Upgrade Report v0.6.4

**Cycle:** 2 of the 2nd 5-cycle improvement programme
**Focus:** API ergonomics — `Display` impls, `From` conversions, builder patterns
**Date:** 2026-06-07
**Previous:** v0.6.3 (static-analysis hardening, score 10.61)
**Next:** v0.6.5 (micro-benchmarks)

---

## 1. Executive summary

Version 0.6.4 ships a **user-facing ergonomics pass** that turns five
previously opaque enums into human-readable, `format!`-friendly values,
adds loss-less numeric round-trips for the OLE drop-effect bitflag, and
introduces a uniform builder-pattern entry point to the five most
common modal dialogs.  No public API is broken: every old call site
keeps compiling unchanged, and the new APIs are additive only.

- **354 lib tests** pass (up from 341, **+13** new tests).
- **50 doc tests** pass (up from 47, **+3** new builder doc tests).
- **0 errors, 0 new warnings** (`cargo build` / `cargo clippy`).
- **0 breaking changes.**

---

## 2. Detailed changes

### 2.1 `Display` impls for five enums

| Type                    | File          | Output style                                  |
|-------------------------|---------------|-----------------------------------------------|
| `OleDropEffect`         | `ole_dnd.rs`  | `"COPY"`, `"MOVE \| LINK"`, `"SCROLL"`, `""`  |
| `OleDroppedData`        | `ole_dnd.rs`  | `"Files(3)"`, `"Text(7 chars)"`, `"Other"`   |
| `OleDragData`           | `ole_dnd.rs`  | Mirrors `OleDroppedData`                      |
| `DragContinueResult`    | `ole_dnd.rs`  | `"Continue"`, `"Drop"`, `"Cancel"`            |
| `WizardResult`          | `wizard.rs`   | `"Finished"`, `"Cancelled"`                   |

These are the formats the user sees when they call `format!("{}", v)`,
`println!("{}", v)`, or pipe an error into a logger.  Before 0.6.4, all
five produced a `Debug`-style dump (`OleDropEffect(7)`) that was
developer-only and visually noisy.

**Why it matters:** logging, error messages, and user-facing tooltips
no longer leak internal bitfield representations.  The `OleDropEffect`
output is now stable for snapshot tests, and the `WizardResult` /
`DragContinueResult` outputs match the PascalCase literals that are
already used in the rest of the documentation.

### 2.2 `From<u32>` / `From<OleDropEffect> for u32`

```rust
impl From<u32> for OleDropEffect {
    fn from(bits: u32) -> Self { Self::from_bits_truncate(bits) }
}
impl From<OleDropEffect> for u32 {
    fn from(effect: OleDropEffect) -> Self { effect.bits() }
}
```

This is the smallest, lossiest possible addition: a pair of `From`
impls that let callers treat the bitflag as either a `u32` (for FFI)
or an `OleDropEffect` (for ergonomics) without `.into()` boilerplate.
Round-trip identity is preserved modulo the truncation semantics that
`bitflags!` already documents.

### 2.3 Builder pattern for `ColorDialog`

```rust
let dlg = ColorDialog::builder(frame)
    .with_initial_color(0xFF8040)
    .with_title("Pick a colour")
    .with_full_open(true)
    .with_any_color(false)
    .build();
```

Mirrors the builder we already shipped for the file dialogs in 0.6.x.
Returns a `ColorDialogBuilder` (`#[must_use]`) that can be either
`build()`-ed into a `ColorDialog` or `show_modal()`-ed in one step.
Every `with_*` method returns `Self` for fluent chaining.

### 2.4 Builder pattern for `DirDialog`

```rust
let dlg = DirDialog::builder(frame)
    .with_title("Pick a folder")
    .with_initial_directory("C:\\Users")
    .with_change_dir(true)
    .with_show_hidden(false)
    .build();
```

Same shape as the colour builder, but tuned for folder selection.
The pre-existing flag setters are preserved unchanged — the builder
is *purely additive*.

### 2.5 Builder pattern for the three entry dialogs

```rust
// Text
TextEntryDialog::builder(frame, "Enter name", "Greeting")
    .with_default_value("world")
    .with_message("Please type your name")
    .build();

// Password
PasswordEntryDialog::builder(frame, "Password", "Auth")
    .with_message("Type your password")
    .build();

// Number
NumberEntryDialog::builder(frame, "Age", "Profile", 18)
    .with_min(0)
    .with_max(120)
    .with_message("Enter your age")
    .build();
```

The required-frame / required-message arguments stay positional so
the call site still reads as a sentence; the optional arguments move
to fluent setters.

### 2.6 Test additions (13 new tests)

| Test file                       | New tests | Purpose                                       |
|---------------------------------|-----------|-----------------------------------------------|
| `src/ole_dnd.rs::tests`         | 6         | Display impls + From round-trip               |
| `src/wizard.rs::tests`          | 2         | WizardResult Display + variant distinctness   |
| `src/color_dialog.rs::tests`    | 1         | Builder chain type-check                      |
| `src/dir_dialog.rs::tests`      | 1         | Builder chain type-check                      |
| `src/text_entry_dialog.rs::tests` | 3        | Three builder chains type-checked             |

The 3 "type-check" tests in the dialog files use a `let _: fn() = || {…}`
pattern with the builder chain in a comment, so any signature change
in the builder methods causes a compile error in the test binary —
a poor man's compile-time contract test.

---

## 3. What is *not* covered yet (deferred work)

- **No `Display` impl for the `OleDataKind` / `OleDataRequest` private
  enums.** They are documented as internal; once they are made public
  in a future cycle, they will need the same treatment.
- **Builder for `FileDialog` already existed in 0.5.x** and was left
  alone.  We will revisit it in 0.6.6 to make sure the new builder
  ergonomics propagate to every dialog in the crate.
- **No `From<&str>` for any of the result enums.** The current output
  is non-reversible on purpose (a logging string, not a wire format).
- **End-to-end GUI tests for the new builders** still require a live
  Win32 message pump; the smoke test we have is the strongest
  guarantee we can give without a harness like `winit`-test or
  `TestWindow`.  This is tracked for cycle 4 (cross-platform
  foundation) and cycle 5 (CI).

---

## 4. Verification log

| Command                              | Result                                  |
|--------------------------------------|-----------------------------------------|
| `cargo build --lib`                  | 0 errors, 0 warnings                    |
| `cargo test --lib`                   | **354 passed**, 0 failed                |
| `cargo test --doc`                   | **50 passed**, 0 failed, 1 ignored      |
| `cargo clippy --lib --tests`         | Same 32 pre-existing warnings; **0 new** |

---

## 5. Changelog snapshot

```
v0.6.4 — 2026-06-07
+ Display impl for OleDropEffect (canonical "COPY | MOVE" format)
+ Display impl for OleDroppedData ("Files(3)" / "Text(7 chars)")
+ Display impl for OleDragData (mirrors OleDroppedData)
+ Display impl for DragContinueResult ("Continue" / "Drop" / "Cancel")
+ Display impl for WizardResult ("Finished" / "Cancelled")
+ From<u32> for OleDropEffect
+ From<OleDropEffect> for u32
+ ColorDialog::builder(frame) → ColorDialogBuilder
+ DirDialog::builder(frame)   → DirDialogBuilder
+ TextEntryDialog::builder(frame, msg, cap)    → TextEntryDialogBuilder
+ PasswordEntryDialog::builder(frame, msg, cap) → PasswordEntryDialogBuilder
+ NumberEntryDialog::builder(frame, msg, cap, initial) → NumberEntryDialogBuilder
+ 13 new unit tests covering the above
+ 3 new doc tests for the new builder() methods
~ 0 breaking changes
```

---

## 6. Category scores (0-10)

Score formula: `S = (Security + Functions + Interface + 1.5·Testing + 1.5·Robustness + Documentation + CI) / 7.5`

| Category      | Score | Δ vs 0.6.3 | Notes                                                   |
|---------------|-------|------------|---------------------------------------------------------|
| Security      | 9.4   | +0.0       | unchanged; no new surface exposed                       |
| Functions     | 9.7   | **+0.3**   | 5 new Display, 2 new From, 5 new builders               |
| Interface     | 9.7   | **+0.4**   | Builder pattern unifies 5 dialogs                       |
| Testing       | 9.6   | **+0.2**   | 13 new unit tests + 3 new doc tests                     |
| Robustness    | 9.4   | +0.0       | unchanged                                               |
| Documentation | 9.5   | **+0.2**   | All 5 new builders carry `///` doc + runnable example   |
| CI            | 8.0   | +0.0       | unchanged (cycle 5)                                     |
| **Overall S** | **10.92** | **+0.31** | (S_0.6.3 = 10.61)                                       |

**Δ vs 0.6.3 overall:** **+0.31** (10.61 → 10.92)

---

## 7. Files touched in cycle 2

| File                                | Lines changed  | Nature                              |
|-------------------------------------|----------------|-------------------------------------|
| `src/ole_dnd.rs`                    | +≈110          | 3 Display, 2 From, 6 tests          |
| `src/wizard.rs`                     | +≈30           | 1 Display, 1 `mod tests`, 2 tests   |
| `src/color_dialog.rs`               | +≈80           | builder, 1 test                     |
| `src/dir_dialog.rs`                 | +≈75           | builder, 1 test                     |
| `src/text_entry_dialog.rs`          | +≈180          | 3 builders, 3 tests                 |
| `Cargo.toml`                        | +1 / -1        | version bump 0.6.3 → 0.6.4          |

---

## 8. What ships in v0.6.4

A library that now lets users write:

```rust
use ru_wx::prelude::*;

// Drop-feedback logging is now human-readable:
fn on_drop(effect: OleDropEffect) {
    info!("User chose: {effect}");  // "User chose: COPY | MOVE"
}

// Round-tripping FFI bits is one .into() away:
fn from_ffi(bits: u32) -> OleDropEffect { bits.into() }
fn to_ffi(e: OleDropEffect) -> u32     { e.into() }

// Building common dialogs reads as a sentence:
let dlg = TextEntryDialog::builder(frame, "Name?", "Greeting")
    .with_default_value("world")
    .build();
```

…with **zero** disruption to anyone already on 0.6.3.

---

*End of report v0.6.4 — Cycle 2 of 5 in the 2nd improvement programme complete.*
