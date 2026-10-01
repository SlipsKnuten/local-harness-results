mod game;
mod tetromino;

use std::io::{self, stdout};
use std::time::Instant;

use crossterm::execute;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};

use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Frame, Terminal};

use game::{Cell, Game, COLS, ROWS};
use tetromino::Tetromino;

const BG: Color = Color::Rgb(12, 14, 24);

/// How each board cell should be drawn.
#[derive(Clone, Copy)]
enum Disp {
    Empty,
    Ghost(Tetromino),
    Filled(Tetromino),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut game = Game::new();
    let mut paused = false;
    let result = run_loop(&mut terminal, &mut game, &mut paused);

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    game: &mut Game,
    paused: &mut bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut high: u32 = 0;
    let mut tick_rate = game.tick_duration();
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| draw(f, game, *paused, high))?;

        let elapsed = Instant::now().duration_since(last_tick);
        let timeout = tick_rate.saturating_sub(elapsed);
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('p') if !game.over => {
                            *paused = !*paused;
                            last_tick = Instant::now();
                        }
                        KeyCode::Char('r') if game.over => {
                            *game = Game::new();
                            *paused = false;
                            last_tick = Instant::now();
                        }
                        _ if !*paused && !game.over => match key.code {
                            KeyCode::Left => game.move_left(),
                            KeyCode::Right => game.move_right(),
                            KeyCode::Down => {
                                game.soft_drop();
                                last_tick = Instant::now();
                            }
                            KeyCode::Up | KeyCode::Char('x') => game.rotate_cw(),
                            KeyCode::Char('z') => game.rotate_ccw(),
                            KeyCode::Char(' ') => {
                                game.hard_drop();
                                last_tick = Instant::now();
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                    high = high.max(game.score);
                    tick_rate = game.tick_duration();
                }
                _ => {}
            }
        }

        // Apply gravity on the tick schedule (skipped while paused or over).
        if !*paused
            && !game.over
            && Instant::now().duration_since(last_tick) >= tick_rate
        {
            game.tick();
            last_tick = Instant::now();
            tick_rate = game.tick_duration();
        }
    }
    Ok(())
}

fn draw(f: &mut Frame, game: &Game, paused: bool, high: u32) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(18)])
        .split(area);

    draw_board(f, chunks[0], game, paused);
    draw_sidebar(f, chunks[1], game, high);
}

fn board_lines(game: &Game) -> Vec<Line<'static>> {
    let mut disp: Vec<Disp> = game
        .board
        .iter()
        .map(|c| match c {
            Cell::Empty => Disp::Empty,
            Cell::Filled(t) => Disp::Filled(*t),
        })
        .collect();

    // Ghost piece: where the active piece will land.
    let gy = game.drop_target_y();
    for (cx, cy) in game.current.kind.cells(game.current.rotation) {
        let bx = game.current.x + cx;
        let by = gy + cy;
        if bx >= 0 && bx < COLS as i32 && by >= 0 && by < ROWS as i32 {
            let idx = by as usize * COLS + bx as usize;
            if matches!(disp[idx], Disp::Empty) {
                disp[idx] = Disp::Ghost(game.current.kind);
            }
        }
    }
    // Active piece on top.
    for (cx, cy) in game.current.kind.cells(game.current.rotation) {
        let bx = game.current.x + cx;
        let by = game.current.y + cy;
        if bx >= 0 && bx < COLS as i32 && by >= 0 && by < ROWS as i32 {
            let idx = by as usize * COLS + bx as usize;
            disp[idx] = Disp::Filled(game.current.kind);
        }
    }

    let mut lines = Vec::with_capacity(ROWS);
    for row in 0..ROWS {
        let mut line = Line::default();
        for col in 0..COLS {
            line.spans.push(match disp[row * COLS + col] {
                Disp::Empty => Span::raw("  "),
                Disp::Ghost(k) => Span::styled("▒▒", Style::default().fg(k.color()).add_modifier(Modifier::DIM)),
                Disp::Filled(k) => Span::styled("██", Style::default().fg(k.color())),
            });
        }
        lines.push(line);
    }
    lines
}

fn draw_board(f: &mut Frame, area: Rect, game: &Game, paused: bool) {
    let title = if game.over {
        "TETRIS  ·  GAME OVER"
    } else if paused {
        "TETRIS  ·  PAUSED"
    } else {
        "TETRIS"
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
        .style(Style::default().bg(BG));
    f.render_widget(
        Paragraph::new(board_lines(game)).block(block).alignment(Alignment::Center),
        area,
    );

    if game.over {
        overlay(
            f,
            area,
            "GAME OVER",
            vec![
                Line::from(Span::styled(
                    format!("Score: {}", game.score),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    "[R] restart   [Q] quit",
                    Style::default().fg(Color::Gray),
                )),
            ],
        );
    } else if paused {
        overlay(
            f,
            area,
            "PAUSED",
            vec![Line::from(Span::styled(
                "[P] resume",
                Style::default().fg(Color::Gray),
            ))],
        );
    }
}

fn overlay(f: &mut Frame, area: Rect, title: &str, lines: Vec<Line<'static>>) {
    let len = (lines.len() + 2) as u16;
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(len)])
        .flex(Flex::Center);
    let rects = layout.split(area);
    let overlay_rect = rects[0];
    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .style(Style::default().bg(BG).fg(Color::White)),
            )
            .alignment(Alignment::Center),
        overlay_rect,
    );
}

/// Render the next piece centered in a 4x4 grid.
fn next_lines(kind: Tetromino) -> Vec<Line<'static>> {
    let size = kind.box_size() as usize;
    let ox = (4usize.saturating_sub(size)) / 2;
    let oy = (4usize.saturating_sub(size)) / 2;
    let cells = kind.cells(0);
    let color = kind.color();
    let mut lines = Vec::new();
    for y in 0..4 {
        let mut line = Line::default();
        for x in 0..4 {
            let filled = cells
                .iter()
                .any(|(cx, cy)| ox + *cx as usize == x && oy + *cy as usize == y);
            if filled {
                line.spans.push(Span::styled("██", Style::default().fg(color)));
            } else {
                line.spans.push(Span::raw("  "));
            }
        }
        lines.push(line);
    }
    lines
}

fn draw_sidebar(f: &mut Frame, area: Rect, game: &Game, high: u32) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Length(7), Constraint::Min(1)])
        .split(area);

    // Next piece.
    f.render_widget(
        Paragraph::new(next_lines(game.next))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("NEXT")
                    .style(Style::default().bg(BG)),
            )
            .alignment(Alignment::Center),
        chunks[0],
    );

    // Stats.
    let stats = vec![
        stat_line("SCORE", game.score.to_string(), Color::White),
        stat_line("HIGH", high.to_string(), Color::Cyan),
        stat_line("LEVEL", game.level.to_string(), Color::Yellow),
        stat_line("LINES", game.lines.to_string(), Color::Green),
    ];
    f.render_widget(
        Paragraph::new(stats)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("STATS")
                    .style(Style::default().bg(BG)),
            )
            .style(Style::default().fg(Color::White)),
        chunks[1],
    );

    // Controls.
    let controls = vec![
        Line::from(vec![key_span("←  →"), label_span("Move")]),
        Line::from(vec![key_span("↓"), label_span("Soft drop")]),
        Line::from(vec![key_span("↑ / X"), label_span("Rotate")]),
        Line::from(vec![key_span("Z"), label_span("Rotate CCW")]),
        Line::from(vec![key_span("SPACE"), label_span("Hard drop")]),
        Line::from(vec![key_span("P"), label_span("Pause")]),
        Line::from(vec![key_span("Q"), label_span("Quit")]),
    ];
    f.render_widget(
        Paragraph::new(controls)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("CONTROLS")
                    .style(Style::default().bg(BG)),
            )
            .style(Style::default().fg(Color::Gray)),
        chunks[2],
    );
}

fn key_span(k: &'static str) -> Span<'static> {
    Span::styled(k, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
}

fn label_span(l: &str) -> Span<'static> {
    Span::raw(format!("  {l:<12}"))
}

fn stat_line(label: &str, value: String, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<6}"), Style::default().fg(Color::Gray)),
        Span::styled(value, Style::default().fg(color).add_modifier(Modifier::BOLD)),
    ])
}
