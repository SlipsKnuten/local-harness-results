//! ratatui rendering: playfield, ghost piece, next/hold sidebar, stats, and
//! the pause / game-over overlays. Pure drawing given a `&Game`.

use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};
use std::io::Stdout;

use crate::game::{Game, BOARD_ROWS, COLS, HIDDEN, VISIBLE_ROWS};
use crate::piece::Tetromino;

fn piece_color(idx: usize) -> Color {
    match idx {
        0 => Color::Cyan,       // I
        1 => Color::Yellow,     // O
        2 => Color::Magenta,    // T
        3 => Color::Green,      // S
        4 => Color::Red,        // Z
        5 => Color::Blue,       // J
        _ => Color::Indexed(208), // L (orange)
    }
}

/// Center a `(w, h)` rect inside `region`, clamping to what fits.
fn centered(region: Rect, w: u16, h: u16) -> Rect {
    let x = region.x + (region.width.saturating_sub(w)) / 2;
    let y = region.y + (region.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w.min(region.width), h.min(region.height))
}

/// A 4x4 text preview of a tetromino in its spawn orientation.
fn preview_lines(kind: Tetromino) -> Vec<String> {
    let size = kind.size() as usize;
    let offset = (4usize.saturating_sub(size)) / 2;
    let mut grid = vec![vec![' '; 4usize]; 4usize];
    for &(r, c) in &kind.base_cells() {
        let rr = offset + r as usize;
        let cc = offset + c as usize;
        if rr < 4 && cc < 4 {
            grid[rr][cc] = '█';
        }
    }
    grid.iter().map(|row| row.iter().collect()).collect()
}

/// Render a horizontal run of tetrominoes as a fixed 4-row-tall strip.
fn strip_lines<'a>(kinds: &'a [Tetromino]) -> Vec<Line<'a>> {
    let mut rows: Vec<Line> = (0..4).map(|_| Line::default()).collect();
    for (index, kind) in kinds.iter().enumerate() {
        if index > 0 {
            for row in rows.iter_mut() {
                row.spans.push(Span::raw(" "));
            }
        }
        let preview = preview_lines(*kind);
        for row in 0..4 {
            for ch in preview[row].chars() {
                rows[row].spans.push(Span::styled(
                    ch.to_string(),
                    Style::default().fg(piece_color(kind.idx())),
                ));
            }
        }
    }
    rows
}

/// Draw one frame of `game` to `terminal`.
pub fn draw(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    game: &Game,
) -> std::io::Result<()> {
    terminal.draw(|frame| render(frame, game))?;
    Ok(())
}

fn render(frame: &mut Frame, game: &Game) {
    let area = frame.area();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    frame.render_widget(
        Paragraph::new("  T E T R I S  ·  rust + ratatui")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center),
        outer[0],
    );

    // Playfield on the left, sidebar right next to it (classic layout).
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((COLS * 2 + 2) as u16),
            Constraint::Length(26),
            Constraint::Min(1),
        ])
        .split(outer[1]);

    let board_area = centered(main[0], (COLS * 2 + 2) as u16, (VISIBLE_ROWS + 2) as u16);
    render_board(frame, game, board_area);
    render_info(frame, game, centered(main[1], 24, 17));

    frame.render_widget(
        Paragraph::new("◀ ▶ move  ▼ drop  ▲/X rot  SPACE drop  C hold  P pause  R restart  Q quit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center),
        outer[2],
    );

    if game.paused {
        overlay(frame, area, "PAUSED", "press P to resume");
    } else if game.over {
        overlay(
            frame,
            area,
            "GAME OVER",
            &format!("score {}  ·  press R to restart, Q to quit", game.score),
        );
    }
}

fn render_board(frame: &mut Frame, game: &Game, area: Rect) {
    // Build the visible cells first (locked stack), then paint ghost + active
    // piece on top.
    let mut grid: Vec<Vec<(char, Style)>> = Vec::with_capacity(VISIBLE_ROWS);
    for visible_row in 0..VISIBLE_ROWS {
        let board_row = HIDDEN + visible_row;
        let mut row = Vec::with_capacity(COLS);
        for col in 0..COLS {
            let value = game.board.filled(board_row, col);
            if value == 0 {
                row.push((' ', Style::default()));
            } else {
                row.push(('█', Style::default().fg(piece_color(value as usize - 1))));
            }
        }
        grid.push(row);
    }

    if !game.over {
        let dist = game.drop_distance();
        // Ghost: the landing spot, only where the cell is currently empty.
        for &(r, c) in &game.current.cells {
            let br = game.current.y + r as i32 + dist;
            let bc = game.current.x + c as i32;
            if br >= HIDDEN as i32 && br < BOARD_ROWS as i32 && bc >= 0 && bc < COLS as i32 {
                let (vr, cc) = ((br - HIDDEN as i32) as usize, bc as usize);
                if grid[vr][cc].0 == ' ' {
                    grid[vr][cc] = (
                        '░',
                        Style::default()
                            .fg(piece_color(game.current.kind.idx()))
                            .add_modifier(Modifier::DIM),
                    );
                }
            }
        }
        // Active piece (overrides the ghost where they overlap).
        for &(r, c) in &game.current.cells {
            let br = game.current.y + r as i32;
            let bc = game.current.x + c as i32;
            if br >= HIDDEN as i32 && br < BOARD_ROWS as i32 && bc >= 0 && bc < COLS as i32 {
                let (vr, cc) = ((br - HIDDEN as i32) as usize, bc as usize);
                grid[vr][cc] = (
                    '█',
                    Style::default()
                        .fg(piece_color(game.current.kind.idx()))
                        .add_modifier(Modifier::BOLD),
                );
            }
        }
    }

    let mut lines: Vec<Line> = Vec::with_capacity(VISIBLE_ROWS);
    for row in &grid {
        let spans: Vec<Span> = row
            .iter()
            .map(|(ch, style)| Span::raw(format!("{} ", ch)).style(*style))
            .collect();
        lines.push(Line::from(spans));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" PLAYFIELD ")
        .style(Style::default().bg(Color::Indexed(234)));
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_info(frame: &mut Frame, game: &Game, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    let next: Vec<Tetromino> = game.queue().iter().take(3).copied().collect();
    lines.push(Line::from(vec![Span::styled(
        "NEXT",
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )]));
    for line in strip_lines(&next) {
        lines.push(line);
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "HOLD",
        Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
    )]));
    let hold: Vec<Tetromino> = game.hold.iter().copied().collect();
    for line in strip_lines(&hold) {
        lines.push(line);
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            "SCORE ",
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{}", game.score)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "LINES ",
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{}", game.lines)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "LEVEL ",
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{}", game.level)),
    ]));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" INFO ")
        .style(Style::default().bg(Color::Indexed(234)));
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn overlay(frame: &mut Frame, area: Rect, title: &str, sub: &str) {
    frame.render_widget(Clear, area);
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(42),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new(title)
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center),
        vertical[1],
    );
    frame.render_widget(
        Paragraph::new(sub)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center),
        vertical[2],
    );
}
