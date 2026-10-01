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
use chrono::NaiveDate;
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
    /// Change a task's title and/or due date
    Edit {
        id: u32,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        due: Option<String>,
    },
    /// Delete a task
    Rm { id: u32 },
    /// Print counts of open, done and overdue tasks
    Stats,
}

fn store_path() -> String {
    std::env::var("TALLY_FILE").unwrap_or_else(|_| "tally.json".to_string())
}

/// Today's date: $TALLY_TODAY (YYYY-MM-DD) if set, otherwise the current date.
fn today() -> NaiveDate {
    if let Ok(t) = std::env::var("TALLY_TODAY") {
        if let Ok(d) = NaiveDate::parse_from_str(&t, "%Y-%m-%d") {
            return d;
        }
    }
    chrono::Local::now().date_naive()
}

/// True if the task is open and its due date is before today.
fn is_overdue(t: &Task, today: NaiveDate) -> bool {
    if t.done {
        return false;
    }
    match t.due.as_deref().and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()) {
        Some(due) => due < today,
        None => false,
    }
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
        line.push_str(&format!(" {}", t.tags.iter().map(|t| format!("+{}", t)).collect::<Vec<_>>().join(" ")));
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
                    if !is_overdue(t, today) {
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
            let Some(task) = store.find_mut(id) else {
                no_task(id);
            };
            task.done = true;
            store.save(&path).expect("could not write the task file");
            println!("done #{}", id);
        }
        Command::Edit { id, title, due } => {
            let Some(task) = store.find_mut(id) else {
                no_task(id);
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
            if !store.remove(id) {
                no_task(id);
            }
            store.save(&path).expect("could not write the task file");
            println!("removed #{}", id);
        }
        Command::Stats => {
            let today = today();
            let open = store.tasks.iter().filter(|t| !t.done).count();
            let done = store.tasks.iter().filter(|t| t.done).count();
            let overdue = store.tasks.iter().filter(|t| is_overdue(t, today)).count();
            println!("open: {}, done: {}, overdue: {}", open, done, overdue);
        }
    }
}
