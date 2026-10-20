//! Standalone unit test for the duplicate-name algorithm.
//! Run with: cargo run --example test_duplicate

use std::fs;
use std::path::{Path, PathBuf};

/// Same logic as `find_unique_duplicate_name` in file_explorer_demo.rs.
fn find_unique_duplicate_name(src: &Path) -> PathBuf {
    let parent = src.parent().unwrap_or_else(|| Path::new("."));
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let ext = src.extension().and_then(|s| s.to_str());
    let candidate = |suffix: &str| -> PathBuf {
        let name = match ext {
            Some(e) if !e.is_empty() => format!("{stem}{suffix}.{e}"),
            _ => format!("{stem}{suffix}"),
        };
        parent.join(name)
    };
    let first = candidate(" - duplicate");
    if !first.exists() {
        return first;
    }
    let mut n = 2u32;
    loop {
        let path = candidate(&format!(" - duplicate {n}"));
        if !path.exists() {
            return path;
        }
        n += 1;
        if n > 10_000 {
            return candidate(" - duplicate ts");
        }
    }
}

fn main() {
    let tmp = std::env::temp_dir().join("ru_wx_dup_test");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    // 1. Empty target dir → first duplicate is "foo - duplicate.txt"
    let input_file = tmp.join("foo.txt");
    fs::write(&input_file, "hello").unwrap();
    let d1 = find_unique_duplicate_name(&input_file);
    assert_eq!(
        d1.file_name().unwrap().to_str().unwrap(),
        "foo - duplicate.txt",
        "expected 'foo - duplicate.txt', got {:?}",
        d1.file_name()
    );
    println!(
        "  pass: foo.txt            → {}",
        d1.file_name().unwrap().to_str().unwrap()
    );
    fs::write(&d1, "hello copy 1").unwrap();

    // 2. foo.txt AND foo - duplicate.txt exist → "foo - duplicate 2.txt"
    let d2 = find_unique_duplicate_name(&input_file);
    assert_eq!(
        d2.file_name().unwrap().to_str().unwrap(),
        "foo - duplicate 2.txt",
        "expected 'foo - duplicate 2.txt', got {:?}",
        d2.file_name()
    );
    println!(
        "  pass: foo.txt (after #1) → {}",
        d2.file_name().unwrap().to_str().unwrap()
    );
    fs::write(&d2, "hello copy 2").unwrap();

    // 3. foo, duplicate, duplicate 2 exist → "foo - duplicate 3.txt"
    let d3 = find_unique_duplicate_name(&input_file);
    assert_eq!(
        d3.file_name().unwrap().to_str().unwrap(),
        "foo - duplicate 3.txt"
    );
    println!(
        "  pass: foo.txt (after #2) → {}",
        d3.file_name().unwrap().to_str().unwrap()
    );
    fs::write(&d3, "hello copy 3").unwrap();

    // 4. Skip ahead — remove d2 and re-query: should return "foo - duplicate 2.txt" again.
    fs::remove_file(&d2).unwrap();
    let d4 = find_unique_duplicate_name(&input_file);
    assert_eq!(
        d4.file_name().unwrap().to_str().unwrap(),
        "foo - duplicate 2.txt",
        "after removing #2, should reuse the gap → got {:?}",
        d4.file_name()
    );
    println!("  pass: after removing #2 → reuses slot");
    fs::write(&d4, "hello copy 2").unwrap();

    // 5. Folder test (no extension)
    let docs = tmp.join("docs");
    fs::create_dir_all(&docs).unwrap();
    let fd1 = find_unique_duplicate_name(&docs);
    assert_eq!(
        fd1.file_name().unwrap().to_str().unwrap(),
        "docs - duplicate"
    );
    println!(
        "  pass: docs/ folder        → {}",
        fd1.file_name().unwrap().to_str().unwrap()
    );
    fs::create_dir_all(&fd1).unwrap();

    let fd2 = find_unique_duplicate_name(&docs);
    assert_eq!(
        fd2.file_name().unwrap().to_str().unwrap(),
        "docs - duplicate 2"
    );
    println!(
        "  pass: docs/ (after #1)   → {}",
        fd2.file_name().unwrap().to_str().unwrap()
    );

    // 6. Cleanup
    fs::remove_dir_all(&tmp).unwrap();

    println!("\nAll duplicate-name tests passed.");
}
