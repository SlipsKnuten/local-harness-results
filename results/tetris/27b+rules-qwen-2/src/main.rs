//! Terminal Tetris built on ratatui + crossterm.
//!
//! Controls:
//!   arrows / wasd  move, soft drop, rotate (up/x clockwise, z counter)
//!   space          hard drop
//!   c              hold piece
//!   p              pause
//!   r              restart (after game over)
//!   q / esc        quit

mod game;

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use game::{Cell, GameState, Game, PieceType, COLS, ROWS};

const BOARD_WIDTH: u16 = (COLS * 2) as u16 + 2;
const SIDEBAR_WIDTH: u16 = 24;
const MIN_WIDTH: u16 = BOARD_WIDTH + SIDEBAR_WIDTH + 2;
const MIN_HEIGHT: u16 = (ROWS + 3) as u16;

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x5EED_5EED)
}

fn run(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    let mut game = Game::new(seed());
    let mut last_tick = Instant::now();

    loop {
        let wait = if game.state == GameState::Playing {
            let elapsed = last_tick.elapsed().as_millis() as u64;
            Duration::from_millis(game.tick_ms().saturating_sub(elapsed))
        } else {
            Duration::from_millis(200)
        };

        let quit = if event::poll(wait)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    handle_key(&mut game, key.code)
                }
                _ => false,
            }
        } else {
            game.tick();
            last_tick = Instant::now();
            false
        };

        if quit {
            return Ok(());
        }
        if game.state != GameState::Playing {
            last_tick = Instant::now();
        }
        terminal.draw(|f| draw(f, &game))?;
    }
}

/// Returns true when the user asked to quit.
fn handle_key(game: &mut Game, code: KeyCode) -> bool {
    if matches!(code, KeyCode::Char('q') | KeyCode::Esc) {
        return true;
    }
    match game.state {
        GameState::Playing => match code {
            KeyCode::Left | KeyCode::Char('a') => game.move_h(-1),
            KeyCode::Right | KeyCode::Char('d') => game.move_h(1),
            KeyCode::Down | KeyCode::Char('s') => game.soft_drop(),
            KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('x') => game.rotate(1),
            KeyCode::Char('z') => game.rotate(-1),
            KeyCode::Char(' ') => game.hard_drop(),
            KeyCode::Char('c') | KeyCode::Char('h') => game.hold(),
            KeyCode::Char('p') => game.state = GameState::Paused,
            _ => {}
        },
        GameState::Paused => match code {
            KeyCode::Char('p') | KeyCode::Char(' ') => game.state = GameState::Playing,
            _ => {}
        },
        GameState::Over => {
            if let KeyCode::Char('r') = code {
                *game = Game::new(seed());
            }
        }
    }
    false
}

fn piece_color(piece: PieceType) -> Color {
    match piece {
        PieceType::I => Color::Cyan,
        PieceType::O => Color::Yellow,
        PieceType::T => Color::Magenta,
        PieceType::S => Color::Green,
        PieceType::Z => Color::Red,
        PieceType::J => Color::Blue,
        PieceType::L => Color::Rgb(255, 165, 0),
    }
}

fn filled_style(piece: PieceType, bold: bool) -> Style {
    let style = Style::default().fg(piece_color(piece));
    if bold {
        style.add_modifier(Modifier::BOLD)
    } else {
        style
    }
}

fn draw(f: &mut Frame, game: &Game) {
    let area = f.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        f.render_widget(
            Paragraph::new(format!(
                "Terminal too small ({}, {}), need {MIN_WIDTH}x{MIN_HEIGHT}",
                area.width, area.height
            ))
            .centered(),
            area,
        );
        return;
    }

    let [main, footer] = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

    let [_, board_col, side, _] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(BOARD_WIDTH),
        Constraint::Length(SIDEBAR_WIDTH),
        Constraint::Min(0),
    ])
    .areas(main);

    let [_, board_area, _] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(ROWS as u16 + 2),
        Constraint::Min(0),
    ])
    .areas(board_col);

    render_board(f, game, board_area);
    render_footer(f, footer);
    render_sidebar(f, game, side);

    match game.state {
        GameState::Paused => render_overlay(
            f,
            board_area,
            vec![
                Line::from("  PAUSED  "),
                Line::from("  press p to resume"),
            ],
            Color::Yellow,
        ),
        GameState::Over => render_overlay(
            f,
            board_area,
            vec![
                Line::from("  GAME OVER  "),
                Line::from(Span::styled(
                    format!("  score: {}  ", game.score),
                    Style::default().fg(Color::White),
                )),
                Line::from("  press r to restart"),
            ],
            Color::Red,
        ),
        GameState::Playing => {}
    }
}

fn render_board(f: &mut Frame, game: &Game, area: Rect) {
    let mut lines = Vec::with_capacity(ROWS);
    for r in 0..ROWS {
        let mut spans = Vec::with_capacity(COLS);
        for c in 0..COLS {
            let (chars, style) = match game.board[r][c] {
                Cell::Filled(piece) => ("\u{2588}\u{2588}", filled_style(piece, false)),
                Cell::Empty if game.is_active_at(r, c) => (
                    "\u{2588}\u{2588}",
                    filled_style(game.current.shape, true),
                ),
                Cell::Empty if game.is_ghost_at(r, c) => (
                    "\u{2591}\u{2591}",
                    Style::default()
                        .fg(piece_color(game.current.shape))
                        .add_modifier(Modifier::DIM),
                ),
                Cell::Empty => ("  ", Style::default()),
            };
            spans.push(Span::styled(chars, style));
        }
        lines.push(Line::from(spans));
    }

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" TETRIS ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ),
        area,
    );
}

fn render_footer(f: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled("move ", Style::default().fg(Color::DarkGray)),
        Span::styled("←→", Style::default().fg(Color::White)),
        Span::styled("  drop ", Style::default().fg(Color::DarkGray)),
        Span::styled("↓", Style::default().fg(Color::White)),
        Span::styled("  rotate ", Style::default().fg(Color::DarkGray)),
        Span::styled("↑ X Z", Style::default().fg(Color::White)),
        Span::styled("  hard ", Style::default().fg(Color::DarkGray)),
        Span::styled("space", Style::default().fg(Color::White)),
        Span::styled("  hold ", Style::default().fg(Color::DarkGray)),
        Span::styled("C", Style::default().fg(Color::White)),
        Span::styled("  pause ", Style::default().fg(Color::DarkGray)),
        Span::styled("P", Style::default().fg(Color::White)),
        Span::styled("  quit ", Style::default().fg(Color::DarkGray)),
        Span::styled("Q", Style::default().fg(Color::White)),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn render_sidebar(f: &mut Frame, game: &Game, area: Rect) {
    let [score_area, stats_area, next_area, hold_area] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(4),
        Constraint::Length(9),
        Constraint::Length(5),
    ])
    .areas(area);

    let score = Paragraph::new(vec![Line::from(format!("Score: {}", game.score))]).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" SCORE ")
            .title_style(Style::default().fg(Color::Cyan)),
    );
    f.render_widget(score, score_area);

    let stats = Paragraph::new(vec![Line::from(format!(
        "Lines: {}   Level: {}",
        game.lines, game.level
    ))])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" STATUS ")
            .title_style(Style::default().fg(Color::Cyan)),
    );
    f.render_widget(stats, stats_area);

    let next_lines: Vec<Line> = game
        .next()
        .iter()
        .flat_map(|piece| {
            piece_mini(*piece)
                .into_iter()
                .chain(std::iter::once(Line::from(" ")))
        })
        .collect();
    let next = Paragraph::new(next_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" NEXT ")
                .title_style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Center);
    f.render_widget(next, next_area);

    let hold_lines = match game.hold {
        Some(piece) => piece_mini(piece),
        None => vec![Line::from(" (empty) ")],
    };
    let hold = Paragraph::new(hold_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" HOLD ")
                .title_style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Center);
    f.render_widget(hold, hold_area);
}

/// Render a piece as a small block of block characters.
fn piece_mini(piece: PieceType) -> Vec<Line<'static>> {
    let offsets = piece.offsets(0);
    let width = offsets.iter().map(|&(x, _)| x).max().unwrap() + 1;
    let min_y = offsets.iter().map(|&(_, y)| y).min().unwrap();
    let max_y = offsets.iter().map(|&(_, y)| y).max().unwrap();
    let color = piece_color(piece);
    (min_y..=max_y)
        .map(|y| {
            (0..width)
                .map(|x| {
                    if offsets.iter().any(|&(ox, oy)| ox == x && oy == y) {
                        Span::styled("\u{2588}", Style::default().fg(color))
                    } else {
                        Span::raw(" ")
                    }
                })
                .collect::<Vec<Span>>()
                .into()
        })
        .collect()
}

fn render_overlay(f: &mut Frame, board_area: Rect, lines: Vec<Line<'static>>, accent: Color) {
    let width = 22u16.min(board_area.width);
    let height = (lines.len() as u16 + 2).min(board_area.height);
    let x = board_area.x + (board_area.width.saturating_sub(width)) / 2;
    let y = board_area.y + (board_area.height.saturating_sub(height)) / 2;
    let area = Rect::new(x, y, width, height);
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(accent))
                    .style(Style::default().bg(Color::Rgb(10, 10, 25))),
            )
            .alignment(Alignment::Center),
        area,
    );
}
