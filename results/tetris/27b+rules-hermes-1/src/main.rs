//! Terminal UI and main loop for the Tetris game.

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use ratatui::Terminal;

mod game;
use game::{piece_cells, Game, PieceType, H, W};

/// Seconds per gravity step at a given level.
fn gravity_delay(level: u32) -> Duration {
    let ms = (500u32.saturating_sub((level - 1) * 40)).max(80);
    Duration::from_millis(ms as u64)
}

/// Color for each locked-cell id / piece type.
fn color_for(id: u8) -> Color {
    match id {
        1 => Color::Cyan,     // I
        2 => Color::Yellow,   // O
        3 => Color::Magenta,  // T
        4 => Color::Green,    // S
        5 => Color::Red,      // Z
        6 => Color::Blue,     // J
        7 => Color::LightRed, // L
        _ => Color::Reset,
    }
}

fn glyph_for(id: u8) -> &'static str {
    match id {
        0 => "·",
        1 => "I",
        2 => "O",
        3 => "T",
        4 => "S",
        5 => "Z",
        6 => "J",
        7 => "L",
        _ => " ",
    }
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let result = run(&mut terminal);

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    // Seed from the system time so every run shuffles the bag differently.
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9);
    let mut game = Game::new(seed);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| draw(f, &game))?;

        // Gravity, independent of the input-event rate.
        if !game.paused && !game.over && last_tick.elapsed() >= gravity_delay(game.level) {
            game.tick();
            last_tick = Instant::now();
        }

        let timeout = Duration::from_millis(20);
        if !event::poll(timeout)? {
            continue;
        }
        let event = event::read()?;
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('p') => game.paused = !game.paused,
                KeyCode::Char('r') => game = Game::new(seed),
                KeyCode::Left => game.move_left(),
                KeyCode::Right => game.move_right(),
                KeyCode::Up | KeyCode::Char('x') => game.rotate_cw(),
                KeyCode::Char('z') => game.rotate_ccw(),
                KeyCode::Down => {
                    game.soft_drop();
                    last_tick = Instant::now();
                }
                KeyCode::Char(' ') => {
                    game.hard_drop();
                    last_tick = Instant::now();
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn draw(f: &mut Frame, game: &Game) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(4),            // left spacer
            Constraint::Length(W as u16 * 2), // board (each cell is 2 chars wide)
            Constraint::Length(2),
            Constraint::Length(24), // sidebar
        ])
        .split(f.area());

    draw_board(f, chunks[1], game);
    draw_sidebar(f, chunks[3], game);
}

fn draw_board(f: &mut Frame, area: Rect, game: &Game) {
    // Board cells are 1 char wide; a 20-row board is 42 rows tall with the
    // border. If the terminal is shorter, ratatui clips rather than panics.
    let mut lines = Vec::with_capacity(H);
    for y in 0..H {
        let mut spans = Vec::with_capacity(W);
        for x in 0..W {
            let locked = game.board[y][x];
            let mut id = locked;
            let mut is_active = false;
            for &(cx, cy) in game.piece.cells().iter() {
                if cx == x as i16 && cy == y as i16 && !game.over {
                    id = game.piece.ty.id();
                    is_active = true;
                }
            }
            let fg = if id == 0 {
                Color::DarkGray
            } else {
                color_for(id)
            };
            spans.push(Span::styled(
                format!("{} ", glyph_for(id)),
                Style::default().fg(fg).add_modifier(if is_active {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
            ));
        }
        lines.push(Line::from(spans));
    }
    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Tetris ")),
        area,
    );
}

fn draw_sidebar(f: &mut Frame, area: Rect, game: &Game) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // next piece
            Constraint::Length(8), // score / lines / level
            Constraint::Min(4),    // controls
        ])
        .split(area);

    // Next piece.
    let mut lines = vec![Line::from(Span::styled(
        "NEXT",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    ))];
    let next_fg = color_for(game.next.id());
    for line in grid_of(game.next).iter() {
        lines.push(Line::from(
            line.iter()
                .map(|ch| {
                    if *ch {
                        Span::styled("██ ", Style::default().fg(Color::White).bg(next_fg))
                    } else {
                        Span::raw("   ")
                    }
                })
                .collect::<Vec<Span>>(),
        ));
    }
    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Next ")),
        vertical[0],
    );

    // Stats.
    let stats = vec![
        Line::from(Span::styled(
            "SCORE",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(format!("  {}", game.score), Color::Yellow)),
        Line::from(""),
        Line::from(Span::styled(
            "LINES",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(format!("  {}", game.lines), Color::Green)),
        Line::from(""),
        Line::from(Span::styled(
            "LEVEL",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(format!("  {}", game.level), Color::Magenta)),
    ];
    f.render_widget(
        Paragraph::new(stats).block(Block::default().borders(Borders::ALL).title(" Stats ")),
        vertical[1],
    );

    // Controls and status.
    let status = if game.over {
        (String::from("GAME OVER — press r to restart"), Color::Red)
    } else if game.paused {
        (String::from("PAUSED — press p to resume"), Color::Yellow)
    } else {
        (String::from("playing"), Color::Gray)
    };
    let controls = vec![
        Line::from(Span::styled("← →  move", Color::Gray)),
        Line::from(Span::styled("↑ / x  rotate cw", Color::Gray)),
        Line::from(Span::styled("z  rotate ccw", Color::Gray)),
        Line::from(Span::styled("↓  soft drop", Color::Gray)),
        Line::from(Span::styled("space  hard drop", Color::Gray)),
        Line::from(Span::styled("p pause · r restart · q quit", Color::Gray)),
        Line::from(Span::styled(
            format!("status: {}", status.0),
            style_with_bold(status.1),
        )),
    ];
    f.render_widget(
        Paragraph::new(controls)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(" Controls ")),
        vertical[2],
    );
}

fn style_with_bold(c: Color) -> Style {
    Style::default().fg(c).add_modifier(Modifier::BOLD)
}

/// A 4x4 grid (row-major, top to bottom) marking the cells of the piece.
fn grid_of(ty: PieceType) -> [[bool; 4]; 4] {
    let mut grid = [[false; 4]; 4];
    for &(cx, cy) in piece_cells(ty, 0).iter() {
        grid[cy as usize][cx as usize] = true;
    }
    grid
}
