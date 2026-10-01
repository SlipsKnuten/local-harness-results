//! Tetris terminal UI built on ratatui + crossterm.

mod game;

use std::collections::{HashMap, HashSet};
use std::io;
use std::process::ExitCode;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Frame, Terminal};

use game::{Cell, Game, GameState, PieceType, RotationDir, COLS, HIDDEN_ROWS, ROWS, VISIBLE_ROWS};

const SIDEBAR_WIDTH: u16 = 26;
const CELL_WIDTH: u16 = 2;

const HELP: [&str; 8] = [
    "<-  ->   Move",
    "v      Soft drop",
    "^/X    Rotate CW",
    "Z      Rotate CCW",
    "Space  Hard drop",
    "P      Pause",
    "R      Restart",
    "Q      Quit",
];

fn cell_color(cell: Cell) -> Color {
    match cell {
        Cell::I => Color::Cyan,
        Cell::O => Color::Yellow,
        Cell::T => Color::Magenta,
        Cell::S => Color::Green,
        Cell::Z => Color::Red,
        Cell::J => Color::Blue,
        Cell::L => Color::Rgb(255, 165, 0),
        Cell::Empty => Color::DarkGray,
    }
}

/// Restores the terminal no matter how the game loop exits.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

fn seed() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(1, |t| t.as_nanos() as u64)
}

fn run() -> io::Result<()> {
    enable_raw_mode()?;
    let _guard = TerminalGuard;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    terminal.clear()?;

    let mut game = Game::new(seed());
    let mut quit = false;
    let mut next_gravity = Instant::now() + Duration::from_millis(game.tick_delay_ms());

    while !quit {
        terminal.draw(|frame| draw(frame, &game))?;

        let timeout = next_gravity.saturating_duration_since(Instant::now());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()?
                && matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
            {
                let before = (game.state(), game.level());
                let reset = handle_key(&mut game, &key, &mut quit);
                if reset || (game.state(), game.level()) != before {
                    next_gravity =
                        Instant::now() + Duration::from_millis(game.tick_delay_ms());
                }
            }
        } else {
            game.tick();
            next_gravity =
                Instant::now() + Duration::from_millis(game.tick_delay_ms());
        }
    }
    Ok(())
}

/// Handle one key event. Returns true when gravity timing should restart
/// (a piece just locked or a new game started).
fn handle_key(game: &mut Game, key: &KeyEvent, quit: &mut bool) -> bool {
    let press = key.kind == KeyEventKind::Press;
    let press_or_repeat = press || key.kind == KeyEventKind::Repeat;
    match key.code {
        KeyCode::Char('q') if press => {
            *quit = true;
            false
        }
        KeyCode::Char('p') if press => {
            game.toggle_pause();
            false
        }
        KeyCode::Char('r') if press => {
            game.restart(seed());
            true
        }
        KeyCode::Left if press_or_repeat => {
            game.move_h(-1);
            false
        }
        KeyCode::Right if press_or_repeat => {
            game.move_h(1);
            false
        }
        KeyCode::Down if press_or_repeat => {
            game.soft_drop();
            false
        }
        KeyCode::Up | KeyCode::Char('x') | KeyCode::Char('X') if press_or_repeat => {
            game.rotate(RotationDir::Clockwise);
            false
        }
        KeyCode::Char('z') | KeyCode::Char('Z') if press_or_repeat => {
            game.rotate(RotationDir::CounterClockwise);
            false
        }
        KeyCode::Char(' ') if press => {
            game.hard_drop();
            true
        }
        _ => false,
    }
}

fn draw(frame: &mut Frame, game: &Game) {
    let (board_width, board_height) = (
        CELL_WIDTH * COLS as u16 + 2,
        VISIBLE_ROWS as u16 + 2,
    );

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(board_height),
            Constraint::Min(1),
        ])
        .split(frame.area());
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(SIDEBAR_WIDTH),
            Constraint::Length(board_width),
            Constraint::Min(0),
        ])
        .split(vertical[1]);

    draw_sidebar(frame, game, horizontal[0]);
    draw_board(frame, game, horizontal[1]);
}

fn draw_sidebar(frame: &mut Frame, game: &Game, area: Rect) {
    let mut lines = vec![
        Line::from(Span::styled(
            "T E T R I S",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::default(),
    ];
    lines.push(stat_line("Score", &format!("{:06}", game.score())));
    lines.push(stat_line("Lines", &format!("{:03}", game.lines())));
    lines.push(stat_line("Level", &game.level().to_string()));
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        "Next",
        Style::default().fg(Color::DarkGray),
    )));
    lines.extend(next_piece_lines(game.next()));
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        "Controls",
        Style::default().fg(Color::DarkGray),
    )));
    for help in HELP {
        lines.push(Line::from(Span::styled(
            help,
            Style::default().fg(Color::DarkGray),
        )));
    }
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(Color::White)),
        area,
    );
}

fn stat_line(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(label.to_string(), Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled(value.to_string(), Style::default().add_modifier(Modifier::BOLD)),
    ])
}

fn next_piece_lines(ty: PieceType) -> Vec<Line<'static>> {
    let size = ty.size();
    let cells: HashSet<(usize, usize)> = ty.cells(0).into_iter().collect();
    let color = cell_color(ty.into());
    (0..size)
        .map(|row| {
            let spans: Vec<Span<'static>> = (0..size)
                .map(|col| {
                    if cells.contains(&(row, col)) {
                        Span::raw("██").style(Style::default().fg(color))
                    } else {
                        Span::raw("  ")
                    }
                })
                .collect();
            Line::from(spans)
        })
        .collect()
}

fn draw_board(frame: &mut Frame, game: &Game, area: Rect) {
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
        area,
    );
    let inner = area.inner(Margin { horizontal: 1, vertical: 1 });

    let active: HashMap<(usize, usize), Cell> = game
        .active_cells()
        .into_iter()
        .map(|(row, col, cell)| ((row, col), cell))
        .collect();
    let ghost: HashSet<(usize, usize)> = game.ghost_cells().into_iter().collect();

    let mut lines = Vec::with_capacity(VISIBLE_ROWS);
    for row in HIDDEN_ROWS..ROWS {
        let mut spans = Vec::with_capacity(COLS);
        for col in 0..COLS {
            let key = (row, col);
            if let Some(cell) = active.get(&key) {
                spans.push(
                    Span::raw("██").style(
                        Style::default()
                            .fg(cell_color(*cell))
                            .add_modifier(Modifier::BOLD),
                    ),
                );
            } else if game.board().at(row, col) != Cell::Empty {
                spans.push(
                    Span::raw("██").style(Style::default().fg(cell_color(
                        game.board().at(row, col),
                    ))),
                );
            } else if ghost.contains(&key) {
                spans.push(
                    Span::raw("░░").style(Style::default().fg(Color::DarkGray)),
                );
            } else {
                spans.push(Span::raw("  "));
            }
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines), inner);

    if game.state() != GameState::Playing {
        let lines: Vec<Line> = match game.state() {
            GameState::Playing => unreachable!("state checked above"),
            GameState::Paused => vec![
                Line::from(Span::styled(
                    "PAUSED",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::from("P to resume")),
            ],
            GameState::Over => vec![
                Line::from(Span::styled(
                    "GAME OVER",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::from("R to restart")),
            ],
        };
        let row = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(5),
                Constraint::Min(0),
            ])
            .split(inner)[1];
        let overlay = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(18),
                Constraint::Min(0),
            ])
            .split(row)[1];
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .style(Style::default().bg(Color::Black)),
            overlay,
        );
        frame.render_widget(
            Paragraph::new(lines)
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::White)),
            overlay.inner(Margin { horizontal: 1, vertical: 1 }),
        );
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("tetris: {err}");
            ExitCode::FAILURE
        }
    }
}
