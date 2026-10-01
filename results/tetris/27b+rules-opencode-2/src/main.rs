//! Tetris terminal UI built on ratatui + crossterm.

mod game;

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Flex, Layout};
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::{Frame, DefaultTerminal};

use game::{Game, GameState, HEIGHT, PieceKind, WIDTH};

const CELL_CHARS: u16 = 2;
const BOARD_W: u16 = WIDTH * CELL_CHARS + 2;
const BOARD_H: u16 = HEIGHT + 2;
const NEXT_W: u16 = 10;
const INFO_W: u16 = 24;
const MIN_WIDTH: u16 = NEXT_W + 2 + BOARD_W + 2 + INFO_W;
const MIN_HEIGHT: u16 = BOARD_H + 2;

fn color_of(kind: PieceKind) -> Color {
    match kind {
        PieceKind::I => Color::Cyan,
        PieceKind::O => Color::Yellow,
        PieceKind::T => Color::Magenta,
        PieceKind::S => Color::Green,
        PieceKind::Z => Color::Red,
        PieceKind::J => Color::Blue,
        PieceKind::L => Color::LightYellow,
    }
}

fn set_cell(buf: &mut Buffer, x: u16, y: u16, ch: char, color: Color) {
    let style = Style::default().fg(color);
    buf[(x, y)].set_char(ch).set_style(style);
    buf[(x + 1, y)].set_char(ch).set_style(style);
}

fn draw_board(frame: &mut Frame, area: Rect, game: &Game) {
    let block = Block::default().borders(Borders::ALL).title("TETRIS");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let buf = frame.buffer_mut();

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if let Some(kind) = game.board.get(x, y) {
                set_cell(
                    buf,
                    inner.x + x * CELL_CHARS,
                    inner.y + y,
                    '\u{2588}',
                    color_of(kind),
                );
            }
        }
    }

    if game.state != GameState::GameOver {
        let ghost = game.ghost();
        for (cx, cy) in ghost.cells() {
            if (0..WIDTH as i16).contains(&cx) && (0..HEIGHT as i16).contains(&cy) {
                set_cell(
                    buf,
                    inner.x + (cx as u16) * CELL_CHARS,
                    inner.y + cy as u16,
                    '\u{2591}',
                    Color::DarkGray,
                );
            }
        }
        for (cx, cy) in game.current.cells() {
            if (0..WIDTH as i16).contains(&cx) && (0..HEIGHT as i16).contains(&cy) {
                set_cell(
                    buf,
                    inner.x + (cx as u16) * CELL_CHARS,
                    inner.y + cy as u16,
                    '\u{2588}',
                    color_of(game.current.kind),
                );
            }
        }
    }

    match game.state {
        GameState::Paused => overlay(frame, area, &["Paused", "p resume - q quit"]),
        GameState::GameOver => overlay(frame, area, &["Game Over", "r restart - q quit"]),
        GameState::Playing => {}
    }
}

fn draw_next(frame: &mut Frame, area: Rect, kind: PieceKind) {
    let block = Block::default().borders(Borders::ALL).title("Next");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let buf = frame.buffer_mut();
    let matrix = kind.base();
    let box_w = 4 * CELL_CHARS;
    let px = inner.x + inner.width.saturating_sub(box_w) / 2;
    let py = inner.y + inner.height.saturating_sub(4) / 2;
    for (row, row_cells) in matrix.iter().enumerate() {
        for (col, &occupied) in row_cells.iter().enumerate() {
            if occupied == 1 {
                set_cell(
                    buf,
                    px + (col as u16) * CELL_CHARS,
                    py + row as u16,
                    '\u{2588}',
                    color_of(kind),
                );
            }
        }
    }
}

fn draw_info(frame: &mut Frame, area: Rect, game: &Game) {
    let block = Block::default().borders(Borders::ALL).title("Info");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let text = Text::from(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Score", bold),
            Span::raw(format!(": {:>6}", game.score)),
        ]),
        Line::from(vec![
            Span::styled("Level", bold),
            Span::raw(format!(": {:>6}", game.level())),
        ]),
        Line::from(vec![
            Span::styled("Lines", bold),
            Span::raw(format!(": {:>6}", game.lines)),
        ]),
        Line::from(""),
        Line::from("Controls:"),
        Line::from("  <-  ->   move"),
        Line::from("  v        soft drop"),
        Line::from("  ^ / z    rotate"),
        Line::from("  x        rotate ccw"),
        Line::from("  space    hard drop"),
        Line::from("  p        pause"),
        Line::from("  q / esc  quit"),
    ]);
    frame.render_widget(Paragraph::new(text), inner);
}

fn overlay(frame: &mut Frame, area: Rect, lines: &[&str]) {
    let text = Text::from(
        lines
            .iter()
            .map(|line| {
                Line::from(Span::styled(
                    *line,
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ))
            })
            .collect::<Vec<Line>>(),
    );
    let width = text
        .iter()
        .map(|line| line.width() as u16)
        .max()
        .unwrap_or(0);
    let height = text.iter().count() as u16;
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    let rect = Rect::new(x, y, width, height);
    frame.render_widget(Clear, rect);
    frame.render_widget(
        Paragraph::new(text).style(Style::default().bg(Color::Gray)),
        rect,
    );
}

fn draw(frame: &mut Frame, game: &Game) {
    let area = frame.area();
    frame.render_widget(Clear, area);
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        let msg = Paragraph::new(Text::from(format!(
            "Terminal too small: need at least {MIN_WIDTH}x{MIN_HEIGHT}"
        )))
        .style(Style::default().fg(Color::Yellow));
        frame.render_widget(msg, area);
        return;
    }
    let row = Layout::default()
        .direction(Direction::Vertical)
        .flex(Flex::Center)
        .constraints([Constraint::Length(BOARD_H)])
        .split(area);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .flex(Flex::Center)
        .constraints([
            Constraint::Length(NEXT_W),
            Constraint::Length(2),
            Constraint::Length(BOARD_W),
            Constraint::Length(2),
            Constraint::Length(INFO_W),
        ])
        .split(row[0]);
    draw_next(frame, cols[0], game.next);
    draw_board(frame, cols[2], game);
    draw_info(frame, cols[4], game);
}

/// Returns true if the game should exit.
fn handle_key(key: &KeyEvent, game: &mut Game, last_gravity: &mut Instant) -> bool {
    if key.modifiers != KeyModifiers::NONE {
        // Only Ctrl+C is honoured; other modified keys are ignored.
        return key
            .modifiers
            .contains(KeyModifiers::CONTROL)
            && key.code == KeyCode::Char('c');
    }
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => true,
        KeyCode::Char('p') => {
            game.toggle_pause();
            *last_gravity = Instant::now();
            false
        }
        KeyCode::Char('r') if game.state == GameState::GameOver => {
            *game = Game::new();
            *last_gravity = Instant::now();
            false
        }
        code if game.state == GameState::Playing => {
            match code {
                KeyCode::Left => game.move_left(),
                KeyCode::Right => game.move_right(),
                KeyCode::Down => game.soft_drop(),
                KeyCode::Up | KeyCode::Char('z') => game.rotate(),
                KeyCode::Char('x') => game.rotate_ccw(),
                KeyCode::Char(' ') => game.hard_drop(),
                _ => {}
            }
            false
        }
        _ => false,
    }
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut game = Game::new();
    let mut running = true;
    let mut last_gravity = Instant::now();
    let frame_time = Duration::from_millis(16);

    while running {
        while event::poll(frame_time)? {
            let quit = match event::read()? {
                Event::Key(key)
                    if key.kind == KeyEventKind::Press
                        || key.kind == KeyEventKind::Repeat =>
                {
                    handle_key(&key, &mut game, &mut last_gravity)
                }
                _ => false,
            };
            if quit {
                running = false;
                break;
            }
        }
        if game.state == GameState::Playing {
            let interval = Duration::from_millis(game.interval());
            if Instant::now().duration_since(last_gravity) >= interval {
                game.tick();
                last_gravity = Instant::now();
            }
        }
        terminal.draw(|frame| draw(frame, &game))?;
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn rendered(game: &Game, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal.draw(|frame| draw(frame, game)).expect("draw");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn render_shows_panels_and_falling_piece() {
        let game = Game::new();
        let text = rendered(&game, 100, 40);
        assert!(text.contains("TETRIS"));
        assert!(text.contains("Next"));
        assert!(text.contains("Score"));
        assert!(text.contains("Controls:"));
        assert!(
            text.contains('\u{2588}'),
            "the current piece must be drawn"
        );
        assert!(
            text.contains('\u{2591}'),
            "the ghost piece must be drawn"
        );
    }

    #[test]
    fn render_shows_game_over_overlay() {
        let mut game = Game::new();
        game.state = GameState::GameOver;
        let text = rendered(&game, 100, 40);
        assert!(text.contains("Game Over"));
        assert!(text.contains("r restart - q quit"));
    }

    #[test]
    fn render_shows_pause_overlay() {
        let mut game = Game::new();
        game.state = GameState::Paused;
        let text = rendered(&game, 100, 40);
        assert!(text.contains("Paused"));
        assert!(text.contains("p resume - q quit"));
    }

    #[test]
    fn render_small_terminal_shows_size_hint() {
        let game = Game::new();
        let text = rendered(&game, 20, 10);
        assert!(text.contains("Terminal too small"));
    }
}
