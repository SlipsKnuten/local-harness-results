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

#[derive(Debug, Serialize, Deserialize)]
pub struct Store {
    pub tasks: Vec<Task>,
    /// The id that will be assigned to the next task. Never decreases, so ids are never reused.
    /// Old files (without this field) load with a value derived from the highest existing id.
    #[serde(default)]
    pub next_id: u32,
}

impl Default for Store {
    fn default() -> Self {
        Store {
            tasks: Vec::new(),
            next_id: 1,
        }
    }
}

impl Store {
    pub fn load(path: &str) -> io::Result<Store> {
        if !Path::new(path).exists() {
            return Ok(Store::default());
        }
        let text = fs::read_to_string(path)?;
        let mut store: Store = serde_json::from_str(&text)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let max_id = store.tasks.iter().map(|t| t.id).max().unwrap_or(0);
        store.next_id = store.next_id.max(max_id + 1);
        Ok(store)
    }

    pub fn save(&self, path: &str) -> io::Result<()> {
        let text = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(path, text)
    }

    /// Adds a task and returns its id. Ids are never reused.
    pub fn add(&mut self, title: String, due: Option<String>, tags: Vec<String>) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.tasks.push(Task {
            id,
            title,
            done: false,
            due,
            tags,
        });
        id
    }

    /// Removes the task with the given id. Returns `true` if a task was removed.
    pub fn remove(&mut self, id: u32) -> bool {
        let before = self.tasks.len();
        self.tasks.retain(|t| t.id != id);
        self.tasks.len() != before
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
    fn ids_not_reused_after_remove() {
        let mut s = Store::default();
        assert_eq!(s.add("a".into(), None, vec![]), 1);
        assert_eq!(s.add("b".into(), None, vec![]), 2);
        assert!(s.remove(2));
        assert!(!s.remove(2));
        assert_eq!(s.add("c".into(), None, vec![]), 3);
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

    #[test]
    fn legacy_file_without_next_id_continues_ids() {
        let dir = std::env::temp_dir().join(format!("tally-test-legacy-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("legacy.json");
        let path = path.to_str().unwrap();
        // A file as written by the previous version: no `next_id` field.
        std::fs::write(
            path,
            r#"{"tasks":[{"id":1,"title":"a","done":false,"due":null,"tags":[]},{"id":2,"title":"b","done":true,"due":null,"tags":[]}]}"#,
        )
        .unwrap();
        let mut s = Store::load(path).unwrap();
        // next_id is derived from the highest existing id (2) + 1 = 3.
        assert_eq!(s.add("c".into(), None, vec![]), 3);
        // Deleting the just-added max id must not let it be reused.
        assert!(s.remove(3));
        assert_eq!(s.add("d".into(), None, vec![]), 4);
    }
}
