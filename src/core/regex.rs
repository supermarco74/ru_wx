//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Pattern matching helper (`wxRegEx`).

/// Simple wildcard matcher (`wxRegEx`).
///
/// Supports `*` (any sequence) and `?` (single character). For full
/// regex semantics integrate a dedicated engine later.
#[derive(Debug, Clone)]
pub struct RegEx {
    pattern: String,
}

impl RegEx {
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
        }
    }

    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    pub fn matches(&self, text: &str) -> bool {
        Self::wildcard_match(&self.pattern, text)
    }

    pub fn replace(&self, text: &str, replacement: &str) -> String {
        if self.matches(text) {
            replacement.to_string()
        } else {
            text.to_string()
        }
    }

    fn wildcard_match(pattern: &str, text: &str) -> bool {
        let p: Vec<char> = pattern.chars().collect();
        let t: Vec<char> = text.chars().collect();
        Self::match_at(&p, &t, 0, 0)
    }

    fn match_at(pattern: &[char], text: &[char], pi: usize, ti: usize) -> bool {
        if pi == pattern.len() {
            return ti == text.len();
        }
        if pattern[pi] == '*' {
            for i in ti..=text.len() {
                if Self::match_at(pattern, text, pi + 1, i) {
                    return true;
                }
            }
            return false;
        }
        if ti >= text.len() {
            return false;
        }
        if pattern[pi] == '?' || pattern[pi] == text[ti] {
            return Self::match_at(pattern, text, pi + 1, ti + 1);
        }
        false
    }
}
