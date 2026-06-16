//! Win11 manifest + ru_wx application icon for every binary that links
//! this crate (library, examples, tests, and downstream apps).
//!
//! `embed-resource` compiles a generated `app.rc` → `app.lib` containing:
//! - `RT_MANIFEST` (Common Controls v6 + PerMonitorV2)
//! - `RT_ICON` id 1 (Explorer / taskbar file icon for the `.exe`)

fn main() {
    if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
        println!("cargo:rerun-if-changed=assets/ru_wx_window_icon.svg");
        println!("cargo:rerun-if-changed=app.manifest");

        let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
        let out = std::path::Path::new(&out_dir);

        let ico_path = out.join("ru_wx_app.ico");
        generate_app_icon("assets/ru_wx_window_icon.svg", &ico_path);

        std::fs::copy("app.manifest", out.join("app.manifest")).expect("copy app.manifest");

        let rc_path = out.join("app.rc");
        std::fs::write(
            &rc_path,
            "#include <winuser.h>\n\
             1 ICON \"ru_wx_app.ico\"\n\
             1 RT_MANIFEST \"app.manifest\"\n",
        )
        .expect("write app.rc");

        embed_resource::compile(&rc_path, embed_resource::NONE);
        println!("cargo:rustc-link-search=native={out_dir}");
        println!("cargo:rustc-link-lib=dylib=app");
    }
}

fn generate_app_icon(svg_path: &str, ico_path: &std::path::Path) {
    let svg_bytes = std::fs::read(svg_path).unwrap_or_else(|e| {
        panic!("failed to read {svg_path}: {e}");
    });
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    for size in [16u32, 32, 48, 256] {
        let rgba = render_svg_to_rgba(&svg_bytes, size, size);
        let image = ico::IconImage::from_rgba_data(size, size, rgba);
        icon_dir
            .add_entry(ico::IconDirEntry::encode(&image).expect("encode icon entry"));
    }
    let file = std::fs::File::create(ico_path).expect("create .ico");
    icon_dir.write(file).expect("write .ico");
}

fn render_svg_to_rgba(svg_bytes: &[u8], width: u32, height: u32) -> Vec<u8> {
    let tree =
        resvg::usvg::Tree::from_data(svg_bytes, &resvg::usvg::Options::default()).expect("parse svg");
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height).expect("alloc pixmap");
    let svg_size = tree.size();
    let scale = (width as f32 / svg_size.width()).min(height as f32 / svg_size.height());
    let offset_x = (width as f32 - svg_size.width() * scale) / 2.0;
    let offset_y = (height as f32 - svg_size.height() * scale) / 2.0;
    let transform =
        resvg::tiny_skia::Transform::from_translate(offset_x, offset_y).post_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    pixmap.data().to_vec()
}
