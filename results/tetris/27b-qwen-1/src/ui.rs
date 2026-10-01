//! Rendering: draws the whole game into a ratatui [`Frame`].

use std::collections::HashSet;

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
use ratatui::Frame;

use crate::board::{HEIGHT, WIDTH};
use crate::game::{Game, GameState};
use crate::tetromino::Tetromino;

const BG: Color = Color::Rgb(15, 17, 22);
const PANE_BG: Color = Color::Rgb(23, 26, 34);
const BORDER: Color = Color::Rgb(74, 80, 98);
const DIM: Color = Color::Rgb(150, 158, 176);
const FILL: &str = "▓▓";
const GHOST: &str = "░░";
const EMPTY: &str = "  ";

/// Multiply an RGB colour by a factor, clamped to 0..=255.
fn shade(c: Color, f: f32) -> Color {
    match c {
        Color::Rgb(r, g, b) => {
            Color::Rgb((r as f32 * f) as u8, (g as f32 * f) as u8, (b as f32 * f) as u8)
        }
        other => other,
    }
}

/// Return `area` shrunk to `w x h` and centred inside `area`.
fn center_rect(area: Rect, w: u16, h: u16) -> Rect {
    let x = area.x.saturating_add(area.width.saturating_sub(w) / 2);
    let y = area.y.saturating_add(area.height.saturating_sub(h) / 2);
    Rect::new(x, y, w.min(area.width), h.min(area.height))
}

/// A widget that paints an area with a single flat background colour.
struct Background {
    bg: Color,
}

impl Widget for Background {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_bg(self.bg);
            }
        }
    }
}

/// Style a single board cell given its active / ghost / locked occupancy.
fn board_cell_span(active: Option<Tetromino>, ghost: Option<Tetromino>, locked: Option<Tetromino>) -> Span<'static> {
    let bg = Style::default().bg(BG);
    match (active, ghost, locked) {
        (Some(kind), _, _) => Span::styled(FILL, bg.fg(shade(kind.color(), 1.0))),
        (None, Some(kind), _) => Span::styled(GHOST, bg.fg(shade(kind.color(), 0.40))),
        (None, None, Some(kind)) => Span::styled(FILL, bg.fg(shade(kind.color(), 0.72))),
        (None, None, None) => Span::styled(EMPTY, bg),
    }
}

pub fn render(f: &mut Frame, game: &Game) {
    let area = f.area();
    f.render_widget(Background { bg: BG }, area);

    let [title, main, help] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .areas(area);

    render_title(f, title, game);
    render_main(f, main, game);
    render_help(f, help, game);
    if game.state != GameState::Playing {
        render_overlay(f, main, game);
    }
}

fn render_title(f: &mut Frame, area: Rect, game: &Game) {
    let line = Line::from(vec![
        Span::styled(
            "  T E T R I S  ",
            Style::default().fg(Color::Rgb(0, 195, 215)).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("  LEVEL {}", game.level), Style::default().fg(DIM)),
    ]);
    f.render_widget(Paragraph::new(line).alignment(Alignment::Center), area);
}

fn render_main(f: &mut Frame, area: Rect, game: &Game) {
    // Centre the [left | board | right] group horizontally with flexible spacers.
    let [_l, left, board, right, _r] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(18),
        Constraint::Length(24),
        Constraint::Length(18),
        Constraint::Min(0),
    ])
    .areas(area);

    let board_box = center_rect(board, 22, 22);
    render_board(f, board_box, game);
    render_left(f, left, game);
    render_right(f, right, game);
}

fn render_board(f: &mut Frame, area: Rect, game: &Game) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let ghost = game.ghost_piece();
    let cur = game.current;
    let ghost_set: HashSet<(i32, i32)> =
        ghost.kind.cells(ghost.rot).iter().map(|&(r, c)| (ghost.y + r, ghost.x + c)).collect();
    let cur_set: HashSet<(i32, i32)> =
        cur.kind.cells(cur.rot).iter().map(|&(r, c)| (cur.y + r, cur.x + c)).collect();

    let mut lines = Vec::with_capacity(HEIGHT);
    for r in 0..HEIGHT {
        let mut spans = Vec::with_capacity(WIDTH);
        for c in 0..WIDTH {
            let (r, c) = (r as i32, c as i32);
            let active = if cur_set.contains(&(r, c)) { Some(cur.kind) } else { None };
            let ghostk = if ghost_set.contains(&(r, c)) { Some(ghost.kind) } else { None };
            let locked = game.board.cell(r as usize, c as usize);
            spans.push(board_cell_span(active, ghostk, locked));
        }
        lines.push(Line::from(spans));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn render_left(f: &mut Frame, area: Rect, game: &Game) {
    // Vertically centre the stack of stat boxes.
    let box_area = center_rect(area, 18, 24);
    let chunks: [Rect; 5] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(4),
        Constraint::Length(4),
        Constraint::Length(4),
        Constraint::Length(8),
    ])
    .areas(box_area);

    render_stat(f, chunks[0], " SCORE ", game.score.to_string());
    render_stat(f, chunks[1], " HIGH ", game.high.to_string());
    render_stat(f, chunks[2], " LEVEL ", game.level.to_string());
    render_stat(f, chunks[3], " LINES ", game.lines.to_string());
    render_hold(f, chunks[4], game);
}

fn render_right(f: &mut Frame, area: Rect, game: &Game) {
    let box_area = center_rect(area, 18, 18);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(PANE_BG))
        .title(Span::styled(" NEXT ", Style::default().fg(DIM)));
    let inner = block.inner(box_area);
    f.render_widget(block, box_area);

    let mut lines: Vec<Line<'static>> = Vec::new();
    for (i, kind) in game.next_pieces(3).iter().enumerate() {
        if i > 0 {
            lines.push(Line::raw(""));
        }
        lines.extend(mini_piece_lines(*kind));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn render_stat(f: &mut Frame, area: Rect, label: &str, value: String) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(PANE_BG))
        .title(Span::styled(label, Style::default().fg(DIM)));
    let inner = block.inner(area);
    f.render_widget(block, area);
    let line = Line::from(Span::styled(
        format!(" {} ", value),
        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
    ));
    f.render_widget(Paragraph::new(line).alignment(Alignment::Center), inner);
}

fn render_hold(f: &mut Frame, area: Rect, game: &Game) {
    let label = if game.can_hold { " HOLD " } else { " HOLD · used " };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(PANE_BG))
        .title(Span::styled(label, Style::default().fg(DIM)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    match game.hold {
        Some(kind) => {
            let mut lines: Vec<Line<'static>> = vec![Line::raw(""), Line::raw("")];
            lines.extend(mini_piece_lines(kind));
            f.render_widget(Paragraph::new(lines), inner);
        }
        None => {
            let line = Line::from(Span::styled("  —  ", Style::default().fg(DIM)));
            f.render_widget(Paragraph::new(line).alignment(Alignment::Center), inner);
        }
    }
}

fn render_help(f: &mut Frame, area: Rect, game: &Game) {
    let text = match game.state {
        GameState::GameOver => "  GAME OVER  —  press [R] to restart, [Q] to quit  ",
        GameState::Paused => "  PAUSED  —  press [P] to resume, [Q] to quit  ",
        GameState::Playing => {
            "  ←/→ move   ↓ soft drop   SPACE hard drop   ↑/X rotate   Z rotate ccw   C hold   P pause   Q quit  "
        }
    };
    let line = Line::from(Span::styled(text.to_string(), Style::default().fg(DIM)));
    f.render_widget(
        Paragraph::new(vec![Line::raw(""), line]).alignment(Alignment::Center),
        area,
    );
}

/// Render a single tetromino as a 4x2 grid of two-character cells, centred in a 16-column row.
fn mini_piece_lines(kind: Tetromino) -> Vec<Line<'static>> {
    let cells = kind.cells(0);
    let min_r = cells.iter().map(|(r, _)| *r).min().unwrap();
    let max_r = cells.iter().map(|(r, _)| *r).max().unwrap();
    let min_c = cells.iter().map(|(_, c)| *c).min().unwrap();
    let max_c = cells.iter().map(|(_, c)| *c).max().unwrap();
    let ph = max_r - min_r + 1;
    let pw = max_c - min_c + 1;
    let off_r = (2 - ph) / 2;
    let off_c = (4 - pw) / 2;

    let fg = kind.color();
    let pad = Span::styled("    ", Style::default().bg(PANE_BG));
    let mut out = Vec::with_capacity(2);
    for r in 0..2 {
        let mut spans = vec![pad.clone()];
        for c in 0..4 {
            let occupied = if r < off_r || c < off_c {
                false
            } else {
                cells.contains(&(min_r + (r - off_r), min_c + (c - off_c)))
            };
            let (sym, style) = if occupied {
                (FILL, Style::default().fg(fg).bg(PANE_BG))
            } else {
                (EMPTY, Style::default().bg(PANE_BG))
            };
            spans.push(Span::styled(sym, style));
        }
        spans.push(pad.clone());
        out.push(Line::from(spans));
    }
    out
}

fn render_overlay(f: &mut Frame, area: Rect, game: &Game) {
    if game.state == GameState::Playing {
        return;
    }
    let (title, sub, accent) = match game.state {
        GameState::Paused => (
            "  PAUSED  ",
            "  press [P] to resume  ".to_string(),
            Color::Rgb(0, 195, 215),
        ),
        GameState::GameOver => (
            "  GAME OVER  ",
            format!("  score {}  ·  press [R] to restart  ", game.score),
            Color::Rgb(232, 72, 72),
        ),
        _ => return,
    };

    let box_area = center_rect(area, 32, 7);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(PANE_BG));
    let inner = block.inner(box_area);
    f.render_widget(block, box_area);

    let lines = vec![
        Line::from(Span::styled(title, Style::default().fg(accent).add_modifier(Modifier::BOLD))),
        Line::raw(""),
        Line::from(Span::styled(sub, Style::default().fg(Color::White))),
    ];
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, GameState};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    /// Render the game to an in-memory buffer and return its text content.
    fn render_to_text(game: &Game) -> String {
        let backend = TestBackend::new(90, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let frame = terminal.draw(|f| render(f, game)).unwrap();
        frame
            .buffer
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect()
    }

    #[test]
    fn draws_title_panels_and_active_piece() {
        let game = Game::new(0);
        let content = render_to_text(&game);
        assert!(content.contains("T E T R I S"), "title should render");
        assert!(content.contains("SCORE"), "score panel should render");
        assert!(content.contains("NEXT"), "next panel should render");
        assert!(content.contains("HOLD"), "hold panel should render");
        assert!(content.contains(FILL), "active piece should render");
    }

    #[test]
    fn draws_game_over_overlay() {
        let mut game = Game::new(0);
        game.state = GameState::GameOver;
        game.score = 1234;
        let content = render_to_text(&game);
        assert!(content.contains("GAME OVER"), "game-over overlay should render");
        assert!(content.contains("1234"), "final score should render");
    }

    #[test]
    fn draws_paused_overlay() {
        let mut game = Game::new(0);
        game.state = GameState::Paused;
        let content = render_to_text(&game);
        assert!(content.contains("PAUSED"), "paused overlay should render");
    }
}
