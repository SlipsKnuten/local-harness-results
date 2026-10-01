use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph, Widget};
use ratatui::DefaultTerminal;

const COLS: usize = 10;
const ROWS: usize = 20;
const TICK_MS: u64 = 1000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Cell {
    Empty,
    Filled(u8),
}

impl Cell {
    fn color(self) -> Option<Color> {
        match self {
            Cell::Empty => None,
            Cell::Filled(c) => match c {
                0 => Some(Color::Cyan),
                1 => Some(Color::Yellow),
                2 => Some(Color::Magenta),
                3 => Some(Color::Green),
                4 => Some(Color::Red),
                5 => Some(Color::Blue),
                _ => Some(Color::White),
            },
        }
    }
}

#[derive(Clone, Copy)]
struct Piece {
    kind: u8,
    x: isize,
    y: isize,
    rot: u8,
}

const SHAPES: [[[(i8, i8); 4]; 4]; 7] = [
    [
        [(0, 0), (1, 0), (2, 0), (3, 0)],
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

struct Game {
    board: Vec<Cell>,
    piece: Option<Piece>,
    bag: VecDeque<u8>,
    next: u8,
    score: u32,
    lines: u32,
    level: u32,
    over: bool,
    paused: bool,
}

impl Game {
    fn new() -> Self {
        let mut bag = VecDeque::new();
        for i in 0..7 {
            bag.push_back(i);
        }
        bag.shuffle(&mut rand::thread_rng());
        let next = bag.pop_front().unwrap_or(0);
        Self {
            board: vec![Cell::Empty; ROWS * COLS],
            piece: None,
            bag,
            next,
            score: 0,
            lines: 0,
            level: 1,
            over: false,
            paused: false,
        }
    }

    fn refill_bag(&mut self) {
        let mut fresh = (0..7).collect::<Vec<u8>>();
        fresh.shuffle(&mut rand::thread_rng());
        self.bag.extend(fresh);
    }

    fn spawn(&mut self) {
        if self.bag.is_empty() {
            self.refill_bag();
        }
        let kind = self.bag.pop_front().unwrap_or(0);
        let x = (COLS / 2) as isize - 2;
        let p = Piece { kind, x, y: 0, rot: 0 };
        if self.collides(&p) {
            self.over = true;
            self.piece = None;
            return;
        }
        self.piece = Some(p);
        if self.bag.is_empty() {
            self.refill_bag();
        }
        self.next = self.bag.front().copied().unwrap_or(0);
    }

    fn idx(&self, x: isize, y: isize) -> Option<usize> {
        if x < 0 || y < 0 || (x as usize) >= COLS || (y as usize) >= ROWS {
            return None;
        }
        Some((y as usize) * COLS + (x as usize) as usize)
    }

    fn collides(&self, p: &Piece) -> bool {
        for (dx, dy) in SHAPES[p.kind as usize][p.rot as usize] {
            let x = p.x + dx;
            let y = p.y + dy;
            if x < 0 || (x as usize) >= COLS || (y as usize) >= ROWS {
                return true;
            }
            if y >= 0 && self.board[((y as usize) * COLS + x as usize)] != Cell::Empty {
                return true;
            }
        }
        false
    }

    fn try_move(&mut self, dx: isize, dy: isize) -> bool {
        let Some(mut p) = self.piece else { return false };
        p.x += dx;
        p.y += dy;
        if self.collides(&p) {
            return false;
        }
        self.piece = Some(p);
        true
    }

    fn try_rotate(&mut self, dir: i8) {
        let Some(mut p) = self.piece else { return };
        let new_rot = (p.rot as i8 + dir + 4) % 4;
        p.rot = new_rot as u8;
        for kick in [0isize, -1, 1, -2, 2] {
            let mut t = p;
            t.x += kick;
            if !self.collides(&t) {
                self.piece = Some(t);
                return;
            }
        }
    }

    fn drop(&mut self) {
        if !self.try_move(0, 1) {
            self.lock();
        }
    }

    fn hard_drop(&mut self) {
        let Some(p) = self.piece else { return };
        let mut d = 0;
        while !self.collides(&Piece { y: p.y + d + 1, ..p }) {
            d += 1;
        }
        self.score += 2 * d as u32;
        self.try_move(0, d);
        self.lock();
    }

    fn ghost_y(&self) -> isize {
        let Some(p) = self.piece else { return 0 };
        let mut d = 0;
        while !self.collides(&Piece { y: p.y + d + 1, ..p }) {
            d += 1;
        }
        p.y + d
    }

    fn lock(&mut self) {
        let Some(p) = self.piece else { return };
        for (dx, dy) in SHAPES[p.kind as usize][p.rot as usize] {
            let x = p.x + dx;
            let y = p.y + dy;
            if let Some(i) = self.idx(x, y) {
                self.board[i] = Cell::Filled(p.kind);
            }
        }
        self.clear_lines();
        self.piece = None;
        self.spawn();
    }

    fn clear_lines(&mut self) {
        let mut new_board: Vec<Cell> = Vec::with_capacity(ROWS * COLS);
        let mut cleared = 0;
        for r in 0..ROWS {
            let row_full = (0..COLS).all(|c| self.board[r * COLS + c] != Cell::Empty);
            if row_full {
                cleared += 1;
            } else {
                new_board.extend_from_slice(&self.board[r * COLS..r * COLS + COLS]);
            }
        }
        if cleared > 0 {
            let empty = vec![Cell::Empty; cleared * COLS];
            new_board.splice(0..0, empty);
            self.lines += cleared as u32;
            self.score += match cleared {
                1 => 100,
                2 => 300,
                3 => 500,
                4 => 800,
                _ => 0,
            } * self.level;
            self.level = (self.lines / 10 + 1) as u32;
            self.board = new_board;
        }
    }

    fn tick(&mut self) {
        if self.paused || self.over || self.piece.is_none() {
            return;
        }
        let speed_ms = (1000 - (self.level - 1) * 90).max(80);
        if speed_ms < TICK_MS {
            return;
        }
        self.drop();
    }
}

fn draw_board(f: &mut Frame, area: Rect, game: &Game) {
    let mut cells = vec![];
    let ghost_y = if game.piece.is_some() { Some(game.ghost_y()) } else { None };

    let mut ghost_cells: Vec<(isize, isize, u8)> = Vec::new();
    let mut live_cells: Vec<(isize, isize, u8)> = Vec::new();
    if let Some(p) = game.piece {
        for (dx, dy) in SHAPES[p.kind as usize][p.rot as usize] {
            live_cells.push((p.x + dx, p.y + dy, p.kind));
        }
        if let Some(gy) = ghost_y {
            for (dx, _) in SHAPES[p.kind as usize][p.rot as usize] {
                ghost_cells.push((p.x + dx, gy, p.kind));
            }
        }
    }

    let ghost_set: Vec<(isize, isize, u8)> = ghost_cells;

    for y in 0..ROWS {
        let mut line = String::new();
        for x in 0..COLS {
            let base = game.board[y * COLS + x];
            let is_live = live_cells.iter().any(|(lx, ly, _)| *lx == x as isize && *ly == y as isize);
            let is_ghost = !is_live && ghost_set.iter().any(|(gx, gy, _)| *gx == x as isize && *gy == y as isize);
            if is_live {
                line.push('█');
                cells.push((x, y, Color::Yellow));
            } else if is_ghost {
                line.push('░');
                cells.push((x, y, Color::DarkGray));
            } else if let Cell::Filled(_) = base {
                line.push('█');
                cells.push((x, y, base.color().unwrap_or(Color::White)));
            } else {
                line.push(' ');
            }
        }
        f.render_str(&line, area.x, area.y + y as u16, Style::default());
    }
    let _ = cells;
}

fn draw_panel_title(text: &str) -> Block {
    Block::default()
        .border_type(BorderType::Rounded)
        .title(Span::styled(
            format!(" {text} "),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ))
}

fn draw_next(f: &mut Frame, area: Rect, next: u8) {
    let inner = area.inner(Rect { x: 0, y: 0, width: area.width, height: area.height });
    let shape = SHAPES[next as usize][0];
    let xs: Vec<i8> = shape.iter().map(|s| s.0).collect();
    let ys: Vec<i8> = shape.iter().map(|s| s.1).collect();
    let min_x = *xs.iter().min().unwrap_or(&0);
    let max_x = *xs.iter().max().unwrap_or(&0);
    let min_y = *ys.iter().min().unwrap_or(&0);
    let max_y = *ys.iter().max().unwrap_or(&0);
    let w = (max_x - min_x + 1) as u16;
    let h = (max_y - min_y + 1) as u16;
    let ox = inner.x + (inner.width.saturating_sub(w)) / 2;
    let oy = inner.y + (inner.height.saturating_sub(h)) / 2;
    for (dx, dy) in shape {
        if ox + (dx - min_x) as u16 < area.x + area.width
            && oy + (dy - min_y) as u16 < area.y + area.height
        {
            f.render_str("██", ox + (dx - min_x) as u16, oy + (dy - min_y) as u16, Style::default().fg(Color::Cyan));
        }
    }
}

fn draw_stats(f: &mut Frame, area: Rect, game: &Game) {
    let lines = vec![
        Line::from(Span::styled("SCORE", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(
            format!("{: >10}", game.score),
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from(Span::styled("LINES", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(
            format!("{: >10}", game.lines),
            Style::default().fg(Color::Green),
        )),
        Line::from(""),
        Line::from(Span::styled("LEVEL", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(
            format!("{: >10}", game.level),
            Style::default().fg(Color::Magenta),
        )),
    ];
    let p = Paragraph::new(lines).style(Style::default().bg(Color::Reset));
    p.render(f, area);
}

fn draw_controls(f: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(Span::styled("CONTROLS", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("← / →   move"),
        Line::from("↑ / x   rotate cw"),
        Line::from("z       rotate ccw"),
        Line::from("↓       soft drop"),
        Line::from("space   hard drop"),
        Line::from("p       pause"),
        Line::from("r       restart"),
        Line::from("q / esc quit"),
    ];
    let p = Paragraph::new(lines);
    p.render(f, area);
}

fn draw_message(f: &mut Frame, area: Rect, text: &str, color: Color) {
    let p = Paragraph::new(Line::from(Span::styled(
        text.to_string(),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )))
    .alignment(Alignment::Center);
    p.render(f, area);
}

pub fn run() -> std::io::Result<()> {
    enable_raw_mode()?;
    std::io::stdout().execute(EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
    let mut terminal = DefaultTerminal::new(backend)?;

    let res = run_loop(&mut terminal);

    disable_raw_mode()?;
    std::io::stdout().execute(LeaveAlternateScreen)?;

    res
}

fn run_loop(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    let mut game = Game::new();
    game.spawn();
    let mut last_tick = Instant::now();
    let tick = Duration::from_millis(TICK_MS);

    loop {
        terminal.draw(|f| {
            let area = f.area();
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(28),
                    Constraint::Length(30),
                    Constraint::Length(24),
                ])
                .split(area);

            let board_area = chunks[1];
            f.render_widget(Clear, board_area);
            let title = Block::default()
                .border_type(BorderType::Thick)
                .title(Span::styled(
                    " TETRIS ",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ));
            let inner = title.inner(board_area);
            draw_board(f, inner, &game);
            f.render_widget(title, board_area);

            let left_inner = draw_panel_title("STATS").inner(chunks[0]);
            f.render_widget(draw_panel_title("STATS"), chunks[0]);
            draw_stats(f, left_inner, &game);

            let right_inner = draw_panel_title("NEXT").inner(chunks[2]);
            f.render_widget(draw_panel_title("NEXT"), chunks[2]);
            draw_next(f, right_inner, game.next);

            let ctrls = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(14)])
                .split(chunks[0]);
            f.render_widget(draw_panel_title("CONTROLS"), ctrls[1]);
            draw_controls(f, ctrls[1].inner(Rect { x: 1, y: 1, width: ctrls[1].width.saturating_sub(2), height: ctrls[1].height.saturating_sub(2) }), );

            if game.paused {
                let msg = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(1), Constraint::Min(0)])
                    .split(inner);
                draw_message(f, msg[1], " PAUSED ", Color::Yellow);
            }
            if game.over {
                let msg = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)])
                    .split(inner);
                draw_message(f, msg[1], " GAME OVER ", Color::Red);
                draw_message(f, msg[2], " press r to restart, q to quit ", Color::White);
            }
        })?;

        let instant = Instant::now();
        if instant >= last_tick + tick {
            game.tick();
            last_tick = instant;
        }

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('p') if !game.over => game.paused = !game.paused,
                    KeyCode::Char('r') => {
                        let g = Game::new();
                        game = g;
                        game.spawn();
                        last_tick = Instant::now();
                    }
                    _ if game.over => {}
                    _ if game.paused => {}
                    KeyCode::Char('c') | KeyCode::ArrowLeft if !game.try_move(-1, 0) => {}
                    KeyCode::Char('v') | KeyCode::ArrowRight if !game.try_move(1, 0) => {}
                    KeyCode::Char('x') | KeyCode::ArrowUp => game.try_rotate(1),
                    KeyCode::Char('z') => game.try_rotate(-1),
                    KeyCode::ArrowDown => {
                        if game.try_move(0, 1) {
                            game.score += 1;
                        }
                    }
                    KeyCode::Char(' ') => game.hard_drop(),
                    _ => {}
                }
            }
        }
    }
}
