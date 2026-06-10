//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
#![windows_subsystem = "windows"]

use ru_wx::{
    App, BoxSizer, ContextHelpButton, DynamicLibrary, Environment, FileName, FileSystem, Frame,
    InputStreamExt, InternetFSHandler, LongLong, MemoryInputStream, PathEnv, PathList,
    SimpleHelpProvider, SortedArrayString, StaticText, StatusBar, StreamBase, TempDir, TextFile,
    WxDir, WxFile,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — round 28")
        .with_size(480, 280)
        .with_modern_style().build();
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Round 28: file/path utils + VFS.", 0);
    let _hint = StaticText::new(&frame, "FileName / Dir / PathList:");
    let mut fname = FileName::new("docs/readme.txt");
    fname.normalize();
    let _ = fname.extension();
    let mut paths = PathList::new();
    paths.add(".");
    paths.add_env_path("PATH");
    let _ = paths.find_valid_path("Cargo.toml");
    let dir = WxDir::new(".");
    let _ = dir.exists();
    let _tmpdir = TempDir::new("ru_wx_r28").ok();
    let wxfile = WxFile::new("Cargo.toml");
    let _ = wxfile.exists();
    let net = InternetFSHandler::new();
    net.register_text_stub("example.com/index.html", "<html/>");
    let fs = FileSystem::new();
    let _ = fs.read_internet("example.com/index.html");
    let mut help = SimpleHelpProvider::new("Application help");
    help.add_control_help(1, "Hint label");
    let help_btn = ContextHelpButton::new(&frame);
    help_btn.on_help(&frame, |_| {});
    let _ll = LongLong::new(42) + LongLong::new(8);
    let _ = Environment::get_var("PATH");
    let _ = PathEnv::get_paths();
    let mut sorted = SortedArrayString::new();
    sorted.add("zebra");
    sorted.add("alpha");
    let mut base = StreamBase::new();
    base.clear_error();
    let mut mem = MemoryInputStream::new(b"round28".to_vec());
    let _ = mem.read_all();
    let _ = TextFile::create(std::env::temp_dir().join("ru_wx_r28_lines.txt"));
    let _dll = DynamicLibrary::load("kernel32.dll");
    let mut sizer = BoxSizer::vertical();
    sizer.add(_hint.as_widget_ref());
    sizer.add(help_btn.as_widget_ref());
    frame.set_sizer(sizer);
    app.run(frame);
}
