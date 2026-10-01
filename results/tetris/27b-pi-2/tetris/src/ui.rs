use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell as TCell, Clear, Paragraph, Row, Table, Wrap,
    },
    Frame,
};

use crate::game::{COLS, Game, ROWS};
use crate::tetrominoes::PieceType;

/// Color associated with each tetromino.
pub fn color_for(kind: PieceType) -> Color {
    match kind {
        PieceType::I => Color::Cyan,
        PieceType::O => Color::Yellow,
        PieceType::T => Color::Magenta,
        PieceType::S => Color::Green,
        PieceType::Z => Color::Red,
        PieceType::J => Color::Blue,
        PieceType::L => Color::Rgb(255, 160, 0),
    }
}

fn piece_from_glyph(glyph: char) -> PieceType {
    PieceType::ALL
        .iter()
        .copied()
        .find(|k| k.glyph() == glyph)
        .unwrap_or(PieceType::T)
}

/// Draw the whole frame.
pub fn draw(f: &mut Frame, game: &Game) {
    let size = f.area();

    let cols = Layout::horizontal([
        Constraint::Length((COLS * 2 + 2) as u16),
        Constraint::Min(20),
    ]);
    let [board_area, side_area] = cols.areas(size);

    draw_board(f, game, board_area);
    draw_side(f, game, side_area);

    if game.over {
        draw_overlay(
            f,
            size,
            &["GAME OVER", "Press 'r' to restart", "Press 'q' to quit"],
        );
    } else if game.paused {
        draw_overlay(f, size, &["PAUSED", "Press 'p' to resume"]);
    }
}

/// Render the play field: locked cells + ghost + active piece.
fn draw_board(f: &mut Frame, game: &Game, area: Rect) {
    // Base layer: locked cells.
    let mut grid: [[Option<(char, Color)>; COLS]; ROWS] = [[None; COLS]; ROWS];
    for r in 0..ROWS {
        for c in 0..COLS {
            if let Some(glyph) = game.board[r][c].glyph() {
                grid[r][c] = Some((glyph, color_for(piece_from_glyph(glyph))));
            }
        }
    }

    // Ghost layer (only over empty cells).
    if let Some(ghost) = game.ghost() {
        let col = color_for(ghost.kind);
        for (c, r) in ghost.cells() {
            let (r, c) = (r as usize, c as usize);
            if r < ROWS && c < COLS && grid[r][c].is_none() {
                grid[r][c] = Some(('░', col));
            }
        }
    }

    // Active layer (on top of everything).
    if let Some(active) = &game.active {
        let col = color_for(active.kind);
        let glyph = active.kind.glyph();
        for (c, r) in active.cells() {
            let (r, c) = (r as usize, c as usize);
            if r < ROWS && c < COLS {
                grid[r][c] = Some((glyph, col));
            }
        }
    }

    // Compose rows of cells. Ghost cells are drawn dim.
    let mut rows: Vec<Row> = Vec::with_capacity(ROWS);
    for r in 0..ROWS {
        let mut cells: Vec<TCell> = Vec::with_capacity(COLS);
        for c in 0..COLS {
            let (symbol, style) = match grid[r][c] {
                Some(('░', col)) => (
                    "░░".to_string(),
                    Style::default().fg(col).add_modifier(Modifier::DIM),
                ),
                Some((_, col)) => (
                    "██".to_string(),
                    Style::default().fg(col),
                ),
                None => ("  ".to_string(), Style::default()),
            };
            cells.push(TCell::new(symbol).style(style));
        }
        rows.push(Row::new(cells));
    }

    let table = Table::new(rows, [Constraint::Length(2); COLS].to_vec()).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" TETRIS "),
    );
    f.render_widget(table, area);
}

/// Render the side panel: next piece, stats, and controls.
fn draw_side(f: &mut Frame, game: &Game, area: Rect) {
    let chunks: [Rect; 3] = Layout::vertical([
        Constraint::Length(6),
        Constraint::Length(9),
        Constraint::Min(1),
    ])
    .areas(area);

    // Next piece box (4x4 grid).
    let next_area = chunks[0];
    let offs = game.next.rotations()[0];
    let mut present = [[false; 4]; 4];
    for (c, r) in offs {
        present[r as usize][c as usize] = true;
    }
    let col = color_for(game.next);
    let mut rows: Vec<Row> = Vec::new();
    for r in 0..4 {
        let mut cells: Vec<TCell> = Vec::new();
        for c in 0..4 {
            let (s, style) = if present[r][c] {
                (
                    game.next.glyph().to_string(),
                    Style::default().fg(col).add_modifier(Modifier::BOLD),
                )
            } else {
                ("  ".to_string(), Style::default())
            };
            cells.push(TCell::new(s).style(style));
        }
        rows.push(Row::new(cells));
    }
    let table = Table::new(rows, [Constraint::Length(2); 4].to_vec()).block(
        Block::default().borders(Borders::ALL).title(" NEXT "),
    );
    f.render_widget(table, next_area);

    // Stats box.
    let stats_area = chunks[1];
    let stats_lines = vec![
        Line::from(vec![
            Span::styled("Score", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(": {:>8}", game.score)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Lines", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(": {:>8}", game.lines)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Level", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(": {:>8}", game.level)),
        ]),
    ];
    let stats = Paragraph::new(stats_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" STATS "),
    );
    f.render_widget(stats, stats_area);

    // Help box.
    let help_area = chunks[2];
    let help = Paragraph::new(vec![
        Line::from(Span::raw("  Arrows   Move / rotate")),
        Line::from(Span::raw("  Z        Rotate CCW")),
        Line::from(Span::raw("  Space    Hard drop")),
        Line::from(Span::raw("  P        Pause")),
        Line::from(Span::raw("  R        Restart")),
        Line::from(Span::raw("  Q        Quit")),
    ])
    .block(Block::default().borders(Borders::ALL).title(" CONTROLS "));
    f.render_widget(help, help_area);
}

/// Centered overlay banner (game over / paused).
fn draw_overlay(f: &mut Frame, area: Rect, lines: &[&str]) {
    let center: [Rect; 3] = Layout::vertical([
        Constraint::Percentage(42),
        Constraint::Length(5),
        Constraint::Percentage(42),
    ])
    .areas(area);
    let banner = center[1];

    f.render_widget(Clear, banner);
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black)),
        banner,
    );
    let text: Vec<Line> = lines.iter().map(|l| Line::from(*l)).collect();
    let para = Paragraph::new(text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
        .wrap(Wrap { trim: false });
    f.render_widget(para, banner);
}
