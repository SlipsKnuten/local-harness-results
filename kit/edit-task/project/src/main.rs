//! tally: a tiny todo list for the terminal.
//!
//!   tally add <title> [--due YYYY-MM-DD] [--tag TAG]...
//!   tally list [--tag TAG] [--all]
//!   tally done <id>
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
    },
    /// Mark a task as done
    Done { id: u32 },
}

fn store_path() -> String {
    std::env::var("TALLY_FILE").unwrap_or_else(|_| "tally.json".to_string())
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
        Command::List { tag, all } => {
            for t in store.tasks.iter() {
                if !all && t.done {
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
            let task = store.tasks.iter_mut().find(|t| t.id == id).unwrap();
            task.done = true;
            store.save(&path).expect("could not write the task file");
            println!("done #{}", id);
        }
    }
}
