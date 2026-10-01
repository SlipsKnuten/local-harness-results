use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    pub id: u32,
    pub title: String,
    pub done: bool,
    pub due: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Store {
    pub tasks: Vec<Task>,
}

impl Store {
    pub fn load(path: &str) -> io::Result<Store> {
        if !Path::new(path).exists() {
            return Ok(Store::default());
        }
        let text = fs::read_to_string(path)?;
        serde_json::from_str(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn save(&self, path: &str) -> io::Result<()> {
        let text = serde_json::to_string_pretty(self).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        fs::write(path, text)
    }

    /// Adds a task and returns its id. Ids are never reused.
    pub fn add(&mut self, title: String, due: Option<String>, tags: Vec<String>) -> u32 {
        let id = self.tasks.len() as u32 + 1;
        self.tasks.push(Task { id, title, done: false, due, tags });
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_assigns_increasing_ids() {
        let mut s = Store::default();
        assert_eq!(s.add("a".into(), None, vec![]), 1);
        assert_eq!(s.add("b".into(), None, vec![]), 2);
    }

    #[test]
    fn roundtrip() {
        let dir = std::env::temp_dir().join(format!("tally-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.json");
        let path = path.to_str().unwrap();
        let mut s = Store::default();
        s.add("a".into(), Some("2026-01-01".into()), vec!["x".into()]);
        s.save(path).unwrap();
        let back = Store::load(path).unwrap();
        assert_eq!(back.tasks, s.tasks);
    }
}
