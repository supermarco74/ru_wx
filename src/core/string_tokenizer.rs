//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! String tokenizer (`wxStringTokenizer`).

/// Split a string on delimiter characters (`wxStringTokenizer`).
#[derive(Debug, Clone)]
pub struct StringTokenizer {
    rest: String,
    delimiters: String,
    include_empty: bool,
}

impl StringTokenizer {
    pub fn new(text: &str, delimiters: &str) -> Self {
        Self {
            rest: text.to_string(),
            delimiters: delimiters.to_string(),
            include_empty: false,
        }
    }

    pub fn include_empty(mut self, include: bool) -> Self {
        self.include_empty = include;
        self
    }

    pub fn has_more_tokens(&self) -> bool {
        !self.rest.is_empty() || self.include_empty
    }

    pub fn next_token(&mut self) -> Option<String> {
        if self.rest.is_empty() {
            return if self.include_empty {
                self.include_empty = false;
                Some(String::new())
            } else {
                None
            };
        }
        let delim_pos = self
            .rest
            .find(|c| self.delimiters.contains(c))
            .unwrap_or(self.rest.len());
        let token = self.rest[..delim_pos].to_string();
        let advance = if delim_pos < self.rest.len() {
            delim_pos + 1
        } else {
            delim_pos
        };
        self.rest = self.rest[advance..].to_string();
        if token.is_empty() && !self.include_empty {
            return self.next_token();
        }
        Some(token)
    }

    pub fn collect_tokens(mut self) -> Vec<String> {
        let mut out = Vec::new();
        while let Some(tok) = self.next_token() {
            out.push(tok);
        }
        out
    }
}
