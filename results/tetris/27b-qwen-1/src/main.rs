//! A terminal Tetris built with ratatui + crossterm.

mod board;
mod game;
mod input;
mod tetromino;
mod ui;

use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event};

use game::{Game, GameState};
use input::Action;

/// Where the high score is persisted (best-effort, in the user's home directory).
fn high_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".tetris-rs-highscore")
}

fn load_high() -> u32 {
    std::fs::read_to_string(high_path())
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(0)
}

fn save_high(score: u32) {
    let _ = std::fs::write(high_path(), score.to_string());
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::try_init()?;
    let high = load_high();
    let mut game = Game::new(high);

    let result = run(&mut terminal, &mut game);

    save_high(game.high);
    ratatui::try_restore()?;
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal, game: &mut Game) -> io::Result<()> {
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::render(f, game))?;

        let playing = game.state == GameState::Playing;
        // Gravity interval while playing; a short poll otherwise so input stays responsive.
        let interval = if playing { game.gravity_delay() } else { 30 };
        let deadline = last_tick + Duration::from_millis(interval);

        // Block until the gravity deadline or until input is available.
        event::poll(deadline.saturating_duration_since(Instant::now()))?;

        // Drain any pending events without blocking.
        let mut locked = false;
        while let Ok(true) = event::poll(Duration::ZERO) {
            match event::read()? {
                Event::Key(key) => {
                    if let Some(action) = input::handle_event(key) {
                        match action {
                            Action::Quit => return Ok(()),
                            other => {
                                if game.apply(other) {
                                    locked = true;
                                }
                            }
                        }
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        // Apply one gravity step: either a fresh interval elapsed, or a new piece just spawned.
        if playing {
            if locked {
                last_tick = Instant::now();
            } else if Instant::now() >= deadline {
                game.tick();
                last_tick = Instant::now();
            }
        }
    }
}
