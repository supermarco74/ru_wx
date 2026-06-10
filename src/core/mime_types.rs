//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! MIME / file-type database (`wxMimeTypesManager`).

/// File-type descriptor (`wxFileTypeInfo`).
#[derive(Debug, Clone)]
pub struct FileTypeInfo {
    pub extension: String,
    pub mime_type: String,
    pub description: String,
}

impl FileTypeInfo {
    pub fn new(extension: &str, mime_type: &str, description: &str) -> Self {
        Self {
            extension: extension.to_string(),
            mime_type: mime_type.to_string(),
            description: description.to_string(),
        }
    }
}

/// Lookup helpers (`wxMimeTypesManager`).
#[derive(Debug, Default)]
pub struct MimeTypesManager;

impl MimeTypesManager {
    pub fn new() -> Self {
        Self
    }

    pub fn get_type_from_extension(&self, ext: &str) -> Option<FileTypeInfo> {
        let key = ext.trim_start_matches('.').to_ascii_lowercase();
        match key.as_str() {
            "txt" => Some(FileTypeInfo::new("txt", "text/plain", "Text Document")),
            "png" => Some(FileTypeInfo::new("png", "image/png", "PNG Image")),
            "jpg" | "jpeg" => Some(FileTypeInfo::new("jpg", "image/jpeg", "JPEG Image")),
            "rs" => Some(FileTypeInfo::new("rs", "text/x-rust", "Rust Source")),
            _ => None,
        }
    }

    pub fn get_extension_from_mime(&self, mime: &str) -> Option<String> {
        match mime {
            "text/plain" => Some("txt".into()),
            "image/png" => Some("png".into()),
            "image/jpeg" => Some("jpg".into()),
            _ => None,
        }
    }
}
