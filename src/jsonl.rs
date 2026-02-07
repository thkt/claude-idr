use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn iter_values(path: &Path) -> impl Iterator<Item = Value> {
    let file = File::open(path).ok();
    let lines: Box<dyn Iterator<Item = String>> = match file {
        Some(f) => Box::new(BufReader::new(f).lines().map_while(Result::ok)),
        None => Box::new(std::iter::empty()),
    };
    lines
        .filter(|l| !l.is_empty())
        .filter_map(|l| serde_json::from_str(&l).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn iter_values_parses_valid_jsonl() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.jsonl");
        let mut f = File::create(&path).unwrap();
        writeln!(f, r#"{{"key":"val1"}}"#).unwrap();
        writeln!(f, r#"{{"key":"val2"}}"#).unwrap();

        let values: Vec<Value> = iter_values(&path).collect();
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn iter_values_skips_invalid_lines() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.jsonl");
        let mut f = File::create(&path).unwrap();
        writeln!(f, "not json").unwrap();
        writeln!(f, r#"{{"key":"val"}}"#).unwrap();
        writeln!(f, "{{broken").unwrap();

        let values: Vec<Value> = iter_values(&path).collect();
        assert_eq!(values.len(), 1);
    }

    #[test]
    fn iter_values_returns_empty_for_nonexistent() {
        let values: Vec<Value> = iter_values(Path::new("/nonexistent")).collect();
        assert!(values.is_empty());
    }

    #[test]
    fn iter_values_skips_empty_lines() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.jsonl");
        let mut f = File::create(&path).unwrap();
        writeln!(f, r#"{{"a":1}}"#).unwrap();
        writeln!(f).unwrap();
        writeln!(f, r#"{{"b":2}}"#).unwrap();

        let values: Vec<Value> = iter_values(&path).collect();
        assert_eq!(values.len(), 2);
    }
}
