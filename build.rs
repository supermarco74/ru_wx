fn main() {
    // Build the Windows manifest as a static .lib via embed-resource,
    // then ALSO emit an explicit `cargo:rustc-link-lib` for downstream
    // binaries. We do both because some cargo versions only forward
    // the search path from a library build script but not the
    // `link-lib` itself; emitting it ourselves guarantees the final
    // .exe picks up `app.lib` even when the example's rustc
    // invocation doesn't inherit the build script's link line.
    if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
        embed_resource::compile("app.rc", embed_resource::NONE);
        // Re-emit the link instruction explicitly so the example's
        // link line includes `app.lib`. The path below is the
        // conventional `OUT_DIR` for the ru_wx build script.
        let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
        println!("cargo:rustc-link-search=native={out_dir}");
        println!("cargo:rustc-link-lib=dylib=app");
    }
}
