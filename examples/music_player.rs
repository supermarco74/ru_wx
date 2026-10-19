//! Demo: lettore musicale MP3 con playlist, metadati ID3 (titolo, artista,
//! album, anno, genere, copertina), toolbar icone e drag-and-drop.
//!
//! ```bash
//! cargo run --example music_player
//! ```

#![windows_subsystem = "windows"]

use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use ru_wx::{
    Accelerator, App, AuiToolBar, Bitmap, BitmapAlign, BoxSizer, Button, Choice, Colour, DirDialog,
    DroppedFiles, FileDialog, FileDialogStyle, Font, FontDesc, FontDialog, Frame, Image, ImageList,
    ListCtrl, ListCtrlStyle, MediaCtrl, MediaState, Menu, MenuBar, Panel, PopupMenu, RawBitmap,
    SearchCtrl, Slider, StaticBitmap, StaticText, StatusBar, TextCtrl, Timer,
};

const PL_COL_TITLE: usize = 0;
const PL_COL_DURATION: usize = 1;
const PL_COL_ARTIST: usize = 2;
const PL_COL_GENRE: usize = 3;
const PL_COL_COMMENT: usize = 4;
const PL_COL_PATH: usize = 5;

const PL_COL_NAMES: [&str; 6] = [
    "Titolo", "Durata", "Autore", "Genere", "Commento", "Percorso",
];

const ID_TOOL_LOAD: u16 = 2001;
const ID_TOOL_LOAD_PLAYLIST: u16 = 2002;
const ID_TOOL_SAVE_PLAYLIST: u16 = 2003;
const ID_TOOL_CATALOG: u16 = 2004;

const CAT_COL_TITLE: usize = 0;
const CAT_COL_DURATION: usize = 1;
const CAT_COL_ARTIST: usize = 2;
const CAT_COL_GENRE: usize = 3;
const CAT_COL_TYPE: usize = 4;
const CAT_COL_PATH: usize = 5;

const CAT_COL_NAMES: [&str; 6] = ["Titolo", "Durata", "Autore", "Genere", "Tipo", "Percorso"];

// ── Palette (tema chiaro) ───────────────────────────────────────────
const CLR_BG: Colour = Colour::new(244, 246, 250, 255);
const CLR_SURFACE: Colour = Colour::new(255, 255, 255, 255);
const CLR_SURFACE_ALT: Colour = Colour::new(248, 250, 252, 255);
const CLR_ACCENT: Colour = Colour::new(79, 70, 229, 255);
const CLR_TEXT: Colour = Colour::new(30, 41, 59, 255);
const CLR_MUTED: Colour = Colour::new(100, 116, 139, 255);
const CLR_DIM: Colour = Colour::new(148, 163, 184, 255);

const ICON_COVER_PLACEHOLDER: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 160 160"><rect width="160" height="160" rx="10" fill="#eef2ff"/><circle cx="80" cy="80" r="46" fill="none" stroke="#6366f1" stroke-width="2.5" opacity="0.5"/><circle cx="80" cy="80" r="34" fill="none" stroke="#818cf8" stroke-width="1.5" opacity="0.35"/><circle cx="80" cy="80" r="9" fill="#6366f1"/><circle cx="80" cy="80" r="46" fill="none" stroke="#c7d2fe" stroke-width="6" stroke-dasharray="3 8" opacity="0.6"/></svg>"##;

const AUDIO_EXT: &[&str] = &["mp3", "wav", "wma", "flac", "ogg", "m4a"];
const COVER_SIZE: u32 = 160;
const TRANSPORT_BTN: i32 = 56;
const CATALOG_BTN: i32 = 40;
const CATALOG_TRANSPORT_H: u32 = 88;
const CATALOG_W: u32 = 1040;
const CATALOG_H: u32 = 680;
const CATALOG_ROW_H: f64 = 28.0;
const STATUS_BAR_RESERVE: i32 = 24;
const TRANSPORT_PANEL_H: u32 = 132;

fn transport_icon_button<W: ru_wx::Window>(parent: &W, svg: &[u8]) -> Button {
    let btn =
        Button::new_with_svg_aligned(parent, "", svg, TRANSPORT_BTN as u32, BitmapAlign::Center);
    btn.as_widget_ref()
        .borrow_mut()
        .set_size(TRANSPORT_BTN as u32, TRANSPORT_BTN as u32);
    btn
}
const DEFAULT_PLAYLIST_NAME: &str = "NuovoFile";
const LABEL_W: u32 = 248;

fn playlist_name_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_PLAYLIST_NAME)
        .to_string()
}

fn normalize_playlist_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        DEFAULT_PLAYLIST_NAME.to_string()
    } else {
        trimmed.to_string()
    }
}

fn playlist_save_filename(name: &str) -> String {
    let base = normalize_playlist_name(name);
    if base.ends_with(".m3u") || base.ends_with(".m3u8") || base.ends_with(".txt") {
        base
    } else {
        format!("{base}.m3u")
    }
}

const ICON_LOAD: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0%" stop-color="#34d399"/><stop offset="100%" stop-color="#059669"/></linearGradient></defs><rect x="2" y="2" width="20" height="20" rx="5" fill="url(#g)"/><path d="M12 6v8 M8 10l4-4 4 4 M6 18h12" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round"/></svg>"##;
const ICON_PLAYLIST: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0%" stop-color="#a855f7"/><stop offset="100%" stop-color="#7c3aed"/></linearGradient></defs><rect x="2" y="2" width="20" height="20" rx="5" fill="url(#g)"/><path d="M7 8h10 M7 12h10 M7 16h6" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round"/><path d="M17 14l3 2-3 2z" fill="white"/></svg>"##;
const ICON_SAVE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0%" stop-color="#60a5fa"/><stop offset="100%" stop-color="#2563eb"/></linearGradient></defs><rect x="2" y="2" width="20" height="20" rx="5" fill="url(#g)"/><path d="M8 7h8v4H8z M8 15h8 M6 6h12v12H6z" fill="none" stroke="white" stroke-width="1.8" stroke-linejoin="round"/></svg>"##;
const ICON_CATALOG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0%" stop-color="#f97316"/><stop offset="100%" stop-color="#ea580c"/></linearGradient></defs><rect x="2" y="2" width="20" height="20" rx="5" fill="url(#g)"/><rect x="6" y="6" width="5" height="5" rx="1" fill="white" opacity="0.95"/><rect x="13" y="6" width="5" height="5" rx="1" fill="white" opacity="0.85"/><rect x="6" y="13" width="5" height="5" rx="1" fill="white" opacity="0.85"/><rect x="13" y="13" width="5" height="5" rx="1" fill="white" opacity="0.75"/></svg>"##;
const ICON_FOLDER: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0%" stop-color="#fbbf24"/><stop offset="100%" stop-color="#d97706"/></linearGradient></defs><path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z" fill="url(#g)"/><path d="M3 9h18" stroke="white" stroke-width="1.2" opacity="0.5"/></svg>"##;
const ICON_PLAY: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48"><circle cx="24" cy="24" r="22" fill="#22c55e"/><path d="M19 15l16 9-16 9z" fill="white"/></svg>"##;
const ICON_PAUSE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48"><circle cx="24" cy="24" r="22" fill="#f59e0b"/><rect x="17" y="15" width="5" height="18" rx="1.5" fill="white"/><rect x="26" y="15" width="5" height="18" rx="1.5" fill="white"/></svg>"##;
const ICON_STOP: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48"><circle cx="24" cy="24" r="22" fill="#ef4444"/><rect x="16" y="16" width="16" height="16" rx="2" fill="white"/></svg>"##;

/// ID3v1 genre index → name (Winamp / ID3 spec list).
const ID3_GENRES: [&str; 192] = [
    "Blues",
    "Classic Rock",
    "Country",
    "Dance",
    "Disco",
    "Funk",
    "Grunge",
    "Hip-Hop",
    "Jazz",
    "Metal",
    "New Age",
    "Oldies",
    "Other",
    "Pop",
    "R&B",
    "Rap",
    "Reggae",
    "Rock",
    "Techno",
    "Industrial",
    "Alternative",
    "Ska",
    "Death Metal",
    "Pranks",
    "Soundtrack",
    "Euro-Techno",
    "Ambient",
    "Trip-Hop",
    "Vocal",
    "Jazz+Funk",
    "Fusion",
    "Trance",
    "Classical",
    "Instrumental",
    "Acid",
    "House",
    "Game",
    "Sound Clip",
    "Gospel",
    "Noise",
    "Alt. Rock",
    "Bass",
    "Soul",
    "Punk",
    "Space",
    "Meditative",
    "Instrumental Pop",
    "Instrumental Rock",
    "Ethnic",
    "Gothic",
    "Darkwave",
    "Techno-Industrial",
    "Electronic",
    "Pop-Folk",
    "Eurodance",
    "Dream",
    "Southern Rock",
    "Comedy",
    "Cult",
    "Gangsta Rap",
    "Top 40",
    "Christian Rap",
    "Pop/Funk",
    "Jungle",
    "Native American",
    "Cabaret",
    "New Wave",
    "Psychedelic",
    "Rave",
    "Showtunes",
    "Trailer",
    "Lo-Fi",
    "Tribal",
    "Acid Punk",
    "Acid Jazz",
    "Polka",
    "Retro",
    "Musical",
    "Rock & Roll",
    "Hard Rock",
    "Folk",
    "Folk-Rock",
    "National Folk",
    "Swing",
    "Fast Fusion",
    "Bebop",
    "Latin",
    "Revival",
    "Celtic",
    "Bluegrass",
    "Avantgarde",
    "Gothic Rock",
    "Progressive Rock",
    "Psychedelic Rock",
    "Symphonic Rock",
    "Slow Rock",
    "Big Band",
    "Chorus",
    "Easy Listening",
    "Acoustic",
    "Humour",
    "Speech",
    "Chanson",
    "Opera",
    "Chamber Music",
    "Sonata",
    "Symphony",
    "Booty Bass",
    "Primus",
    "Porn Groove",
    "Satire",
    "Slow Jam",
    "Club",
    "Tango",
    "Samba",
    "Folklore",
    "Ballad",
    "Power Ballad",
    "Rhythmic Soul",
    "Freestyle",
    "Duet",
    "Punk Rock",
    "Drum Solo",
    "A Cappella",
    "Euro-House",
    "Dance Hall",
    "Goa",
    "Drum & Bass",
    "Club-House",
    "Hardcore",
    "Terror",
    "Indie",
    "BritPop",
    "Afro-Punk",
    "Polsk Punk",
    "Beat",
    "Christian Gangsta",
    "Heavy Metal",
    "Black Metal",
    "Crossover",
    "Contemporary Christian",
    "Christian Rock",
    "Merengue",
    "Salsa",
    "Thrash Metal",
    "Anime",
    "JPop",
    "Synthpop",
    "Abstract",
    "Art Rock",
    "Baroque",
    "Bhangra",
    "Big Beat",
    "Breakbeat",
    "Chillout",
    "Downtempo",
    "Dub",
    "EBM",
    "Eclectic",
    "Electro",
    "Electroclash",
    "Emo",
    "Experimental",
    "Garage",
    "Global",
    "IDM",
    "Illbient",
    "Industro-Goth",
    "Jam Band",
    "Krautrock",
    "Leftfield",
    "Lounge",
    "Math Rock",
    "New Romantic",
    "Nu-Breakz",
    "Post-Punk",
    "Post-Rock",
    "Psytrance",
    "Shoegaze",
    "Space Rock",
    "Trop Rock",
    "World Music",
    "Neoclassical",
    "Audiobook",
    "Audio Theatre",
    "Neue Deutsche Welle",
    "Podcast",
    "Indie Rock",
    "G-Funk",
    "Dubstep",
    "Garage Rock",
    "Psybient",
];

/// Wall-clock progress when MCI `status position` is unavailable or stale.
struct ProgressClock {
    anchor_ms: u64,
    length_ms: u64,
    running: bool,
    since: Option<Instant>,
    /// True after the user drags the slider until AVPlayer catches up.
    seek_pending: bool,
}

impl ProgressClock {
    fn new() -> Self {
        Self {
            anchor_ms: 0,
            length_ms: 0,
            running: false,
            since: None,
            seek_pending: false,
        }
    }

    fn begin(&mut self, length_ms: u64) {
        self.anchor_ms = 0;
        self.length_ms = length_ms;
        self.running = true;
        self.since = Some(Instant::now());
        self.seek_pending = false;
    }

    fn pause(&mut self) {
        if let Some(t) = self.since.take() {
            self.anchor_ms += t.elapsed().as_millis() as u64;
        }
        self.running = false;
    }

    fn resume(&mut self) {
        if !self.running {
            self.running = true;
            self.since = Some(Instant::now());
        }
    }

    fn stop(&mut self) {
        self.anchor_ms = 0;
        self.running = false;
        self.since = None;
        self.seek_pending = false;
    }

    fn seek(&mut self, ms: u64) {
        self.anchor_ms = ms.min(self.length_ms);
        self.seek_pending = true;
        if self.running {
            self.since = Some(Instant::now());
        }
    }

    fn set_length(&mut self, length_ms: u64) {
        if length_ms > 0 {
            self.length_ms = length_ms;
        }
    }

    fn sync_from_mci(&mut self, mci_pos: u64) {
        if self.seek_pending {
            let delta = (mci_pos as i64 - self.anchor_ms as i64).abs();
            if delta > 2000 {
                return;
            }
            self.seek_pending = false;
        } else if mci_pos + 1500 < self.anchor_ms {
            // AVPlayer still reports the old position right after a seek.
            return;
        }

        if mci_pos > self.anchor_ms.saturating_sub(1500) {
            self.anchor_ms = mci_pos.min(self.length_ms);
            if self.running {
                self.since = Some(Instant::now());
            }
        }
    }

    fn position_ms(&self) -> u64 {
        let extra = if self.running {
            self.since
                .map(|t| t.elapsed().as_millis() as u64)
                .unwrap_or(0)
        } else {
            0
        };
        (self.anchor_ms + extra).min(self.length_ms)
    }
}

fn format_ms(ms: u64) -> String {
    let total = ms / 1000;
    format!("{:02}:{:02}", total / 60, total % 60)
}

fn format_file_size(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes / 1024)
    } else {
        format!("{bytes} B")
    }
}

fn is_catalog_media(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("mp3") || e.eq_ignore_ascii_case("mp4"))
        .unwrap_or(false)
}

fn is_playlist_media(path: &Path) -> bool {
    is_audio_file(path) || is_catalog_media(path)
}

fn media_type_label(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_uppercase())
        .unwrap_or_else(|| "?".to_string())
}

fn collect_catalog_media(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(read) = fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_catalog_media(&path, out);
        } else if is_catalog_media(&path) {
            out.push(path);
        }
    }
}

fn scan_catalog_dir(root: &Path) -> Vec<Track> {
    let mut paths = Vec::new();
    collect_catalog_media(root, &mut paths);
    paths.sort();
    paths.into_iter().map(Track::from_path).collect()
}

fn catalog_icon_button<W: ru_wx::Window>(parent: &W, svg: &[u8]) -> Button {
    let btn =
        Button::new_with_svg_aligned(parent, "", svg, CATALOG_BTN as u32, BitmapAlign::Center);
    btn.as_widget_ref()
        .borrow_mut()
        .set_size(CATALOG_BTN as u32, CATALOG_BTN as u32);
    btn
}

fn insert_catalog_row(list: &ListCtrl, row: usize, track: &Track) {
    let item = list.insert_item(row, &track.meta.title);
    list.set_item_text(item, CAT_COL_DURATION, &duration_cell(track.duration_ms));
    list.set_item_text(
        item,
        CAT_COL_ARTIST,
        &cell_or_dash(track.meta.display_artist()),
    );
    list.set_item_text(item, CAT_COL_GENRE, &cell_or_dash(&track.meta.genre));
    list.set_item_text(item, CAT_COL_TYPE, &media_type_label(&track.path));
    list.set_item_text(item, CAT_COL_PATH, &track.path.display().to_string());
}

fn path_from_catalog_row(list: &ListCtrl, row: usize) -> Option<PathBuf> {
    let path_str = list.get_item_text(row, CAT_COL_PATH);
    if path_str.is_empty() {
        return None;
    }
    let path = PathBuf::from(&path_str);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

struct CatalogView {
    filter: String,
    row_map: Vec<usize>,
}

impl CatalogView {
    fn new() -> Self {
        Self {
            filter: String::new(),
            row_map: Vec::new(),
        }
    }

    fn track_index_for_row(&self, row: usize) -> Option<usize> {
        self.row_map.get(row).copied()
    }
}

fn rebuild_catalog_view(
    list: &ListCtrl,
    view: &mut CatalogView,
    tracks: &[Track],
    select_index: Option<usize>,
) {
    let mut indices: Vec<usize> = (0..tracks.len())
        .filter(|&i| track_matches_filter(&tracks[i], &view.filter))
        .collect();
    indices.sort_by(|&a, &b| {
        tracks[a]
            .path
            .display()
            .to_string()
            .to_lowercase()
            .cmp(&tracks[b].path.display().to_string().to_lowercase())
    });
    view.row_map = indices.clone();
    list.delete_all_items();
    for (row, &ti) in indices.iter().enumerate() {
        insert_catalog_row(list, row, &tracks[ti]);
    }
    if let Some(ti) = select_index {
        if let Some(row) = view.row_map.iter().position(|&i| i == ti) {
            list.select(row);
        }
    }
}

fn open_catalog_window(parent: &Frame, append_to_playlist: Rc<dyn Fn(PathBuf)>) -> Frame {
    let cat_frame = Frame::builder()
        .with_title("Catalogo — ru_wx Music Player")
        .with_size(CATALOG_W, CATALOG_H)
        .with_quit_on_destroy(false)
        .build();
    cat_frame.set_background_colour(CLR_BG);

    let status = StatusBar::new(&cat_frame, 1);
    status.set_status_text("Seleziona una cartella per cercare MP3 e MP4.", 0);

    let panel = Panel::new(&cat_frame);
    panel.set_background_colour(CLR_SURFACE);

    let btn_folder = catalog_icon_button(&panel, ICON_FOLDER);
    let path_field = TextCtrl::new(&panel, "");
    path_field.as_widget_ref().borrow_mut().set_size(640, 28);

    let search_label = StaticText::new(&panel, "Cerca");
    style_muted(&search_label, 48, 20, 1);
    let search = SearchCtrl::new(&panel, "Titolo, artista, genere, percorso…");
    search.as_widget_ref().borrow_mut().set_size(280, 26);

    let btn_add = Button::new(&panel, "Aggiungi a playlist");
    btn_add.as_widget_ref().borrow_mut().set_size(160, 30);

    let file_list = ListCtrl::new(&panel, ListCtrlStyle::Report);
    for (i, name) in CAT_COL_NAMES.iter().enumerate() {
        let w = match i {
            CAT_COL_TITLE => 220,
            CAT_COL_DURATION => 64,
            CAT_COL_ARTIST => 150,
            CAT_COL_GENRE => 110,
            CAT_COL_TYPE => 52,
            CAT_COL_PATH => 420,
            _ => 120,
        };
        file_list.insert_column(i as u32, name, w);
    }
    file_list.set_row_height(CATALOG_ROW_H);
    file_list.set_extended_style(0x20); // LVS_EX_FULLROWSELECT
    file_list
        .as_widget_ref()
        .borrow_mut()
        .set_size(CATALOG_W - 40, 480);

    let transport_panel = Panel::new(&cat_frame);
    transport_panel.set_background_colour(CLR_SURFACE);

    let time_label = StaticText::new(&transport_panel, "00:00 / 00:00");
    style_heading(&time_label, 13, false, CLR_MUTED, 160, 20, 1);
    let progress = Slider::new(&transport_panel, 0, 1000, 0);
    progress
        .as_widget_ref()
        .borrow_mut()
        .set_size(CATALOG_W - 80, 28);
    let btn_play = transport_icon_button(&transport_panel, ICON_PLAY);

    let media = Rc::new(RefCell::new(MediaCtrl::new(&cat_frame)));
    let entries = Rc::new(RefCell::new(Vec::<Track>::new()));
    let catalog_view = Rc::new(RefCell::new(CatalogView::new()));
    let root_path = Rc::new(RefCell::new(None::<PathBuf>));
    let programmatic_slider = Rc::new(Cell::new(false));
    let progress_clock = Rc::new(RefCell::new(ProgressClock::new()));
    let progress_timer = Rc::new(Timer::new(&cat_frame));

    let rebuild_catalog: Rc<dyn Fn(Option<usize>)> = {
        let file_list = file_list.clone();
        let entries = entries.clone();
        let view = catalog_view.clone();
        Rc::new(move |select| {
            let tracks = entries.borrow().clone();
            rebuild_catalog_view(&file_list, &mut view.borrow_mut(), &tracks, select);
        })
    };

    let scan_folder: Rc<dyn Fn(PathBuf)> = {
        let entries = entries.clone();
        let path_field = path_field.clone();
        let status = status.clone();
        let rebuild = rebuild_catalog.clone();
        let root_path = root_path.clone();
        Rc::new(move |dir| {
            if !dir.is_dir() {
                status.set_status_text("Percorso cartella non valido.", 0);
                return;
            }
            status.set_status_text("Scansione in corso…", 0);
            let found = scan_catalog_dir(&dir);
            let n = found.len();
            let dir_label = dir.display().to_string();
            path_field.set_value(&dir_label);
            *root_path.borrow_mut() = Some(dir);
            *entries.borrow_mut() = found;
            rebuild(None);
            status.set_status_text(&format!("Trovati {n} file MP3/MP4 in {dir_label}"), 0);
        })
    };

    let browse_folder: Rc<dyn Fn()> = {
        let parent = parent.clone();
        let scan = scan_folder.clone();
        let path_field = path_field.clone();
        Rc::new(move || {
            let initial = path_field.get_value();
            let mut builder = DirDialog::builder(&parent).with_title("Seleziona cartella");
            if !initial.trim().is_empty() {
                builder = builder.with_initial_directory(initial.trim());
            }
            if let Some(chosen) = builder.show_modal() {
                scan(PathBuf::from(chosen));
            }
        })
    };

    let play_path: Rc<dyn Fn(PathBuf)> = {
        let media = media.clone();
        let status = status.clone();
        let time_label = time_label.clone();
        let progress = progress.clone();
        let programmatic_slider = programmatic_slider.clone();
        let clock = progress_clock.clone();
        let progress_timer = progress_timer.clone();
        Rc::new(move |path| {
            if let Err(e) = media.borrow().load(&path) {
                status.set_status_text(&format!("Errore caricamento: {e}"), 0);
                return;
            }
            if let Err(e) = media.borrow().play() {
                status.set_status_text(&format!("Errore riproduzione: {e}"), 0);
                return;
            }
            let len = media.borrow().length_ms().unwrap_or(0).max(1);
            status.set_status_text(
                &format!(
                    "Anteprima: {}",
                    path.file_name().and_then(|s| s.to_str()).unwrap_or("?")
                ),
                0,
            );
            clock.borrow_mut().begin(len);
            let max = len.min(i32::MAX as u64) as i32;
            progress.set_range(0, max);
            programmatic_slider.set(true);
            progress.set_value(0);
            programmatic_slider.set(false);
            time_label.set_label(&format!("00:00 / {}", format_ms(len)));
            progress_timer.start(Duration::from_millis(250));
        })
    };

    let play_row: Rc<dyn Fn(usize)> = {
        let file_list = file_list.clone();
        let play_path = play_path.clone();
        Rc::new(move |row| {
            if let Some(path) = path_from_catalog_row(&file_list, row) {
                play_path(path);
            }
        })
    };

    btn_folder.on_click(&cat_frame, {
        let browse = browse_folder.clone();
        move || browse()
    });

    path_field.on_change(&cat_frame, {
        let scan = scan_folder.clone();
        let path_field = path_field.clone();
        move || {
            let text = path_field.get_value();
            let path = PathBuf::from(text.trim());
            if path.is_dir() {
                scan(path);
            }
        }
    });

    search.on_change(&cat_frame, {
        let search_field = search.clone();
        let view = catalog_view.clone();
        let rebuild = rebuild_catalog.clone();
        let status = status.clone();
        let entries = entries.clone();
        move || {
            view.borrow_mut().filter = search_field.value();
            rebuild(None);
            let shown = view.borrow().row_map.len();
            let total = entries.borrow().len();
            status.set_status_text(&format!("Mostrati {shown} di {total} file"), 0);
        }
    });

    btn_add.on_click(&cat_frame, {
        let file_list = file_list.clone();
        let view = catalog_view.clone();
        let entries = entries.clone();
        let append = append_to_playlist.clone();
        let status = status.clone();
        move || {
            let selected_rows = file_list.get_selected_items();
            let indices: Vec<usize> = if selected_rows.is_empty() {
                file_list
                    .get_selected_item()
                    .and_then(|row| view.borrow().track_index_for_row(row))
                    .into_iter()
                    .collect()
            } else {
                selected_rows
                    .into_iter()
                    .filter_map(|row| view.borrow().track_index_for_row(row))
                    .collect()
            };
            if indices.is_empty() {
                status.set_status_text("Seleziona uno o più brani da aggiungere.", 0);
                return;
            }
            let tracks = entries.borrow();
            let mut added = 0usize;
            for idx in indices {
                if let Some(track) = tracks.get(idx) {
                    append(track.path.clone());
                    added += 1;
                }
            }
            status.set_status_text(&format!("Aggiunti {added} brani alla playlist."), 0);
        }
    });

    btn_play.on_click(&cat_frame, {
        let file_list = file_list.clone();
        let play_row = play_row.clone();
        move || {
            if let Some(row) = file_list.get_selected_item().or(Some(0)) {
                play_row(row);
            }
        }
    });

    file_list.on_item_selected(&cat_frame, {
        let view = catalog_view.clone();
        let entries = entries.clone();
        let status = status.clone();
        move |row| {
            let Some(row) = row else {
                return;
            };
            if let Some(ti) = view.borrow().track_index_for_row(row) {
                if let Some(track) = entries.borrow().get(ti) {
                    let title = if track.meta.title.is_empty() {
                        track
                            .path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("?")
                            .to_string()
                    } else {
                        track.meta.title.clone()
                    };
                    status.set_status_text(
                        &format!(
                            "{} — {} ({})",
                            title,
                            media_type_label(&track.path),
                            format_file_size(track.size_bytes)
                        ),
                        0,
                    );
                }
            }
        }
    });

    file_list.on_item_double_click(&cat_frame, {
        let play_row = play_row.clone();
        move |row| play_row(row)
    });

    progress.on_value_change(&cat_frame, {
        let media = media.clone();
        let progress = progress.clone();
        let programmatic = programmatic_slider.clone();
        let clock = progress_clock.clone();
        let time_label = time_label.clone();
        move || {
            if programmatic.get() {
                return;
            }
            let pos = progress.get_value().max(0) as u64;
            clock.borrow_mut().seek(pos);
            let _ = media.borrow().seek_ms(pos);
            let len = media
                .borrow()
                .length_ms()
                .unwrap_or_else(|| clock.borrow().length_ms)
                .max(1);
            time_label.set_label(&format!("{} / {}", format_ms(pos), format_ms(len)));
        }
    });

    let progress_tick = progress.clone();
    let time_tick = time_label.clone();
    let media_tick = media.clone();
    let programmatic_tick = programmatic_slider.clone();
    let clock_tick = progress_clock.clone();
    progress_timer.on_tick(move || {
        if !clock_tick.borrow().running {
            return;
        }
        let state = media_tick.borrow().state();
        if state != MediaState::Playing && state != MediaState::Paused {
            return;
        }
        let (len, pos) = {
            let len = media_tick.borrow().length_ms().unwrap_or(0).max(1);
            if let Some(p) = media_tick.borrow().position_ms() {
                clock_tick.borrow_mut().sync_from_mci(p);
            }
            let pos = clock_tick.borrow().position_ms();
            (len, pos)
        };
        let pos_i = pos.min(i32::MAX as u64) as i32;
        let len_i = len.min(i32::MAX as u64) as i32;
        progress_tick.set_range(0, len_i);
        if !programmatic_tick.get() {
            programmatic_tick.set(true);
            progress_tick.set_value(pos_i);
            programmatic_tick.set(false);
        }
        time_tick.set_label(&format!("{} / {}", format_ms(pos), format_ms(len)));
    });

    let mut top_row = BoxSizer::horizontal();
    top_row.add(btn_folder.as_widget_ref());
    top_row.add_spacer(8);
    top_row.add_with_proportion(path_field.as_widget_ref(), 1);

    let mut filter_row = BoxSizer::horizontal();
    filter_row.add(search_label.as_widget_ref());
    filter_row.add_spacer(6);
    filter_row.add_with_proportion(search.as_widget_ref(), 1);
    filter_row.add_spacer(12);
    filter_row.add(btn_add.as_widget_ref());

    let mut panel_inner = BoxSizer::vertical();
    panel_inner.set_padding(12);
    panel_inner.add_sizer(top_row);
    panel_inner.add_spacer(8);
    panel_inner.add_sizer(filter_row);
    panel_inner.add_spacer(8);
    panel_inner.add_with_proportion(file_list.as_widget_ref(), 1);
    panel.set_sizer(panel_inner);

    let mut transport = BoxSizer::horizontal();
    transport.add_stretch(1);
    transport.add(btn_play.as_widget_ref());
    transport.add_stretch(1);

    let mut transport_inner = BoxSizer::vertical();
    transport_inner.set_padding(8);
    transport_inner.add_sizer(transport);
    transport_inner.add_spacer(8);
    transport_inner.add(progress.as_widget_ref());
    transport_inner.add_spacer(4);
    transport_inner.add(time_label.as_widget_ref());
    transport_panel.set_sizer(transport_inner);
    transport_panel
        .as_widget_ref()
        .borrow_mut()
        .set_size(CATALOG_W - 40, CATALOG_TRANSPORT_H);

    let mut sizer = BoxSizer::vertical();
    sizer.set_padding(8);
    sizer.add_with_proportion(panel.as_widget_ref(), 1);
    sizer.add_spacer(8);
    sizer.add(transport_panel.as_widget_ref());
    sizer.add_spacer(STATUS_BAR_RESERVE);
    cat_frame.set_sizer(sizer);

    #[cfg(target_os = "macos")]
    {
        use ru_wx::platform::appkit::{button_rewire_click_target, mac_view_raise_to_front};
        let panel_h = transport_panel.hwnd();
        let play_id = btn_play.id();
        let slider_id = progress.id();
        let play_h = btn_play.as_widget_ref().borrow().native_handle();
        let slider_h = progress.as_widget_ref().borrow().native_handle();
        let folder_h = btn_folder.as_widget_ref().borrow().native_handle();
        ru_wx::platform::appkit::run_on_next_event(move || {
            mac_view_raise_to_front(panel_h);
            mac_view_raise_to_front(play_h);
            mac_view_raise_to_front(slider_h);
            mac_view_raise_to_front(folder_h);
            button_rewire_click_target(play_h, panel_h, play_id);
            button_rewire_click_target(slider_h, panel_h, slider_id);
        });
    }

    let _media_keepalive = media;
    cat_frame
}

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| AUDIO_EXT.iter().any(|a| e.eq_ignore_ascii_case(a)))
        .unwrap_or(false)
}

#[derive(Clone, Default)]
struct TrackMeta {
    title: String,
    artist: String,
    album_artist: String,
    album: String,
    year: String,
    genre: String,
    track_num: String,
    composer: String,
    comment: String,
    /// Milliseconds from ID3 `TLEN`, if present.
    duration_ms: u64,
    /// Cover art bytes (JPEG/PNG) from ID3 APIC frame.
    cover: Option<Vec<u8>>,
}

impl TrackMeta {
    fn display_artist(&self) -> &str {
        if !self.artist.is_empty() {
            &self.artist
        } else if !self.album_artist.is_empty() {
            &self.album_artist
        } else {
            ""
        }
    }
}

#[derive(Clone)]
struct Track {
    path: PathBuf,
    meta: TrackMeta,
    size_bytes: u64,
    duration_ms: u64,
}

impl Track {
    fn from_path(path: PathBuf) -> Self {
        let size_bytes = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let mut meta = read_audio_metadata(&path);
        if meta.title.is_empty() {
            meta.title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Senza titolo")
                .to_string();
        }
        let duration_ms = if meta.duration_ms > 0 {
            meta.duration_ms
        } else {
            estimate_audio_duration_ms(&path, size_bytes)
        };
        Self {
            path,
            meta,
            size_bytes,
            duration_ms,
        }
    }
}

fn cell_or_dash(value: &str) -> String {
    if value.trim().is_empty() {
        "—".to_string()
    } else {
        value.to_string()
    }
}

fn duration_cell(ms: u64) -> String {
    if ms == 0 {
        "—".to_string()
    } else {
        format_ms(ms)
    }
}

fn insert_playlist_row(list: &ListCtrl, row: usize, track: &Track) -> usize {
    let item = list.insert_item(row, &track.meta.title);
    list.set_item_text(item, PL_COL_DURATION, &duration_cell(track.duration_ms));
    list.set_item_text(
        item,
        PL_COL_ARTIST,
        &cell_or_dash(track.meta.display_artist()),
    );
    list.set_item_text(item, PL_COL_GENRE, &cell_or_dash(&track.meta.genre));
    list.set_item_text(item, PL_COL_COMMENT, &cell_or_dash(&track.meta.comment));
    list.set_item_text(item, PL_COL_PATH, &track.path.display().to_string());
    item
}

fn playlist_cell_value(list: &ListCtrl, row: usize, col: usize) -> String {
    list.get_item_text(row, col)
}

fn path_from_playlist_row(list: &ListCtrl, row: usize) -> Option<PathBuf> {
    let path_str = list.get_item_text(row, PL_COL_PATH);
    if path_str.is_empty() {
        return None;
    }
    let path = PathBuf::from(&path_str);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

fn find_track_index_by_path(tracks: &[Track], path: &Path) -> Option<usize> {
    tracks.iter().position(|t| t.path == path)
}

/// Colonna di ordinamento playlist (0 = ordine di caricamento).
#[derive(Clone, Copy, PartialEq, Eq)]
enum PlaylistSort {
    Manual,
    Title,
    Duration,
    Artist,
    Genre,
    Comment,
    Path,
}

impl PlaylistSort {
    fn from_choice_index(i: usize) -> Self {
        match i {
            1 => Self::Title,
            2 => Self::Duration,
            3 => Self::Artist,
            4 => Self::Genre,
            5 => Self::Comment,
            6 => Self::Path,
            _ => Self::Manual,
        }
    }
}

/// Stato vista tabella: filtro testuale, ordinamento e mappa riga → indice traccia.
struct PlaylistView {
    filter: String,
    sort: PlaylistSort,
    sort_asc: bool,
    /// `row_map[row]` = indice in `Playlist.tracks`.
    row_map: Vec<usize>,
}

impl PlaylistView {
    fn new() -> Self {
        Self {
            filter: String::new(),
            sort: PlaylistSort::Manual,
            sort_asc: true,
            row_map: Vec::new(),
        }
    }

    fn track_index_for_row(&self, row: usize) -> Option<usize> {
        self.row_map.get(row).copied()
    }

    fn row_for_track(&self, track_index: usize) -> Option<usize> {
        self.row_map.iter().position(|&i| i == track_index)
    }
}

fn track_matches_filter(track: &Track, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    let haystack = format!(
        "{} {} {} {} {} {}",
        track.meta.title,
        track.meta.display_artist(),
        track.meta.album,
        track.meta.genre,
        track.meta.comment,
        track
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
    )
    .to_lowercase();
    haystack.contains(&q)
}

fn sort_track_indices(indices: &mut [usize], tracks: &[Track], sort: PlaylistSort, asc: bool) {
    if sort == PlaylistSort::Manual {
        return;
    }
    indices.sort_by(|&a, &b| {
        let ta = &tracks[a];
        let tb = &tracks[b];
        let ord = match sort {
            PlaylistSort::Manual => a.cmp(&b),
            PlaylistSort::Title => ta
                .meta
                .title
                .to_lowercase()
                .cmp(&tb.meta.title.to_lowercase()),
            PlaylistSort::Duration => ta.duration_ms.cmp(&tb.duration_ms),
            PlaylistSort::Artist => ta
                .meta
                .display_artist()
                .to_lowercase()
                .cmp(&tb.meta.display_artist().to_lowercase()),
            PlaylistSort::Genre => ta
                .meta
                .genre
                .to_lowercase()
                .cmp(&tb.meta.genre.to_lowercase()),
            PlaylistSort::Comment => ta
                .meta
                .comment
                .to_lowercase()
                .cmp(&tb.meta.comment.to_lowercase()),
            PlaylistSort::Path => ta
                .path
                .display()
                .to_string()
                .to_lowercase()
                .cmp(&tb.path.display().to_string().to_lowercase()),
        };
        if asc {
            ord
        } else {
            ord.reverse()
        }
    });
}

fn rebuild_playlist_view(
    list: &ListCtrl,
    view: &mut PlaylistView,
    tracks: &[Track],
    select_track: Option<usize>,
) {
    let mut indices: Vec<usize> = (0..tracks.len())
        .filter(|&i| track_matches_filter(&tracks[i], &view.filter))
        .collect();
    sort_track_indices(&mut indices, tracks, view.sort, view.sort_asc);
    view.row_map = indices.clone();

    list.delete_all_items();
    for (row, &ti) in indices.iter().enumerate() {
        insert_playlist_row(list, row, &tracks[ti]);
    }
    if let Some(ti) = select_track {
        if let Some(row) = view.row_for_track(ti) {
            list.select(row);
        }
    }
}

fn sort_direction_label(asc: bool) -> &'static str {
    if asc {
        "A→Z"
    } else {
        "Z→A"
    }
}

/// Rough length when tags omit `TLEN` (128 kbps CBR assumption for MP3).
fn estimate_audio_duration_ms(path: &Path, size_bytes: u64) -> u64 {
    if size_bytes == 0 {
        return 0;
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext.eq_ignore_ascii_case("wav") {
        if let Ok(data) = fs::read(path) {
            if data.len() >= 44 && &data[0..4] == b"RIFF" && &data[8..12] == b"WAVE" {
                let byte_rate =
                    u64::from(u32::from_le_bytes([data[28], data[29], data[30], data[31]]));
                if let Some(duration) =
                    (size_bytes.saturating_sub(44) * 1000).checked_div(byte_rate)
                {
                    return duration;
                }
            }
        }
        return 0;
    }
    if ext.eq_ignore_ascii_case("mp3") {
        return size_bytes * 1000 / 16_000;
    }
    if ext.eq_ignore_ascii_case("mp4") {
        return size_bytes * 1000 / 200_000;
    }
    0
}

fn decode_genre(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        if let Ok(n) = trimmed[1..trimmed.len() - 1].parse::<usize>() {
            if n < ID3_GENRES.len() {
                return ID3_GENRES[n].to_string();
            }
        }
    }
    trimmed.to_string()
}

fn read_audio_metadata(path: &Path) -> TrackMeta {
    let Ok(data) = fs::read(path) else {
        return TrackMeta::default();
    };
    let mut meta = read_id3v2(&data);
    let v1 = read_id3v1(&data);
    meta.merge_missing(v1);
    meta
}

impl TrackMeta {
    fn merge_missing(&mut self, other: Self) {
        if self.title.is_empty() {
            self.title = other.title;
        }
        if self.artist.is_empty() {
            self.artist = other.artist;
        }
        if self.album.is_empty() {
            self.album = other.album;
        }
        if self.year.is_empty() {
            self.year = other.year;
        }
        if self.genre.is_empty() {
            self.genre = other.genre;
        }
        if self.comment.is_empty() {
            self.comment = other.comment;
        }
        if self.cover.is_none() {
            self.cover = other.cover;
        }
    }
}

fn read_id3v1(data: &[u8]) -> TrackMeta {
    if data.len() < 128 {
        return TrackMeta::default();
    }
    let start = data.len() - 128;
    if &data[start..start + 3] != b"TAG" {
        return TrackMeta::default();
    }
    let slice = |off: usize, len: usize| -> String {
        let raw = &data[start + off..start + off + len];
        String::from_utf8_lossy(raw)
            .trim_matches('\0')
            .trim()
            .to_string()
    };
    let genre_byte = data[start + 127];
    let genre = if genre_byte == 255 {
        String::new()
    } else {
        ID3_GENRES
            .get(genre_byte as usize)
            .copied()
            .unwrap_or("")
            .to_string()
    };
    TrackMeta {
        title: slice(3, 30),
        artist: slice(33, 30),
        album: slice(63, 30),
        year: slice(93, 4),
        comment: slice(97, 30),
        genre,
        ..TrackMeta::default()
    }
}

fn read_id3v2(data: &[u8]) -> TrackMeta {
    let mut meta = TrackMeta::default();
    if data.len() < 10 || &data[0..3] != b"ID3" {
        return meta;
    }
    let id3_major = data[3];
    let tag_size = synchsafe_size(&data[6..10]) as usize;
    let tag_end = (10 + tag_size).min(data.len());
    let mut pos = 10usize;
    while pos + 10 <= tag_end {
        let id = &data[pos..pos + 4];
        let frame_size = if id3_major == 4 {
            synchsafe_size(&data[pos + 4..pos + 8]) as usize
        } else {
            u32::from_be_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
                as usize
        };
        pos += 10;
        if frame_size == 0 || pos + frame_size > tag_end {
            break;
        }
        let payload = &data[pos..pos + frame_size];
        pos += frame_size;
        match id {
            b"TIT2" => meta.title = decode_id3_text(payload),
            b"TPE1" => meta.artist = decode_id3_text(payload),
            b"TPE2" => meta.album_artist = decode_id3_text(payload),
            b"TCOM" => meta.composer = decode_id3_text(payload),
            b"TALB" => meta.album = decode_id3_text(payload),
            b"TYER" | b"TDRC" => {
                let y = decode_id3_text(payload);
                if meta.year.is_empty() {
                    meta.year = y.chars().take(4).collect();
                }
            }
            b"TCON" => meta.genre = decode_genre(&decode_id3_text(payload)),
            b"TRCK" => meta.track_num = decode_id3_text(payload),
            b"COMM" => {
                if meta.comment.is_empty() {
                    meta.comment = decode_id3_comm(payload);
                }
            }
            b"TLEN" => {
                if let Ok(ms) = decode_id3_text(payload).trim().parse::<u64>() {
                    meta.duration_ms = ms;
                }
            }
            b"APIC" | b"PIC " if meta.cover.is_none() => {
                meta.cover = parse_apic_payload(payload);
            }
            _ => {}
        }
    }
    meta
}

fn decode_id3_comm(payload: &[u8]) -> String {
    if payload.len() < 5 {
        return String::new();
    }
    let enc = payload[0];
    let mut i = 4usize;
    i = skip_id3_cstring(payload, i, enc);
    if i >= payload.len() {
        return String::new();
    }
    let mut buf = vec![enc];
    buf.extend_from_slice(&payload[i..]);
    decode_id3_text(&buf)
}

fn skip_id3_cstring(payload: &[u8], mut i: usize, enc: u8) -> usize {
    if i >= payload.len() {
        return i;
    }
    match enc {
        1 | 2 => {
            while i + 1 < payload.len() {
                if payload[i] == 0 && payload[i + 1] == 0 {
                    return i + 2;
                }
                i += 2;
            }
            payload.len()
        }
        _ => {
            while i < payload.len() && payload[i] != 0 {
                i += 1;
            }
            (i + 1).min(payload.len())
        }
    }
}

fn parse_apic_payload(payload: &[u8]) -> Option<Vec<u8>> {
    if payload.is_empty() {
        return None;
    }
    let enc = payload[0];
    let mut i = 1usize;
    while i < payload.len() && payload[i] != 0 {
        i += 1;
    }
    if i >= payload.len() {
        return None;
    }
    i += 1;
    if i >= payload.len() {
        return None;
    }
    i += 1;
    i = skip_id3_cstring(payload, i, enc);
    if i >= payload.len() {
        return None;
    }
    Some(payload[i..].to_vec())
}

fn meta_label(field: &str, value: &str) -> String {
    if value.trim().is_empty() {
        format!("{field}: —")
    } else {
        format!("{field}: {value}")
    }
}

fn label_box(label: &StaticText, w: u32, h: u32, max_lines: i32) {
    label.as_widget_ref().borrow_mut().set_size(w, h);
    label.set_max_lines(max_lines);
}

fn style_heading(
    label: &StaticText,
    size: i32,
    bold: bool,
    colour: Colour,
    w: u32,
    h: u32,
    max_lines: i32,
) {
    let mut desc = FontDesc::new("Helvetica Neue", size);
    if bold {
        desc = desc.bold();
    }
    label.set_font(&Font::new(desc));
    label.set_foreground_colour(colour);
    label_box(label, w, h, max_lines);
}

fn style_muted(label: &StaticText, w: u32, h: u32, max_lines: i32) {
    style_heading(label, 12, false, CLR_MUTED, w, h, max_lines);
}

fn style_detail(label: &StaticText) {
    style_heading(label, 11, false, CLR_DIM, LABEL_W, 20, 1);
}

fn set_cover_art(cover: &StaticBitmap, bytes: Option<&[u8]>) {
    let loaded = bytes.and_then(|data| {
        Image::load_from_memory(data)
            .ok()
            .map(|img| img.scale_to_fit(COVER_SIZE, COVER_SIZE).to_bitmap())
    });
    let bmp = loaded
        .unwrap_or_else(|| Bitmap::from_svg_bytes(ICON_COVER_PLACEHOLDER, COVER_SIZE, COVER_SIZE));
    cover.set_bitmap(RawBitmap {
        hbitmap: bmp.handle(),
        width: bmp.width,
        height: bmp.height,
    });
}

#[allow(clippy::too_many_arguments)]
// the panel mirrors one label widget per metadata field
fn refresh_metadata_panel(
    track: Option<&Track>,
    cover: &StaticBitmap,
    now_title: &StaticText,
    now_artist: &StaticText,
    now_album: &StaticText,
    lbl_year: &StaticText,
    lbl_genre: &StaticText,
    lbl_track: &StaticText,
    lbl_composer: &StaticText,
    lbl_comment: &StaticText,
    lbl_file: &StaticText,
) {
    let Some(track) = track else {
        set_cover_art(cover, None);
        now_title.set_label("Nessun brano selezionato");
        now_artist.set_label("Trascina file audio o usa la toolbar");
        now_album.set_label("");
        lbl_year.set_label("Anno: —");
        lbl_genre.set_label("Genere: —");
        lbl_track.set_label("Traccia: —");
        lbl_composer.set_label("Compositore: —");
        lbl_comment.set_label("Commento: —");
        lbl_file.set_label("File: —");
        return;
    };
    let m = &track.meta;
    set_cover_art(cover, m.cover.as_deref());
    let title = if m.title.is_empty() {
        track
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Brano sconosciuto")
            .to_string()
    } else {
        m.title.clone()
    };
    now_title.set_label(&title);
    let artist = m.display_artist();
    let artist_line = if artist.is_empty() {
        "Artista sconosciuto".to_string()
    } else {
        artist.to_string()
    };
    now_artist.set_label(&artist_line);
    let album_line = if m.album.is_empty() {
        String::new()
    } else {
        format!("Album · {}", m.album)
    };
    now_album.set_label(&album_line);
    lbl_year.set_label(&meta_label("Anno", &m.year));
    lbl_genre.set_label(&meta_label("Genere", &m.genre));
    lbl_track.set_label(&meta_label("Traccia", &m.track_num));
    lbl_composer.set_label(&meta_label("Compositore", &m.composer));
    lbl_comment.set_label(&meta_label("Commento", &m.comment));
    lbl_file.set_label(&format!(
        "File: {}  ({})",
        track
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("?"),
        format_file_size(track.size_bytes)
    ));
}

/// Decode playlist / text files: UTF-8 (with optional BOM), else Windows-1252-ish Latin-1.
fn decode_text_file(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(&bytes[3..]).to_string();
    }
    if bytes.starts_with(&[0xFF, 0xFE]) && bytes.len() >= 2 {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|&u| u != 0)
            .collect();
        return String::from_utf16_lossy(&units);
    }
    if bytes.starts_with(&[0xFE, 0xFF]) && bytes.len() >= 2 {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .take_while(|&u| u != 0)
            .collect();
        return String::from_utf16_lossy(&units);
    }
    std::str::from_utf8(bytes)
        .map(|s| s.to_string())
        .unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect())
}

fn synchsafe_size(bytes: &[u8]) -> u32 {
    if bytes.len() < 4 {
        return 0;
    }
    ((bytes[0] as u32 & 0x7f) << 21)
        | ((bytes[1] as u32 & 0x7f) << 14)
        | ((bytes[2] as u32 & 0x7f) << 7)
        | (bytes[3] as u32 & 0x7f)
}

/// Decode an ID3v2 text frame payload (first byte = encoding).
///
/// Encodings per spec: 0 = ISO-8859-1, 1 = UTF-16 with BOM,
/// 2 = UTF-16BE, 3 = UTF-8.
fn decode_id3_text(payload: &[u8]) -> String {
    if payload.is_empty() {
        return String::new();
    }
    let text = match payload[0] {
        // ISO-8859-1 — one byte per character in U+0000..U+00FF.
        0 => payload[1..]
            .split(|&b| b == 0)
            .next()
            .unwrap_or(&[])
            .iter()
            .map(|&b| b as char)
            .collect::<String>(),
        // UTF-16 with BOM (LE or BE).
        1 => decode_id3_utf16_with_bom(&payload[1..]),
        // UTF-16BE without BOM.
        2 => decode_id3_utf16(&payload[1..], false),
        // UTF-8 (most common in modern MP3 tags).
        3 => {
            let raw = payload[1..].split(|&b| b == 0).next().unwrap_or(&[]);
            std::str::from_utf8(raw)
                .map(|s| s.to_string())
                .unwrap_or_else(|_| String::from_utf8_lossy(raw).to_string())
        }
        // Unknown encoding byte — try UTF-8, then Latin-1.
        _ => std::str::from_utf8(payload)
            .map(|s| s.to_string())
            .unwrap_or_else(|_| {
                payload
                    .iter()
                    .take_while(|&&b| b != 0)
                    .map(|&b| b as char)
                    .collect()
            }),
    };
    text.trim().to_string()
}

fn decode_id3_utf16_with_bom(bytes: &[u8]) -> String {
    if bytes.len() < 2 {
        return String::new();
    }
    let bom = u16::from_be_bytes([bytes[0], bytes[1]]);
    match bom {
        0xFEFF => decode_id3_utf16(&bytes[2..], false),
        0xFFFE => decode_id3_utf16(&bytes[2..], true),
        _ => decode_id3_utf16(bytes, true),
    }
}

fn decode_id3_utf16(bytes: &[u8], little_endian: bool) -> String {
    let mut units = Vec::new();
    for chunk in bytes.chunks_exact(2) {
        let u = if little_endian {
            u16::from_le_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_be_bytes([chunk[0], chunk[1]])
        };
        if u == 0 {
            break;
        }
        units.push(u);
    }
    String::from_utf16_lossy(&units)
}

fn parse_playlist_file(path: &Path) -> Vec<PathBuf> {
    let Ok(bytes) = fs::read(path) else {
        return Vec::new();
    };
    let text = decode_text_file(&bytes);
    let base = path.parent().unwrap_or(Path::new("."));
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let p = PathBuf::from(line);
        let resolved = if p.is_absolute() { p } else { base.join(p) };
        if resolved.is_file() && is_audio_file(&resolved) {
            out.push(resolved);
        }
    }
    out
}

fn open_audio_dialog(frame: &Frame, title: &str) -> Option<String> {
    let mut dlg = FileDialog::new(frame, FileDialogStyle::Open);
    dlg.set_title(title);
    dlg.set_wildcard(
        "Audio (*.mp3;*.wav;*.wma)|*.mp3;*.wav;*.wma|\
         MP3 (*.mp3)|*.mp3|\
         Tutti i file (*.*)|*.*",
    );
    dlg.show_modal()
}

fn open_playlist_dialog(frame: &Frame) -> Option<String> {
    let mut dlg = FileDialog::new(frame, FileDialogStyle::Open);
    dlg.set_title("Carica playlist");
    dlg.set_wildcard(
        "Playlist M3U (*.m3u;*.m3u8)|*.m3u;*.m3u8|\
         Testo (*.txt)|*.txt|\
         Tutti i file (*.*)|*.*",
    );
    dlg.show_modal()
}

fn save_playlist_dialog(frame: &Frame, default_name: &str) -> Option<String> {
    let mut dlg = FileDialog::new(frame, FileDialogStyle::Save);
    dlg.set_title("Salva playlist");
    dlg.set_filename(&playlist_save_filename(default_name));
    dlg.set_wildcard(
        "Playlist M3U (*.m3u)|*.m3u|\
         Testo (*.txt)|*.txt|\
         Tutti i file (*.*)|*.*",
    );
    dlg.show_modal()
}

fn path_for_playlist_entry(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

fn write_playlist_file(path: &Path, tracks: &[Track]) -> Result<(), String> {
    let mut lines = vec!["#EXTM3U".to_string()];
    for track in tracks {
        lines.push(path_for_playlist_entry(&track.path));
    }
    let content = format!("{}\r\n", lines.join("\r\n"));
    fs::write(path, content.as_bytes()).map_err(|e| e.to_string())
}

fn playlist_save_path(mut path: PathBuf) -> PathBuf {
    let ext_ok = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            e.eq_ignore_ascii_case("m3u")
                || e.eq_ignore_ascii_case("m3u8")
                || e.eq_ignore_ascii_case("txt")
        })
        .unwrap_or(false);
    if !ext_ok {
        path.set_extension("m3u");
    }
    path
}

struct PlaylistDoc {
    path: Option<PathBuf>,
}

struct Playlist {
    tracks: Vec<Track>,
    playing: Option<usize>,
}

impl Playlist {
    fn add(&mut self, track: Track) -> usize {
        let idx = self.tracks.len();
        self.tracks.push(track);
        idx
    }

    fn remove(&mut self, index: usize) -> Option<Track> {
        if index >= self.tracks.len() {
            return None;
        }
        let removed = self.tracks.remove(index);
        self.playing = match self.playing {
            Some(i) if i == index => None,
            Some(i) if i > index => Some(i - 1),
            other => other,
        };
        Some(removed)
    }

    fn move_track(&mut self, from: usize, to: usize) {
        if from == to || from >= self.tracks.len() || to >= self.tracks.len() {
            return;
        }
        let track = self.tracks.remove(from);
        self.tracks.insert(to, track);
        self.playing = match self.playing {
            None => None,
            Some(i) if i == from => Some(to),
            Some(i) if from < i && to >= i => Some(i - 1),
            Some(i) if from > i && to <= i => Some(i + 1),
            Some(i) => Some(i),
        };
    }

    fn path(&self, index: usize) -> Option<&Path> {
        self.tracks.get(index).map(|t| t.path.as_path())
    }

    fn len(&self) -> usize {
        self.tracks.len()
    }
}

fn main() {
    human_panic::setup_panic!();

    let app = App::new();
    let frame = Frame::builder()
        .with_title("ru_wx — Music Player")
        .with_size(920, 720)
        .build();
    frame.set_background_colour(CLR_BG);

    let status = StatusBar::new(&frame, 2);
    status.set_status_text(
        "Carica brani con la toolbar o trascinali sulla playlist.",
        0,
    );
    status.set_status_text("Nessun brano in coda.", 1);

    // ── Toolbar icone ────────────────────────────────────────────────
    let images = ImageList::new(40, 40);
    images.add_svg_bytes(ICON_LOAD);
    images.add_svg_bytes(ICON_PLAYLIST);
    images.add_svg_bytes(ICON_SAVE);
    images.add_svg_bytes(ICON_CATALOG);

    let toolbar = AuiToolBar::new(&frame);
    toolbar.set_toolbar_height(52);
    toolbar.set_image_list(&images);
    toolbar.add_tool(ID_TOOL_LOAD, "Carica file audio…", 0);
    toolbar.add_tool(ID_TOOL_LOAD_PLAYLIST, "Carica playlist…", 1);
    toolbar.add_tool(ID_TOOL_SAVE_PLAYLIST, "Salva playlist…", 2);
    toolbar.add_tool(ID_TOOL_CATALOG, "Catalogo cartelle…", 3);
    toolbar.realize();
    let toolbar_h = toolbar.reserved_height();

    let meta_panel = Panel::new(&frame);
    meta_panel.set_background_colour(CLR_SURFACE);

    let playlist_panel = Panel::new(&frame);
    playlist_panel.set_background_colour(CLR_SURFACE_ALT);

    let playlist_heading = StaticText::new(&playlist_panel, "Playlist");
    style_heading(&playlist_heading, 15, true, CLR_TEXT, 72, 22, 1);

    let playlist_name = TextCtrl::new(&playlist_panel, DEFAULT_PLAYLIST_NAME);
    playlist_name.as_widget_ref().borrow_mut().set_size(320, 26);

    let search_label = StaticText::new(&playlist_panel, "Cerca");
    style_muted(&search_label, 48, 20, 1);
    let playlist_search = SearchCtrl::new(&playlist_panel, "Titolo, artista, genere…");
    playlist_search
        .as_widget_ref()
        .borrow_mut()
        .set_size(220, 26);

    let sort_label = StaticText::new(&playlist_panel, "Ordina");
    style_muted(&sort_label, 48, 20, 1);
    let sort_choice = Choice::new(&playlist_panel);
    sort_choice.append("Ordine playlist");
    sort_choice.append("Titolo");
    sort_choice.append("Durata");
    sort_choice.append("Autore");
    sort_choice.append("Genere");
    sort_choice.append("Commento");
    sort_choice.append("Percorso file");
    sort_choice.set_selection(0);
    sort_choice.as_widget_ref().borrow_mut().set_size(128, 26);

    let sort_dir_btn = Button::new(&playlist_panel, sort_direction_label(true));
    sort_dir_btn.as_widget_ref().borrow_mut().set_size(52, 26);

    let playlist = ListCtrl::new(&playlist_panel, ListCtrlStyle::Report);
    playlist.insert_column(0, PL_COL_NAMES[PL_COL_TITLE], 120);
    playlist.insert_column(1, PL_COL_NAMES[PL_COL_DURATION], 52);
    playlist.insert_column(2, PL_COL_NAMES[PL_COL_ARTIST], 90);
    playlist.insert_column(3, PL_COL_NAMES[PL_COL_GENRE], 72);
    playlist.insert_column(4, PL_COL_NAMES[PL_COL_COMMENT], 90);
    playlist.insert_column(5, PL_COL_NAMES[PL_COL_PATH], 200);
    playlist.as_widget_ref().borrow_mut().set_size(520, 280);

    // ── Now playing + metadati ID3 ───────────────────────────────────
    let cover_art = StaticBitmap::with_size(&meta_panel, COVER_SIZE, COVER_SIZE);
    set_cover_art(&cover_art, None);

    let now_title = StaticText::new(&meta_panel, "Nessun brano selezionato");
    style_heading(&now_title, 17, true, CLR_TEXT, LABEL_W, 44, 2);
    let now_artist = StaticText::new(&meta_panel, "Trascina file audio o usa la toolbar");
    style_heading(&now_artist, 13, false, CLR_ACCENT, LABEL_W, 22, 1);
    let now_album = StaticText::new(&meta_panel, "");
    style_muted(&now_album, LABEL_W, 20, 1);

    let meta_year = StaticText::new(&meta_panel, "Anno: —");
    let meta_genre = StaticText::new(&meta_panel, "Genere: —");
    let meta_track = StaticText::new(&meta_panel, "Traccia: —");
    let meta_composer = StaticText::new(&meta_panel, "Compositore: —");
    let meta_comment = StaticText::new(&meta_panel, "Commento: —");
    let meta_file = StaticText::new(&meta_panel, "File: —");
    for lbl in [
        &meta_year,
        &meta_genre,
        &meta_track,
        &meta_composer,
        &meta_comment,
    ] {
        style_detail(lbl);
    }
    style_heading(&meta_file, 11, false, CLR_DIM, LABEL_W, 36, 2);

    let transport_panel = Panel::new(&frame);
    transport_panel.set_background_colour(CLR_SURFACE);

    let time_label = StaticText::new(&transport_panel, "00:00 / 00:00");
    style_heading(&time_label, 13, false, CLR_MUTED, 160, 20, 1);

    let progress = Slider::new(&transport_panel, 0, 1000, 0);
    progress.as_widget_ref().borrow_mut().set_size(720, 28);

    let btn_play = transport_icon_button(&transport_panel, ICON_PLAY);
    let btn_pause = transport_icon_button(&transport_panel, ICON_PAUSE);
    let btn_stop = transport_icon_button(&transport_panel, ICON_STOP);

    let media = Rc::new(RefCell::new(MediaCtrl::new(&frame)));
    let tracks = Rc::new(RefCell::new(Playlist {
        tracks: Vec::new(),
        playing: None,
    }));
    let playlist_doc = Rc::new(RefCell::new(PlaylistDoc { path: None }));

    let sync_window_title: Rc<dyn Fn()> = {
        let frame = frame.clone();
        let name_ctrl = playlist_name.clone();
        Rc::new(move || {
            let name = normalize_playlist_name(&name_ctrl.get_value());
            frame.set_title(&format!("ru_wx — {name}"));
        })
    };
    sync_window_title();

    let stop_after_each = Rc::new(Cell::new(false));
    let programmatic_slider = Rc::new(Cell::new(false));
    let track_finished_flag = Rc::new(Cell::new(false));
    let progress_clock = Rc::new(RefCell::new(ProgressClock::new()));
    let playlist_view = Rc::new(RefCell::new(PlaylistView::new()));
    let progress_timer = Rc::new(Timer::new(&frame));

    let rebuild_playlist: Rc<dyn Fn(Option<usize>)> = {
        let playlist = playlist.clone();
        let tracks = tracks.clone();
        let view = playlist_view.clone();
        Rc::new(move |select_track| {
            let track_list = tracks.borrow().tracks.clone();
            rebuild_playlist_view(&playlist, &mut view.borrow_mut(), &track_list, select_track);
        })
    };

    let update_status_count = {
        let tracks = tracks.clone();
        let status = status.clone();
        Rc::new(move || {
            status.set_status_text(&format!("Brani in playlist: {}", tracks.borrow().len()), 1);
        })
    };

    let refresh_meta: Rc<dyn Fn(Option<usize>)> = {
        let tracks = tracks.clone();
        let cover = cover_art.clone();
        let now_title = now_title.clone();
        let now_artist = now_artist.clone();
        let now_album = now_album.clone();
        let meta_year = meta_year.clone();
        let meta_genre = meta_genre.clone();
        let meta_track = meta_track.clone();
        let meta_composer = meta_composer.clone();
        let meta_comment = meta_comment.clone();
        let meta_file = meta_file.clone();
        Rc::new(move |index: Option<usize>| {
            let track = index.and_then(|i| tracks.borrow().tracks.get(i).cloned());
            refresh_metadata_panel(
                track.as_ref(),
                &cover,
                &now_title,
                &now_artist,
                &now_album,
                &meta_year,
                &meta_genre,
                &meta_track,
                &meta_composer,
                &meta_comment,
                &meta_file,
            );
        })
    };

    let append_track: Rc<dyn Fn(PathBuf)> = {
        let tracks = tracks.clone();
        let status = status.clone();
        let update_count = update_status_count.clone();
        let refresh_meta = refresh_meta.clone();
        let rebuild = rebuild_playlist.clone();
        Rc::new(move |path| {
            if !is_playlist_media(&path) {
                status.set_status_text("File non supportato ignorato.", 0);
                return;
            }
            let track = Track::from_path(path);
            let label = track.meta.title.clone();
            let idx = tracks.borrow_mut().add(track);
            rebuild(Some(idx));
            status.set_status_text(&format!("Aggiunto: {label}"), 0);
            update_count();
            refresh_meta(Some(idx));
        })
    };

    let append_paths: Rc<dyn Fn(Vec<PathBuf>)> = {
        let append = append_track.clone();
        Rc::new(move |paths| {
            for p in paths {
                append(p);
            }
        })
    };

    let catalog_frame: Rc<RefCell<Option<Frame>>> = Rc::new(RefCell::new(None));
    let toggle_catalog: Rc<dyn Fn()> = {
        let parent = frame.clone();
        let append = append_track.clone();
        let catalog_frame = catalog_frame.clone();
        let status = status.clone();
        Rc::new(move || {
            if let Some(ref cat) = *catalog_frame.borrow() {
                if cat.is_visible() {
                    cat.hide();
                    status.set_status_text("Catalogo chiuso.", 0);
                    return;
                }
            }
            if catalog_frame.borrow().is_none() {
                let cat = open_catalog_window(&parent, append.clone());
                *catalog_frame.borrow_mut() = Some(cat);
            }
            let catalog_present = catalog_frame.clone();
            #[cfg(target_os = "macos")]
            ru_wx::platform::appkit::run_on_next_event(move || {
                if let Some(ref cat) = *catalog_present.borrow() {
                    cat.present();
                }
            });
            #[cfg(not(target_os = "macos"))]
            {
                if let Some(ref cat) = *catalog_present.borrow() {
                    cat.present();
                }
            }
            status.set_status_text("Catalogo: cerca MP3/MP4 nelle cartelle.", 0);
        })
    };

    let play_file_path: Rc<dyn Fn(PathBuf)> = {
        let media = media.clone();
        let tracks = tracks.clone();
        let playlist = playlist.clone();
        let status = status.clone();
        let time_label = time_label.clone();
        let progress = progress.clone();
        let programmatic_slider = programmatic_slider.clone();
        let track_finished_flag = track_finished_flag.clone();
        let clock = progress_clock.clone();
        let refresh_meta = refresh_meta.clone();
        let view = playlist_view.clone();
        let progress_timer = progress_timer.clone();
        Rc::new(move |path: PathBuf| {
            track_finished_flag.set(false);
            let track_index = {
                let t = tracks.borrow();
                find_track_index_by_path(&t.tracks, &path)
            };
            tracks.borrow_mut().playing = track_index;
            if let Some(idx) = track_index {
                if let Some(row) = view.borrow().row_for_track(idx) {
                    playlist.select(row);
                }
                refresh_meta(Some(idx));
            }
            if let Err(e) = media.borrow().load(&path) {
                status.set_status_text(&format!("Errore caricamento: {e}"), 0);
                return;
            }
            if let Err(e) = media.borrow().play() {
                status.set_status_text(&format!("Errore riproduzione: {e}"), 0);
                return;
            }
            let len = media.borrow().length_ms().unwrap_or(0).max(1);
            let meta = track_index.and_then(|i| tracks.borrow().tracks.get(i).cloned());
            let (title, artist, album) = meta
                .map(|t| {
                    (
                        t.meta.title.clone(),
                        t.meta.display_artist().to_string(),
                        t.meta.album.clone(),
                    )
                })
                .unwrap_or_else(|| ("Brano".to_string(), String::new(), String::new()));
            let now_playing = match (artist.as_str(), album.as_str()) {
                (a, al) if !a.is_empty() && !al.is_empty() => format!("{a} — {title}  [{al}]"),
                (a, _) if !a.is_empty() => format!("{a} — {title}"),
                (_, al) if !al.is_empty() => format!("{title}  [{al}]"),
                _ => title,
            };
            status.set_status_text(&format!("In riproduzione: {now_playing}"), 0);
            clock.borrow_mut().begin(len);
            let max = len.min(i32::MAX as u64) as i32;
            progress.set_range(0, max);
            programmatic_slider.set(true);
            progress.set_value(0);
            programmatic_slider.set(false);
            time_label.set_label(&format!("00:00 / {}", format_ms(len)));
            progress_timer.start(Duration::from_millis(250));
        })
    };

    let play_row: Rc<dyn Fn(usize)> = {
        let playlist = playlist.clone();
        let status = status.clone();
        let play_file = play_file_path.clone();
        Rc::new(move |row| {
            if let Some(path) = path_from_playlist_row(&playlist, row) {
                play_file(path);
            } else {
                status.set_status_text("Percorso file non valido o mancante.", 0);
            }
        })
    };

    let play_track: Rc<dyn Fn(usize)> = {
        let tracks = tracks.clone();
        let play_file = play_file_path.clone();
        Rc::new(move |index: usize| {
            let path = {
                let t = tracks.borrow();
                t.path(index).map(|p| p.to_path_buf())
            };
            if let Some(path) = path {
                play_file(path);
            }
        })
    };

    let play_next: Rc<dyn Fn()> = {
        let tracks = tracks.clone();
        let play_track = play_track.clone();
        Rc::new(move || {
            let next = {
                let t = tracks.borrow();
                match t.playing {
                    Some(i) if i + 1 < t.len() => Some(i + 1),
                    _ => None,
                }
            };
            if let Some(idx) = next {
                play_track(idx);
            }
        })
    };

    let do_play: Rc<dyn Fn()> = {
        let media = media.clone();
        let status = status.clone();
        let clock = progress_clock.clone();
        let playlist = playlist.clone();
        let tracks = tracks.clone();
        let play_file = play_file_path.clone();
        let play_row = play_row.clone();
        let progress_timer = progress_timer.clone();
        Rc::new(move || {
            match media.borrow().state() {
                MediaState::Playing => return,
                MediaState::Paused => {
                    if media.borrow().play().is_ok() {
                        clock.borrow_mut().resume();
                        progress_timer.start(Duration::from_millis(250));
                    }
                    return;
                }
                MediaState::Stopped => {}
            }

            if let Some(row) = playlist.get_selected_item() {
                if path_from_playlist_row(&playlist, row).is_some() {
                    play_row(row);
                    return;
                }
            }

            let resume_path = {
                let t = tracks.borrow();
                t.playing.and_then(|i| t.path(i).map(|p| p.to_path_buf()))
            };
            match resume_path {
                Some(path) => play_file(path),
                _ if tracks.borrow().len() > 0 => {
                    if let Some(row) = playlist.get_selected_item().or(Some(0)) {
                        play_row(row);
                    }
                }
                _ => status.set_status_text("Seleziona un brano dalla playlist.", 0),
            }
        })
    };

    let do_pause: Rc<dyn Fn()> = {
        let media = media.clone();
        let status = status.clone();
        let clock = progress_clock.clone();
        let progress_timer = progress_timer.clone();
        Rc::new(move || {
            let m = media.borrow();
            if let Err(e) = m.pause() {
                status.set_status_text(&format!("Pausa: {e}"), 0);
            } else {
                clock.borrow_mut().pause();
                progress_timer.stop();
            }
        })
    };

    let do_stop: Rc<dyn Fn()> = {
        let media = media.clone();
        let tracks = tracks.clone();
        let progress = progress.clone();
        let time_label = time_label.clone();
        let status = status.clone();
        let programmatic_slider = programmatic_slider.clone();
        let track_finished_flag = track_finished_flag.clone();
        let clock = progress_clock.clone();
        let progress_timer = progress_timer.clone();
        Rc::new(move || {
            {
                let m = media.borrow();
                if let Err(e) = m.stop() {
                    status.set_status_text(&format!("Stop: {e}"), 0);
                }
            }
            progress_timer.stop();
            clock.borrow_mut().stop();
            tracks.borrow_mut().playing = None;
            track_finished_flag.set(false);
            programmatic_slider.set(true);
            progress.set_value(0);
            programmatic_slider.set(false);
            time_label.set_label("00:00 / 00:00");
            status.set_status_text("Riproduzione arrestata.", 0);
        })
    };

    let on_track_finished: Rc<dyn Fn()> = {
        let tracks = tracks.clone();
        let stop_after = stop_after_each.clone();
        let do_stop = do_stop.clone();
        let play_next = play_next.clone();
        let status = status.clone();
        let track_finished_flag = track_finished_flag.clone();
        Rc::new(move || {
            if track_finished_flag.get() {
                return;
            }
            track_finished_flag.set(true);
            let (count, _) = {
                let t = tracks.borrow();
                (t.len(), t.playing)
            };
            if count <= 1 {
                do_stop();
                status.set_status_text("Fine brano.", 0);
            } else if stop_after.get() {
                do_stop();
                status.set_status_text("Fine brano — pausa (opzione attiva).", 0);
            } else {
                status.set_status_text("Brano successivo…", 0);
                let advance = play_next.clone();
                #[cfg(target_os = "macos")]
                ru_wx::platform::appkit::run_on_next_event(move || advance());
                #[cfg(not(target_os = "macos"))]
                advance();
            }
        })
    };

    let load_one_file: Rc<dyn Fn()> = {
        let frame = frame.clone();
        let append = append_track.clone();
        Rc::new(move || {
            if let Some(path_str) = open_audio_dialog(&frame, "Seleziona un file audio") {
                append(PathBuf::from(path_str));
            }
        })
    };

    let load_playlist_file: Rc<dyn Fn()> = {
        let frame = frame.clone();
        let append_paths = append_paths.clone();
        let status = status.clone();
        let playlist_name = playlist_name.clone();
        let playlist_doc = playlist_doc.clone();
        let sync_title = sync_window_title.clone();
        Rc::new(move || {
            if let Some(path_str) = open_playlist_dialog(&frame) {
                let path = PathBuf::from(&path_str);
                let paths = parse_playlist_file(&path);
                if paths.is_empty() {
                    status.set_status_text("Playlist vuota o non valida.", 0);
                } else {
                    let n = paths.len();
                    append_paths(paths);
                    playlist_name.set_value(&playlist_name_from_path(&path));
                    playlist_doc.borrow_mut().path = Some(path);
                    sync_title();
                    status.set_status_text(&format!("Playlist caricata: {n} brani."), 0);
                }
            }
        })
    };

    let save_playlist_file: Rc<dyn Fn()> = {
        let frame = frame.clone();
        let tracks = tracks.clone();
        let status = status.clone();
        let playlist_name = playlist_name.clone();
        let playlist_doc = playlist_doc.clone();
        let sync_title = sync_window_title.clone();
        Rc::new(move || {
            let list = tracks.borrow().tracks.clone();
            if list.is_empty() {
                status.set_status_text("Playlist vuota: niente da salvare.", 0);
                return;
            }
            let name = normalize_playlist_name(&playlist_name.get_value());
            if let Some(path_str) = save_playlist_dialog(&frame, &name) {
                let path = playlist_save_path(PathBuf::from(path_str));
                match write_playlist_file(&path, &list) {
                    Ok(()) => {
                        playlist_name.set_value(&playlist_name_from_path(&path));
                        playlist_doc.borrow_mut().path = Some(path.clone());
                        sync_title();
                        status.set_status_text(
                            &format!(
                                "Playlist salvata ({n} brani): {path}",
                                n = list.len(),
                                path = path.display()
                            ),
                            0,
                        );
                    }
                    Err(e) => status.set_status_text(&format!("Errore salvataggio: {e}"), 0),
                }
            }
        })
    };

    let remove_at: Rc<dyn Fn(usize)> = {
        let playlist = playlist.clone();
        let tracks = tracks.clone();
        let status = status.clone();
        let do_stop = do_stop.clone();
        let update_count = update_status_count.clone();
        let refresh_meta = refresh_meta.clone();
        let rebuild = rebuild_playlist.clone();
        let view = playlist_view.clone();
        Rc::new(move |index: usize| {
            let removed = {
                let mut t = tracks.borrow_mut();
                let name = t
                    .tracks
                    .get(index)
                    .map(|tr| tr.meta.title.clone())
                    .unwrap_or_default();
                let was_playing = t.playing == Some(index);
                t.remove(index);
                update_count();
                (name, was_playing)
            };
            rebuild(None);
            if removed.1 {
                do_stop();
            }
            let sel = playlist
                .get_selected_item()
                .and_then(|row| view.borrow().track_index_for_row(row));
            refresh_meta(sel);
            status.set_status_text(&format!("Rimosso: {}", removed.0), 0);
        })
    };

    // ── Menu ─────────────────────────────────────────────────────────
    let mut file_menu = Menu::new("&File");
    let load_menu = load_one_file.clone();
    file_menu.append_with_shortcut(
        "&Apri file audio…",
        Accelerator::parse("Ctrl+O").unwrap(),
        &frame,
        move || load_menu(),
    );
    let pl_menu = load_playlist_file.clone();
    file_menu.append_with_shortcut(
        "Apri &playlist…",
        Accelerator::parse("Ctrl+L").unwrap(),
        &frame,
        move || pl_menu(),
    );
    let save_menu = save_playlist_file.clone();
    file_menu.append_with_shortcut(
        "Salva pla&ylist…",
        Accelerator::parse("Ctrl+S").unwrap(),
        &frame,
        move || save_menu(),
    );
    file_menu.append_separator();
    let frame_exit = frame.clone();
    file_menu.append("E&sci", &frame, move || frame_exit.close());

    let mut opts_menu = Menu::new("&Opzioni");
    let ui_font: Rc<RefCell<Option<Font>>> = Rc::new(RefCell::new(None));
    let stop_flag = stop_after_each.clone();
    let status_flag = status.clone();
    // The native HMENU is only consumed by the Windows menu-bar setup below.
    #[cfg(target_os = "windows")]
    let opts_hmenu = opts_menu.hmenu();
    let stop_after_id_cell = Rc::new(Cell::new(0u16));
    // Only the Win32 `CheckMenuItem` and the AppKit `menu_set_item_checked`
    // calls below read this id.
    #[cfg(not(target_os = "linux"))]
    let stop_after_id_for_cb = stop_after_id_cell.clone();
    let stop_after_id =
        opts_menu.append_check_item("Ferma dopo ogni brano (playlist)", &frame, move || {
            let v = !stop_flag.get();
            stop_flag.set(v);
            #[cfg(target_os = "windows")]
            unsafe {
                use windows_sys::Win32::UI::WindowsAndMessaging::{
                    CheckMenuItem, HMENU, MF_BYCOMMAND, MF_CHECKED, MF_UNCHECKED,
                };
                let flags = if v { MF_CHECKED } else { MF_UNCHECKED };
                CheckMenuItem(
                    opts_hmenu as HMENU,
                    stop_after_id_for_cb.get() as u32,
                    MF_BYCOMMAND | flags,
                );
            }
            #[cfg(target_os = "macos")]
            {
                ru_wx::platform::appkit::menu_set_item_checked(stop_after_id_for_cb.get(), v);
            }
            status_flag.set_status_text(
                if v {
                    "Opzione: ferma dopo ogni brano — attiva."
                } else {
                    "Opzione: ferma dopo ogni brano — disattiva (avanzamento automatico)."
                },
                0,
            );
        });
    stop_after_id_cell.set(stop_after_id);

    // Font UI (playlist + metadati + tempo)
    let ui_font_store = ui_font.clone();
    let frame_for_font = frame.clone();
    let playlist_heading_for_font = playlist_heading.clone();
    let now_title_for_font = now_title.clone();
    let now_artist_for_font = now_artist.clone();
    let now_album_for_font = now_album.clone();
    let meta_year_for_font = meta_year.clone();
    let meta_genre_for_font = meta_genre.clone();
    let meta_track_for_font = meta_track.clone();
    let meta_composer_for_font = meta_composer.clone();
    let meta_comment_for_font = meta_comment.clone();
    let meta_file_for_font = meta_file.clone();
    let time_label_for_font = time_label.clone();
    opts_menu.append("Font &testo…", &frame, move || {
        let mut dlg = FontDialog::new(&frame_for_font);
        if let Some(new_font) = dlg.show_modal() {
            *ui_font_store.borrow_mut() = Some(new_font);
            if let Some(ref font) = *ui_font_store.borrow() {
                playlist_heading_for_font.set_font(font);
                now_title_for_font.set_font(font);
                now_artist_for_font.set_font(font);
                now_album_for_font.set_font(font);
                meta_year_for_font.set_font(font);
                meta_genre_for_font.set_font(font);
                meta_track_for_font.set_font(font);
                meta_composer_for_font.set_font(font);
                meta_comment_for_font.set_font(font);
                meta_file_for_font.set_font(font);
                time_label_for_font.set_font(font);
            }
        }
    });

    let mut ctrl_menu = Menu::new("&Controlli");
    let play_for_menu = do_play.clone();
    ctrl_menu.append_with_shortcut(
        "&Play",
        Accelerator::parse("F5").unwrap(),
        &frame,
        move || play_for_menu(),
    );
    let pause_for_menu = do_pause.clone();
    ctrl_menu.append_with_shortcut(
        "&Pausa",
        Accelerator::parse("F6").unwrap(),
        &frame,
        move || pause_for_menu(),
    );
    let stop_for_menu = do_stop.clone();
    ctrl_menu.append_with_shortcut(
        "&Stop",
        Accelerator::parse("F7").unwrap(),
        &frame,
        move || stop_for_menu(),
    );

    let mut menubar = MenuBar::new();
    menubar.append(file_menu);
    menubar.append(opts_menu);
    menubar.append(ctrl_menu);
    frame.set_menu_bar(menubar);
    let _ = stop_after_id;

    // ── Toolbar click ────────────────────────────────────────────────
    let load_tool = load_one_file.clone();
    let pl_tool = load_playlist_file.clone();
    let save_tool = save_playlist_file.clone();
    let catalog_tool = toggle_catalog.clone();
    let status_tool = status.clone();
    toolbar.on_tool_clicked(&frame, move |id| match id {
        ID_TOOL_LOAD => {
            load_tool();
            status_tool.set_status_text("Toolbar: carica file.", 0);
        }
        ID_TOOL_LOAD_PLAYLIST => {
            pl_tool();
            status_tool.set_status_text("Toolbar: carica playlist.", 0);
        }
        ID_TOOL_SAVE_PLAYLIST => {
            save_tool();
            status_tool.set_status_text("Toolbar: salva playlist.", 0);
        }
        ID_TOOL_CATALOG => {
            catalog_tool();
        }
        _ => {}
    });

    // ── Drag & drop sulla finestra ───────────────────────────────────
    let append_drop = append_paths.clone();
    let status_drop = status.clone();
    frame.set_drop_files_callback(move |files: DroppedFiles| {
        let paths: Vec<PathBuf> = files
            .paths()
            .iter()
            .filter(|p| p.is_file())
            .cloned()
            .collect();
        if paths.is_empty() {
            status_drop.set_status_text("Nessun file valido nel drop.", 0);
            return;
        }
        let audio: Vec<PathBuf> = paths.into_iter().filter(|p| is_audio_file(p)).collect();
        if audio.is_empty() {
            status_drop.set_status_text("Trascina file audio (MP3, WAV, …).", 0);
        } else {
            let n = audio.len();
            append_drop(audio);
            status_drop.set_status_text(&format!("Aggiunti {n} file tramite drag-and-drop."), 0);
        }
    });

    // ── Riordino playlist (drag righe) ───────────────────────────────
    let tracks_reorder = tracks.clone();
    let refresh_reorder = refresh_meta.clone();
    let rebuild_reorder = rebuild_playlist.clone();
    let view_reorder = playlist_view.clone();
    playlist.on_row_reordered(&frame, move |from_row, to_row| {
        let (from, to) = {
            let v = view_reorder.borrow();
            (
                v.track_index_for_row(from_row),
                v.track_index_for_row(to_row),
            )
        };
        if let (Some(f), Some(t)) = (from, to) {
            tracks_reorder.borrow_mut().move_track(f, t);
            let sel = tracks_reorder.borrow().playing;
            rebuild_reorder(sel);
            refresh_reorder(sel);
        }
    });

    // ── Selezione playlist → aggiorna metadati; doppio clic → riproduci ─
    let refresh_sel = refresh_meta.clone();
    let view_sel = playlist_view.clone();
    let status_sel = status.clone();
    let playlist_sel = playlist.clone();
    playlist.on_item_selected(&frame, move |row| {
        let track_idx = row.and_then(|r| view_sel.borrow().track_index_for_row(r));
        refresh_sel(track_idx);
        if let Some(r) = row {
            let col = playlist_sel
                .last_hit_cell()
                .map(|(_, c)| c)
                .unwrap_or(PL_COL_TITLE);
            let col_name = PL_COL_NAMES.get(col).copied().unwrap_or("?");
            let value = playlist_cell_value(&playlist_sel, r, col);
            status_sel.set_status_text(&format!("Riga {r}, {col_name}: {value}"), 0);
        }
    });
    let play_for_dbl = play_row.clone();
    playlist.on_item_double_click(&frame, move |row| {
        play_for_dbl(row);
    });

    // ── Nome playlist (modificabile) ─────────────────────────────────
    let sync_title_name = sync_window_title.clone();
    playlist_name.on_change(&frame, move || {
        sync_title_name();
    });

    // ── Ricerca e ordinamento playlist ───────────────────────────────
    let rebuild_search = rebuild_playlist.clone();
    let view_search = playlist_view.clone();
    let tracks_search = tracks.clone();
    let status_search = status.clone();
    playlist_search.on_change(&frame, {
        let search_field = playlist_search.clone();
        move || {
            view_search.borrow_mut().filter = search_field.value();
            let sel = tracks_search.borrow().playing;
            rebuild_search(sel);
            let n = view_search.borrow().row_map.len();
            let total = tracks_search.borrow().len();
            if view_search.borrow().filter.trim().is_empty() {
                status_search.set_status_text(&format!("Brani in playlist: {total}"), 1);
            } else {
                status_search.set_status_text(&format!("Filtro: {n} / {total} brani"), 1);
            }
        }
    });

    let rebuild_sort = rebuild_playlist.clone();
    let view_sort = playlist_view.clone();
    let tracks_sort = tracks.clone();
    sort_choice.on_selection_change(&frame, {
        let sort_pick = sort_choice.clone();
        move || {
            let col = PlaylistSort::from_choice_index(sort_pick.get_selection().unwrap_or(0));
            view_sort.borrow_mut().sort = col;
            rebuild_sort(tracks_sort.borrow().playing);
        }
    });

    let rebuild_dir = rebuild_playlist.clone();
    let view_dir = playlist_view.clone();
    let tracks_dir = tracks.clone();
    sort_dir_btn.on_click(&frame, {
        let sort_dir_btn_click = sort_dir_btn.clone();
        move || {
            let mut v = view_dir.borrow_mut();
            v.sort_asc = !v.sort_asc;
            let asc = v.sort_asc;
            drop(v);
            sort_dir_btn_click.set_label(sort_direction_label(asc));
            rebuild_dir(tracks_dir.borrow().playing);
        }
    });

    // ── Menu contestuale ─────────────────────────────────────────────
    let playlist_ctx = playlist.clone();
    let frame_ctx = frame.clone();
    let remove_for_ctx = remove_at.clone();
    let view_ctx = playlist_view.clone();
    frame.on_context_menu(move |ev| {
        if ev.is_keyboard {
            return;
        }
        let Some(row) = playlist_ctx.get_selected_item() else {
            return;
        };
        let Some(idx) = view_ctx.borrow().track_index_for_row(row) else {
            return;
        };
        playlist_ctx.select(row);
        let mut popup = PopupMenu::new();
        let rm = remove_for_ctx.clone();
        popup.append("Rimuovi brano", &frame_ctx, move || rm(idx));
        popup.popup_at(&frame_ctx, ev.position.x, ev.position.y);
    });

    // ── Pulsanti ─────────────────────────────────────────────────────
    let play_btn = do_play.clone();
    btn_play.on_click(&frame, move || play_btn());
    let pause_btn = do_pause.clone();
    btn_pause.on_click(&frame, move || pause_btn());
    let stop_btn = do_stop.clone();
    btn_stop.on_click(&frame, move || stop_btn());

    // ── Slider seek (riprende da solo dopo lo spostamento) ───────────
    let media_seek = media.clone();
    let progress_seek = progress.clone();
    let time_seek = time_label.clone();
    let programmatic_seek = programmatic_slider.clone();
    let clock_seek = progress_clock.clone();
    progress.on_value_change(&frame, move || {
        if programmatic_seek.get() {
            return;
        }
        let pos_ms = progress_seek.get_value().max(0) as u64;
        {
            let m = media_seek.borrow();
            let _ = m.seek_ms(pos_ms);
        }
        let total = media_seek
            .borrow()
            .length_ms()
            .unwrap_or_else(|| clock_seek.borrow().length_ms);
        clock_seek.borrow_mut().seek(pos_ms);
        time_seek.set_label(&format!(
            "{} / {}",
            format_ms(pos_ms),
            format_ms(total.max(1))
        ));
    });

    // ── Timer: avanzamento barra + fine brano ────────────────────────
    let refresh = progress_timer.clone();
    let media_tick = media.clone();
    let progress_tick = progress.clone();
    let time_tick = time_label.clone();
    let programmatic_tick = programmatic_slider.clone();
    let clock_tick = progress_clock.clone();
    let on_finished = on_track_finished.clone();
    refresh.on_tick(move || {
        if !clock_tick.borrow().running {
            return;
        }
        let seeking = clock_tick.borrow().seek_pending;
        let (len, pos) = {
            let m = media_tick.borrow();
            let len = {
                let mut c = clock_tick.borrow_mut();
                if let Some(l) = m.length_ms() {
                    c.set_length(l);
                }
                c.length_ms.max(1)
            };
            if !seeking {
                if let Some(p) = m.position_ms() {
                    clock_tick.borrow_mut().sync_from_mci(p);
                }
            }
            let pos = clock_tick.borrow().position_ms();
            (len, pos)
        };
        let pos_i = pos.min(i32::MAX as u64) as i32;
        let len_i = len.min(i32::MAX as u64) as i32;
        progress_tick.set_range(0, len_i);
        if !seeking {
            programmatic_tick.set(true);
            progress_tick.set_value(pos_i);
            programmatic_tick.set(false);
        }
        time_tick.set_label(&format!("{} / {}", format_ms(pos), format_ms(len)));
        if pos + 400 >= len {
            let finished = on_finished.clone();
            #[cfg(target_os = "macos")]
            ru_wx::platform::appkit::run_on_next_event(move || finished());
            #[cfg(not(target_os = "macos"))]
            finished();
        }
    });

    // ── Layout ───────────────────────────────────────────────────────
    let mut meta_details = BoxSizer::vertical();
    meta_details.add(meta_year.as_widget_ref());
    meta_details.add_spacer(2);
    meta_details.add(meta_genre.as_widget_ref());
    meta_details.add_spacer(2);
    meta_details.add(meta_track.as_widget_ref());
    meta_details.add_spacer(2);
    meta_details.add(meta_composer.as_widget_ref());
    meta_details.add_spacer(2);
    meta_details.add(meta_comment.as_widget_ref());
    meta_details.add_spacer(2);
    meta_details.add(meta_file.as_widget_ref());

    let mut meta_inner = BoxSizer::vertical();
    meta_inner.set_padding(12);
    meta_inner.add(cover_art.as_widget_ref());
    meta_inner.add_spacer(10);
    meta_inner.add(now_title.as_widget_ref());
    meta_inner.add_spacer(4);
    meta_inner.add(now_artist.as_widget_ref());
    meta_inner.add_spacer(2);
    meta_inner.add(now_album.as_widget_ref());
    meta_inner.add_spacer(10);
    meta_inner.add_sizer(meta_details);
    meta_inner.add_stretch(1);
    meta_panel.set_sizer(meta_inner);
    meta_panel.as_widget_ref().borrow_mut().set_size(272, 480);

    let mut playlist_inner = BoxSizer::vertical();
    playlist_inner.set_padding(12);
    let mut title_row = BoxSizer::horizontal();
    title_row.add(playlist_heading.as_widget_ref());
    title_row.add_spacer(8);
    title_row.add_with_proportion(playlist_name.as_widget_ref(), 1);
    playlist_inner.add_sizer(title_row);
    playlist_inner.add_spacer(6);

    let mut filter_row = BoxSizer::horizontal();
    filter_row.add(search_label.as_widget_ref());
    filter_row.add_spacer(6);
    filter_row.add_with_proportion(playlist_search.as_widget_ref(), 1);
    filter_row.add_spacer(12);
    filter_row.add(sort_label.as_widget_ref());
    filter_row.add_spacer(6);
    filter_row.add(sort_choice.as_widget_ref());
    filter_row.add_spacer(6);
    filter_row.add(sort_dir_btn.as_widget_ref());
    playlist_inner.add_sizer(filter_row);
    playlist_inner.add_spacer(6);
    playlist_inner.add_with_proportion(playlist.as_widget_ref(), 1);
    playlist_panel.set_sizer(playlist_inner);

    let mut top_row = BoxSizer::horizontal();
    top_row.add_spacer(10);
    top_row.add(meta_panel.as_widget_ref());
    top_row.add_spacer(10);
    top_row.add_with_proportion(playlist_panel.as_widget_ref(), 1);
    top_row.add_spacer(10);

    let mut transport = BoxSizer::horizontal();
    transport.add_stretch(1);
    transport.add(btn_play.as_widget_ref());
    transport.add_spacer(12);
    transport.add(btn_pause.as_widget_ref());
    transport.add_spacer(12);
    transport.add(btn_stop.as_widget_ref());
    transport.add_stretch(1);

    let mut transport_inner = BoxSizer::vertical();
    transport_inner.set_padding(8);
    transport_inner.add_sizer(transport);
    transport_inner.add_spacer(8);
    transport_inner.add(progress.as_widget_ref());
    transport_inner.add_spacer(4);
    transport_inner.add(time_label.as_widget_ref());
    transport_panel.set_sizer(transport_inner);
    transport_panel
        .as_widget_ref()
        .borrow_mut()
        .set_size(720, TRANSPORT_PANEL_H);

    let mut sizer = BoxSizer::vertical();
    sizer.set_padding(10);
    sizer.add_spacer(toolbar_h);
    sizer.add_sizer_with_proportion(top_row, 1);
    sizer.add_spacer(10);
    sizer.add(transport_panel.as_widget_ref());
    sizer.add_spacer(STATUS_BAR_RESERVE);
    frame.set_sizer(sizer);

    toolbar.refresh_layout();
    toolbar.bring_to_front();
    #[cfg(target_os = "macos")]
    {
        use ru_wx::platform::appkit::{button_rewire_click_target, mac_view_raise_to_front};
        let panel = transport_panel.hwnd();
        let play_id = btn_play.id();
        let pause_id = btn_pause.id();
        let stop_id = btn_stop.id();
        let slider_id = progress.id();
        let play_h = btn_play.as_widget_ref().borrow().native_handle();
        let pause_h = btn_pause.as_widget_ref().borrow().native_handle();
        let stop_h = btn_stop.as_widget_ref().borrow().native_handle();
        let slider_h = progress.as_widget_ref().borrow().native_handle();
        ru_wx::platform::appkit::run_on_next_event(move || {
            mac_view_raise_to_front(panel);
            mac_view_raise_to_front(play_h);
            mac_view_raise_to_front(pause_h);
            mac_view_raise_to_front(stop_h);
            mac_view_raise_to_front(slider_h);
            button_rewire_click_target(play_h, panel, play_id);
            button_rewire_click_target(pause_h, panel, pause_id);
            button_rewire_click_target(stop_h, panel, stop_id);
            button_rewire_click_target(slider_h, panel, slider_id);
        });
    }

    #[cfg(target_os = "macos")]
    if std::env::var("RU_WX_MUSIC_SELF_TEST").is_ok() {
        run_music_self_test(
            &frame,
            append_track.clone(),
            btn_play.id(),
            btn_pause.id(),
            btn_stop.id(),
            progress.clone(),
            media.clone(),
            progress_clock.clone(),
        );
    }

    let _media_keepalive = media;

    app.run(frame);
}

#[cfg(target_os = "macos")]
#[allow(clippy::too_many_arguments)] // self-test harness: one argument per control under test
fn run_music_self_test(
    frame: &Frame,
    append: Rc<dyn Fn(PathBuf)>,
    play_id: u16,
    pause_id: u16,
    stop_id: u16,
    progress: Slider,
    media: Rc<RefCell<MediaCtrl>>,
    clock: Rc<RefCell<ProgressClock>>,
) {
    use ru_wx::platform::appkit::run_on_next_event;

    let log_path = std::env::var("RU_WX_MUSIC_SELF_TEST_LOG")
        .unwrap_or_else(|_| "/tmp/ru_wx_music_selftest.txt".into());
    let track_path = std::env::var("RU_WX_MUSIC_SELF_TEST_PATH")
        .unwrap_or_else(|_| "/tmp/ru_wx_test_track.mp3".into());

    let _ = std::fs::write(&log_path, "music_player self-test\n");

    let frame = frame.clone();
    run_on_next_event(move || {
        append(PathBuf::from(&track_path));
        self_test_log(&log_path, "append track");

        frame.dispatch_command(play_id);
        self_test_log(&log_path, "dispatch play");
        let playing = media.borrow().state() == MediaState::Playing;
        self_test_log(&log_path, &format!("after play: playing={playing}"));

        frame.dispatch_command(pause_id);
        self_test_log(&log_path, "dispatch pause");
        let paused = media.borrow().state() == MediaState::Paused;
        self_test_log(&log_path, &format!("after pause: paused={paused}"));

        frame.dispatch_command(play_id);
        self_test_log(&log_path, "dispatch resume");
        let resumed = media.borrow().state() == MediaState::Playing;
        self_test_log(&log_path, &format!("after resume: playing={resumed}"));

        progress.set_value(500);
        frame.dispatch_command(progress.id());
        self_test_log(&log_path, "dispatch seek");
        let seeked = clock.borrow().seek_pending;
        self_test_log(&log_path, &format!("after seek: pending={seeked}"));

        frame.dispatch_command(stop_id);
        self_test_log(&log_path, "dispatch stop");
        let stopped = media.borrow().state() == MediaState::Stopped;
        self_test_log(&log_path, &format!("after stop: stopped={stopped}"));

        if playing && paused && resumed && seeked && stopped {
            self_test_log(&log_path, "PASS");
        } else {
            self_test_log(&log_path, "FAIL");
        }
        frame.close();
    });
}

#[cfg(target_os = "macos")]
fn self_test_log(path: &str, msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "{msg}");
    }
}
