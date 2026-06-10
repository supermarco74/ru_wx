//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Drop-files event (`wxDropFilesEvent`).

use std::path::PathBuf;

use crate::core::geometry::Point;
use crate::dnd::drop_target::DroppedFiles;

/// Files dropped on a window (`wxDropFilesEvent`).
#[derive(Debug, Clone)]
pub struct DropFilesEvent {
    pub paths: Vec<PathBuf>,
    pub position: Point,
}

impl DropFilesEvent {
    pub fn new(paths: Vec<PathBuf>, position: Point) -> Self {
        Self { paths, position }
    }

    pub fn from_dropped(files: DroppedFiles, position: Point) -> Self {
        Self {
            paths: files.into_paths(),
            position,
        }
    }

    pub fn len(&self) -> usize {
        self.paths.len()
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }
}
