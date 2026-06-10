//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Line-oriented text file (`wxTextFile`).

use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Read or write a text file line by line (`wxTextFile`).
pub struct TextFile {
    path: PathBuf,
    lines: Vec<String>,
    modified: bool,
}

impl TextFile {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;
        Ok(Self {
            path,
            lines,
            modified: false,
        })
    }

    pub fn create(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self {
            path: path.as_ref().to_path_buf(),
            lines: Vec::new(),
            modified: false,
        })
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn get_line(&self, index: usize) -> Option<&str> {
        self.lines.get(index).map(String::as_str)
    }

    pub fn add_line(&mut self, line: &str) {
        self.lines.push(line.to_string());
        self.modified = true;
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }

    pub fn write(&self) -> io::Result<()> {
        let mut file = File::create(&self.path)?;
        for (i, line) in self.lines.iter().enumerate() {
            if i > 0 {
                writeln!(file)?;
            }
            write!(file, "{line}")?;
        }
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
