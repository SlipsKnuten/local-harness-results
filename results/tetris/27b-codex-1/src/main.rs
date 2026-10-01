//! Entry point: terminal setup, the game loop, timing, and key handling.

mod game;
mod piece;
mod render;

use crossterm::cursor;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::execute;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;
use std::time::{Duration, Instant};

use game::Game;

/// Target frame interval (~60 fps). We poll for input this often, so the
/// playfield stays smooth even when no key is pressed.
const FRAME_MS: u64 = 16;

fn main() {
    if let Err(err) = run() {
        // Best-effort restore so the user's terminal is left usable.
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), cursor::Show, LeaveAlternateScreen);
        eprintln!("Tetris error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    execute!(terminal.backend_mut(), cursor::Hide)?;

    let mut game = Game::new();
    let mut gravity_accumulator: u128 = 0;
    let mut last = Instant::now();

    'game: loop {
        // Drain any pending key events.
        while event::poll(Duration::from_millis(FRAME_MS))? {
            if let Event::Key(key) = event::read()? {
                match handle_key(&mut game, key) {
                    KeyAction::Quit => break 'game,
                    KeyAction::None => {}
                }
            }
        }

        // Advance gravity based on elapsed wall time.
        let now = Instant::now();
        let delta = now - last;
        last = now;
        if !game.over && !game.paused {
            gravity_accumulator += delta.as_millis();
            let interval = game.gravity_interval_ms();
            while gravity_accumulator >= interval {
                gravity_accumulator -= interval;
                game.step();
            }
        } else {
            gravity_accumulator = 0;
        }

        render::draw(&mut terminal, &game)?;
    }

    // Clean shutdown: restore the terminal.
    execute!(terminal.backend_mut(), cursor::Show, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

enum KeyAction {
    None,
    Quit,
}

fn handle_key(game: &mut Game, key: KeyEvent) -> KeyAction {
    // Global keys that work in any state.
    if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') {
        return KeyAction::Quit;
    }

    if game.over {
        if key.code == KeyCode::Char('r') || key.code == KeyCode::Char('R') {
            *game = Game::new();
        }
        return KeyAction::None;
    }

    if key.code == KeyCode::Char('p') || key.code == KeyCode::Char('P') {
        game.paused = !game.paused;
        return KeyAction::None;
    }

    if game.paused {
        return KeyAction::None;
    }

    match key.code {
        KeyCode::Left => game.move_h(-1),
        KeyCode::Right => game.move_h(1),
        KeyCode::Down => game.soft_drop(),
        KeyCode::Up | KeyCode::Char('x') | KeyCode::Char('X') => game.rotate(true),
        KeyCode::Char('z') | KeyCode::Char('Z') | KeyCode::Backspace => game.rotate(false),
        KeyCode::Char(' ') => game.hard_drop(),
        KeyCode::Char('c') | KeyCode::Char('C') => game.hold(),
        KeyCode::Char('r') | KeyCode::Char('R') => *game = Game::new(),
        _ => {}
    }
    KeyAction::None
}
