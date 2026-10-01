//! Ratatui rendering: the play field, the live piece, the "next" preview and
//! the status panel (score / lines / level / controls).

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use crate::board::{Board, Kind, HEIGHT, WIDTH};
use crate::game::{Game, Status};
use crate::tetromino::Piece;

/// Map a piece kind to a foreground/background color pair.
fn style_for(kind: Kind) -> Style {
    let color = match kind {
        Kind::I => Color::Cyan,
        Kind::O => Color::Yellow,
        Kind::T => Color::Magenta,
        Kind::S => Color::Green,
        Kind::Z => Color::Red,
        Kind::J => Color::Blue,
        Kind::L => Color::Rgb(220, 130, 0),
    };
    Style::new().fg(color).bg(Color::Black)
}

/// Draw a single 4x2 cell: filled blocks get a colored glyph, empty space is a
/// faint dot so the field stays visible.
fn cell_glyph(kind: Option<Kind>) -> Line<'static> {
    match kind {
        Some(k) => Line::from(Span::styled("██", style_for(k))),
        None => Line::from(Span::styled("  ", Style::default().fg(Color::Reset))),
    }
}

/// Render a piece's blocks into a grid, positioned relative to the piece's own
/// bounding box so it draws correctly regardless of where it sits on the board.
fn piece_lines(piece: &Piece, rows: usize) -> Text<'static> {
    let cells = piece.cells();
    let min_x = cells.iter().map(|&(x, _)| x).min().unwrap_or(0);
    let min_y = cells.iter().map(|&(_, y)| y).min().unwrap_or(0);

    let mut text = Text::default();
    for dy in 0..rows as isize {
        let mut line = Line::default();
        for dx in 0..4 {
            let occupied = cells
                .iter()
                .any(|&(x, y)| x - min_x == dx && y - min_y == dy);
            if occupied {
                line.push_span(Span::styled("██", style_for(piece.kind)));
            } else {
                line.push_span(Span::raw("  "));
            }
        }
        text.lines.push(line);
    }
    text
}

struct FieldWidget<'a> {
    board: &'a Board,
    current: Option<&'a Piece>,
}

impl Widget for FieldWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Each cell is drawn as two columns ("██" or a blank). We scale the
        // area to the logical board so cells line up regardless of the
        // terminal size.
        let mut buf_lines = Vec::with_capacity(HEIGHT);
        for y in 0..HEIGHT {
            let mut line = Line::default();
            for x in 0..WIDTH {
                let occupied = self.board.cell(x, y);
                line.extend(cell_glyph(occupied).spans.iter().cloned());
            }
            buf_lines.push(line);
        }

        // Overlay the live piece on top (cells it covers take its color).
        // Each board cell is a single 2-char span, so cell (x, y) is spans[y][x].
        if let Some(piece) = self.current {
            for (x, y) in piece.cells() {
                if x < 0 || x >= WIDTH as isize || y < 0 || y >= HEIGHT as isize {
                    continue;
                }
                let y = y as usize;
                let x = x as usize;
                buf_lines[y].spans[x] = cell_glyph(Some(piece.kind)).spans[0].clone();
            }
        }

        let p = Paragraph::new(Text::from(buf_lines))
            .block(Block::default().borders(Borders::ALL).title(" TETRIS "));
        p.render(area, buf);
    }
}

/// The right-hand status panel: next piece, score, lines, level and keys.
fn next_panel(game: &Game) -> Paragraph<'static> {
    let preview = piece_lines(&Piece::spawn(game.next), 4);
    let status_text = match game.status {
        Status::Playing => "Playing".to_string(),
        Status::Over => "GAME OVER".to_string(),
    };
    let controls = [
        "←/→ : move",
        "↑/x : rotate",
        "↓     : soft drop",
        "space : hard drop",
        "p     : pause",
        "q/esc : quit",
    ]
    .iter()
    .map(|s| Line::from(Span::styled(*s, Style::default().fg(Color::Gray))))
    .collect::<Vec<_>>();

    let mut lines: Vec<Line<'static>> = vec![Line::from(Span::styled(
        "NEXT",
        Style::default().add_modifier(Modifier::BOLD),
    ))];
    lines.extend(preview.lines);
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("Score: {}", game.score),
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::raw(format!("Lines: {}", game.lines))));
    lines.push(Line::from(Span::raw(format!("Level: {}", game.level))));
    lines.push(Line::from(Span::styled(
        format!("Status: {status_text}"),
        Style::default().fg(if game.status == Status::Over {
            Color::Red
        } else {
            Color::Green
        }),
    )));
    lines.push(Line::from(""));
    lines.extend(controls);

    Paragraph::new(Text::from(lines)).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" INFO ")
            .border_style(Style::default().fg(Color::Cyan)),
    )
}

/// Compose the full frame.
pub fn draw(f: &mut ratatui::Frame, game: &Game) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(WIDTH as u16 * 2 + 2),
            Constraint::Min(12),
        ])
        .split(f.area());

    // Anchor the field to the top of the left column so its border wraps the
    // 20-row board snugly and lines up with the info panel, instead of
    // stretching to the terminal's full height.
    let left = Layout::default()
        .direction(Direction::Vertical)
        .flex(Flex::Start)
        .constraints([Constraint::Length((HEIGHT + 2) as u16), Constraint::Min(0)])
        .split(chunks[0]);

    FieldWidget {
        board: &game.board,
        current: Some(&game.current),
    }
    .render(left[0], f.buffer_mut());

    let right = Layout::default()
        .direction(Direction::Vertical)
        .flex(Flex::Start)
        .constraints([Constraint::Length(16), Constraint::Min(0)])
        .split(chunks[1]);

    let panel = next_panel(game);
    panel.render(right[0], f.buffer_mut());

    // A thin single-line hint at the bottom of the info column.
    let hint = Paragraph::new(Text::from(vec![Line::from(Span::styled(
        "Rust Tetris — q to quit",
        Style::default().fg(Color::DarkGray),
    ))]))
    .wrap(Wrap { trim: true });
    hint.render(right[1], f.buffer_mut());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_glyph_is_two_columns() {
        assert_eq!(cell_glyph(Some(Kind::I)).to_string(), "██");
        assert_eq!(cell_glyph(None).to_string(), "  ");
    }

    #[test]
    fn piece_lines_covers_four_blocks() {
        let piece = Piece::spawn(Kind::O);
        let text = piece_lines(&piece, 4);
        // The O piece occupies 4 cells; the rendered grid has 4 rows x 4 cols
        // of two-char cells, so 8 blocks ("██") are drawn.
        let rendered: String = text
            .lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.to_string())
                    .collect::<String>()
            })
            .collect::<String>();
        assert_eq!(rendered.matches("██").count(), 4);
    }

    #[test]
    fn style_for_returns_distinct_colors() {
        let i = style_for(Kind::I);
        let t = style_for(Kind::T);
        assert_eq!(i.fg, Some(Color::Cyan));
        assert_eq!(t.fg, Some(Color::Magenta));
    }

    /// Regression test: a live piece occupying a high column (x >= 5) used to
    /// index the overlay spans out of bounds and panic mid-game.
    #[test]
    fn field_render_does_not_panic_for_piece_at_high_column() {
        let board = Board::new();
        // Force the live piece to sit against the right wall, covering x=9.
        let piece = Piece {
            kind: Kind::O,
            rot: 0,
            x: 8,
            y: 5,
        };
        let area = Rect::new(0, 0, WIDTH as u16 * 2, HEIGHT as u16);
        let mut buf = Buffer::empty(area);
        FieldWidget {
            board: &board,
            current: Some(&piece),
        }
        .render(area, &mut buf);
    }
}
