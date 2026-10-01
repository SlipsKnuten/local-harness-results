mod game;
mod tetrominoes;
mod ui;

use std::io::stdout;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use game::Game;

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Terminal setup.
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut game = Game::new();

    let res = game_loop(&mut terminal, &mut game);

    // Always restore the terminal, even on error.
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn game_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    game: &mut Game,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        // How long to wait before the next gravity step.
        let playing = !game.over && !game.paused;
        let timeout = if playing {
            game.drop_interval()
        } else {
            Duration::from_millis(100)
        };

        // Block until a key event or the timeout elapses.
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                // On some terminals each keypress generates press/release/repeat;
                // only act on press.
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        KeyCode::Left => {
                            game.try_move(-1, 0);
                        }
                        KeyCode::Right => {
                            game.try_move(1, 0);
                        }
                        KeyCode::Down => {
                            game.soft_drop();
                        }
                        KeyCode::Up => {
                            game.rotate(true);
                        }
                        KeyCode::Char('z') | KeyCode::Char('Z') => {
                            game.rotate(false);
                        }
                        KeyCode::Char(' ') => {
                            game.hard_drop();
                        }
                        KeyCode::Char('p') | KeyCode::Char('P') => {
                            game.toggle_pause();
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => {
                            game.reset();
                        }
                        _ => {}
                    }
                }
            }
        } else if playing {
            game.tick();
        }

        terminal.draw(|f| ui::draw(f, game))?;
    }
}

fn main() {
    // Headless self-test: script some moves through the same public methods the
    // key handler uses, then print the resulting state. Useful for CI/verification.
    if std::env::args().any(|a| a == "--selftest") {
        selftest();
        return;
    }
    if let Err(err) = run() {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}

/// Run a scripted sequence of inputs and report the final state.
fn selftest() {
    use std::io::Write as _;
    let mut game = Game::new();

    // Fill the bottom row so that hard drops can clear lines deterministically.
    for c in 0..game::COLS {
        game.board[game::ROWS - 1][c] = game::Cell::filled('x');
    }

    // Play a handful of pieces with a mix of moves, rotations, and hard drops.
    for i in 0..12 {
        for _ in 0..(i % 4) {
            game.try_move(if i % 2 == 0 { -1 } else { 1 }, 0);
        }
        if i % 3 == 0 {
            game.rotate(true);
        }
        game.soft_drop();
        game.hard_drop();
    }

    let filled = game
        .board
        .iter()
        .flat_map(|row| row.iter())
        .filter(|c| !c.is_empty())
        .count();

    let out = format!(
        "selftest OK score={} lines={} level={} filled_cells={} over={} next={:?}\n",
        game.score, game.lines, game.level, filled, game.over, game.next
    );
    // Print to stdout, flushing to be safe.
    print!("{}", out);
    let _ = std::io::stdout().flush();
}
