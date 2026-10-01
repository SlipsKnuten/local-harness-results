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
    /// Id handed out by the next `add`. Files written before this field existed
    /// have no value for it, so it is derived from the existing tasks instead.
    #[serde(default)]
    pub next_id: u32,
}

impl Store {
    pub fn load(path: &str) -> io::Result<Store> {
        if !Path::new(path).exists() {
            return Ok(Store::default());
        }
        let text = fs::read_to_string(path)?;
        let mut store: Store = serde_json::from_str(&text)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        store.fix_up_next_id();
        Ok(store)
    }

    pub fn save(&self, path: &str) -> io::Result<()> {
        let text = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(path, text)
    }

    pub fn find_mut(&mut self, id: u32) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    /// Adds a task and returns its id. Ids are never reused.
    pub fn add(&mut self, title: String, due: Option<String>, tags: Vec<String>) -> u32 {
        self.fix_up_next_id();
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

    /// Deletes the task with the given id. Returns false if there is no such task.
    pub fn remove(&mut self, id: u32) -> bool {
        let before = self.tasks.len();
        self.tasks.retain(|t| t.id != id);
        self.tasks.len() != before
    }

    /// Files written before the `next_id` field existed have it default to 0;
    /// derive the next free id from the tasks that are actually present.
    fn fix_up_next_id(&mut self) {
        if self.next_id == 0 {
            self.next_id = self
                .tasks
                .iter()
                .map(|t| t.id)
                .max()
                .map(|m| m + 1)
                .unwrap_or(1);
        }
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
        assert_eq!(s.add("a".into(), None, vec![]), 1);
        assert_eq!(s.add("b".into(), None, vec![]), 2);
        assert!(s.remove(1));
        assert_eq!(s.add("c".into(), None, vec![]), 3);
        assert!(s.remove(3));
        assert!(s.remove(2));
        assert!(!s.remove(1));
        assert_eq!(s.add("d".into(), None, vec![]), 4);
    }

    #[test]
    fn next_id_is_derived_for_old_files() {
        let dir = std::env::temp_dir().join(format!("tally-test-old-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("old.json");
        std::fs::write(
            &path,
            r#"{"tasks":[{"id":1,"title":"a","done":false,"due":null,"tags":[]}]}"#,
        )
        .unwrap();
        let mut s = Store::load(path.to_str().unwrap()).unwrap();
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
