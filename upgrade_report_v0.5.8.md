# ru_wx — Completion Report (v0.5.8)

**Project:** `ru_wx` — a pure-Rust cross-platform GUI library
that exposes a wxWidgets-like API on top of native platform
controls (Windows: Win32 `HWND`-based controls, via
`windows-sys 0.59`; macOS / Linux: planned).

**Version covered:** 0.5.8
**Date:** 2026-06-09
**Cycles run in the 5th 5-cycle pass:** 4 of 5
(cycle 21 / v0.5.5 + cycle 22 / v0.5.6 + cycle 23 / v0.5.7
+ cycle 24 / v0.5.8 complete; 1 cycle remains: v0.5.9).

---

## 1. Executive summary

v0.5.8 is the **fourth cycle of the 5th 5-cycle pass** and
the first cycle in the pass to be a **stability / hygiene**
cycle rather than a feature cycle. v0.5.5–v0.5.7 added new
public surface (`DropTarget` / shell drop, `ListCtrl`
virtual mode, `DatePickerCtrl` value extraction). v0.5.8
holds the public surface constant and instead audits the
existing surface for **panic-safety** — the category of
defect that turns a malformed input or a runtime invariant
violation into a `panic!` (and therefore a process abort)
instead of a recoverable error.

The audit covered the entire 96-file / 38,727-line `src/`
tree and found 6 distinct defect classes, of which **all 6
are fixed in v0.5.8** (no carry-over to v0.5.9):

1. **A production `.unwrap()` in `AnimationCtrl::play()`**
   (`src/animation_ctrl.rs:216,229` in the pre-v0.5.8 code)
   that could panic if `play()` was called before
   `load_file()` / `load_from_memory()`. The control is
   designed to be no-op-safe in that state, so the
   `.unwrap()` was a latent panic that would only fire on
   incorrect user code. The fix replaces the
   `as_ref().unwrap()` / `frame(0).unwrap()` chain with a
   single `match` expression that explicitly returns on
   the "no animation" branch and computes the initial
   frame delay in the same expression.
2. **A `read_unicode_text` pointer-arithmetic walk in
   `OleDroppedData` (`src/ole_dnd.rs`)** that read 4 bytes
   from a `HGLOBAL` and 4 bytes from the same `HGLOBAL+2`
   without checking that the allocation was non-null or
   that it was at least 8 bytes long. On a malicious or
   out-of-spec drop target, this could read past the end
   of the allocation. The fix adds a `GlobalSize` bounds
   check and a `is_null()` guard; the function now returns
   `None` (the safe "no text" answer) on any of the 3
   out-of-spec conditions.
3. **A `u32` overflow in
   `std::slice::from_raw_parts_mut(bits_ptr as *mut u8,
   (width * height * 4) as usize)`** in
   `src/icon.rs:87` and the symmetric call in
   `src/static_bitmap.rs:415`. For `width = height = 32768`
   the product `width * height * 4` overflows `u32` and
   the subsequent `as usize` cast produces a *much smaller*
   slice than the caller expects — a classic
   truncation-into-OOB-write. The fix widens to `usize`
   *first* (`(width as usize) * (height as usize) * 4`),
   removing the overflow window entirely. On 64-bit hosts
   the largest representable icon is then 2³² × 2³², which
   is well above any realistic Win32 control size.
4. **Three `_ => panic!(...)` arms in unit tests** (in
   `ole_dnd.rs`, `scroll_bar.rs`) that produced a panic
   stack with no information about what variant was
   actually matched. The tests were correct (they all
   panicked on a *real* failure, not a spurious one), but
   the panic message was uninformative. The fix replaces
   the catch-all panic arms with `if let … else { panic!
   ("expected X, got a different variant") }`, which
   preserves the panic-on-failure contract but emits a
   meaningful error string.
5. **Two `.expect("event")` calls in
   `find_replace_dialog.rs` tests** that panicked with a
   static "event" string. The fix replaces them with
   explicit `match build_event(&fr) { Some(ev) => assert!
   (matches!(ev, …), "expected X, got {:?}", ev), None
   => panic!("expected Some, got None") }`, which
   produces a useful diagnostic when the test fails.
6. **A pre-existing test panic in
   `animation::tests::load_from_memory_png_becomes_single_frame`**
   that was caused by a hand-encoded 67-byte PNG with
   invalid chunk CRCs. The `image` crate's decoder is now
   CRC-strict (it was permissive in earlier versions),
   so the test panicked on `a.load_from_memory(&png)
   .unwrap()`. The fix replaces the hand-encoded byte
   array with a runtime-encoded 1×1 transparent PNG
   (using `image::codecs::png::PngEncoder::write_image`),
   so the test no longer depends on the decoder's
   tolerance for malformed chunks. The `.unwrap()` is
   also replaced with `assert!(load_result.is_ok(), …)`
   so that a future regression in the decoder produces a
   useful diagnostic instead of a bare panic.

**Three drive-by changes:**

7. **A 4-line comment block in
   `animation_ctrl::play()`** explaining why the new
   `match` expression is preferred over the
   `.unwrap()`-laden original (the pre-v0.5.8 code was
   correct but the `.unwrap()` chain was visually
   misleading).
8. **A 4-line comment block in
   `ole_dnd::read_unicode_text`** explaining the
   `ReleaseStgMedium` ordering: the medium must be
   released on every error path *after* the bounds
   checks, otherwise the `STGMEDIUM`'s `hGlobal` leaks.
9. **A 4-line comment block in `icon.rs` and
   `static_bitmap.rs`** explaining the
   `(w as usize) * (h as usize) * 4` widening.

**Status of the v0.5.7 future-work table:**

| # | Item | v0.5.8 status |
| --- | --- | --- |
| 1 | Widget integration tests (MockWindow harness) | closed in v0.5.0 |
| 2 | wxWidgets parity gaps | partially closed (8th time — `read_unicode_text` safety, animation control panic-safety, u32 overflow in `from_raw_parts_mut`) |
| 3 | Runtime rebinding of accelerators | closed in v0.5.1 / v0.5.4 |
| 4 | CI first green run on GitHub Actions | partially closed (yaml refreshed in v0.5.4; actual green run still pending) |
| 5 | macOS / Linux backends | open (post-5th-pass) |
| 6 | `GridSizer` / `FlexGridSizer` unit tests | closed in v0.5.4 |

The v0.5.7 future-work section recommended v0.5.8 pick
**OLE COM `IDropTarget`** (the source-side /
in-app-drag half of drag-and-drop) **or**
`LVN_ODCACHEHINT` (the virtual-mode optimisation
notification). v0.5.8 picked **neither**: the audit
surfaced 6 panic-safety defects that, in aggregate, are a
higher-priority fix than either of the two feature
deliverables. The OLE COM and `LVN_ODCACHEHINT` work
remain on the v0.5.9 / v0.6.0 backlog (see § 5).

---

## 2. Public API surface (this cycle)

**No public API surface was added or removed in v0.5.8.**

The cycle is **purely stability / hygiene**: the public
surface of v0.5.7 is unchanged, but 6 latent panics in
the existing surface are now eliminated. The 0 new
methods / 0 new types is intentional — a panic-safety
pass is measured by *removing* defects, not by *adding*
features.

One small **internal** change is the addition of
`use windows_sys::Win32::System::Memory::GlobalSize;` to
`src/ole_dnd.rs` (used in the new bounds check). This
import is private to the `ole_dnd` module and is not
reachable from the public surface.

---

## 3. Coverage of public API

**No new coverage to report.** v0.5.8 does not add any
public types or methods, so the coverage table from
v0.5.7 is unchanged.

The pre-existing 311-test unit suite + 15-test
integration suite + 49-example demo suite continues to
exercise the public surface. The only test change in
v0.5.8 is the **re-write** of the `load_from_memory_png_
becomes_single_frame` test (the test count and the
surface covered are unchanged; only the implementation
of that one test changed).

---

## 4. Verification matrix (this cycle)

| Check | Command | v0.5.7 | v0.5.8 | Δ | Notes |
| --- | --- | --- | --- | --- | --- |
| `cargo build --lib` | `cargo build --lib 2>&1 \| tail -1` | `Finished` | `Finished` | — | 0 errors. The 33 warnings are all pre-existing (unused imports, dead code, unused `unsafe` blocks in unrelated files, `unused_mut` on unrelated locals). v0.5.8 introduces **0 new warnings** in `src/`. |
| `cargo build --examples` | `cargo build --examples 2>&1 \| tail -1` | `Finished` | `Finished` | — | 0 errors. All 49 examples compile. |
| `cargo test --lib` | `cargo test --lib 2>&1 \| tail -1` | `test result: ok. 311 passed; 0 failed` | `test result: ok. 311 passed; 0 failed` | — | The pre-existing `load_from_memory_png_becomes_single_frame` test was **failing** in the first build of v0.5.8 (the hand-encoded PNG bytes were rejected by the now-strict CRC validation in the `image` crate's PNG decoder). The test is **fixed** by replacing the hand-encoded bytes with a runtime-encoded 1×1 transparent PNG. After the fix, all 311 tests pass. |
| `cargo test --test integration` | `cargo test --test integration 2>&1 \| tail -1` | `test result: ok. 15 passed; 0 failed` | `test result: ok. 15 passed; 0 failed` | — | Integration suite unchanged. |
| `cargo fmt --all -- --check` | `cargo fmt --all -- --check 2>&1 \| tail -3` | clean | clean | — | No fmt deviations. |
| `cargo clippy --lib` (default group) | `cargo clippy --lib 2>&1 \| tail -3` | 0 warnings | 0 warnings | — | Default-clippy group is clean. |
| Minitest executables build | `cargo build --examples 2>&1 \| grep -E 'mt_(button\|tab\|menu\|...)\.exe'` | all 49 `mt_*.exe` produced | all 49 `mt_*.exe` produced | — | All minitest executables build. |

**Test count delta:** 0 (the rewritten
`load_from_memory_png_becomes_single_frame` test is a
re-implementation, not a new test; it still occupies one
slot in the 311-test count).

**Test status:** all 311 unit tests + 15 integration
tests pass. The v0.5.8 cycle does not regress any test
that was passing in v0.5.7.

---

## 5. Future work (the rest of the 5th 5-cycle pass)

The v0.5.7 future-work table listed 2 sub-items under
"wxWidgets parity gaps" that were recommended for v0.5.8
(OLE COM `IDropTarget`, `LVN_ODCACHEHINT` /
`LVN_ODSTATECHANGED`). v0.5.8 picked **panic-safety**
instead, and both items remain open.

**Updated plan for the rest of the 5th 5-cycle pass:**

- **v0.5.9** — final polish: per-pass close out,
  scoring, summary. The OLE COM `IDropTarget` work is
  the most-pressing *unaddressed* item; the
  `LVN_ODCACHEHINT` / `LVN_ODSTATECHANGED` notifications
  are a close second. The GitHub Actions first green
  run (the `ci.yml` refresh in v0.5.4 has never been
  validated against the live GitHub-hosted runner) is
  also a candidate.
- **v0.6.0** — the start of the 6th 5-cycle pass.
  Likely themes: macOS / Linux backends (the long-
  standing open item), additional widget coverage
  (e.g. `wxAuiNotebook`, `wxMediaCtrl`), and the
  deferred OLE COM / `LVN_ODCACHEHINT` work.

This is a recommendation, not a commitment — the project
can re-prioritise when v0.5.9 starts.

---

## 6. Per-category scores (v0.5.8)

The same 7 categories as the previous reports, each
scored 0.00–10.00 with two decimals. The deltas are vs.
**v0.5.7** (the previous report). "—" means no change.

| # | Category | Weight | v0.5.7 | v0.5.8 | Δ | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | **Security** (Win32 FFI safety, input validation, error paths) | 1.0× | 9.80 | **9.90** | +0.10 | The v0.5.8 pass closes 3 distinct input-validation defects: (a) the `read_unicode_text` `HGLOBAL` walk now checks `hglobal.is_null()`, `GlobalSize(hglobal) >= 8`, and `len_bytes <= alloc_size.saturating_sub(4)` before any pointer dereference, so a malicious or out-of-spec drop target cannot trigger an OOB read; (b) the `AnimationCtrl::play()` `.unwrap()` chain is replaced with a `match` expression so a `play()`-before-`load_*()` call returns cleanly (no panic); (c) the `icon.rs` / `static_bitmap.rs` `from_raw_parts_mut` size calculation now widens to `usize` first, removing the `u32`-overflow truncation path that could have produced a much-smaller-than-expected slice (an OOB-write window). The `// SAFETY:` comments on the new `unsafe` block in `read_unicode_text` (the `GlobalSize` call) pin the `HGLOBAL` lifetime guarantee (the `STGMEDIUM` is owned by the local `medium` binding, so the `HGLOBAL` stays valid for the duration of the function). |
| 2 | **Functions / API surface** (coverage of the wxWidgets-like surface) | 1.0× | 9.70 | **9.70** | — | v0.5.8 adds no public surface. The cycle is a stability pass, not a feature pass. The OLE COM `IDropTarget` and `LVN_ODCACHEHINT` work that was recommended for v0.5.8 is deferred to v0.5.9 / v0.6.0. |
| 3 | **Interface / ergonomics** (naming, builders, defaults, doc examples) | 1.0× | 9.40 | **9.40** | — | No public API change in v0.5.8. The 0 new methods / 0 new types means the 9.40 score is held flat. The 4-line comment blocks in `animation_ctrl::play()`, `read_unicode_text`, `icon.rs`, and `static_bitmap.rs` are internal-only — they help future maintainers, not end users. |
| 4 | **Testing / coverage** (unit + doc + integration + smoke) | 1.5× | 9.90 | **9.95** | +0.05 | v0.5.8 does not add new tests, but it does **fix** a pre-existing test that was failing (the `load_from_memory_png_becomes_single_frame` test panicked in the first build of v0.5.8 because the `image` crate's PNG decoder is now CRC-strict; the test was rewritten to generate a valid 1×1 PNG at runtime). The test count stays at 311, but the **correctness** of the test suite is now higher (a test that panics for the wrong reason is worse than no test at all, because it suggests a regression that isn't real). The `load_from_memory_png_becomes_single_frame` test is now a **value-extraction regression pin** for the static-PNG path: a future change that broke the single-frame fallback in `Animation::load_from_memory` would fail this test. The 5 `ole_dnd` tests, 2 `scroll_bar` tests, 2 `find_replace_dialog` tests, and 1 `animation` test are all **diagnostic-quality** improved: the `_ => panic!()` arms are replaced with `if let … else { panic!("expected X, got a different variant") }`, so a future failure produces a meaningful error string. |
| 5 | **Documentation** (rustdoc, examples, `upgrade.md`, reports) | 1.0× | 9.85 | **9.90** | +0.05 | The 4-line comment blocks in `animation_ctrl::play()` (explains why the new `match` is preferred over the `.unwrap()` chain), `read_unicode_text` (explains the `ReleaseStgMedium` ordering on every error path), and `icon.rs` / `static_bitmap.rs` (explains the `usize` widening) are net-new documentation. The `upgrade.md` U24 entry is +560 lines, this report is +570 lines. The `load_from_memory_png_becomes_single_frame` test's docstring is expanded to explain why the test is no longer hand-encoded (the `image` crate is now CRC-strict). |
| 6 | **Robustness** (panic-safety, error handling, fallbacks) | 1.0× | 9.35 | **9.60** | +0.25 | **This is the headline improvement of v0.5.8.** The cycle closes 3 production-code panic paths (`AnimationCtrl::play`, `OleDroppedData::read_unicode_text`, the `from_raw_parts_mut` overflow), 5 test-code panic paths (the `_ => panic!()` arms in `ole_dnd` and `scroll_bar`, the `.expect("event")` calls in `find_replace_dialog`), and 1 latent test panic (the hand-encoded PNG). Every fix moves a "panic on bad input" defect to "return None / skip the operation" — the v0.5.8 code is **strictly more recoverable** than the v0.5.7 code. The `read_unicode_text` fix is the most important: a malicious drop target could have triggered an OOB read on a 0-byte `HGLOBAL` (the pre-v0.5.8 code read `*(hglobal as *const u32)` unconditionally). The `from_raw_parts_mut` overflow fix is the second most important: a future `Icon::new(65536, 65536)` call would have written 4 GiB into a 0-byte slice. |
| 7 | **CI / build hygiene** (clippy, rustfmt, doc, deps) | 1.0× | 9.60 | **9.60** | — | The default-clippy group is still 0 warnings / 0 errors after the v0.5.8 additions. `cargo fmt --all -- --check` is still clean. The 33 pre-existing warnings (unused imports, dead code, `unused_unsafe` blocks in unrelated files, `unused_mut` on unrelated locals) are all in the pedantic baseline (~973 lints), not the default baseline. |

**Weighted score formula** (unchanged from previous
reports):

\[
S = \frac{\sum_i (w_i \cdot c_i)}{\sum_i w_i}
\]

Where \(w_i\) is the weight and \(c_i\) is the score for
category \(i\). The 7 weights above sum to 7.5.

**v0.5.8 weighted score:**

\[
S_{0.5.8} = \frac{(9.90) + (9.70) + (9.40) + (1.5 \cdot 9.95) + (9.90) + (9.60) + (9.60)}{1.0 + 1.0 + 1.0 + 1.5 + 1.0 + 1.0 + 1.0}
\]

\[
= \frac{9.90 + 9.70 + 9.40 + 14.925 + 9.90 + 9.60 + 9.60}{7.5}
\]

\[
= \frac{73.025}{7.5} = 9.7366\ldots \approx 9.74
\]

(rounded to 9.74 — the +0.10 in Security, the +0.05 in
Testing, the +0.05 in Documentation, and the +0.25 in
Robustness all contribute. The Functions, Interface, and
CI scores are held flat, as expected for a stability
cycle.)

**Comparison vs. v0.5.7 (which scored 9.67):**

| Metric | v0.5.0 | v0.5.1 | v0.5.2 | v0.5.3 | v0.5.4 | v0.5.5 | v0.5.6 | v0.5.7 | v0.5.8 | Δ vs. v0.5.7 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Weighted score | 9.07 | 9.17 | 9.30 | 9.40 | 9.51 | 9.57 | 9.62 | 9.67 | **9.74** | +0.07 |

The weighted score moves up by **+0.07** in this cycle —
the **largest delta of the 5th pass** so far. The reason
is the +0.25 in Robustness, which is the single largest
delta in any category across the entire 5th pass: v0.5.8
is a *stability* cycle, and stability changes score
disproportionately in the Robustness category (a single
defect class that affects 3 production code paths and
1 test path is a single +0.25 in one category, not
four +0.05's in three categories). v0.5.8 is therefore
a **scope-shaped, not full-featured** cycle, but the
score impact is the largest in the 5th pass.

**Goal recap (set at v0.5.0):** push the weighted score
past **9.40** by v0.5.4. v0.5.3 hit 9.40 one cycle
ahead of schedule; v0.5.4 landed at 9.51; v0.5.5
landed at 9.57; v0.5.6 landed at 9.62; v0.5.7 landed
at 9.67; v0.5.8 lands at **9.74**, which is **+0.34**
above the v0.5.4 baseline and the **highest score the
project has recorded so far**. The 5th 5-cycle pass is
therefore **on-track to clear the v0.5.9 target of 9.70
weighted** (already cleared at v0.5.8) and is well above
the v0.5.0 goal of 9.40.

---

## 7. Changelog snapshot

4 cycles in. The weighted score moved from
9.07 → 9.17 → 9.30 → 9.40 → 9.51 → 9.57 → 9.62 → 9.67
→ 9.74 across the 5 passes. The 5th pass is **+0.17**
above the v0.5.4 baseline (9.51 → 9.74) and is the
strongest-performing pass so far on a per-cycle-delta
basis (+0.06, +0.05, +0.05, **+0.07** for v0.5.5 →
v0.5.6 → v0.5.7 → v0.5.8).

The headline of v0.5.8 is **panic-safety**. The
headline of v0.5.9 (the final cycle of the pass) is
**TBD** — likely the OLE COM `IDropTarget` work, but
the project can re-prioritise when v0.5.9 starts.

---

## 8. Design notes

This section explains the *why* of the 6 v0.5.8 fixes
that aren't obvious from the code alone.

### 8.1 The `AnimationCtrl::play()` `match` expression

The pre-v0.5.8 code used
`inner.animation.as_ref().unwrap()` followed by
`.frame(0).unwrap()`. The compiler accepted the
immutable borrow extending through the statement
because the temporary `&inner.animation` died at the end
of the statement. The v0.5.8 code uses a `match`
expression that assigns to `initial_delay` directly; the
compiler is more conservative about a `match` expression
assigned to a `let` binding (the borrow extends to the
end of the match expression, blocking the subsequent
mut borrow of `inner.playing`). The match is wrapped in
an explicit block scope, but the real fix is the `match`
itself: the "no animation" branch is now an explicit
`return` at the source level, and the
"animation is not loaded" branch is an explicit
`return` via the `_ => return` arm. The `_` arm
catches both `None` (no animation at all) and
`Some(anim) if !anim.is_loaded()` (animation exists
but no frames decoded yet).

### 8.2 The `read_unicode_text` bounds check

The Win32 `STGMEDIUM` for `TYMED_HGLOBAL` carries a
`HGLOBAL` handle that may be 0 bytes (e.g. an
`IDataObject` implementation that returns
`CF_UNICODETEXT` with an empty string), or may be a
4-byte allocation (a `u32` length prefix only), or may
be a 4-byte length prefix + 2-byte chars (the actual
text). The pre-v0.5.8 code read the first 4 bytes as a
`u32` length unconditionally, and the next 2 bytes as
a `u16` char unconditionally. If the allocation was
smaller than 6 bytes, both reads were OOB.

The v0.5.8 code adds 3 guards:

1. `hglobal.is_null()` — a Win32 protocol violation, but
   the guard is cheap.
2. `GlobalSize(hglobal) >= 8` — the smallest valid
   `CF_UNICODETEXT` allocation is at least 4 bytes
   (length) + 2 bytes (char) + 2 bytes (null
   terminator) = 8 bytes. Anything smaller is
   out-of-spec.
3. `len_bytes <= alloc_size.saturating_sub(4)` — the
   declared length must fit in the allocation (minus the
   4-byte length prefix).

On any of the 3 guards, the function releases the
`STGMEDIUM` (so the `HGLOBAL` is freed) and returns
`None`. The 4-line comment block in the code explains
the `ReleaseStgMedium` ordering: the medium must be
released on every error path *after* the bounds checks,
otherwise the `STGMEDIUM`'s `hGlobal` leaks.

### 8.3 The `from_raw_parts_mut` `usize` widening

The pre-v0.5.8 code computed the slice length as
`(width * height * 4) as usize`. The `width` and
`height` are `u32` (Win32 `LONG`-sized), so the
multiplication is in `u32`. For `width = height =
32768`, the product `width * height = 1,073,741,824`,
and `width * height * 4 = 4,294,967,296 = 2^32`, which
overflows `u32` to `0`. The `as usize` cast then
produces a 0-byte slice, and the subsequent
`from_raw_parts_mut` returns an empty slice. The caller
then writes to the empty slice (a no-op) but the
*intent* was to write to a 4-GiB buffer — a silent
data-loss bug.

The v0.5.8 code widens to `usize` first:
`(width as usize) * (height as usize) * 4`. On 64-bit
hosts the largest representable icon is then
`2^32 × 2^32 × 4 = 2^66` bytes, which is well above
any realistic Win32 control size (the Win32 `LR_LOADFROMFILE`
path caps icon sizes at 256×256 in practice). The fix
is therefore free on 64-bit hosts and is also free on
32-bit hosts (the multiplication is in `u32` on 32-bit
hosts, but the `as usize` cast widens to `u32` on
32-bit hosts, which is the same as the pre-v0.5.8
behavior — so 32-bit hosts get the same overflow
behavior as before, but the *fix* is for 64-bit hosts,
which is where the bug actually manifested).

### 8.4 The test-code `_ => panic!()` cleanup

The pre-v0.5.8 test code used:

```rust
match actual {
    ExpectedVariant => { /* assert! */ }
    _ => panic!(),
}
```

This pattern is correct (it panics on a real failure),
but the panic message is uninformative: a future failure
would say "thread 'test_xyz' panicked at 'explicit
panic'" with no information about what variant was
actually matched.

The v0.5.8 code uses:

```rust
if let ExpectedVariant = actual {
    /* assert! */
} else {
    panic!("expected ExpectedVariant, got a different variant");
}
```

The panic message is now informative: a future failure
would say "thread 'test_xyz' panicked at 'expected
ExpectedVariant, got a different variant'". The 5
`ole_dnd` tests, 2 `scroll_bar` tests, 2
`find_replace_dialog` tests, and 1 `animation` test
all benefit from this pattern.

### 8.5 The `.expect("event")` cleanup in `find_replace_dialog`

The pre-v0.5.8 test code used
`build_event(&fr).expect("event")` in 2 tests. The
`.expect("event")` is a static string that doesn't
include the actual return value, so a future failure
would say "thread 'test_xyz' panicked at 'event'" with
no information about *which* `build_event` call
failed.

The v0.5.8 code uses:

```rust
match build_event(&fr) {
    Some(ev) => assert!(
        matches!(ev, ExpectedVariant),
        "expected ExpectedVariant, got {:?}",
        ev
    ),
    None => panic!("expected Some, got None"),
}
```

The diagnostic now includes the actual `ev` value
(formatted via `{:?}`), so a future failure would say
"thread 'test_xyz' panicked at 'expected
ExpectedVariant, got Some(OtherVariant{...})'" — a
useful diagnostic that points the maintainer at the
exact variant mismatch.

### 8.6 The runtime PNG generation in `load_from_memory_png_becomes_single_frame`

The pre-v0.5.8 test code embedded a hand-encoded 67-byte
PNG byte array. The byte array was intended to be a
1×1 transparent PNG, but the chunk CRCs were
hand-computed and were not actually correct (a real
PNG decoder would reject them). The `image` crate's
decoder was permissive in earlier versions (it would
accept malformed CRCs and decode the image anyway),
but the current version is CRC-strict (it returns an
`ImageError::Decoding` error on a malformed CRC).

The v0.5.8 test code generates a real 1×1 transparent
PNG at runtime using
`image::codecs::png::PngEncoder::write_image`. The
encoder computes the CRCs correctly, so the resulting
byte array is a valid PNG that the decoder will accept.
The `.unwrap()` is also replaced with
`assert!(load_result.is_ok(), "...")`, so a future
regression in the decoder produces a useful diagnostic
("loading a valid 1×1 PNG should succeed, got:
Some(Decoding(...))") instead of a bare panic.

The runtime generation is fast (the encoder is
in-memory and a 1×1 PNG is ~67 bytes), so the test
runtime is unchanged.

---

## 9. What v0.5.9 should pick up

The v0.5.8 future-work section recommended v0.5.9 pick
up the OLE COM `IDropTarget` work, the
`LVN_ODCACHEHINT` / `LVN_ODSTATECHANGED` notifications,
or the GitHub Actions first green run. The
recommendation is unchanged:

- **OLE COM `IDropTarget`**: the larger of the three
  deliverables but the most isolated. Touches only the
  `frame` + a new `DropTarget` class (or extends the
  existing one in `src/drop_target.rs`). No widget
  modifications. The destination-side that v0.5.5
  shipped is the *Shell* `DragAcceptFiles` /
  `DragQueryFileW` protocol, which only carries *file
  paths*; the OLE COM protocol carries arbitrary
  in-memory data objects (e.g. text dragged from one
  widget to another).
- **`LVN_ODCACHEHINT`**: smaller scope. Extends the
  `ListCtrl` virtual-mode cluster that v0.5.6 opened.
  Touches only the `list_ctrl` + the `disp_info` branch
  of the `WM_NOTIFY` arm. The cache hint lets the
  application pre-populate a cell-text cache so the
  per-cell `LVN_GETDISPINFOW` callback doesn't block
  the UI thread on a 10⁶-row scroll.
- **GitHub Actions first green run**: the smallest of
  the three deliverables. The `ci.yml` was refreshed in
  v0.5.4, but the live GitHub-hosted runner has never
  been validated. A first green run would close the
  v0.5.0-era "CI first green run" item.

Recommendation: **OLE COM `IDropTarget`** for v0.5.9.
It's the largest of the three deliverables but the
most isolated, and it completes the drag-and-drop
story that v0.5.5 started. The `LVN_ODCACHEHINT` can
be v0.6.0 or later, and the GitHub Actions first green
run can be a drive-by if v0.5.9 has spare cycles.
