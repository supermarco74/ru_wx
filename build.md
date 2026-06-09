# `ru_wx/build.rs` — Win32 resource embedder

Build script for the `ru_wx` library crate. Compiles `app.rc` into a
static `app.lib`, then re-emits the link directives so example
binaries that depend on `ru_wx` also pick the resource up.

## Source

```rust
fn main() {
    if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
        embed_resource::compile("app.rc", embed_resource::NONE);
        let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
        println!("cargo:rustc-link-search=native={out_dir}");
        println!("cargo:rustc-link-lib=dylib=app");
    }
}
```

## Why this exists

`app.rc` lives in the crate root (next to `Cargo.toml`) and references
`app.manifest` (the Windows application manifest, with
`PerMonitorV2` DPI awareness, `UTF-8` code page, `SegmentHeap`,
`LongPathAware`, and `AsInvoker` execution level). The resource
needs to be linked into every binary that uses `ru_wx`, including
each `examples/*.exe` and each `examples/minitest/*.exe`.

The `embed_resource` crate is configured with `embed_resource::NONE`
because the manifest lives in a separate `app.manifest` file; the
default flavour would inline a manifest blob into the `.rc` and
ignore the standalone file.

## Step by step

1. **Windows guard** — the build script short-circuits to a no-op on
   non-Windows targets (`CARGO_CFG_WINDOWS` is set by cargo for the
   `*-pc-windows-gnu` and `*-pc-windows-msvc` triples only).
2. **`embed_resource::compile("app.rc", embed_resource::NONE)`** —
   runs the platform `rc.exe` / `windres` and produces
   `target/.../out/app.lib`. The build script depends on `app.rc`
   transitively; the `embed-resource` crate also emits
   `cargo:rerun-if-changed=app.rc` automatically.
3. **`cargo:rustc-link-search=native={OUT_DIR}`** — adds the
   `OUT_DIR` to the linker search path. Required so the consumer's
   link step can find `app.lib`.
4. **`cargo:rustc-link-lib=dylib=app`** — links `app.lib` as a
   static library into the final binary. The re-emission is needed
   because some cargo versions only forward the search path from a
   library build script, not the `link-lib` itself; this explicit
   pair guarantees the resource is linked into every example too.

## Inputs / outputs

| Input                | Effect                                                  |
| -------------------- | ------------------------------------------------------- |
| `CARGO_CFG_WINDOWS`  | If unset (non-Windows target), the script is a no-op.   |
| `OUT_DIR`            | cargo-provided build directory for the link artefacts.  |
| `app.rc`             | The Win32 resource script (next to `Cargo.toml`).       |
| `app.manifest`       | The standalone manifest, included by `app.rc`.          |

| Output                                    | Effect                                                |
| ----------------------------------------- | ----------------------------------------------------- |
| `target/.../out/app.lib`                 | Static library produced by `rc.exe` / `windres`.      |
| `cargo:rustc-link-search=native=...`      | Tells the consumer's linker where `app.lib` lives.    |
| `cargo:rustc-link-lib=dylib=app`          | Tells the consumer's linker to actually link it in.   |

## Comparison with `wxwin11_demo/build.rs`

The sibling project `wxwin11_demo` does roughly the same job but
uses the `embed-manifest = "1.4"` crate instead, which generates the
manifest XML in-code rather than consuming a standalone
`app.manifest`. See [wxwin11_demo/build.md](../../wxwin11_demo/build.md)
for the contrast.

## Cross-references

- [`Cargo.toml`](../Cargo.toml) — declares `embed-resource` as a
  build dependency.
- [`app.rc`](../app.rc) — the Win32 resource script compiled by
  this build script.
- [`app.manifest`](../app.manifest) — the manifest embedded into
  every consumer binary.
- [`wxwin11_demo/build.md`](../../wxwin11_demo/build.md) — the
  sibling manifest-embedder.
