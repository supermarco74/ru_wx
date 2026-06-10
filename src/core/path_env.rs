//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! PATH environment helper (`wxPathEnv`).

use std::env;
use std::path::PathBuf;

/// Read and update the process `PATH` variable (`wxPathEnv`).
#[derive(Debug, Default)]
pub struct PathEnv;

impl PathEnv {
    pub fn new() -> Self {
        Self
    }

    pub fn get_paths() -> Vec<PathBuf> {
        let Ok(value) = env::var("PATH") else {
            return Vec::new();
        };
        #[cfg(target_os = "windows")]
        let sep = ';';
        #[cfg(not(target_os = "windows"))]
        let sep = ':';
        value
            .split(sep)
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .collect()
    }

    pub fn prepend(path: &str) -> bool {
        let mut paths = Self::get_paths();
        paths.insert(0, PathBuf::from(path));
        Self::set_paths(&paths)
    }

    pub fn append(path: &str) -> bool {
        let mut paths = Self::get_paths();
        paths.push(PathBuf::from(path));
        Self::set_paths(&paths)
    }

    pub fn set_paths(paths: &[PathBuf]) -> bool {
        #[cfg(target_os = "windows")]
        let sep = ';';
        #[cfg(not(target_os = "windows"))]
        let sep = ':';
        let joined = paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(&sep.to_string());
        env::set_var("PATH", joined);
        true
    }
}
