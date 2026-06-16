//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
#![windows_subsystem = "windows"]

use std::sync::Arc;

use ru_wx::{
    App, ArchiveEntry, ArchiveFSHandler, BitmapHandler, BoxSizer, CalculateLayoutEvent,
    Cell, CountingInputStream, Frame, GridBlock, GridCoords, GridRange, ImageHandler,
    LogBuffer, LogChain, NcHitTestEvent, NullTarget, Point, PropertyGrid, PropertyGridIterator,
    PropertyValue, QueryLayoutEvent, RichTextStyle, Size, StaticText, StatusBar, TarEntry,
    TeeInputStream, WxInputStream,
};

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("Minitest — round 33")
        .with_size(480, 280)
        .build();
    let status = StatusBar::new(&frame, 1);
    status.set_status_text("Round 33: archive/tar + streams + layout events.", 0);
    let _hint = StaticText::new(&frame, "ArchiveEntry / GridRange / ImageHandler:");
    let archive = ArchiveFSHandler::new();
    archive.add_text("readme.txt", "archive");
    let _entry = ArchiveEntry::new("readme.txt", 7);
    let _tar = TarEntry::new("data.bin", 128);
    let _ = archive.list_entries();
    let _nc = NcHitTestEvent::new(Point::new(4, 4), 1);
    let _query = QueryLayoutEvent::new(Size::new(100, 80));
    let _calc = CalculateLayoutEvent::new(Point::new(0, 0), Size::new(200, 120));
    let range = GridRange::new(GridCoords::new(0, 0), GridCoords::new(1, 2));
    let _block = GridBlock::new(range, Cell::Text("A1".into()));
    let _ = range.contains(GridCoords::new(1, 1));
    let _png = ImageHandler::new("png");
    let _bmp = BitmapHandler::new();
    let mut counter = CountingInputStream::new(b"round33".to_vec());
    let mut buf = [0u8; 7];
    let _ = counter.read(&mut buf);
    let _ = counter.bytes_read();
    let mut tee = TeeInputStream::new(b"tee".to_vec());
    let _ = tee.read(&mut buf[..3]);
    let _ = tee.tee_data();
    let buf_target = Arc::new(LogBuffer::new());
    let _chain = LogChain::chain(Arc::new(NullTarget), buf_target.clone());
    let mut grid = PropertyGrid::new(&frame);
    grid.append("Name", PropertyValue::String("ru_wx".into()));
    let mut iter = PropertyGridIterator::new(&grid);
    let _ = iter.next();
    let _style = RichTextStyle::new("Heading").with_indent(8, 4);
    frame.on_query_layout(|_| {});
    frame.on_calculate_layout(|_| {});
    let mut sizer = BoxSizer::vertical();
    sizer.add(_hint.as_widget_ref());
    frame.set_sizer(sizer);
    app.run(frame);
}
