//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Command-line parser (`wxCmdLineParser`).

use std::collections::HashMap;

/// Parsed command-line switches and positional args (`wxCmdLineParser`).
#[derive(Debug, Clone, Default)]
pub struct CmdLineParser {
    switches: HashMap<String, Option<String>>,
    positional: Vec<String>,
}

impl CmdLineParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse<I, S>(&mut self, args: I) -> Result<(), String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.switches.clear();
        self.positional.clear();
        let mut iter = args.into_iter().peekable();
        while let Some(arg) = iter.next() {
            let text = arg.as_ref();
            if let Some(name) = text.strip_prefix("--") {
                let (key, value) = match name.split_once('=') {
                    Some((k, v)) => (k.to_string(), Some(v.to_string())),
                    None => {
                        if let Some(next) = iter.peek() {
                            let n = next.as_ref();
                            if !n.starts_with('-') {
                                let v = iter.next().unwrap();
                                (name.to_string(), Some(v.as_ref().to_string()))
                            } else {
                                (name.to_string(), None)
                            }
                        } else {
                            (name.to_string(), None)
                        }
                    }
                };
                self.switches.insert(key, value);
            } else if let Some(ch) = text.strip_prefix('-').and_then(|s| s.chars().next()) {
                self.switches
                    .insert(ch.to_string(), text.get(2..).map(str::to_string));
            } else {
                self.positional.push(text.to_string());
            }
        }
        Ok(())
    }

    pub fn found_switch(&self, name: &str) -> bool {
        self.switches.contains_key(name)
    }

    pub fn switch_value(&self, name: &str) -> Option<&str> {
        self.switches.get(name).and_then(|v| v.as_deref())
    }

    pub fn positional(&self) -> &[String] {
        &self.positional
    }
}
