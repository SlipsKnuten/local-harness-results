mod game;
mod ui;

use std::io::{self, Stdout};
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use game::Game;

const FRAME: Duration = Duration::from_millis(16);
const DAS: Duration = Duration::from_millis(160);
const ARR: Duration = Duration::from_millis(40);
const SOFT_DROP_INTERVAL: Duration = Duration::from_millis(35);
const LOCK_DELAY: Duration = Duration::from_millis(150);

struct Held {
    pressed_at: Instant,
    last_repeat_at: Instant,
}

fn main() -> io::Result<()> {
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, DisableMouseCapture)?;

    let result = run(&mut terminal);

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, EnableMouseCapture)?;
    terminal.clear()?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    let mut game = Game::new();
    let mut paused = false;
    let mut left: Option<Held> = None;
    let mut right: Option<Held> = None;
    let mut soft: Option<Held> = None;
    let mut grounded_since: Option<Instant> = None;
    let mut next_gravity = Instant::now() + game.gravity_interval();

    loop {
        let now = Instant::now();

        if event::poll(FRAME)? {
            loop {
                match event::read()? {
                    Event::Key(key) => match key.kind {
                        KeyEventKind::Press => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char('p') => paused = !paused,
                            KeyCode::Char('r') if game.game_over() => {
                                game.reset();
                                paused = false;
                                grounded_since = None;
                                next_gravity = now + game.gravity_interval();
                            }
                            KeyCode::Left | KeyCode::Char('a')
                                if !paused && !game.game_over() =>
                            {
                                game.move_h(-1);
                                left = Some(Held {
                                    pressed_at: now,
                                    last_repeat_at: now,
                                });
                                right = None;
                            }
                            KeyCode::Right | KeyCode::Char('d')
                                if !paused && !game.game_over() =>
                            {
                                game.move_h(1);
                                right = Some(Held {
                                    pressed_at: now,
                                    last_repeat_at: now,
                                });
                                left = None;
                            }
                            KeyCode::Down | KeyCode::Char('s')
                                if !paused && !game.game_over() =>
                            {
                                game.soft_drop();
                                soft = Some(Held {
                                    pressed_at: now,
                                    last_repeat_at: now,
                                });
                            }
                            KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('x')
                                if !paused && !game.game_over() =>
                            {
                                game.rotate_cw();
                            }
                            KeyCode::Char('z') if !paused && !game.game_over() => {
                                game.rotate_ccw();
                            }
                            KeyCode::Char(' ') if !paused && !game.game_over() => {
                                game.hard_drop();
                                grounded_since = None;
                            }
                            _ => {}
                        },
                        KeyEventKind::Release => match key.code {
                            KeyCode::Left | KeyCode::Char('a') => left = None,
                            KeyCode::Right | KeyCode::Char('d') => right = None,
                            KeyCode::Down | KeyCode::Char('s') => soft = None,
                            _ => {}
                        },
                        _ => {}
                    },
                    Event::FocusGained => terminal.clear()?,
                    _ => {}
                }
                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }

        if !paused && !game.game_over() {
            if let Some(held) = left.as_mut()
                && now.duration_since(held.pressed_at) >= DAS
                && now.duration_since(held.last_repeat_at) >= ARR
            {
                game.move_h(-1);
                held.last_repeat_at = now;
            }
            if let Some(held) = right.as_mut()
                && now.duration_since(held.pressed_at) >= DAS
                && now.duration_since(held.last_repeat_at) >= ARR
            {
                game.move_h(1);
                held.last_repeat_at = now;
            }
            if let Some(held) = soft.as_mut()
                && now.duration_since(held.last_repeat_at) >= SOFT_DROP_INTERVAL
            {
                game.soft_drop();
                held.last_repeat_at = now;
            }

            if now >= next_gravity {
                game.move_down();
                next_gravity = now + game.gravity_interval();
            }

            if game.move_down_possible() {
                grounded_since = None;
            } else if let Some(since) = grounded_since {
                if now.duration_since(since) >= LOCK_DELAY {
                    game.lock();
                    grounded_since = None;
                    next_gravity = Instant::now() + game.gravity_interval();
                }
            } else {
                grounded_since = Some(now);
            }
        }

        terminal.draw(|f| ui::render(f, &game, paused))?;
    }
}
