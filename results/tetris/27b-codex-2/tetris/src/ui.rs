use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::game::{Game, COLS, ROWS};

const BLOCK_CHAR: &str = "█";

pub fn color_for(idx: usize) -> Color {
    match idx {
        0 => Color::Cyan,
        1 => Color::Yellow,
        2 => Color::Magenta,
        3 => Color::Green,
        4 => Color::Red,
        5 => Color::Blue,
        _ => Color::Rgb(230, 126, 34),
    }
}

pub fn render(f: &mut Frame, game: &Game, paused: bool) {
    let area = f.area();
    if area.width < 46 || area.height < 14 {
        return;
    }

    let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area);
    let cols = Layout::horizontal([
        Constraint::Length(14),
        Constraint::Min(1),
        Constraint::Length(26),
    ])
    .split(rows[1]);

    f.render_widget(Paragraph::new(title_line()), rows[0]);
    render_board(f, game, board_rect(cols[1]), paused);
    render_side(f, game, cols[0], cols[2]);
}

fn title_line() -> Line<'static> {
    let bold = |c: Color| Style::default().fg(c).add_modifier(Modifier::BOLD);
    Line::from(vec![
        Span::raw("  "),
        Span::styled("T", bold(Color::Cyan)),
        Span::styled("E", bold(Color::Green)),
        Span::styled("T", bold(Color::Magenta)),
        Span::styled("R", bold(Color::Red)),
        Span::styled("I", bold(Color::Blue)),
        Span::styled("S", bold(Color::Yellow)),
        Span::styled("  ·  rust + ratatui", Style::default().fg(Color::Gray)),
    ])
}

fn board_rect(area: Rect) -> Rect {
    let width = (COLS as u16 + 2).min(area.width);
    let height = (ROWS as u16 + 2).min(area.height);
    Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    }
}

fn render_board(f: &mut Frame, game: &Game, rect: Rect, paused: bool) {
    let current = game.cells();
    let ghost_y = game.ghost_y();
    let ghost: [(isize, isize); 4] = if ghost_y != game.y {
        game.cells_at(game.x, ghost_y, game.rot)
    } else {
        [(-1, -1); 4]
    };

    let mut lines = Vec::with_capacity(ROWS);
    for row in 0..ROWS {
        let mut spans = Vec::with_capacity(COLS);
        for col in 0..COLS {
            let idx = row * COLS + col;
            let locked = game.grid[idx];
            if locked != 0 {
                spans.push(Span::styled(
                    BLOCK_CHAR,
                    Style::default().fg(color_for((locked - 1) as usize)),
                ));
            } else if current.iter().any(|&(x, y)| x == col as isize && y == row as isize) {
                spans.push(Span::styled(
                    BLOCK_CHAR,
                    Style::default().fg(color_for(game.kind.idx())),
                ));
            } else if ghost.iter().any(|&(x, y)| x == col as isize && y == row as isize) {
                spans.push(Span::styled(
                    BLOCK_CHAR,
                    Style::default()
                        .fg(color_for(game.kind.idx()))
                        .add_modifier(Modifier::DIM),
                ));
            } else {
                spans.push(Span::raw(" "));
            }
        }
        lines.push(Line::from(spans));
    }

    f.render_widget(
        Paragraph::new(lines).block(
            Block::bordered().style(Style::default().bg(Color::Rgb(10, 10, 14))),
        ),
        rect,
    );

    if paused && !game.game_over() {
        let bar = Rect::new(rect.x, rect.y + rect.height / 2, rect.width, 1);
        let line = Line::from(Span::styled(
            "  PAUSED  ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
        f.render_widget(Paragraph::new(line).alignment(Alignment::Center), bar);
    }

    if game.game_over() {
        let width = rect.width.saturating_sub(4).max(1);
        let panel = Rect::new(
            rect.x + rect.width / 2 - width / 2,
            rect.y + rect.height / 2 - 4,
            width,
            7.min(rect.height),
        );
        f.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Rgb(16, 16, 22))),
            panel,
        );
        let text = Paragraph::new(vec![
            Line::from(Span::styled(
                "GAME OVER",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("Score: {}", game.score),
                Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "r restart    q quit",
                Style::default().fg(Color::Gray),
            )),
        ])
        .alignment(Alignment::Center);
        f.render_widget(text, panel);
    }
}

fn render_side(f: &mut Frame, game: &Game, left: Rect, right: Rect) {
    let left_cols = Layout::vertical([Constraint::Length(8), Constraint::Min(0)]).split(left);
    render_next(f, game, left_cols[0]);
    render_controls(f, left_cols[1]);
    render_stats(f, game, right);
}

fn render_next(f: &mut Frame, game: &Game, rect: Rect) {
    let (size, base) = game.next.shape();
    let offset = (4 - size) / 2;
    let mut lines = vec![Line::from("")];
    for row in 0..4 {
        let mut spans = Vec::with_capacity(4);
        for col in 0..4 {
            let filled = base
                .iter()
                .any(|&(cx, cy)| cx + offset == col && cy + offset == row);
            if filled {
                spans.push(Span::styled(
                    BLOCK_CHAR,
                    Style::default().fg(color_for(game.next.idx())),
                ));
            } else {
                spans.push(Span::raw(" "));
            }
        }
        lines.push(Line::from(spans));
    }
    f.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(Block::bordered().title("NEXT")),
        rect,
    );
}

fn render_controls(f: &mut Frame, rect: Rect) {
    let dim = Style::default().fg(Color::Gray);
    let lines = [
        ("←/a   →/d", "move"),
        ("↓/s", "soft drop"),
        ("↑/w   x", "rotate"),
        ("z", "rotate ccw"),
        ("space", "hard drop"),
        ("p", "pause"),
        ("q", "quit"),
    ]
    .into_iter()
    .map(|(keys, action)| {
        Line::from(vec![
            Span::styled(keys, Style::default().fg(Color::White)),
            Span::raw("  "),
            Span::styled(action, dim),
        ])
    })
    .collect::<Vec<_>>();
    f.render_widget(
        Paragraph::new(lines).block(Block::bordered().title("KEYS")),
        rect,
    );
}

fn render_stats(f: &mut Frame, game: &Game, rect: Rect) {
    let dim = Style::default().fg(Color::Gray);
    let bold = |text: String, color: Color| {
        Span::styled(text, Style::default().fg(color).add_modifier(Modifier::BOLD))
    };
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Score  ", dim),
            bold(game.score.to_string(), Color::White),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Level  ", dim),
            bold(game.level.to_string(), Color::Yellow),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Lines  ", dim),
            bold(game.lines.to_string(), Color::Cyan),
        ]),
    ];
    f.render_widget(
        Paragraph::new(lines).block(Block::bordered().title("STATS")),
        rect,
    );
}
