use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::{rngs::ThreadRng, Rng};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Terminal,
};
use std::{
    io::{self, Stdout},
    time::Duration,
};

const COLS: usize = 10;
const ROWS: usize = 20;

type B = CrosstermBackend<Stdout>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Cell {
    Empty,
    Filled(Color),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

const KINDS: [Kind; 7] = [Kind::I, Kind::O, Kind::T, Kind::S, Kind::Z, Kind::J, Kind::L];

fn piece_color(k: Kind) -> Color {
    match k {
        Kind::I => Color::Cyan,
        Kind::O => Color::Yellow,
        Kind::T => Color::Magenta,
        Kind::S => Color::Green,
        Kind::Z => Color::Red,
        Kind::J => Color::Blue,
        Kind::L => Color::Rgb(255, 140, 0),
    }
}

fn ghost_color(k: Kind) -> Color {
    match k {
        Kind::I => Color::DarkGray,
        _ => Color::DarkGray,
    }
}

#[derive(Clone, Copy, Debug)]
struct Piece {
    kind: Kind,
    x: i32,
    y: i32,
    rot: u8,
}

fn shape(k: Kind, rot: u8) -> [(i32, i32); 4] {
    let shapes: [[[(i32, i32); 4]; 4]; 7] = [
        [
            [(0, 1), (1, 1), (2, 1), (3, 1)],
            [(2, 0), (2, 1), (2, 2), (2, 3)],
            [(0, 2), (1, 2), (2, 2), (3, 2)],
            [(1, 0), (1, 1), (1, 2), (1, 3)],
        ],
        [
            [(1, 0), (2, 0), (1, 1), (2, 1)],
            [(1, 0), (2, 0), (1, 1), (2, 1)],
            [(1, 0), (2, 0), (1, 1), (2, 1)],
            [(1, 0), (2, 0), (1, 1), (2, 1)],
        ],
        [
            [(1, 0), (0, 1), (1, 1), (2, 1)],
            [(1, 0), (1, 1), (2, 1), (1, 2)],
            [(0, 1), (1, 1), (2, 1), (1, 2)],
            [(1, 0), (0, 1), (1, 1), (1, 2)],
        ],
        [
            [(1, 0), (2, 0), (0, 1), (1, 1)],
            [(1, 0), (1, 1), (2, 1), (2, 2)],
            [(1, 1), (2, 1), (0, 2), (1, 2)],
            [(0, 0), (0, 1), (1, 1), (1, 2)],
        ],
        [
            [(0, 0), (1, 0), (1, 1), (2, 1)],
            [(2, 0), (1, 1), (2, 1), (1, 2)],
            [(0, 1), (1, 1), (1, 2), (2, 2)],
            [(1, 0), (0, 1), (1, 1), (0, 2)],
        ],
        [
            [(0, 0), (0, 1), (1, 1), (2, 1)],
            [(1, 0), (2, 0), (1, 1), (1, 2)],
            [(0, 1), (1, 1), (2, 1), (2, 2)],
            [(1, 0), (1, 1), (0, 2), (1, 2)],
        ],
        [
            [(2, 0), (0, 1), (1, 1), (2, 1)],
            [(1, 0), (1, 1), (1, 2), (2, 2)],
            [(0, 1), (1, 1), (2, 1), (0, 2)],
            [(0, 0), (1, 0), (1, 1), (1, 2)],
        ],
    ];
    let idx = match k {
        Kind::I => 0,
        Kind::O => 1,
        Kind::T => 2,
        Kind::S => 3,
        Kind::Z => 4,
        Kind::J => 5,
        Kind::L => 6,
    };
    shapes[idx][rot as usize]
}

struct Game {
    board: Vec<Cell>,
    piece: Piece,
    next: Kind,
    score: u32,
    lines: u32,
    level: u32,
    over: bool,
    paused: bool,
    rng: ThreadRng,
}

impl Game {
    fn new() -> Self {
        let board = vec![Cell::Empty; COLS * ROWS];
        let mut rng = rand::thread_rng();
        let next = KINDS[rng.gen_range(0..7)];
        let piece = Self::spawn(next);
        Game {
            board,
            piece,
            next,
            score: 0,
            lines: 0,
            level: 1,
            over: false,
            paused: false,
            rng,
        }
    }

    fn spawn(kind: Kind) -> Piece {
        Piece { kind, x: 3, y: 0, rot: 0 }
    }

    fn cells(&self, p: &Piece) -> Vec<(usize, usize)> {
        shape(p.kind, p.rot)
            .iter()
            .map(|&(dx, dy)| (p.x + dx, p.y + dy))
            .filter(|&(cx, cy)| cx >= 0 && (cx as usize) < COLS && cy >= 0 && (cy as usize) < ROWS)
            .map(|(cx, cy)| (cx as usize, cy as usize))
            .collect()
    }

    fn valid(&self, p: &Piece) -> bool {
        for &(dx, dy) in &shape(p.kind, p.rot) {
            let cx = p.x + dx;
            let cy = p.y + dy;
            if cx < 0 || cx >= COLS as i32 || cy >= ROWS as i32 {
                return false;
            }
            if cy >= 0 && self.board[(cy as usize) * COLS + (cx as usize)] != Cell::Empty {
                return false;
            }
        }
        true
    }

    fn try_move(&mut self, dx: i32, dy: i32) -> bool {
        let np = Piece { x: self.piece.x + dx, y: self.piece.y + dy, ..self.piece };
        if self.valid(&np) {
            self.piece = np;
            true
        } else {
            false
        }
    }

    fn try_rotate(&mut self, dir: i32) {
        let nr = ((self.piece.rot as i32 + dir).rem_euclid(4)) as u8;
        let kx = [0, 1, -1, 2, -2];
        for dx in kx {
            let c = Piece { rot: nr, x: self.piece.x + dx, ..self.piece };
            if self.valid(&c) {
                self.piece = c;
                return;
            }
        }
    }

    fn lock(&mut self) {
        let color = piece_color(self.piece.kind);
        for (cx, cy) in self.cells(&self.piece) {
            self.board[cy * COLS + cx] = Cell::Filled(color);
        }
        self.clear_lines();
        self.next = KINDS[self.rng.gen_range(0..7)];
        let np = Self::spawn(self.next);
        if !self.valid(&np) {
            self.over = true;
        }
        self.piece = np;
    }

    fn clear_lines(&mut self) {
        let mut cleared = 0usize;
        let mut new_board = vec![Cell::Empty; COLS * ROWS];
        let mut write_row = ROWS - 1;
        for y in (0..ROWS).rev() {
            let full = (0..COLS).all(|x| self.board[y * COLS + x] != Cell::Empty);
            if full {
                cleared += 1;
            } else {
                for x in 0..COLS {
                    new_board[write_row * COLS + x] = self.board[y * COLS + x];
                }
                write_row -= 1;
            }
        }
        if cleared > 0 {
            let pts = match cleared {
                1 => 100,
                2 => 300,
                3 => 500,
                4 => 800,
                _ => 0,
            } * self.level as u32;
            self.score += pts;
            self.lines += cleared as u32;
            self.level = (self.lines / 10 + 1).min(20);
        }
        self.board = new_board;
    }

    fn gravity_interval(&self) -> Duration {
        let lvl = self.level.max(1) as u64;
        Duration::from_millis((1000u64 / lvl).max(60))
    }

    fn soft_drop(&mut self) {
        if self.over || self.paused {
            return;
        }
        if self.try_move(0, 1) {
            self.score += 1;
        }
    }

    fn hard_drop(&mut self) {
        if self.over || self.paused {
            return;
        }
        while self.try_move(0, 1) {}
        self.lock();
    }

    fn ghost_y(&self) -> i32 {
        let mut gy = self.piece.y;
        while self.valid(&Piece { y: gy + 1, ..self.piece }) {
            gy += 1;
        }
        gy
    }
}

fn board_lines(g: &Game) -> Vec<Line<'static>> {
    let ghost_y = if g.over { -100 } else { g.ghost_y() };
    let mut rows: Vec<Vec<Span<'static>>> = Vec::with_capacity(ROWS);
    for y in 0..ROWS {
        let mut row: Vec<Span<'static>> = Vec::with_capacity(COLS);
        for x in 0..COLS {
            let idx = y * COLS + x;
            let c = g.board[idx];
            let s = if c == Cell::Empty {
                Span::raw("  ")
            } else {
                if let Cell::Filled(color) = c {
                    Span::styled("██", Style::default().fg(color))
                } else {
                    Span::raw("  ")
                }
            };
            row.push(s);
        }
        rows.push(row);
    }
    if !g.over {
        let gc = ghost_color(g.piece.kind);
        for &(dx, dy) in &shape(g.piece.kind, g.piece.rot) {
            let cx = g.piece.x + dx;
            let cy = ghost_y + dy;
            if cx >= 0 && (cx as usize) < COLS && cy >= 0 && (cy as usize) < ROWS {
                let idx = (cy as usize) * COLS + (cx as usize);
                if g.board[idx] == Cell::Empty {
                    rows[cy as usize][cx as usize] =
                        Span::styled("░░", Style::default().fg(gc));
                }
            }
        }
        let ac = piece_color(g.piece.kind);
        for &(dx, dy) in &shape(g.piece.kind, g.piece.rot) {
            let cx = g.piece.x + dx;
            let cy = g.piece.y + dy;
            if cx >= 0 && (cx as usize) < COLS && cy >= 0 && (cy as usize) < ROWS {
                rows[cy as usize][cx as usize] = Span::styled(
                    "██",
                    Style::default().fg(ac).add_modifier(Modifier::BOLD),
                );
            }
        }
    }
    rows.into_iter().map(|row| Line::from(row)).collect()
}

fn next_lines(kind: Kind) -> Vec<Line<'static>> {
    let s = shape(kind, 0);
    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    for &(dx, dy) in &s {
        min_x = min_x.min(dx);
        max_x = max_x.max(dx);
        max_y = max_y.max(dy);
    }
    let w = (max_x - min_x + 1) as usize;
    let h = (max_y + 1) as usize;
    let mut rows: Vec<Vec<Span<'static>>> = Vec::with_capacity(h);
    for y in 0..h {
        let mut row: Vec<Span<'static>> = Vec::with_capacity(w);
        for x in 0..w {
            let hit = s.iter().any(|&(dx, dy)| dx - min_x == x as i32 && dy == y as i32);
            if hit {
                row.push(Span::styled(
                    "██",
                    Style::default().fg(piece_color(kind)).add_modifier(Modifier::BOLD),
                ));
            } else {
                row.push(Span::raw("  "));
            }
        }
        rows.push(row);
    }
    rows.into_iter().map(|r| Line::from(r)).collect()
}

fn draw(f: &mut ratatui::Frame, area: Rect, g: &Game) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Length(34)])
        .split(area);

    let board = Paragraph::new(board_lines(g)).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("Tetris  [L{}]", g.level)),
    );
    f.render_widget(board, layout[0]);

    let side = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Min(0),
        ])
        .split(layout[1]);

    let stats = Paragraph::new(vec![
        Line::from(format!("Score: {}", g.score)),
        Line::from(format!("Lines: {}", g.lines)),
        Line::from(format!("Level: {}", g.level)),
    ])
    .block(Block::default().borders(Borders::ALL).title("Stats"));
    f.render_widget(stats, side[0]);

    let next = Paragraph::new(next_lines(g.next)).block(
        Block::default().borders(Borders::ALL).title("Next"),
    );
    f.render_widget(next, side[1]);

    let help = Paragraph::new(vec![
        Line::from("Controls"),
        Line::from(""),
        Line::from("  Left/Right   move"),
        Line::from("  Up / X       rotate cw"),
        Line::from("  Z            rotate ccw"),
        Line::from("  Down         soft drop"),
        Line::from("  Space        hard drop"),
        Line::from("  P            pause"),
        Line::from("  R            restart"),
        Line::from("  Q / Ctrl-C   quit"),
    ])
    .block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(help, side[2]);

    if g.paused && !g.over {
        let c = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(3),
                Constraint::Fill(1),
            ])
            .split(area);
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  PAUSED  ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )))
            .alignment(ratatui::layout::Alignment::Center),
            c[1],
        );
    }
    if g.over {
        let c = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(4),
                Constraint::Fill(1),
            ])
            .split(area);
        f.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    "  GAME OVER  ",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Red)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(format!(" Score: {}  —  press R to restart", g.score)),
            ])
            .alignment(ratatui::layout::Alignment::Center),
            c[1],
        );
    }
}

fn setup_terminal() -> io::Result<Terminal<B>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn teardown_terminal(terminal: &mut Terminal<B>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn game_loop(terminal: &mut Terminal<B>) -> io::Result<()> {
    let mut g = Game::new();
    let frame_rate = Duration::from_millis(80);
    let mut acc = std::time::Instant::now();

    loop {
        terminal.draw(|f| {
            if g.over {
                f.render_widget(Clear, f.area());
            }
            draw(f, f.area(), &g);
        })?;

        let timeout = if g.paused || g.over {
            frame_rate
        } else {
            g.gravity_interval().min(frame_rate)
        };

        let now = std::time::Instant::now();
        let mut elapsed = now.duration_since(acc);
        acc = now;

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                let quit = (key.modifiers.contains(KeyModifiers::CONTROL)
                    && key.code == KeyCode::Char('c'))
                    || (!g.over && key.code == KeyCode::Char('q'));
                if quit {
                    return Ok(());
                }
                if g.over {
                    if key.code == KeyCode::Char('r') {
                        g = Game::new();
                        acc = std::time::Instant::now();
                    }
                    continue;
                }
                match key.code {
                    KeyCode::Char('p') => g.paused = !g.paused,
                    KeyCode::Char('r') => {
                        g = Game::new();
                        acc = std::time::Instant::now();
                    }
                    KeyCode::Char('z') => g.try_rotate(-1),
                    KeyCode::Up | KeyCode::Char('x') => g.try_rotate(1),
                    KeyCode::Left => {
                        g.try_move(-1, 0);
                        acc = std::time::Instant::now();
                    }
                    KeyCode::Right => {
                        g.try_move(1, 0);
                        acc = std::time::Instant::now();
                    }
                    KeyCode::Down => {
                        g.soft_drop();
                        acc = std::time::Instant::now();
                    }
                    KeyCode::Char(' ') => g.hard_drop(),
                    _ => {}
                }
            }
        } else {
            elapsed += timeout;
        }

        let interval = g.gravity_interval();
        while !g.over && !g.paused && elapsed >= interval {
            if !g.try_move(0, 1) {
                g.lock();
            }
            elapsed -= interval;
            if g.over {
                break;
            }
        }
    }
}

fn main() -> io::Result<()> {
    let mut t = setup_terminal()?;
    let r = game_loop(&mut t);
    teardown_terminal(&mut t)?;
    r
}
