//! Ratatui rendering for the Tetris UI.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::border,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::game::{COLS, ROWS, Game, Kind, Status};

/// Colors in the same order as `Kind::ALL`: I, O, T, S, Z, J, L.
const KIND_COLORS: [Color; 7] = [
    Color::Cyan,
    Color::Yellow,
    Color::Magenta,
    Color::Green,
    Color::Red,
    Color::Blue,
    Color::Indexed(214),
];

fn color(kind: Kind) -> Color {
    KIND_COLORS[kind as usize]
}

fn rounded_block(title: &str, title_color: Color) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(title_color),
        ))
}

fn inner(area: Rect) -> Rect {
    Rect::new(
        area.x + 1,
        area.y + 1,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    )
}

pub fn draw(f: &mut Frame, game: &Game) {
    let area = f.area();
    if area.width < 64 || area.height < 24 {
        f.render_widget(
            Paragraph::new("TETRIS needs at least a 64x24 terminal.")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Yellow)),
            area,
        );
        return;
    }

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Fill(1)])
        .split(area);
    f.render_widget(title_bar(), outer[0]);

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20),
            Constraint::Length(COLS as u16 * 2 + 2),
            Constraint::Length(22),
            Constraint::Min(0),
        ])
        .split(outer[1]);

    draw_controls(f, main[0]);
    draw_board(f, game, main[1]);
    draw_side_panel(f, game, main[2]);
}

fn title_bar() -> Paragraph<'static> {
    let letters = ['T', 'E', 'T', 'R', 'I', 'S'];
    let palette = [
        Color::Cyan,
        Color::Magenta,
        Color::Cyan,
        Color::Magenta,
        Color::Cyan,
        Color::Magenta,
    ];
    let mut spans: Vec<Span> = Vec::with_capacity(letters.len() + 2);
    spans.push(Span::raw("  "));
    for (i, ch) in letters.iter().enumerate() {
        spans.push(Span::styled(
            format!("{ch} "),
            Style::default()
                .fg(palette[i])
                .add_modifier(Modifier::BOLD),
        ));
    }
    Paragraph::new(Line::from(spans).alignment(Alignment::Center))
}

fn draw_controls(f: &mut Frame, area: Rect) {
    let rows = [
        ("← →", "Move"),
        ("↑ / x", "Rotate"),
        ("z", "Rotate back"),
        ("↓", "Soft drop"),
        ("space", "Hard drop"),
        ("p", "Pause"),
        ("r", "Restart"),
        ("q", "Quit"),
    ];
    let lines = rows
        .iter()
        .map(|(key, desc)| {
            Line::from(vec![
                Span::styled(
                    format!("{key:<6}"),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(*desc, Style::default().fg(Color::Gray)),
            ])
        })
        .collect::<Vec<_>>();
    let widget = Paragraph::new(Text::from(lines));
    f.render_widget(rounded_block("Controls", Color::Cyan), area);
    f.render_widget(widget, inner(area));
}

fn draw_board(f: &mut Frame, game: &Game, area: Rect) {
    // Center the fixed-height board vertically in the column.
    let column = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Fill(1), Constraint::Length(ROWS as u16 + 2), Constraint::Fill(1)])
        .split(area);
    let board_area = column[1];

    f.render_widget(rounded_block("TETRIS", Color::Cyan), board_area);
    let field = inner(board_area);

    // grid[r][c] = (kind, is_solid); ghosts are non-solid.
    let mut grid: Vec<Vec<Option<(Kind, bool)>>> = vec![vec![None; COLS]; ROWS];
    for (r, row) in game.board().rows().iter().enumerate() {
        for (c, &v) in row.iter().enumerate() {
            if v != 0 {
                grid[r][c] = Some((Kind::from_idx(v as usize - 1), true));
            }
        }
    }
    if let Some(piece) = game.current() {
        let dist = game.drop_distance() as isize;
        for &(c, r) in &piece.cells() {
            let gr = r + dist;
            if gr >= 0 && gr < ROWS as isize && c >= 0 && c < COLS as isize
                && grid[gr as usize][c as usize].is_none()
            {
                grid[gr as usize][c as usize] = Some((piece.kind, false));
            }
        }
        for &(c, r) in &piece.cells() {
            if r >= 0 && r < ROWS as isize && c >= 0 && c < COLS as isize {
                grid[r as usize][c as usize] = Some((piece.kind, true));
            }
        }
    }

    let lines = (0..ROWS)
        .map(|r| {
            let spans = (0..COLS)
                .map(|c| match grid[r][c] {
                    Some((kind, true)) => {
                        Span::styled("[]", Style::default().fg(color(kind)))
                    }
                    Some((kind, false)) => Span::styled(
                        "[]",
                        Style::default()
                            .fg(color(kind))
                            .add_modifier(Modifier::DIM),
                    ),
                    None => Span::raw("  "),
                })
                .collect::<Vec<_>>();
            Line::from(spans)
        })
        .collect::<Vec<_>>();
    f.render_widget(Paragraph::new(Text::from(lines)), field);

    match game.status() {
        Status::Paused => overlay(f, field, "PAUSED", Color::Yellow, "press P to resume"),
        Status::Over => overlay(f, field, "GAME OVER", Color::Red, "press R to restart"),
        Status::Playing => {}
    }
}

fn overlay(f: &mut Frame, area: Rect, title: &str, title_color: Color, sub: &str) {
    let y = area.y.saturating_add(area.height / 2 - 2);
    let rect = Rect::new(area.x, y, area.width, 4);
    let text = Text::from(vec![
        Line::from(Span::styled(
            title.to_string(),
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        Line::from(Span::styled(sub.to_string(), Style::default().fg(Color::Gray)))
            .alignment(Alignment::Center),
    ]);
    f.render_widget(Paragraph::new(text), rect);
}

fn draw_side_panel(f: &mut Frame, game: &Game, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Min(0),
        ])
        .split(area);

    f.render_widget(rounded_block("Next", Color::Magenta), layout[0]);
    f.render_widget(
        Paragraph::new(Text::from(piece_preview(game.next()))),
        inner(layout[0]),
    );

    f.render_widget(rounded_block("Score", Color::Yellow), layout[1]);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("{:07}", game.score()),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ))),
        inner(layout[1]),
    );

    f.render_widget(rounded_block("Level", Color::Green), layout[2]);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            game.level().to_string(),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ))),
        inner(layout[2]),
    );

    f.render_widget(rounded_block("Lines", Color::Blue), layout[3]);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            game.lines().to_string(),
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        ))),
        inner(layout[3]),
    );
}

/// Render the next piece centered on a 4x4 cell grid (each cell is "[]" wide).
fn piece_preview(next: Option<Kind>) -> Vec<Line<'static>> {
    let Some(kind) = next else {
        return vec![Line::raw("  (none)  ".to_string())];
    };
    let box_size = kind.box_size();
    let ox = (4 - box_size) / 2;
    let oy = (4 - box_size) / 2;
    let cells: Vec<(usize, usize)> = kind
        .shape_cells(0)
        .iter()
        .map(|&(x, y)| (x + ox, y + oy))
        .collect();
    (0..4)
        .map(|r| {
            let spans = (0..4)
                .map(|c| {
                    if cells.contains(&(c, r)) {
                        Span::styled("[]", Style::default().fg(color(kind)))
                    } else {
                        Span::raw("  ")
                    }
                })
                .collect::<Vec<_>>();
            Line::from(spans)
        })
        .collect()
}
