//! Terminal Tetris built with ratatui + crossterm.

mod game;
mod render;

use std::io::{self, Stdout};
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

fn main() -> io::Result<()> {
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    install_panic_hook();
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    terminal.hide_cursor()?;

    let result = run_game(&mut terminal);

    terminal.show_cursor()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;

    result
}

/// Restore the terminal even when the game panics.
fn install_panic_hook() {
    let original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original(info);
    }));
}

fn run_game(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    let mut game = game::Game::new(rand::random());
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| render::draw(f, &game))?;

        let timeout = if !game.is_paused() && !game.is_over() {
            game.gravity_interval().saturating_sub(last_tick.elapsed())
        } else {
            Duration::from_millis(50)
        };

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? && key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('r') => {
                        game = game::Game::new(rand::random());
                    }
                    KeyCode::Char('p') => game.toggle_pause(),
                    KeyCode::Left => game.move_left(),
                    KeyCode::Right => game.move_right(),
                    KeyCode::Down => game.soft_drop(),
                    KeyCode::Up | KeyCode::Char('x') => game.rotate_cw(),
                    KeyCode::Char('z') => game.rotate_ccw(),
                    KeyCode::Char(' ') => game.hard_drop(),
                    _ => {}
                }
            }
        } else {
            game.step();
        }
        last_tick = Instant::now();
    }
}
