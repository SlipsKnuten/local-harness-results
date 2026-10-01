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
    /// The id to assign to the next task. Monotonic: never decreases and
    /// never hands out an id that was already used. Defaults to 1 so files
    /// written before this field existed still load.
    #[serde(default = "Store::default_next_id")]
    pub next_id: u32,
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

impl Store {
    fn default_next_id() -> u32 {
        1
    }

    pub fn new() -> Self {
        Store { tasks: Vec::new(), next_id: 1 }
    }

    pub fn load(path: &str) -> io::Result<Store> {
        if !Path::new(path).exists() {
            return Ok(Store::new());
        }
        let text = fs::read_to_string(path)?;
        let mut store: Store =
            serde_json::from_str(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        // Files written before `next_id` existed default it to 1, which may
        // collide with existing ids. Bump it past any id already in use.
        let max_used = store.tasks.iter().map(|t| t.id).max().unwrap_or(0);
        if store.next_id <= max_used {
            store.next_id = max_used + 1;
        }
        Ok(store)
    }

    pub fn save(&self, path: &str) -> io::Result<()> {
        let text = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(path, text)
    }

    /// Adds a task and returns its id. Ids are never reused,
    /// even after tasks are removed.
    pub fn add(&mut self, title: String, due: Option<String>, tags: Vec<String>) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.tasks.push(Task { id, title, done: false, due, tags });
        id
    }

    /// Returns the task with the given id, if any.
    pub fn find_mut(&mut self, id: u32) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    /// Removes the task with the given id. Returns true if a task was removed.
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
    fn ids_are_not_reused_after_remove() {
        let mut s = Store::default();
        s.add("a".into(), None, vec![]);
        s.add("b".into(), None, vec![]);
        assert!(s.remove(2));
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
        assert_eq!(back.next_id, s.next_id);
    }

    #[test]
    fn loads_files_written_before_next_id_existed() {
        let dir = std::env::temp_dir().join(format!("tally-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("old.json");
        let path = path.to_str().unwrap();
        let legacy = r#"{"tasks":[{"id":1,"title":"a","done":false,"due":null,"tags":[]},{"id":3,"title":"c","done":true,"due":"2026-01-01","tags":["x"]}]}"#;
        std::fs::write(path, legacy).unwrap();
        let mut s = Store::load(path).unwrap();
        assert_eq!(s.tasks.len(), 2);
        assert_eq!(s.add("d".into(), None, vec![]), 4);
        assert_eq!(s.add("e".into(), None, vec![]), 5);
    }

    #[test]
    fn no_reuse_after_removal_roundtrip() {
        let dir = std::env::temp_dir().join(format!("tally-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("r.json");
        let path = path.to_str().unwrap();
        let mut s = Store::default();
        s.add("a".into(), None, vec![]);
        s.add("b".into(), None, vec![]);
        s.remove(2);
        s.save(path).unwrap();
        let mut back = Store::load(path).unwrap();
        assert_eq!(back.add("c".into(), None, vec![]), 3);
    }
}
