use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use rand::seq::SliceRandom;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};
use std::io::Stdout;
use std::time::{Duration, Instant};

const W: usize = 10; // board width (columns)
const H: usize = 20; // board height (rows)

/// A single tetromino type.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Piece {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl Piece {
    fn all() -> Vec<Piece> {
        vec![Piece::I, Piece::O, Piece::T, Piece::S, Piece::Z, Piece::J, Piece::L]
    }

    fn color(&self) -> Color {
        match self {
            Piece::I => Color::Cyan,
            Piece::O => Color::Yellow,
            Piece::T => Color::Magenta,
            Piece::S => Color::Green,
            Piece::Z => Color::Red,
            Piece::J => Color::Blue,
            Piece::L => Color::Rgb(255, 140, 0),
        }
    }

    /// Size of the bounding box this piece rotates within.
    fn box_size(&self) -> usize {
        match self {
            Piece::I => 4,
            Piece::O => 2,
            _ => 3,
        }
    }

    /// Spawn (rotation 0) cells as (row, col) within its bounding box.
    fn base_cells(&self) -> Vec<(usize, usize)> {
        match self {
            Piece::I => vec![(1, 0), (1, 1), (1, 2), (1, 3)],
            Piece::O => vec![(0, 0), (0, 1), (1, 0), (1, 1)],
            Piece::T => vec![(0, 1), (1, 0), (1, 1), (1, 2)],
            Piece::S => vec![(0, 1), (0, 2), (1, 0), (1, 1)],
            Piece::Z => vec![(0, 0), (0, 1), (1, 1), (1, 2)],
            Piece::J => vec![(0, 0), (1, 0), (1, 1), (1, 2)],
            Piece::L => vec![(0, 2), (1, 0), (1, 1), (1, 2)],
        }
    }

    /// All four clockwise rotation states, computed from the base shape.
    fn rotations(&self) -> Vec<Vec<(usize, usize)>> {
        let n = self.box_size();
        let mut states = vec![self.base_cells()];
        for _ in 1..4 {
            let prev = &states[states.len() - 1];
            // Clockwise rotation around the box center: (r, c) -> (c, n-1-r)
            let rotated: Vec<(usize, usize)> = prev
                .iter()
                .map(|&(r, c)| (c, n - 1 - r))
                .collect();
            states.push(rotated);
        }
        states
    }
}

/// The active (falling) piece: type + anchor position + rotation state.
#[derive(Clone)]
struct Active {
    kind: Piece,
    row: i32,
    col: i32,
    rot: usize,
    states: Vec<Vec<(usize, usize)>>,
}

impl Active {
    fn cells(&self) -> Vec<(i32, i32)> {
        self.states[self.rot]
            .iter()
            .map(|&(r, c)| (self.row + r as i32, self.col + c as i32))
            .collect()
    }
}

struct Game {
    grid: Vec<Vec<Option<Color>>>,
    current: Option<Active>,
    next: Piece,
    bag: Vec<Piece>,
    rng: rand::rngs::ThreadRng,
    score: i32,
    best: i32,
    level: i32,
    lines: i32,
    paused: bool,
    over: bool,
    quit: bool,
}

impl Game {
    fn new(best: i32) -> Self {
        let grid = vec![vec![None; W]; H];
        let mut g = Game {
            grid,
            current: None,
            next: Piece::O,
            bag: Vec::new(),
            rng: rand::thread_rng(),
            score: 0,
            best,
            level: 1,
            lines: 0,
            paused: false,
            over: false,
            quit: false,
        };
        g.spawn();
        g
    }

    /// Draw from the 7-bag, refilling as needed.
    fn draw_piece(&mut self) -> Piece {
        if self.bag.is_empty() {
            self.bag = Piece::all();
            self.bag.shuffle(&mut self.rng);
        }
        self.bag.pop().unwrap()
    }

    fn spawn(&mut self) {
        let kind = self.draw_piece();
        let states = kind.rotations();
        let col = (W as i32 - kind.box_size() as i32) / 2;
        let active = Active {
            kind,
            row: -1,
            col,
            rot: 0,
            states,
        };
        self.next = self.draw_piece();
        if self.collides(&active) {
            self.over = true;
        }
        self.current = Some(active);
    }

    fn collides(&self, a: &Active) -> bool {
        for (r, c) in a.cells() {
            if c < 0 || c >= W as i32 {
                return true;
            }
            if r >= H as i32 {
                return true;
            }
            if r >= 0 && self.grid[r as usize][c as usize].is_some() {
                return true;
            }
        }
        false
    }

    fn try_move(&mut self, dr: i32, dc: i32) -> bool {
        if let Some(mut a) = self.current.clone() {
            a.row += dr;
            a.col += dc;
            if !self.collides(&a) {
                self.current = Some(a);
                return true;
            }
        }
        false
    }

    fn collides_shifted(&self, a: &Active, dr: i32) -> bool {
        for (r, c) in a.cells() {
            let nr = r + dr;
            if c < 0 || c >= W as i32 {
                return true;
            }
            if nr >= H as i32 {
                return true;
            }
            if nr >= 0 && self.grid[nr as usize][c as usize].is_some() {
                return true;
            }
        }
        false
    }

    /// Try rotating with a small set of wall-kick offsets.
    fn try_rotate(&mut self, cw: bool) {
        if let Some(a) = self.current.clone() {
            let n = 4;
            let new_rot = if cw { (a.rot + 1) % n } else { (a.rot + n - 1) % n };
            let kicks: [(i32, i32); 7] = [
                (0, 0),
                (0, -1),
                (0, 1),
                (-1, 0),
                (0, -2),
                (0, 2),
                (1, 0),
            ];
            let mut candidate = a.clone();
            candidate.rot = new_rot;
            for (kr, kc) in kicks {
                let mut c = candidate.clone();
                c.row += kr;
                c.col += kc;
                if !self.collides(&c) {
                    self.current = Some(c);
                    return;
                }
            }
        }
    }

    fn drop_distance(&self) -> i32 {
        let mut dist = 0;
        if let Some(mut a) = self.current.clone() {
            while !self.collides_shifted(&a, 1) {
                a.row += 1;
                dist += 1;
            }
        }
        dist
    }

    fn lock(&mut self) {
        if let Some(a) = self.current.clone() {
            for (r, c) in a.cells() {
                if r < 0 {
                    self.over = true;
                    continue;
                }
                self.grid[r as usize][c as usize] = Some(a.kind.color());
            }
            self.clear_lines();
            self.current = None;
            if !self.over {
                self.spawn();
            }
        }
    }

    fn clear_lines(&mut self) {
        let mut cleared = 0;
        let mut kept = Vec::new();
        for row in self.grid.iter() {
            if row.iter().all(|c| c.is_some()) {
                cleared += 1;
            } else {
                kept.push(row.clone());
            }
        }
        while kept.len() < H {
            kept.insert(0, vec![None; W]);
        }
        self.grid = kept;

        if cleared > 0 {
            let base = [0, 100, 300, 500, 800];
            self.score += base[cleared.min(4)] * self.level;
            self.lines += cleared as i32;
            self.level = 1 + self.lines / 10;
        }
    }

    fn hard_drop(&mut self) {
        if self.paused || self.over {
            return;
        }
        if let Some(mut a) = self.current.clone() {
            let dist = self.drop_distance();
            self.score += 2 * dist as i32;
            a.row += dist;
            self.current = Some(a);
            self.lock();
        }
    }

    fn soft_drop(&mut self) {
        if self.paused || self.over {
            return;
        }
        if self.try_move(1, 0) {
            self.score += 1;
        } else {
            self.lock();
        }
    }

    fn tick(&mut self) {
        if self.paused || self.over || self.quit {
            return;
        }
        if !self.try_move(1, 0) {
            self.lock();
        }
    }

    fn reset(&mut self) {
        let best = self.best.max(self.score);
        *self = Game::new(best);
    }

    fn ghost_cells(&self) -> Vec<(i32, i32)> {
        if let Some(a) = self.current.as_ref() {
            let dist = self.drop_distance();
            return a.cells()
                .into_iter()
                .map(|(r, c)| (r + dist, c))
                .collect();
        }
        Vec::new()
    }

    fn tick_ms(level: i32) -> u64 {
        let lvl = level.max(1) as u64;
        (800u64.saturating_sub((lvl - 1) * 50)).max(60)
    }
}

fn render(f: &mut Frame, game: &Game) {
    let area = f.area();

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Length(20),
                Constraint::Min(0),
                Constraint::Length(20),
            ]
            .as_ref(),
        )
        .split(area);

    let left = cols[0];
    let middle = cols[1];
    let right = cols[2];

    let bw = W as u16 + 2;
    let bh = H as u16 + 2;
    let board_x = middle.x + middle.width.saturating_sub(bw) / 2;
    let board_y = middle.y + middle.height.saturating_sub(bh) / 2;
    let board = Rect::new(board_x, board_y, bw, bh);

    // Clear board interior.
    let interior = Rect::new(board.x + 1, board.y + 1, W as u16, H as u16);
    f.render_widget(Clear, interior);

    // Settled cells.
    for r in 0..H {
        for c in 0..W {
            if let Some(color) = game.grid[r][c] {
                f.buffer_mut().set_string(
                    board.x + 1 + c as u16,
                    board.y + 1 + r as u16,
                    "█",
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                );
            }
        }
    }

    // Ghost piece.
    let ghost_color = game
        .current
        .as_ref()
        .map(|a| a.kind.color())
        .unwrap_or(Color::Gray);
    for (r, c) in game.ghost_cells() {
        if r >= 0 && r < H as i32 && c >= 0 && c < W as i32 {
            f.buffer_mut().set_string(
                board.x + 1 + c as u16,
                board.y + 1 + r as u16,
                "▒",
                Style::default().fg(ghost_color),
            );
        }
    }

    // Active piece (drawn over the ghost).
    if let Some(a) = &game.current {
        for (r, c) in a.cells() {
            if r >= 0 && r < H as i32 && c >= 0 && c < W as i32 {
                f.buffer_mut().set_string(
                    board.x + 1 + c as u16,
                    board.y + 1 + r as u16,
                    "█",
                    Style::default()
                        .fg(a.kind.color())
                        .add_modifier(Modifier::BOLD),
                );
            }
        }
    }

    // Board frame.
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" TETRIS "),
        board,
    );

    // Overlays.
    let overlay: Option<String> = if game.over {
        Some("GAME OVER\nPress R to restart\nQ to quit".to_string())
    } else if game.paused {
        Some("PAUSED\nPress P to resume".to_string())
    } else {
        None
    };
    if let Some(text) = overlay {
        let w = 22u16;
        let h = 5u16;
        let ox = board.x + (board.width.saturating_sub(w)) / 2;
        let oy = board.y + (board.height.saturating_sub(h)) / 2;
        let p = Paragraph::new(
            text.lines()
                .map(|l| Line::from(Span::styled(
                    l,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )))
                .collect::<Vec<_>>(),
        )
        .centered()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black).fg(Color::White)),
        )
        .style(Style::default().bg(Color::Black));
        f.render_widget(Clear, Rect::new(ox, oy, w, h));
        f.render_widget(p, Rect::new(ox, oy, w, h));
    }

    // --- Left panel: Next + controls ---
    let lv = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Min(0)].as_ref())
        .split(left);

    f.render_widget(
        Block::default().borders(Borders::ALL).title(" NEXT "),
        lv[0],
    );
    let next = game.next;
    let states = next.rotations();
    let cells = &states[0];
    let bs = next.box_size();
    let off = (4 - bs as i32) / 2;
    for r in 0..4 {
        for c in 0..4 {
            let dr = r as i32 - off;
            let dc = c as i32 - off;
            let filled = cells.iter().any(|&(pr, pc)| pr as i32 == dr && pc as i32 == dc);
            let ch = if filled { "█" } else { " " };
            f.buffer_mut().set_string(
                lv[0].x + 1 + c as u16,
                lv[0].y + 1 + r as u16,
                ch,
                Style::default()
                    .fg(if filled { next.color() } else { Color::Reset })
                    .add_modifier(if filled { Modifier::BOLD } else { Modifier::empty() }),
            );
        }
    }

    let controls = Paragraph::new(vec![
        Line::from(Span::styled(
            "CONTROLS",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("←/→   move"),
        Line::from("↑/x   rotate cw"),
        Line::from("z     rotate ccw"),
        Line::from("↓     soft drop"),
        Line::from("space hard drop"),
        Line::from("p     pause"),
        Line::from("r     restart"),
        Line::from("q     quit"),
    ])
    .block(Block::default().borders(Borders::ALL).title(" CONTROLS "));
    f.render_widget(controls, lv[1]);

    // --- Right panel: stats ---
    let rv = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(right);

    let stats: [(&str, i32); 4] = [
        ("SCORE", game.score),
        ("LEVEL", game.level),
        ("LINES", game.lines),
        ("BEST", game.best.max(game.score)),
    ];
    for (i, (label, val)) in stats.iter().enumerate() {
        let p = Paragraph::new(Line::from(vec![
            Span::styled(format!("{label:<6}"), Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{val}"),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
        ]))
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(p, rv[i]);
    }
}

fn main() -> std::io::Result<()> {
    let mut stdout = std::io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;
    terminal.clear()?;

    let result = run(&mut terminal);

    terminal::disable_raw_mode()?;
    std::io::stdout().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run(terminal: &mut Terminal<ratatui::backend::CrosstermBackend<Stdout>>) -> std::io::Result<()> {
    let mut game = Game::new(0);
    let mut next_gravity =
        Instant::now() + Duration::from_millis(Game::tick_ms(game.level));

    loop {
        if game.quit {
            break;
        }

        // Gravity: drop on schedule.
        let now = Instant::now();
        if now >= next_gravity {
            game.tick();
            next_gravity = now + Duration::from_millis(Game::tick_ms(game.level));
        }

        // Poll for input without blocking past the next gravity tick.
        let wait = (next_gravity - Instant::now()).max(Duration::from_millis(1));
        if event::poll(wait)? {
            if let Event::Key(key) = event::read()? {
                handle_key(key, &mut game);
            }
        }

        terminal.draw(|f| render(f, &game))?;
    }
    Ok(())
}

fn handle_key(key: crossterm::event::KeyEvent, game: &mut Game) {
    let press_or_repeat = matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat);
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => game.quit = true,
        KeyCode::Char('p') if key.kind == KeyEventKind::Press => game.paused = !game.paused,
        KeyCode::Char('r') | KeyCode::Char('R') if key.kind == KeyEventKind::Press => {
            if game.over {
                game.reset();
            }
        }
        KeyCode::Left if press_or_repeat => {
            if !game.paused && !game.over {
                game.try_move(0, -1);
            }
        }
        KeyCode::Right if press_or_repeat => {
            if !game.paused && !game.over {
                game.try_move(0, 1);
            }
        }
        KeyCode::Down if press_or_repeat => game.soft_drop(),
        KeyCode::Up | KeyCode::Char('x') | KeyCode::Char('X') if key.kind == KeyEventKind::Press => {
            if !game.paused && !game.over {
                game.try_rotate(true);
            }
        }
        KeyCode::Char('z') | KeyCode::Char('Z') if key.kind == KeyEventKind::Press => {
            if !game.paused && !game.over {
                game.try_rotate(false);
            }
        }
        KeyCode::Char(' ') if key.kind == KeyEventKind::Press => game.hard_drop(),
        _ => {}
    }
}
