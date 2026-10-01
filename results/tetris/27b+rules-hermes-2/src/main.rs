//! Rust Tetris built on ratatui + crossterm.
//!
//! Controls:
//!   ← / →     move
//!   ↑ / x     rotate
//!   ↓         soft drop
//!   space     hard drop
//!   p         pause
//!   q / esc   quit

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

mod board;
mod game;
mod rng;
mod tetromino;
mod ui;

use game::{Game, Status};

fn main() -> io::Result<()> {
    let seed = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or_else(|| crate::rng::Rng::seed().value());

    let mut game = Game::new(seed);
    let mut paused = false;
    let mut last_drop = Instant::now();

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, &mut game, &mut paused, &mut last_drop);

    // Always restore the terminal, even on the error path.
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    result
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    game: &mut Game,
    paused: &mut bool,
    last_drop: &mut Instant,
) -> io::Result<()> {
    const TICK: Duration = Duration::from_millis(50);

    loop {
        terminal.draw(|f| ui::draw(f, game))?;

        // Block at most one tick for input; when idle, the elapsed check below
        // drives gravity. This keeps the loop responsive and the timing simple.
        if !event::poll(TICK)? {
            if !*paused
                && game.status == Status::Playing
                && last_drop.elapsed() >= Duration::from_millis(game.drop_interval_ms())
            {
                game.step_down();
                *last_drop = Instant::now();
            }
            continue;
        }

        let ev = event::read()?;
        let Event::Key(key) = ev else { continue };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        let quit = key.code == KeyCode::Char('q')
            || key.code == KeyCode::Char('Q')
            || (key.code == KeyCode::Esc && key.modifiers.is_empty());

        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                if quit {
                    return Ok(());
                }
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                if game.status == Status::Playing {
                    *paused = !*paused;
                    *last_drop = Instant::now();
                }
            }
            KeyCode::Left => game.move_horizontal(-1),
            KeyCode::Right => game.move_horizontal(1),
            KeyCode::Up | KeyCode::Char('x') | KeyCode::Char('X') => game.rotate(),
            KeyCode::Down => {
                game.soft_drop();
                *last_drop = Instant::now();
            }
            KeyCode::Char(' ') if key.modifiers == KeyModifiers::NONE => {
                game.hard_drop();
                *last_drop = Instant::now();
            }
            _ => {}
        }
    }
}
