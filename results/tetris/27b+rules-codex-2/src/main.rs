//! Tetris for the terminal, built with ratatui + crossterm.

mod game;

use std::io;
use std::time::Instant;

use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{run, DefaultTerminal, Frame};

use game::{Game, HEIGHT, PieceKind, RotateDir, WIDTH};

const BOARD_COLS: u16 = (WIDTH * 2) as u16 + 2;
const SIDEBAR_COLS: u16 = 26;
const BOARD_ROWS: u16 = HEIGHT as u16 + 2;

fn main() -> io::Result<()> {
    run(app)
}

fn app(terminal: &mut DefaultTerminal) -> io::Result<()> {
    terminal.hide_cursor()?;
    let mut game = Game::new();
    let mut paused = false;
    let mut quit = false;
    let mut next_tick = Instant::now() + game.gravity_interval();

    while !quit {
        terminal.draw(|frame| render(frame, &game, paused))?;

        if event::poll(next_tick.saturating_duration_since(Instant::now()))? {
            if let Event::Key(key) = event::read()? {
                handle_input(&mut game, key, &mut paused, &mut next_tick, &mut quit);
            }
        } else if !paused {
            game.tick();
            next_tick = Instant::now() + game.gravity_interval();
        }
    }

    terminal.show_cursor()?;
    Ok(())
}

fn handle_input(
    game: &mut Game,
    key: KeyEvent,
    paused: &mut bool,
    next_tick: &mut Instant,
    quit: &mut bool,
) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
            *quit = true;
        }
        KeyCode::Char('p') | KeyCode::Char('P') if !game.is_over() => {
            *paused = !*paused;
            *next_tick = Instant::now() + game.gravity_interval();
        }
        KeyCode::Char('r') | KeyCode::Char('R') if game.is_over() => {
            game.restart();
            *paused = false;
            *next_tick = Instant::now() + game.gravity_interval();
        }
        _ if !*paused && !game.is_over() => handle_gameplay_key(game, key, next_tick),
        _ => {}
    }
}

fn handle_gameplay_key(game: &mut Game, key: KeyEvent, next_tick: &mut Instant) {
    match key.code {
        KeyCode::Left => {
            game.move_left();
        }
        KeyCode::Right => {
            game.move_right();
        }
        KeyCode::Up | KeyCode::Char('x') | KeyCode::Char('X') => {
            game.rotate(RotateDir::Clockwise);
        }
        KeyCode::Char('z') | KeyCode::Char('Z') => {
            game.rotate(RotateDir::CounterClockwise);
        }
        KeyCode::Down => {
            if game.soft_drop() {
                *next_tick = Instant::now() + game.gravity_interval();
            }
        }
        KeyCode::Char(' ') => {
            game.hard_drop();
            *next_tick = Instant::now() + game.gravity_interval();
        }
        _ => {}
    }
}

fn render(frame: &mut Frame, game: &Game, paused: bool) {
    let area = frame.area();
    let game_width = BOARD_COLS + SIDEBAR_COLS;
    let game_area = if area.width >= game_width && area.height >= BOARD_ROWS {
        Rect {
            x: area.x + (area.width - game_width) / 2,
            y: area.y + (area.height - BOARD_ROWS) / 2,
            width: game_width,
            height: BOARD_ROWS,
        }
    } else {
        area
    };
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(BOARD_COLS), Constraint::Min(1)])
        .split(game_area);

    draw_board(frame, game, paused, columns[0]);
    draw_sidebar(frame, game, columns[1]);
}

fn cell_span(color: Color, ghost: bool) -> Span<'static> {
    let style = if ghost {
        Style::default().fg(color).add_modifier(Modifier::DIM)
    } else {
        Style::default().fg(color)
    };
    Span::styled("██", style)
}

fn draw_board(frame: &mut Frame, game: &Game, paused: bool, area: Rect) {
    let board = game.board();
    let active = game.piece().cells();
    let ghost = game.ghost_piece().cells();
    let piece_color = game.piece().kind.color();

    let mut lines = Vec::with_capacity(HEIGHT);
    for y in 0..HEIGHT {
        let mut spans = Vec::with_capacity(WIDTH);
        for x in 0..WIDTH {
            let pos = (x as i16, y as i16);
            let span = if let Some(kind) = board.get(pos.0, pos.1) {
                cell_span(kind.color(), false)
            } else if active.contains(&pos) {
                cell_span(piece_color, false)
            } else if ghost.contains(&pos) {
                cell_span(piece_color, true)
            } else {
                Span::raw("  ")
            };
            spans.push(span);
        }
        lines.push(Line::from(spans));
    }

    let title = if paused { "TETRIS (paused)" } else { "TETRIS" };
    let block = Block::default().borders(Borders::ALL).title(title);
    frame.render_widget(Paragraph::new(Text::from(lines)).block(block), area);

    if paused || game.is_over() {
        let message = if game.is_over() {
            "GAME OVER  (R restarts)"
        } else {
            "PAUSED  (P resumes)"
        };
        let overlay = Rect {
            x: area.x,
            y: area.y + BOARD_ROWS / 2 - 1,
            width: BOARD_COLS,
            height: 3,
        };
        let paragraph = Paragraph::new(Line::from(Span::styled(
            format!(" {message} "),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center)
        .block(Block::default().style(Style::default().bg(Color::Black)));
        frame.render_widget(paragraph, overlay);
    }
}

fn draw_sidebar(frame: &mut Frame, game: &Game, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(5),
            Constraint::Min(1),
        ])
        .split(area);

    let preview = Paragraph::new(preview_lines(game.preview_kind()))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Next"));
    frame.render_widget(preview, rows[0]);

    let stats = Paragraph::new(Text::from(vec![
        Line::from(format!("Score: {}", game.score())),
        Line::from(format!("Level: {}", game.level())),
        Line::from(format!("Lines: {}", game.lines())),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Stats"));
    frame.render_widget(stats, rows[1]);

    let help = Paragraph::new(Text::from(vec![
        Line::from("Left/Right   move"),
        Line::from("Up / X       rotate"),
        Line::from("Z            rotate back"),
        Line::from("Down         soft drop"),
        Line::from("Space        hard drop"),
        Line::from("P            pause"),
        Line::from("Q            quit"),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Controls"));
    frame.render_widget(help, rows[2]);
}

fn preview_lines(kind: PieceKind) -> Vec<Line<'static>> {
    let state = kind.spawn_state();
    let color = kind.color();
    (0..4)
        .map(|row| {
            let mut spans = Vec::with_capacity(4);
            for col in 0..4 {
                if state.contains(&(row, col)) {
                    spans.push(cell_span(color, false));
                } else {
                    spans.push(Span::raw("  "));
                }
            }
            Line::from(spans)
        })
        .collect()
}
