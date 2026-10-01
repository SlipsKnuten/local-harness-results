//! tally: a tiny todo list for the terminal.
//!
//!   tally add <title> [--due YYYY-MM-DD] [--tag TAG]...
//!   tally list [--tag TAG] [--all] [--overdue]
//!   tally done <id>
//!   tally edit <id> [--title TEXT] [--due YYYY-MM-DD]
//!   tally rm <id>
//!   tally stats
//!
//! Tasks are stored as JSON in $TALLY_FILE (default: ./tally.json).
mod store;

use clap::{Parser, Subcommand};
use store::{Store, Task};

#[derive(Parser)]
#[command(name = "tally", about = "A tiny todo list")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Add a task
    Add {
        title: String,
        #[arg(long)]
        due: Option<String>,
        #[arg(long = "tag")]
        tags: Vec<String>,
    },
    /// List open tasks (or all with --all)
    List {
        #[arg(long)]
        tag: Option<String>,
        #[arg(long)]
        all: bool,
        /// Only open tasks whose due date is before today
        #[arg(long)]
        overdue: bool,
    },
    /// Mark a task as done
    Done { id: u32 },
    /// Edit a task's title and/or due date
    Edit {
        id: u32,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        due: Option<String>,
    },
    /// Delete a task
    Rm { id: u32 },
    /// Print open/done/overdue counts
    Stats,
}

fn store_path() -> String {
    std::env::var("TALLY_FILE").unwrap_or_else(|_| "tally.json".to_string())
}

/// Today as YYYY-MM-DD: `$TALLY_TODAY` if set, else the current date.
fn today() -> String {
    if let Ok(s) = std::env::var("TALLY_TODAY") {
        if !s.is_empty() {
            return s;
        }
    }
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86400);
    let (y, m, d) = civil_from_days(days);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

/// Converts days since 1970-01-01 to a (year, month, day) triple (proleptic Gregorian).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// An open task is overdue when its due date is before `today`.
fn is_overdue(t: &Task, today: &str) -> bool {
    !t.done && matches!(&t.due, Some(d) if d.as_str() < today)
}

fn no_task(id: u32) -> ! {
    eprintln!("no task #{}", id);
    std::process::exit(1);
}

fn format_task(t: &Task) -> String {
    let mark = if t.done { "x" } else { " " };
    let mut line = format!("#{} [{}] {}", t.id, mark, t.title);
    if let Some(due) = &t.due {
        line.push_str(&format!(" (due {})", due));
    }
    if !t.tags.is_empty() {
        line.push_str(&format!(
            " {}",
            t.tags.iter().map(|t| format!("+{}", t)).collect::<Vec<_>>().join(" ")
        ));
    }
    line
}

fn main() {
    let cli = Cli::parse();
    let path = store_path();
    let mut store = Store::load(&path).expect("could not read the task file");

    match cli.command {
        Command::Add { title, due, tags } => {
            let id = store.add(title, due, tags);
            store.save(&path).expect("could not write the task file");
            println!("added #{}", id);
        }
        Command::List { tag, all, overdue } => {
            let today = today();
            for t in store.tasks.iter() {
                if overdue {
                    if !is_overdue(t, &today) {
                        continue;
                    }
                } else if !all && t.done {
                    continue;
                }
                if let Some(tag) = &tag {
                    if !t.tags.contains(tag) {
                        continue;
                    }
                }
                println!("{}", format_task(t));
            }
        }
        Command::Done { id } => {
            let task = match store.tasks.iter_mut().find(|t| t.id == id) {
                Some(t) => t,
                None => no_task(id),
            };
            task.done = true;
            store.save(&path).expect("could not write the task file");
            println!("done #{}", id);
        }
        Command::Edit { id, title, due } => {
            let task = match store.tasks.iter_mut().find(|t| t.id == id) {
                Some(t) => t,
                None => no_task(id),
            };
            if let Some(title) = title {
                task.title = title;
            }
            if let Some(due) = due {
                task.due = Some(due);
            }
            store.save(&path).expect("could not write the task file");
            println!("edited #{}", id);
        }
        Command::Rm { id } => {
            if !store.tasks.iter().any(|t| t.id == id) {
                no_task(id);
            }
            store.tasks.retain(|t| t.id != id);
            store.save(&path).expect("could not write the task file");
            println!("removed #{}", id);
        }
        Command::Stats => {
            let today = today();
            let open = store.tasks.iter().filter(|t| !t.done).count();
            let done = store.tasks.iter().filter(|t| t.done).count();
            let overdue = store.tasks.iter().filter(|t| is_overdue(t, &today)).count();
            println!("open: {}, done: {}, overdue: {}", open, done, overdue);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_maps_to_1970() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
    }

    #[test]
    fn known_date() {
        // 2026-01-01 is 20454 days after the epoch.
        assert_eq!(civil_from_days(20454), (2026, 1, 1));
    }

    #[test]
    fn overdue_only_open_past_due() {
        let mut s = Store::default();
        let a = s.add("open past".into(), Some("2020-01-01".into()), vec![]);
        let b = s.add("open future".into(), Some("2999-01-01".into()), vec![]);
        let c = s.add("open none".into(), None, vec![]);
        let d = s.add("done past".into(), Some("2020-01-01".into()), vec![]);
        s.tasks.iter_mut().find(|t| t.id == d).unwrap().done = true;
        let today = "2026-01-01";
        assert!(is_overdue(&s.tasks[a as usize - 1], today));
        assert!(!is_overdue(&s.tasks[b as usize - 1], today));
        assert!(!is_overdue(&s.tasks[c as usize - 1], today));
        assert!(!is_overdue(&s.tasks[d as usize - 1], today));
    }

    #[test]
    fn ids_never_reused_after_delete() {
        let mut s = Store::default();
        s.add("a".into(), None, vec![]);
        s.add("b".into(), None, vec![]);
        s.add("c".into(), None, vec![]);
        s.tasks.retain(|t| t.id != 2);
        assert_eq!(s.add("d".into(), None, vec![]), 4);
    }
}
