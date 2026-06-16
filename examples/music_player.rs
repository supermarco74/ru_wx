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
    Accelerator, App, AuiToolBar, BitmapBundle, BoxSizer, Button, DroppedFiles, FileDialog,
    FileDialogStyle, Font, FontDialog, Frame, Image, ImageList, ListBox, MediaCtrl, Menu, MenuBar,
    PopupMenu, RawBitmap, Slider, StaticBitmap, StaticText, StatusBar, Timer, Bitmap,
};

const ID_TOOL_LOAD: u16 = 2001;
const ID_TOOL_LOAD_PLAYLIST: u16 = 2002;
const ID_TOOL_SAVE_PLAYLIST: u16 = 2003;

const ICON_LOAD: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#10B981"/><path d="M12 6v8 M8 10l4-4 4 4 M6 18h12" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round"/></svg>"##;
const ICON_PLAYLIST: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#4F46E5"/><path d="M7 8h10 M7 12h10 M7 16h6" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round"/><path d="M17 14l3 2-3 2z" fill="white"/></svg>"##;
const ICON_SAVE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="3" fill="#2563EB"/><path d="M8 7h8v4H8z M8 15h8 M6 6h12v12H6z" fill="none" stroke="white" stroke-width="1.8" stroke-linejoin="round"/></svg>"##;
const ICON_COVER_PLACEHOLDER: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128"><rect width="128" height="128" rx="8" fill="#E5E7EB"/><circle cx="64" cy="64" r="28" fill="none" stroke="#9CA3AF" stroke-width="4"/><circle cx="64" cy="64" r="8" fill="#9CA3AF"/><path d="M64 36v-8 M64 100v-8 M36 64h-8 M100 64h-8" stroke="#9CA3AF" stroke-width="3" stroke-linecap="round"/></svg>"##;

const AUDIO_EXT: &[&str] = &["mp3", "wav", "wma", "flac", "ogg", "m4a"];
const COVER_SIZE: u32 = 128;

/// ID3v1 genre index → name (Winamp / ID3 spec list).
const ID3_GENRES: [&str; 192] = [
    "Blues", "Classic Rock", "Country", "Dance", "Disco", "Funk", "Grunge", "Hip-Hop",
    "Jazz", "Metal", "New Age", "Oldies", "Other", "Pop", "R&B", "Rap", "Reggae", "Rock",
    "Techno", "Industrial", "Alternative", "Ska", "Death Metal", "Pranks", "Soundtrack",
    "Euro-Techno", "Ambient", "Trip-Hop", "Vocal", "Jazz+Funk", "Fusion", "Trance",
    "Classical", "Instrumental", "Acid", "House", "Game", "Sound Clip", "Gospel", "Noise",
    "Alt. Rock", "Bass", "Soul", "Punk", "Space", "Meditative", "Instrumental Pop",
    "Instrumental Rock", "Ethnic", "Gothic", "Darkwave", "Techno-Industrial",
    "Electronic", "Pop-Folk", "Eurodance", "Dream", "Southern Rock", "Comedy", "Cult",
    "Gangsta Rap", "Top 40", "Christian Rap", "Pop/Funk", "Jungle", "Native American",
    "Cabaret", "New Wave", "Psychedelic", "Rave", "Showtunes", "Trailer", "Lo-Fi",
    "Tribal", "Acid Punk", "Acid Jazz", "Polka", "Retro", "Musical", "Rock & Roll",
    "Hard Rock", "Folk", "Folk-Rock", "National Folk", "Swing", "Fast Fusion", "Bebop",
    "Latin", "Revival", "Celtic", "Bluegrass", "Avantgarde", "Gothic Rock",
    "Progressive Rock", "Psychedelic Rock", "Symphonic Rock", "Slow Rock", "Big Band",
    "Chorus", "Easy Listening", "Acoustic", "Humour", "Speech", "Chanson", "Opera",
    "Chamber Music", "Sonata", "Symphony", "Booty Bass", "Primus", "Porn Groove",
    "Satire", "Slow Jam", "Club", "Tango", "Samba", "Folklore", "Ballad", "Power Ballad",
    "Rhythmic Soul", "Freestyle", "Duet", "Punk Rock", "Drum Solo", "A Cappella",
    "Euro-House", "Dance Hall", "Goa", "Drum & Bass", "Club-House", "Hardcore",
    "Terror", "Indie", "BritPop", "Afro-Punk", "Polsk Punk", "Beat", "Christian Gangsta",
    "Heavy Metal", "Black Metal", "Crossover", "Contemporary Christian", "Christian Rock",
    "Merengue", "Salsa", "Thrash Metal", "Anime", "JPop", "Synthpop", "Abstract",
    "Art Rock", "Baroque", "Bhangra", "Big Beat", "Breakbeat", "Chillout", "Downtempo",
    "Dub", "EBM", "Eclectic", "Electro", "Electroclash", "Emo", "Experimental", "Garage",
    "Global", "IDM", "Illbient", "Industro-Goth", "Jam Band", "Krautrock", "Leftfield",
    "Lounge", "Math Rock", "New Romantic", "Nu-Breakz", "Post-Punk", "Post-Rock",
    "Psytrance", "Shoegaze", "Space Rock", "Trop Rock", "World Music", "Neoclassical",
    "Audiobook", "Audio Theatre", "Neue Deutsche Welle", "Podcast", "Indie Rock",
    "G-Funk", "Dubstep", "Garage Rock", "Psybient",
];

/// Wall-clock progress when MCI `status position` is unavailable or stale.
struct ProgressClock {
    anchor_ms: u64,
    length_ms: u64,
    running: bool,
    since: Option<Instant>,
}

impl ProgressClock {
    fn new() -> Self {
        Self {
            anchor_ms: 0,
            length_ms: 0,
            running: false,
            since: None,
        }
    }

    fn begin(&mut self, length_ms: u64) {
        self.anchor_ms = 0;
        self.length_ms = length_ms;
        self.running = true;
        self.since = Some(Instant::now());
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
    }

    fn seek(&mut self, ms: u64) {
        self.anchor_ms = ms.min(self.length_ms);
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
    display: String,
    size_bytes: u64,
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
        let display = format_track_line(&meta, size_bytes);
        Self {
            path,
            meta,
            display,
            size_bytes,
        }
    }
}

fn format_track_line(meta: &TrackMeta, size_bytes: u64) -> String {
    let size = format_file_size(size_bytes);
    let artist = meta.display_artist();
    let album = meta.album.as_str();
    match (artist.is_empty(), album.is_empty()) {
        (true, true) => format!("{:<42}  {size}", meta.title),
        (false, true) => format!("{artist} — {:<32}  {size}", meta.title),
        (true, false) => format!("{:<32}  [{album}]  {size}", meta.title),
        (false, false) => format!("{artist} — {title}  [{album}]  {size}", title = meta.title),
    }
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
            u32::from_be_bytes([
                data[pos + 4],
                data[pos + 5],
                data[pos + 6],
                data[pos + 7],
            ]) as usize
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
            b"APIC" | b"PIC " => {
                if meta.cover.is_none() {
                    meta.cover = parse_apic_payload(payload);
                }
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

fn set_cover_art(cover: &StaticBitmap, bytes: Option<&[u8]>) {
    let loaded = bytes.and_then(|data| Image::load_from_memory(data).ok().map(|img| img.to_bitmap()));
    let bmp = loaded.unwrap_or_else(|| Bitmap::from_svg_bytes(ICON_COVER_PLACEHOLDER, COVER_SIZE, COVER_SIZE));
    cover.set_bitmap(RawBitmap {
        hbitmap: bmp.handle(),
        width: bmp.width,
        height: bmp.height,
    });
}

fn refresh_metadata_panel(
    track: Option<&Track>,
    cover: &StaticBitmap,
    lbl_title: &StaticText,
    lbl_artist: &StaticText,
    lbl_album: &StaticText,
    lbl_year: &StaticText,
    lbl_genre: &StaticText,
    lbl_track: &StaticText,
    lbl_composer: &StaticText,
    lbl_comment: &StaticText,
    lbl_file: &StaticText,
) {
    let Some(track) = track else {
        set_cover_art(cover, None);
        lbl_title.set_label("Titolo: —");
        lbl_artist.set_label("Artista: —");
        lbl_album.set_label("Album: —");
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
    lbl_title.set_label(&meta_label("Titolo", &m.title));
    lbl_artist.set_label(&meta_label(
        "Artista",
        if !m.artist.is_empty() {
            &m.artist
        } else {
            m.album_artist.as_str()
        },
    ));
    lbl_album.set_label(&meta_label("Album", &m.album));
    lbl_year.set_label(&meta_label("Anno", &m.year));
    lbl_genre.set_label(&meta_label("Genere", &m.genre));
    lbl_track.set_label(&meta_label("Traccia", &m.track_num));
    lbl_composer.set_label(&meta_label("Compositore", &m.composer));
    lbl_comment.set_label(&meta_label("Commento", &m.comment));
    lbl_file.set_label(&format!(
        "File: {}  ({})",
        track.path.file_name().and_then(|s| s.to_str()).unwrap_or("?"),
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
            let raw = payload[1..]
                .split(|&b| b == 0)
                .next()
                .unwrap_or(&[]);
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
        let resolved = if p.is_absolute() {
            p
        } else {
            base.join(p)
        };
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

fn save_playlist_dialog(frame: &Frame) -> Option<String> {
    let mut dlg = FileDialog::new(frame, FileDialogStyle::Save);
    dlg.set_title("Salva playlist");
    dlg.set_filename("playlist.m3u");
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

    fn path(&self, index: usize) -> Option<&Path> {
        self.tracks.get(index).map(|t| t.path.as_path())
    }

    fn len(&self) -> usize {
        self.tracks.len()
    }
}

fn main() {
    let app = App::new();
    let frame = Frame::builder()
        .with_title("ru_wx — Music Player")
        .with_size(780, 640)
        .build();

    let status = StatusBar::new(&frame, 2);
    status.set_status_text("Carica brani con la toolbar o trascinali sulla lista.", 0);
    status.set_status_text("Nessun brano in coda.", 1);

    // ── Toolbar icone ────────────────────────────────────────────────
    let icon_sizes: [(u32, u32); 2] = [(32, 32), (40, 40)];
    let bundle_load = BitmapBundle::from_svg_bytes(ICON_LOAD, &icon_sizes);
    let bundle_list = BitmapBundle::from_svg_bytes(ICON_PLAYLIST, &icon_sizes);
    let bundle_save = BitmapBundle::from_svg_bytes(ICON_SAVE, &icon_sizes);
    let images = ImageList::new(32, 32);
    if let Some(bmp) = bundle_load.best_for_size((32, 32)) {
        images.add_bitmap(bmp.hbitmap);
    }
    if let Some(bmp) = bundle_list.best_for_size((32, 32)) {
        images.add_bitmap(bmp.hbitmap);
    }
    if let Some(bmp) = bundle_save.best_for_size((32, 32)) {
        images.add_bitmap(bmp.hbitmap);
    }

    let toolbar = AuiToolBar::new(&frame);
    toolbar.set_toolbar_height(48);
    toolbar.set_image_list(&images);
    toolbar.add_tool(ID_TOOL_LOAD, "Carica file audio…", 0);
    toolbar.add_tool(ID_TOOL_LOAD_PLAYLIST, "Carica playlist…", 1);
    toolbar.add_tool(ID_TOOL_SAVE_PLAYLIST, "Salva playlist…", 2);
    toolbar.realize();
    let toolbar_h = toolbar.reserved_height();

    let playlist_label = StaticText::new(
        &frame,
        "Playlist — doppio clic: riproduci | tasto destro: rimuovi | trascina MP3 qui | seleziona per metadati ID3:",
    );
    let playlist = ListBox::new(&frame);
    playlist
        .as_widget_ref()
        .borrow_mut()
        .set_size(480, 240);

    // ── Pannello metadati ID3 (tag internet / embedded) ─────────────
    let cover_art = StaticBitmap::with_size(&frame, COVER_SIZE, COVER_SIZE);
    set_cover_art(&cover_art, None);

    let meta_title = StaticText::new(&frame, "Titolo: —");
    let meta_artist = StaticText::new(&frame, "Artista: —");
    let meta_album = StaticText::new(&frame, "Album: —");
    let meta_year = StaticText::new(&frame, "Anno: —");
    let meta_genre = StaticText::new(&frame, "Genere: —");
    let meta_track = StaticText::new(&frame, "Traccia: —");
    let meta_composer = StaticText::new(&frame, "Compositore: —");
    let meta_comment = StaticText::new(&frame, "Commento: —");
    let meta_file = StaticText::new(&frame, "File: —");

    let time_label = StaticText::new(&frame, "00:00 / 00:00");
    let progress = Slider::new(&frame, 0, 1000, 0);
    progress.as_widget_ref().borrow_mut().set_size(640, 32);

    let btn_play = Button::new(&frame, "Play");
    let btn_pause = Button::new(&frame, "Pausa");
    let btn_stop = Button::new(&frame, "Stop");

    let media = Rc::new(RefCell::new(MediaCtrl::new(&frame)));
    let tracks = Rc::new(RefCell::new(Playlist {
        tracks: Vec::new(),
        playing: None,
    }));
    let stop_after_each = Rc::new(Cell::new(false));
    let programmatic_slider = Rc::new(Cell::new(false));
    let track_finished_flag = Rc::new(Cell::new(false));
    let progress_clock = Rc::new(RefCell::new(ProgressClock::new()));

    let update_status_count = {
        let tracks = tracks.clone();
        let status = status.clone();
        Rc::new(move || {
            status.set_status_text(
                &format!("Brani in playlist: {}", tracks.borrow().len()),
                1,
            );
        })
    };

    let refresh_meta: Rc<dyn Fn(Option<usize>)> = {
        let tracks = tracks.clone();
        let cover = cover_art.clone();
        let meta_title = meta_title.clone();
        let meta_artist = meta_artist.clone();
        let meta_album = meta_album.clone();
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
                &meta_title,
                &meta_artist,
                &meta_album,
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
        let playlist = playlist.clone();
        let tracks = tracks.clone();
        let status = status.clone();
        let update_count = update_status_count.clone();
        let refresh_meta = refresh_meta.clone();
        Rc::new(move |path| {
            if !is_audio_file(&path) {
                status.set_status_text("File non audio ignorato.", 0);
                return;
            }
            let track = Track::from_path(path);
            let label = track.display.clone();
            let idx = tracks.borrow_mut().add(track);
            playlist.append(&label);
            status.set_status_text(&format!("Aggiunto: {label}"), 0);
            update_count();
            playlist.set_selection(idx);
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

    let play_track: Rc<dyn Fn(usize)> = {
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
        Rc::new(move |index: usize| {
            track_finished_flag.set(false);
            let path = {
                let mut t = tracks.borrow_mut();
                t.playing = Some(index);
                match t.path(index) {
                    Some(p) => p.to_path_buf(),
                    None => return,
                }
            };
            playlist.set_selection(index);
            refresh_meta(Some(index));
            let m = media.borrow_mut();
            if let Err(e) = m.load(&path) {
                status.set_status_text(&format!("Errore caricamento: {e}"), 0);
                return;
            }
            if let Err(e) = m.play() {
                status.set_status_text(&format!("Errore riproduzione: {e}"), 0);
                return;
            }
            let title = tracks
                .borrow()
                .tracks
                .get(index)
                .map(|t| t.meta.title.clone())
                .unwrap_or_else(|| "Brano".to_string());
            let artist = tracks
                .borrow()
                .tracks
                .get(index)
                .and_then(|t| {
                    let a = t.meta.display_artist();
                    if a.is_empty() { None } else { Some(a.to_string()) }
                });
            let album = tracks
                .borrow()
                .tracks
                .get(index)
                .and_then(|t| {
                    if t.meta.album.is_empty() {
                        None
                    } else {
                        Some(t.meta.album.clone())
                    }
                });
            let now_playing = match (artist.as_deref(), album.as_deref()) {
                (Some(a), Some(al)) => format!("{a} — {title}  [{al}]"),
                (Some(a), None) => format!("{a} — {title}"),
                (None, Some(al)) => format!("{title}  [{al}]"),
                (None, None) => title.clone(),
            };
            status.set_status_text(&format!("In riproduzione: {now_playing}"), 0);
            let len = m.length_ms().unwrap_or(0).max(1);
            clock.borrow_mut().begin(len);
            let max = len.min(i32::MAX as u64) as i32;
            progress.set_range(0, max);
            programmatic_slider.set(true);
            progress.set_value(0);
            programmatic_slider.set(false);
            time_label.set_label(&format!("00:00 / {}", format_ms(len)));
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
        Rc::new(move || {
            let m = media.borrow();
            if let Err(e) = m.play() {
                status.set_status_text(&format!("Play: {e}"), 0);
            } else {
                clock.borrow_mut().resume();
            }
        })
    };

    let do_pause: Rc<dyn Fn()> = {
        let media = media.clone();
        let status = status.clone();
        let clock = progress_clock.clone();
        Rc::new(move || {
            let m = media.borrow();
            if let Err(e) = m.pause() {
                status.set_status_text(&format!("Pausa: {e}"), 0);
            } else {
                clock.borrow_mut().pause();
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
        Rc::new(move || {
            let m = media.borrow();
            if let Err(e) = m.stop() {
                status.set_status_text(&format!("Stop: {e}"), 0);
            }
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
                play_next();
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
        Rc::new(move || {
            if let Some(path_str) = open_playlist_dialog(&frame) {
                let paths = parse_playlist_file(Path::new(&path_str));
                if paths.is_empty() {
                    status.set_status_text("Playlist vuota o non valida.", 0);
                } else {
                    let n = paths.len();
                    append_paths(paths);
                    status.set_status_text(&format!("Playlist caricata: {n} brani."), 0);
                }
            }
        })
    };

    let save_playlist_file: Rc<dyn Fn()> = {
        let frame = frame.clone();
        let tracks = tracks.clone();
        let status = status.clone();
        Rc::new(move || {
            let list = tracks.borrow().tracks.clone();
            if list.is_empty() {
                status.set_status_text("Playlist vuota: niente da salvare.", 0);
                return;
            }
            if let Some(path_str) = save_playlist_dialog(&frame) {
                let path = playlist_save_path(PathBuf::from(path_str));
                match write_playlist_file(&path, &list) {
                    Ok(()) => {
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
                playlist.remove(index);
                update_count();
                (name, was_playing)
            };
            if removed.1 {
                do_stop();
            }
            let sel = playlist.get_selection();
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
    let opts_hmenu = opts_menu.hmenu();
    let stop_after_id_cell = Rc::new(Cell::new(0u16));
    let stop_after_id_for_cb = stop_after_id_cell.clone();
    let stop_after_id = opts_menu.append_check_item(
        "Ferma dopo ogni brano (playlist)",
        &frame,
        move || {
            let v = !stop_flag.get();
            stop_flag.set(v);
            #[cfg(target_os = "windows")]
            unsafe {
                use windows_sys::Win32::UI::WindowsAndMessaging::{
                    CheckMenuItem, MF_BYCOMMAND, MF_CHECKED, MF_UNCHECKED,
                };
                let flags = if v { MF_CHECKED } else { MF_UNCHECKED };
                CheckMenuItem(
                    opts_hmenu,
                    stop_after_id_for_cb.get() as u32,
                    MF_BYCOMMAND | flags,
                );
            }
            status_flag.set_status_text(
                if v {
                    "Opzione: ferma dopo ogni brano — attiva."
                } else {
                    "Opzione: ferma dopo ogni brano — disattiva (avanzamento automatico)."
                },
                0,
            );
        },
    );
    stop_after_id_cell.set(stop_after_id);

    // Font UI (playlist + metadati + tempo)
    let ui_font_store = ui_font.clone();
    let frame_for_font = frame.clone();
    let playlist_for_font = playlist.clone();
    let playlist_label_for_font = playlist_label.clone();
    let meta_title_for_font = meta_title.clone();
    let meta_artist_for_font = meta_artist.clone();
    let meta_album_for_font = meta_album.clone();
    let meta_year_for_font = meta_year.clone();
    let meta_genre_for_font = meta_genre.clone();
    let meta_track_for_font = meta_track.clone();
    let meta_composer_for_font = meta_composer.clone();
    let meta_comment_for_font = meta_comment.clone();
    let meta_file_for_font = meta_file.clone();
    let time_label_for_font = time_label.clone();
    opts_menu.append(
        "Font &testo…",
        &frame,
        move || {
            let mut dlg = FontDialog::new(&frame_for_font);
            if let Some(new_font) = dlg.show_modal() {
                *ui_font_store.borrow_mut() = Some(new_font);
                if let Some(ref font) = *ui_font_store.borrow() {
                    playlist_for_font.set_font(font);
                    playlist_label_for_font.set_font(font);
                    meta_title_for_font.set_font(font);
                    meta_artist_for_font.set_font(font);
                    meta_album_for_font.set_font(font);
                    meta_year_for_font.set_font(font);
                    meta_genre_for_font.set_font(font);
                    meta_track_for_font.set_font(font);
                    meta_composer_for_font.set_font(font);
                    meta_comment_for_font.set_font(font);
                    meta_file_for_font.set_font(font);
                    time_label_for_font.set_font(font);
                }
            }
        },
    );

    let mut ctrl_menu = Menu::new("&Controlli");
    let play_for_menu = do_play.clone();
    ctrl_menu.append("&Play", &frame, move || play_for_menu());
    let pause_for_menu = do_pause.clone();
    ctrl_menu.append("&Pausa", &frame, move || pause_for_menu());
    let stop_for_menu = do_stop.clone();
    ctrl_menu.append("&Stop", &frame, move || stop_for_menu());

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
    let status_tool = status.clone();
    toolbar.on_tool_clicked(&frame, move |id| {
        match id {
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
            _ => {}
        }
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

    // ── Selezione playlist → aggiorna metadati ───────────────────────
    let refresh_sel = refresh_meta.clone();
    let playlist_sel = playlist.clone();
    playlist.on_selection_change(&frame, move || {
        refresh_sel(playlist_sel.get_selection());
    });

    // ── Doppio clic ──────────────────────────────────────────────────
    let play_for_dbl = play_track.clone();
    let playlist_dbl = playlist.clone();
    playlist.on_double_click(&frame, move || {
        if let Some(idx) = playlist_dbl.get_selection() {
            play_for_dbl(idx);
        }
    });

    // ── Menu contestuale ─────────────────────────────────────────────
    let playlist_ctx = playlist.clone();
    let frame_ctx = frame.clone();
    let remove_for_ctx = remove_at.clone();
    frame.on_context_menu(move |ev| {
        if ev.is_keyboard {
            return;
        }
        let Some(idx) = playlist_ctx.item_at_screen_point(ev.position.x, ev.position.y) else {
            return;
        };
        playlist_ctx.set_selection(idx);
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
        let m = media_seek.borrow();
        let _ = m.seek_ms(pos_ms);
        let total = m.length_ms().unwrap_or_else(|| clock_seek.borrow().length_ms);
        clock_seek.borrow_mut().seek(pos_ms);
        time_seek.set_label(&format!("{} / {}", format_ms(pos_ms), format_ms(total.max(1))));
    });

    // ── Timer: avanzamento barra + fine brano ────────────────────────
    let refresh = Timer::new(&frame);
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
        let m = media_tick.borrow();
        let len = {
            let mut c = clock_tick.borrow_mut();
            if let Some(l) = m.length_ms() {
                c.set_length(l);
            }
            c.length_ms.max(1)
        };
        if let Some(p) = m.position_ms() {
            clock_tick.borrow_mut().sync_from_mci(p);
        }
        let pos = clock_tick.borrow().position_ms();
        let pos_i = pos.min(i32::MAX as u64) as i32;
        let len_i = len.min(i32::MAX as u64) as i32;
        progress_tick.set_range(0, len_i);
        programmatic_tick.set(true);
        progress_tick.set_value(pos_i);
        programmatic_tick.set(false);
        time_tick.set_label(&format!("{} / {}", format_ms(pos), format_ms(len)));
        if pos + 400 >= len {
            on_finished();
        }
    });
    refresh.start(Duration::from_millis(100));

    // ── Layout ───────────────────────────────────────────────────────
    let mut meta_lines = BoxSizer::vertical();
    meta_lines.add(meta_title.as_widget_ref());
    meta_lines.add(meta_artist.as_widget_ref());
    meta_lines.add(meta_album.as_widget_ref());
    meta_lines.add(meta_year.as_widget_ref());
    meta_lines.add(meta_genre.as_widget_ref());
    meta_lines.add(meta_track.as_widget_ref());
    meta_lines.add(meta_composer.as_widget_ref());
    meta_lines.add(meta_comment.as_widget_ref());
    meta_lines.add(meta_file.as_widget_ref());

    let mut meta_col = BoxSizer::vertical();
    meta_col.add(cover_art.as_widget_ref());
    meta_col.add_spacer(8);
    meta_col.add_sizer(meta_lines);

    let mut playlist_col = BoxSizer::vertical();
    playlist_col.add(playlist_label.as_widget_ref());
    playlist_col.add_spacer(4);
    playlist_col.add_with_proportion(playlist.as_widget_ref(), 1);

    let mut top_row = BoxSizer::horizontal();
    top_row.add_sizer(meta_col);
    top_row.add_spacer(12);
    top_row.add_sizer_with_proportion(playlist_col, 1);

    let mut transport = BoxSizer::horizontal();
    transport.add(btn_play.as_widget_ref());
    transport.add(btn_pause.as_widget_ref());
    transport.add(btn_stop.as_widget_ref());

    let mut sizer = BoxSizer::vertical();
    sizer.set_padding(8);
    sizer.add_spacer(toolbar_h);
    sizer.add_sizer_with_proportion(top_row, 1);
    sizer.add_spacer(8);
    sizer.add_sizer(transport);
    sizer.add_spacer(6);
    sizer.add(progress.as_widget_ref());
    sizer.add(time_label.as_widget_ref());
    frame.set_sizer(sizer);

    toolbar.bring_to_front();

    let _media_keepalive = media;
    app.run(frame);
}
