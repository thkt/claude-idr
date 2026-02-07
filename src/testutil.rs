#![cfg(test)]

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn write_jsonl(dir: &Path, name: &str, lines: &[&str]) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let mut file = File::create(&path).unwrap();
    for line in lines {
        writeln!(file, "{line}").unwrap();
    }
    path
}
