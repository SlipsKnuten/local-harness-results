mod board;
mod game;
mod piece;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::time::Duration;

use crate::board::{Cell, HEIGHT, WIDTH};
use crate::game::Game;
use crate::piece::SHAPES;

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_game(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_game(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut game = Game::new();
    loop {
        let timeout = if game.is_paused || game.is_game_over {
            Duration::from_millis(100)
        } else {
            game.tick_interval()
        };

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if !handle_input(&mut game, key) {
                    break;
                }
            }
        } else if !game.is_paused && !game.is_game_over {
            game.tick();
        }

        terminal.draw(|f| draw(f, &game))?;
    }
    Ok(())
}

/// Handle a key event. Returns false when the game should quit.
fn handle_input(game: &mut Game, key: KeyEvent) -> bool {
    let is_press = key.kind == KeyEventKind::Press;
    let is_repeat = key.kind == KeyEventKind::Repeat;
    if !is_press && !is_repeat {
        return true;
    }

    match key.code {
        KeyCode::Char('q') => return false,
        KeyCode::Char('p') | KeyCode::Char('P') if is_press => {
            if !game.is_game_over {
                game.is_paused = !game.is_paused;
            }
        }
        KeyCode::Char('r') if is_press && game.is_game_over => {
            *game = Game::new();
        }
        KeyCode::Left => game.move_left(),
        KeyCode::Right => game.move_right(),
        KeyCode::Down => game.soft_drop(),
        KeyCode::Up if is_press => game.rotate(),
        KeyCode::Char(' ') if is_press => game.hard_drop(),
        _ => {}
    }
    true
}

fn draw(f: &mut Frame, game: &Game) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(26), Constraint::Min(16)])
        .split(size);

    draw_board(f, game, chunks[0]);
    draw_sidebar(f, game, chunks[1]);
}

fn draw_board(f: &mut Frame, game: &Game, area: Rect) {
    let current_cells: Vec<(i16, i16)> = game.current.absolute_cells().to_vec();
    let ghost_cells: Vec<(i16, i16)> = {
        let mut p = game.current;
        p.y = game.ghost_position();
        p.absolute_cells().to_vec()
    };

    let mut lines = Vec::with_capacity(HEIGHT as usize);
    for y in 0..HEIGHT {
        let mut spans = Vec::with_capacity(WIDTH as usize);
        for x in 0..WIDTH {
            let is_current = current_cells.contains(&(x, y));
            let is_ghost = ghost_cells.contains(&(x, y));

            let symbol = if is_current || matches!(game.board.get(x, y), Cell::Filled(_)) {
                "██"
            } else if is_ghost {
                "□□"
            } else {
                "  "
            };
            let style = if is_current {
                Style::default()
                    .fg(game.current.piece_type.color())
                    .add_modifier(Modifier::BOLD)
            } else if let Cell::Filled(pt) = game.board.get(x, y) {
                Style::default().fg(pt.color())
            } else if is_ghost {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };
            spans.push(Span::styled(symbol.to_string(), style));
        }
        lines.push(Line::from(spans));
    }

    let title = if game.is_game_over {
        " TETRIS - GAME OVER "
    } else if game.is_paused {
        " TETRIS - PAUSED "
    } else {
        " TETRIS "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));
    let paragraph = Paragraph::new(Text::from(lines)).block(block);
    f.render_widget(paragraph, area);

    if game.is_paused || game.is_game_over {
        render_centered_overlay(f, area, game);
    }
}

fn render_centered_overlay(f: &mut Frame, area: Rect, game: &Game) {
    let score_line = format!("Score: {}", game.score);
    let content: Vec<&str> = if game.is_game_over {
        vec!["GAME OVER", &score_line, "", "Press 'r' to restart"]
    } else {
        vec!["PAUSED", "", "Press 'p' to resume"]
    };
    let lines: Vec<Line> = content
        .iter()
        .map(|l| {
            Line::from(Span::styled(
                *l,
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ))
        })
        .collect();

    let width = lines.iter().map(|l| l.width()).max().unwrap_or(0) as u16 + 4;
    let height = lines.len() as u16 + 2;
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    let rect = Rect::new(x, y, width, height);

    let frame = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));
    f.render_widget(Clear, rect);
    f.render_widget(Paragraph::new(Text::from(lines)).block(frame), rect);
}

fn draw_sidebar(f: &mut Frame, game: &Game, area: Rect) {
    let vchunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Length(7), Constraint::Min(1)])
        .split(area);

    let info = Paragraph::new(vec![
        Line::from(format!(" Score:  {}", game.score)),
        Line::from(format!(" Level:  {}", game.level)),
        Line::from(format!(" Lines:  {}", game.lines)),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Info "));
    f.render_widget(info, vchunks[0]);

    draw_next(f, game, vchunks[1]);

    let controls = Paragraph::new(vec![
        Line::from(" ←/→    Move"),
        Line::from(" ↑      Rotate"),
        Line::from(" ↓      Soft drop"),
        Line::from(" Space  Hard drop"),
        Line::from(" p      Pause"),
        Line::from(" q      Quit"),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Controls "));
    f.render_widget(controls, vchunks[2]);
}

fn draw_next(f: &mut Frame, game: &Game, area: Rect) {
    let shape = SHAPES[game.next as usize][0];
    let mut lines = Vec::with_capacity(4);
    for cy in 0..4i16 {
        let mut spans = Vec::with_capacity(4);
        for cx in 0..4i16 {
            if shape.contains(&(cx, cy)) {
                spans.push(Span::styled(
                    "██",
                    Style::default().fg(game.next.color()).add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::raw("  "));
            }
        }
        lines.push(Line::from(spans));
    }
    let para = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title(" Next "));
    f.render_widget(para, area);
}
