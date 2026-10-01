mod game;

use std::io;
use std::time::Duration;

use crossterm::event;
use crossterm::event::{KeyCode, KeyEvent};
use game::{Game, HEIGHT, Shape, WIDTH};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph};
use rand::rngs::StdRng;
use rand::SeedableRng;

const CONTROLS: [&str; 7] = [
    "arrows: move / rotate",
    "down: soft drop",
    "space: hard drop",
    "z / x: rotate",
    "p: pause",
    "enter: restart",
    "q / esc: quit",
];

fn main() -> io::Result<()> {
    let terminal = ratatui::try_init()?;
    let result = run(terminal);
    ratatui::restore();
    result
}

fn run(mut terminal: ratatui::DefaultTerminal) -> io::Result<()> {
    let mut game = new_game();
    let mut paused = false;
    loop {
        terminal.draw(|f| draw(f, &game, paused))?;

        if !event::poll(Duration::from_millis(game.delay_ms()))? {
            if !paused && !game.over {
                game.tick();
            }
            continue;
        }
        match event::read()? {
            event::Event::Key(key) if key.modifiers.is_empty() => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => break,
                KeyCode::Char('p') | KeyCode::Char('P') if !game.over => paused = !paused,
                KeyCode::Enter if game.over => {
                    game = new_game();
                    paused = false;
                }
                _ if !paused && !game.over => handle_move(key, &mut game),
                _ => {}
            },
            _ => {}
        }
    }
    Ok(())
}

fn handle_move(key: KeyEvent, game: &mut Game<StdRng>) {
    match key.code {
        KeyCode::Left => {
            game.move_left();
        }
        KeyCode::Right => {
            game.move_right();
        }
        KeyCode::Down => {
            game.soft_drop();
        }
        KeyCode::Up | KeyCode::Char('x') | KeyCode::Char('X') => game.rotate(1),
        KeyCode::Char('z') | KeyCode::Char('Z') => game.rotate(-1),
        KeyCode::Char(' ') => game.hard_drop(),
        _ => {}
    }
}

fn new_game() -> Game<StdRng> {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1);
    Game::new(StdRng::seed_from_u64(seed))
}

fn draw(f: &mut Frame, game: &Game<StdRng>, paused: bool) {
    let size = f.area();
    let min_width = WIDTH as u16 + 2 + 24;
    let min_height = HEIGHT as u16 + 2;
    if size.width < min_width || size.height < min_height {
        f.render_widget(
            Paragraph::new(format!("terminal too small, need {min_width}x{min_height}"))
                .block(Block::bordered().title("Tetris")),
            size,
        );
        return;
    }

    // The playfield is exactly the grid plus its border; center it vertically.
    let content_height = (HEIGHT + 2) as u16;
    let content = Rect {
        x: size.x,
        y: size.y + size.height.saturating_sub(content_height) / 2,
        width: size.width,
        height: content_height,
    };
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(WIDTH as u16 + 2), Constraint::Min(22)])
        .split(content);
    let board = layout[0];
    let side = layout[1];

    draw_board(f, game, board);
    draw_side(f, game, side);
    if game.over {
        draw_overlay(f, "GAME OVER", "press enter to restart");
    } else if paused {
        draw_overlay(f, "PAUSED", "press p to resume");
    }
}

fn shape_color(shape: Shape) -> Color {
    match shape {
        Shape::I => Color::Cyan,
        Shape::O => Color::Yellow,
        Shape::T => Color::Magenta,
        Shape::S => Color::Green,
        Shape::Z => Color::Red,
        Shape::J => Color::Blue,
        Shape::L => Color::LightYellow,
    }
}

fn draw_board(f: &mut Frame, game: &Game<StdRng>, area: Rect) {
    let inner = area.inner(Margin { horizontal: 1, vertical: 1 });
    let buffer = f.buffer_mut();

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if let Some(shape) = game.board.get(x, y) {
                let cell = buffer.cell_mut((x as u16 + inner.x, y as u16 + inner.y)).unwrap();
                cell.set_char('█');
                cell.set_style(Style::default().fg(shape_color(shape)));
            }
        }
    }

    if !game.over {
        let ghost = game.ghost_y();
        if ghost > game.current.y {
            let ghost_piece = game::Piece { y: ghost, ..game.current };
            for (x, y) in game.piece_cells(&ghost_piece) {
                if y < 0 {
                    continue;
                }
                let cell = buffer.cell_mut((x as u16 + inner.x, y as u16 + inner.y)).unwrap();
                cell.set_char('▒');
                cell.set_style(Style::default().fg(shape_color(game.current.shape)));
            }
        }
        for (x, y) in game.piece_cells(&game.current) {
            if y < 0 {
                continue;
            }
            let cell = buffer.cell_mut((x as u16 + inner.x, y as u16 + inner.y)).unwrap();
            cell.set_char('█');
            cell.set_style(
                Style::default()
                    .fg(shape_color(game.current.shape))
                    .add_modifier(Modifier::BOLD),
            );
        }
    }

    f.render_widget(
        Block::bordered()
            .title(Line::from(Span::styled(" TETRIS ", Style::default().bold()))),
        area,
    );
}

fn draw_side(f: &mut Frame, game: &Game<StdRng>, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Length(5), Constraint::Min(10)])
        .split(area);
    let next = layout[0];
    let stats = layout[1];
    let controls = layout[2];

    f.render_widget(Block::bordered().title(" Next "), next);
    f.render_widget(
        Paragraph::new(next_piece_lines(game.next)).alignment(Alignment::Center),
        next.inner(Margin { horizontal: 0, vertical: 0 }),
    );

    let stats_text = Paragraph::new(vec![
        Line::from(format!("Score:  {}", game.score)),
        Line::from(format!("Level:  {}", game.level)),
        Line::from(format!("Lines:  {}", game.lines)),
    ]);
    f.render_widget(Block::bordered().title(" Stats "), stats);
    f.render_widget(
        stats_text,
        stats.inner(Margin { horizontal: 1, vertical: 0 }),
    );

    let controls_text =
        Paragraph::new(CONTROLS.iter().map(|line| Line::from(*line)).collect::<Vec<_>>());
    f.render_widget(Block::bordered().title(" Controls "), controls);
    f.render_widget(
        controls_text,
        controls.inner(Margin { horizontal: 1, vertical: 0 }),
    );
}

fn next_piece_lines(shape: Shape) -> Vec<Line<'static>> {
    // Pre-pad to the max piece width so `insert` stays on a char boundary.
    let mut rows: Vec<String> = (0..4).map(|_| "    ".to_string()).collect();
    for (x, y) in shape.cells(0) {
        rows[*y as usize].insert(*x as usize, '#');
    }
    rows.into_iter()
        .map(|row| {
            let trimmed: String = row.chars().filter(|c| *c != ' ').collect();
            Line::from(if trimmed.is_empty() { " ".to_string() } else { trimmed })
        })
        .collect()
}

fn draw_overlay(f: &mut Frame, title: &str, hint: &str) {
    let area = f.area();
    let rect = Rect {
        x: area.x + area.width / 2 - 16,
        y: area.y + area.height / 2 - 2,
        width: 32,
        height: 4,
    };
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(rect);
    let top = layout[0];
    let bottom = layout[1];
    f.render_widget(
        Block::bordered()
            .style(Style::default().bg(Color::Black))
            .title(title),
        top,
    );
    f.render_widget(
        Paragraph::new(hint).alignment(Alignment::Center),
        bottom,
    );
}
