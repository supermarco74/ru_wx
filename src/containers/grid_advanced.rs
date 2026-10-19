//! Advanced grid helpers: filter matching, merge registry, PDF export.

use std::collections::HashMap;

use crate::containers::grid_range::GridRange;

/// Returns true when row text matches a global query and per-column filters.
pub fn row_text_matches_filter(
    col_count: usize,
    cell_text_at: &dyn Fn(usize, usize) -> String,
    logical_row: usize,
    filter_query: &str,
    column_filters: &HashMap<usize, String>,
) -> bool {
    if !filter_query.is_empty() {
        let mut hay = String::new();
        for c in 0..col_count {
            hay.push_str(&cell_text_at(logical_row, c));
            hay.push('\t');
        }
        if !hay.to_lowercase().contains(filter_query) {
            return false;
        }
    }
    for (&col, needle) in column_filters {
        if needle.is_empty() {
            continue;
        }
        let text = cell_text_at(logical_row, col).to_lowercase();
        if !text.contains(needle.as_str()) {
            return false;
        }
    }
    true
}

/// True when `(logical_row, col)` is covered by a merge (not the anchor).
pub fn is_merge_covered(
    merge_spans: &HashMap<(usize, usize), (usize, usize)>,
    logical_row: usize,
    col: usize,
) -> bool {
    for (&(ar, ac), &(h, w)) in merge_spans {
        if logical_row >= ar
            && logical_row < ar + h
            && col >= ac
            && col < ac + w
            && (logical_row != ar || col != ac)
        {
            return true;
        }
    }
    false
}

/// Merge span at anchor `(logical_row, col)`, if any.
pub fn merge_span_at(
    merge_spans: &HashMap<(usize, usize), (usize, usize)>,
    logical_row: usize,
    col: usize,
) -> Option<(usize, usize)> {
    merge_spans.get(&(logical_row, col)).copied()
}

/// Register a merged rectangular range (logical coordinates).
pub fn register_merge(
    merge_spans: &mut HashMap<(usize, usize), (usize, usize)>,
    range: GridRange,
) -> bool {
    let h = range.row_count();
    let w = range.col_count();
    if h == 0 || w == 0 {
        return false;
    }
    merge_spans.insert((range.top_left.row, range.top_left.col), (h, w));
    true
}

/// Build a minimal PDF 1.4 document containing TSV lines as monospaced text.
pub fn tsv_to_pdf_bytes(title: &str, tsv: &str) -> Vec<u8> {
    let mut lines: Vec<String> = Vec::new();
    lines.push(pdf_escape_text(title));
    lines.push(String::new());
    for line in tsv.lines().take(52) {
        lines.push(pdf_escape_text(line));
    }
    let mut y = 780_i32;
    let mut ops = String::from("BT\n/F1 9 Tf\n");
    for line in &lines {
        ops.push_str(&format!("1 0 0 1 36 {} Tm ({line}) Tj\n", y));
        y -= 14;
    }
    ops.push_str("ET");
    let stream_len = ops.len();
    format!(
        "%PDF-1.4\n\
1 0 obj<< /Type /Catalog /Pages 2 0 R >>endobj\n\
2 0 obj<< /Type /Pages /Kids [3 0 R] /Count 1 >>endobj\n\
3 0 obj<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
/Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>endobj\n\
4 0 obj<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>endobj\n\
5 0 obj<< /Length {stream_len} >>stream\n{ops}\nendstream\nendobj\n\
xref\n0 6\n0000000000 65535 f \n0000000009 00000 n \n0000000058 00000 n \n\
0000000115 00000 n \n0000000250 00000 n \n0000000320 00000 n \n\
trailer<< /Size 6 /Root 1 0 R >>\nstartxref\n450\n%%EOF"
    )
    .into_bytes()
}

fn pdf_escape_text(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}
