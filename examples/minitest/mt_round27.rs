//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
#![windows_subsystem = "windows"]

use ru_wx::{
    App, ArchiveFSHandler, ArrayString, BoxSizer, BufferedOutputStream, CmdLineParser, DateSpan,
    DateTime, FileOutputStream, FileSystem, FilterOutputStream, Frame, HelpProvider, PlatformInfo,
    StaticText, StatusBar, StringTokenizer, TimeSpan, VersionInfo, WxOutputStream, ZipFSHandler,
    ZlibOutputStream,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — round 27")
        .with_size(480, 280)
        .build();
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Round 27: output streams + utilities.", 0);
    let _hint = StaticText::new(&frame, "Output streams / date / VFS:");
    let _ver = VersionInfo::new("ru_wx", env!("CARGO_PKG_VERSION"))
        .with_description("round 27 minitest");
    let _plat = PlatformInfo::current();
    let _dt = DateTime::from_ymd(2026, 6, 10).add_days(1) + TimeSpan::hours(2);
    let _span = DateSpan::months(1);
    let mut arr = ArrayString::from_slice(&["alpha", "beta"]);
    arr.add("gamma");
    let _tok = StringTokenizer::new("a,b,c", ",").collect_tokens();
    let mut cli = CmdLineParser::new();
    let _ = cli.parse(["app", "--verbose", "file.txt"]);
    let mut help = HelpProvider::new();
    help.set_default_help("No help");
    help.add_help(1, "Button help");
    let _ = help.get_help(1);
    let archive = ArchiveFSHandler::new();
    archive.add_text("readme.txt", "hello archive");
    let zip = ZipFSHandler::new();
    zip.add_text("data.txt", "hello zip");
    let fs = FileSystem::new();
    let _ = fs.read_archive("readme.txt");
    let _ = fs.read_zip("data.txt");
    let mut buf_out = BufferedOutputStream::new(256);
    let _ = buf_out.write(b"buffered");
    let mut filt_out = FilterOutputStream::new().with_append_cr(true);
    let _ = filt_out.write(b"line\n");
    let mut zlib_out = ZlibOutputStream::new();
    let _ = zlib_out.write(b"zlib");
    let tmp = std::env::temp_dir().join("ru_wx_round27_out.tmp");
    let _ = FileOutputStream::create(&tmp);
    let mut sizer = BoxSizer::vertical();
    sizer.add(_hint.as_widget_ref());
    frame.set_sizer(sizer);
    app.run(frame);
}
