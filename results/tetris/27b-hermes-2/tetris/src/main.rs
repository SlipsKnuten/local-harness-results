//! Tetris built with ratatui + crossterm.
//!
//! Controls:
//!   ← / →   move
//!   ↓        soft drop
//!   ↑ / x    rotate
//!   space    hard drop
//!   p        pause
//!   r        restart (on game over)
//!   q / esc  quit

use std::io::Stdout;
use std::time::{Duration, Instant};

use crossterm::{
    execute,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

// ---------------------------------------------------------------------------
// Board / pieces
// ---------------------------------------------------------------------------

const COLS: usize = 10;
const ROWS: usize = 20;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Kind {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl Kind {
    fn all() -> [Kind; 7] {
        [Kind::I, Kind::O, Kind::T, Kind::S, Kind::Z, Kind::J, Kind::L]
    }

    fn color(self) -> Color {
        match self {
            Kind::I => Color::Cyan,
            Kind::O => Color::Yellow,
            Kind::T => Color::Magenta,
            Kind::S => Color::Green,
            Kind::Z => Color::Red,
            Kind::J => Color::Blue,
            Kind::L => Color::Rgb(255, 165, 0),
        }
    }
}

/// The seven base shapes, each given as a small boolean grid.
fn base_grid(kind: Kind) -> Vec<Vec<bool>> {
    match kind {
        Kind::I => vec![
            vec![false, false, false, false],
            vec![true, true, true, true],
            vec![false, false, false, false],
            vec![false, false, false, false],
        ],
        Kind::O => vec![vec![true, true], vec![true, true]],
        Kind::T => vec![
            vec![false, true, false],
            vec![true, true, true],
            vec![false, false, false],
        ],
        Kind::S => vec![
            vec![false, true, true],
            vec![true, true, false],
            vec![false, false, false],
        ],
        Kind::Z => vec![
            vec![true, true, false],
            vec![false, true, true],
            vec![false, false, false],
        ],
        Kind::J => vec![
            vec![true, false, false],
            vec![true, true, true],
            vec![false, false, false],
        ],
        Kind::L => vec![
            vec![false, false, true],
            vec![true, true, true],
            vec![false, false, false],
        ],
    }
}

/// Rotate a grid 90° clockwise (transpose, then reverse each row).
fn rotate_cw(grid: &Vec<Vec<bool>>) -> Vec<Vec<bool>> {
    let rows = grid.len();
    let cols = grid[0].len();
    let mut out = vec![vec![false; rows]; cols];
    for (r, row) in grid.iter().enumerate() {
        for (c, &v) in row.iter().enumerate() {
            if v {
                out[c][rows - 1 - r] = true;
            }
        }
    }
    out
}

struct Piece {
    kind: Kind,
    grid: Vec<Vec<bool>>,
    x: i32, // left column of the grid on the board
    y: i32, // top row of the grid on the board
}

impl Piece {
    fn new(kind: Kind) -> Self {
        let grid = base_grid(kind);
        let w = grid[0].len() as i32;
        let x = (COLS as i32 - w) / 2;
        let y = 0; // spawn at the very top of the visible field
        Piece { kind, grid, x, y }
    }

    /// Board-space cell coordinates for every filled square of the piece.
    fn cells(&self) -> Vec<(i32, i32)> {
        let mut out = Vec::new();
        for (r, row) in self.grid.iter().enumerate() {
            for (c, &v) in row.iter().enumerate() {
                if v {
                    out.push((self.x + c as i32, self.y + r as i32));
                }
            }
        }
        out
    }

    /// Does the (possibly offset) piece collide with walls, the floor, or the stack?
    fn collides(&self, board: &Board, dx: i32, dy: i32) -> bool {
        for (cx, cy) in &self.cells() {
            let px = cx + dx;
            let py = cy + dy;
            if px < 0 || px >= COLS as i32 || py >= ROWS as i32 {
                return true;
            }
            if py >= 0 && board.get(px as usize, py as usize) {
                return true;
            }
        }
        false
    }

    fn rotate(&mut self, board: &Board) {
        let old_grid = std::mem::replace(&mut self.grid, Vec::new());
        self.grid = rotate_cw(&old_grid);
        // Simple wall kicks: try a set of horizontal offsets.
        for &kick in &[0i32, -1, 1, -2, 2] {
            if !self.collides(board, kick, 0) {
                self.x += kick;
                return;
            }
        }
        // Revert if no offset worked.
        self.grid = old_grid;
    }
}

struct Board {
    grid: Vec<Vec<Option<Kind>>>,
}

impl Board {
    fn new() -> Self {
        Board {
            grid: vec![vec![None; COLS]; ROWS],
        }
    }

    fn get(&self, x: usize, y: usize) -> bool {
        self.grid[y][x].is_some()
    }

    fn set(&mut self, x: usize, y: usize, kind: Kind) {
        self.grid[y][x] = Some(kind);
    }

    /// Merge a locked piece into the board.
    fn lock(&mut self, piece: &Piece) {
        for (cx, cy) in piece.cells() {
            if cy >= 0 && cy < ROWS as i32 && cx >= 0 && cx < COLS as i32 {
                self.set(cx as usize, cy as usize, piece.kind);
            }
        }
    }

    /// Remove all full rows, return how many were cleared.
    fn clear_lines(&mut self) -> usize {
        let mut cleared = 0;
        let mut keep: Vec<Vec<Option<Kind>>> = Vec::new();
        for row in self.grid.iter().rev() {
            if row.iter().all(|c| c.is_some()) {
                cleared += 1;
            } else {
                keep.push(row.clone());
            }
        }
        while keep.len() < ROWS {
            keep.push(vec![None; COLS]);
        }
        // `keep` is bottom-up; restore top-down order.
        self.grid = keep.into_iter().rev().collect();
        cleared
    }

    /// Is the very top row occupied (block out risk)?
    fn top_out(&self) -> bool {
        self.grid[0].iter().any(|c| c.is_some())
    }
}

// ---------------------------------------------------------------------------
// 7-bag randomizer + a tiny PRNG (no external dep)
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn seed() -> Self {
        let mut v: u64 = 0x2545F4914F6CDD1D;
        v ^= (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64)
            .wrapping_mul(0x9E3779B97F4A7C15);
        Rng(v)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: u64) -> usize {
        (self.next_u64() % n) as usize
    }
}

struct Bag {
    queue: Vec<Kind>,
    rng: Rng,
}

impl Bag {
    fn new(rng: Rng) -> Self {
        Bag {
            queue: Vec::new(),
            rng,
        }
    }

    fn next(&mut self) -> Kind {
        if self.queue.is_empty() {
            let mut bag = Kind::all().to_vec();
            for i in (1..bag.len()).rev() {
                let j = self.rng.below((i + 1) as u64);
                bag.swap(i, j);
            }
            self.queue = bag;
        }
        self.queue.pop().unwrap()
    }
}

// ---------------------------------------------------------------------------
// Game state
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    Playing,
    Paused,
    GameOver,
}

struct Game {
    board: Board,
    current: Piece,
    next: Kind,
    bag: Bag,
    score: u32,
    lines: usize,
    level: usize,
    phase: Phase,
}

impl Game {
    fn new() -> Self {
        let mut bag = Bag::new(Rng::seed());
        let first = bag.next();
        let second = bag.next();
        Game {
            board: Board::new(),
            current: Piece::new(first),
            next: second,
            bag,
            score: 0,
            lines: 0,
            level: 1,
            phase: Phase::Playing,
        }
    }

    fn restart(&mut self) {
        *self = Game::new();
    }

    /// How often gravity ticks, in ms (faster with each level).
    fn tick_interval(&self) -> Duration {
        let ms = (800.0 * 0.8_f64.powi((self.level - 1) as i32)).max(50.0) as u64;
        Duration::from_millis(ms)
    }

    /// Award points for `cleared` lines at the current level.
    fn score_clear(&mut self, cleared: usize) {
        if cleared == 0 {
            return;
        }
        let base = match cleared {
            1 => 100,
            2 => 300,
            3 => 500,
            _ => 800,
        };
        self.score += (base * self.level) as u32;
        self.lines += cleared;
        self.level = self.lines / 10 + 1;
    }

    /// Commit the current piece, clear lines, spawn the next, flag game over.
    fn commit_and_spawn(&mut self) {
        self.board.lock(&self.current);
        let cleared = self.board.clear_lines();
        self.score_clear(cleared);

        let spawn_kind = self.next;
        self.next = self.bag.next();
        self.current = Piece::new(spawn_kind);

        if self.board.top_out() || self.current.collides(&self.board, 0, 0) {
            self.phase = Phase::GameOver;
        }
    }

    /// Move the current piece down one row; if it can't, commit and spawn.
    fn step(&mut self) {
        if self.phase != Phase::Playing {
            return;
        }
        if !self.current.collides(&self.board, 0, 1) {
            self.current.y += 1;
            return;
        }
        self.commit_and_spawn();
    }

    fn try_move(&mut self, dx: i32) {
        if self.phase == Phase::Playing && !self.current.collides(&self.board, dx, 0) {
            self.current.x += dx;
        }
    }

    fn try_rotate(&mut self) {
        if self.phase == Phase::Playing {
            self.current.rotate(&self.board);
        }
    }

    fn soft_drop(&mut self) {
        if self.phase == Phase::Playing && !self.current.collides(&self.board, 0, 1) {
            self.current.y += 1;
            self.score += 1;
        }
    }

    fn hard_drop(&mut self) {
        if self.phase != Phase::Playing {
            return;
        }
        let mut dist = 0;
        while !self.current.collides(&self.board, 0, 1) {
            self.current.y += 1;
            dist += 1;
        }
        self.score += dist * 2;
        self.current.y -= 1; // restore to rest position before locking
        self.commit_and_spawn();
    }

    /// Row where the current piece would come to rest (for the ghost).
    fn ghost_y(&self) -> i32 {
        let mut dy = 0;
        while !self.current.collides(&self.board, 0, dy + 1) {
            dy += 1;
        }
        self.current.y + dy
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn render(frame: &mut Frame, game: &Game) {
    let area = frame.area();
    let outer = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(22), // board (10 cols * 2 + 2 borders)
            Constraint::Min(1),     // sidebar
        ])
        .split(area);

    draw_board(frame, game, outer[0]);
    draw_sidebar(frame, game, outer[1]);
}

fn draw_board(frame: &mut Frame, game: &Game, area: Rect) {
    // Combined display grid: stack + ghost (dimmed) + live piece (solid).
    let mut grid: Vec<Vec<Option<Kind>>> = vec![vec![None; COLS]; ROWS];
    for (y, row) in game.board.grid.iter().enumerate() {
        for (x, cell) in row.iter().enumerate() {
            if let Some(k) = *cell {
                grid[y][x] = Some(k);
            }
        }
    }
    // Ghost cells (would-be landing spots) that are not yet filled.
    let gy = game.ghost_y();
    for (r, row) in game.current.grid.iter().enumerate() {
        for (c, &v) in row.iter().enumerate() {
            if v {
                let px = (game.current.x + c as i32) as usize;
                let py = (gy + r as i32) as usize;
                if px < COLS && py < ROWS && grid[py][px].is_none() {
                    grid[py][px] = Some(game.current.kind);
                }
            }
        }
    }
    // Live piece (solid, overrides ghost).
    for (cx, cy) in game.current.cells() {
        let px = cx as usize;
        let py = cy as usize;
        if px < COLS && py < ROWS {
            grid[py][px] = Some(game.current.kind);
        }
    }
    // Set of live cells so we can tell ghosts apart when drawing.
    let live: std::collections::HashSet<(usize, usize)> = game
        .current
        .cells()
        .into_iter()
        .filter(|(cx, cy)| *cx >= 0 && *cx < COLS as i32 && *cy >= 0 && *cy < ROWS as i32)
        .map(|(cx, cy)| (cx as usize, cy as usize))
        .collect();

    let mut text = vec![];
    for r in 0..ROWS {
        let mut line = vec![];
        for c in 0..COLS {
            match grid[r][c] {
                Some(k) => {
                    if live.contains(&(c, r)) {
                        line.push(Span::styled(
                            "██".to_string(),
                            Style::default()
                                .fg(k.color())
                                .add_modifier(Modifier::BOLD),
                        ));
                    } else {
                        line.push(Span::styled(
                            "░░".to_string(),
                            Style::default().fg(Color::DarkGray),
                        ));
                    }
                }
                None => line.push(Span::styled("  ".to_string(), Style::default())),
            }
        }
        if r + 1 < ROWS {
            line.push(Span::styled("\n".to_string(), Style::default()));
        }
        text.push(Line::from(line));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray))
        .title(" Tetris ");
    frame.render_widget(Clear, area);
    let para = Paragraph::new(text).block(block).alignment(Alignment::Center);
    frame.render_widget(para, area);
}

fn draw_sidebar(frame: &mut Frame, game: &Game, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),  // stats
            Constraint::Length(6),  // next piece
            Constraint::Min(1),     // controls
        ])
        .split(area);

    let stats = vec![
        Line::from(Span::styled("Score", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(game.score.to_string()),
        Line::from(" "),
        Line::from(Span::styled(
            "Level",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(game.level.to_string()),
        Line::from(Span::styled(
            "Lines",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(game.lines.to_string()),
    ];
    frame.render_widget(
        Paragraph::new(stats).block(Block::default().borders(Borders::ALL).title(" Stats ")),
        rows[0],
    );

    // Next piece preview in a 4x4 box.
    let next_cells = next_preview_cells(game.next);
    let mut text = vec![];
    for r in 0..4 {
        let mut line = vec![Span::styled("  ".to_string(), Style::default())];
        for c in 0..4 {
            if next_cells.get(&(r, c)).copied().unwrap_or(false) {
                line.push(Span::styled(
                    "██".to_string(),
                    Style::default()
                        .fg(game.next.color())
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                line.push(Span::styled("  ".to_string(), Style::default()));
            }
        }
        if r + 1 < 4 {
            line.push(Span::styled("\n".to_string(), Style::default()));
        }
        text.push(Line::from(line));
    }
    frame.render_widget(
        Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title(" Next "))
            .alignment(Alignment::Center),
        rows[1],
    );

    let controls = vec![
        Line::from(Span::styled(
            "Controls",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  ←  →  move"),
        Line::from("  ↓     soft drop"),
        Line::from("  ↑ / x rotate"),
        Line::from("  space hard drop"),
        Line::from("  p     pause"),
        Line::from("  r     restart"),
        Line::from("  q / esc quit"),
    ];
    frame.render_widget(
        Paragraph::new(controls)
            .block(Block::default().borders(Borders::ALL).title(" Help ")),
        rows[2],
    );
}

/// Filled (row, col) cells of a piece inside a 4x4 preview box, centered.
fn next_preview_cells(kind: Kind) -> std::collections::HashMap<(usize, usize), bool> {
    let mut map: std::collections::HashMap<(usize, usize), bool> =
        std::collections::HashMap::new();
    let g = base_grid(kind);
    let rows = g.len();
    let cols = g[0].len();
    let ox = (4 - cols) as i32 / 2;
    let oy = (4 - rows) as i32 / 2;
    for (r, row) in g.iter().enumerate() {
        for (c, &v) in row.iter().enumerate() {
            let pr = (r as i32 + oy) as usize;
            let pc = (c as i32 + ox) as usize;
            if pr < 4 && pc < 4 {
                map.insert((pr, pc), v);
            }
        }
    }
    map
}

// ---------------------------------------------------------------------------
// Input + main loop
// ---------------------------------------------------------------------------

fn handle_key(key: KeyEvent, game: &mut Game) -> bool {
    // Returns true if the app should exit.
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return true,
        KeyCode::Char('p') => match game.phase {
            Phase::Playing => game.phase = Phase::Paused,
            Phase::Paused => game.phase = Phase::Playing,
            _ => {}
        },
        KeyCode::Char('r') => {
            if game.phase == Phase::GameOver {
                game.restart();
            }
        }
        KeyCode::Char(' ') => game.hard_drop(),
        KeyCode::Char('x') | KeyCode::Up => game.try_rotate(),
        KeyCode::Char('s') => game.soft_drop(),
        KeyCode::Left => game.try_move(-1),
        KeyCode::Right => game.try_move(1),
        KeyCode::Down => game.soft_drop(),
        _ => {}
    }
    false
}

fn draw_overlay(frame: &mut Frame, title: &str, sub: &str, color: Color) {
    let area = frame.area();
    let x = (area.x + area.width / 2).saturating_sub(9);
    let y = (area.y + area.height / 2).saturating_sub(2);
    let lines = vec![
        Line::from(Span::styled(
            format!("  {}  ", title),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(sub, Style::default().fg(Color::Gray))),
    ];
    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
    let w = title.len() as u16 + 6;
    let rect = Rect {
        x,
        y,
        width: w.max(16),
        height: 4,
    };
    frame.render_widget(Clear, rect);
    frame.render_widget(para, rect);
}

fn run(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<Stdout>>,
) -> std::io::Result<()> {
    let mut game = Game::new();
    let frame_delay = Duration::from_millis(33);
    let mut last_gravity = Instant::now();

    loop {
        terminal.draw(|f| {
            if let Phase::Paused = game.phase {
                draw_overlay(f, " PAUSED ", "press p to resume", Color::Yellow);
            } else if let Phase::GameOver = game.phase {
                draw_overlay(f, " GAME OVER ", "press r to restart", Color::Red);
            }
            render(f, &game);
        })?;

        if game.phase == Phase::GameOver {
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(k) = event::read()? {
                    if handle_key(k, &mut game) {
                        break;
                    }
                }
            }
            continue;
        }

        if event::poll(frame_delay)? {
            let ev = event::read()?;
            if let Event::Key(k) = ev {
                if k.modifiers.contains(KeyModifiers::CONTROL) {
                    continue;
                }
                if handle_key(k, &mut game) {
                    break;
                }
            }
        }

        if game.phase == Phase::Playing {
            let interval = game.tick_interval();
            if last_gravity.elapsed() >= interval {
                game.step();
                last_gravity = Instant::now();
            }
        } else {
            last_gravity = Instant::now();
        }
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let result = run(&mut terminal);

    // Always try to restore the terminal, even on panic/exit.
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

// ---------------------------------------------------------------------------
// Tests — exercise the game mechanics (no terminal needed)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn kinds() -> [Kind; 7] {
        [Kind::I, Kind::O, Kind::T, Kind::S, Kind::Z, Kind::J, Kind::L]
    }

    #[test]
    fn rotation_returns_to_original_after_four() {
        let empty = Board::new();
        for k in kinds() {
            let mut p = Piece::new(k);
            let n = p.cells().len();
            assert_eq!(n, 4, "piece {:?} must have 4 cells", k);
            let orig = p.grid.clone();
            for _ in 0..4 {
                p.rotate(&empty);
                assert_eq!(p.cells().len(), n, "rotation must preserve cell count");
            }
            assert_eq!(p.grid, orig, "four rotations must return to the start");
        }
    }

    #[test]
    fn i_piece_changes_orientation_then_back() {
        let empty = Board::new();
        let mut p = Piece::new(Kind::I);
        let orig = p.grid.clone();
        p.rotate(&empty);
        assert_ne!(p.grid, orig, "one rotation should change the I orientation");
        // The I piece is vertical after one rotation: it spans 4 distinct rows.
        let rows: HashSet<usize> = p.cells().iter().map(|(_, y)| *y as usize).collect();
        assert_eq!(rows.len(), 4);
    }

    #[test]
    fn piece_spawns_centered_and_in_bounds() {
        for k in kinds() {
            let p = Piece::new(k);
            for (x, y) in p.cells() {
                assert!(x >= 0 && x < COLS as i32, "spawn x out of bounds");
                assert!(y >= 0 && y < ROWS as i32, "spawn y out of bounds");
            }
        }
    }

    #[test]
    fn wall_keeps_piece_in_bounds() {
        let mut g = Game::new();
        for _ in 0..25 {
            g.try_move(-1);
        }
        for (x, _) in g.current.cells() {
            assert!(x >= 0 && x < COLS as i32, "piece must never cross the left wall");
        }
        for _ in 0..25 {
            g.try_move(1);
        }
        for (x, _) in g.current.cells() {
            assert!(x >= 0 && x < COLS as i32, "piece must never cross the right wall");
        }
    }

    #[test]
    fn clears_full_rows_only() {
        // Fill only the bottom row -> clears exactly one line.
        let mut b = Board::new();
        for x in 0..COLS {
            b.set(x, ROWS - 1, Kind::T);
        }
        assert_eq!(b.clear_lines(), 1);
        assert!(b.grid[ROWS - 1].iter().all(|c| c.is_none()));

        // A nearly full row does NOT clear.
        let mut b2 = Board::new();
        for x in 0..(COLS - 1) {
            b2.set(x, ROWS - 1, Kind::T);
        }
        assert_eq!(b2.clear_lines(), 0);

        // Two full rows at the bottom -> clears two, everything shifts down.
        let mut b3 = Board::new();
        for x in 0..COLS {
            b3.set(x, ROWS - 1, Kind::J);
            b3.set(x, ROWS - 2, Kind::J);
            // one marker above the cleared zone
            b3.set(0, ROWS - 3, Kind::J);
        }
        assert_eq!(b3.clear_lines(), 2);
        // The marker (originally at ROWS-3) falls by exactly two rows to ROWS-1.
        assert!(b3.get(0, ROWS - 1));
        assert!(!b3.get(0, ROWS - 3));
    }

    #[test]
    fn hard_drop_locks_piece_at_floor() {
        let mut g = Game::new();
        g.hard_drop();
        let filled: usize = g.board.grid.iter().flatten().filter(|c| c.is_some()).count();
        assert_eq!(filled, 4, "exactly one 4-cell piece should be locked");
        assert_eq!(g.phase, Phase::Playing);
        assert!(g.score > 0, "hard drop must score points");
        // The locked piece should be resting in the bottom of the board.
        let bottom = g.board.grid[ROWS - 1]
            .iter()
            .chain(&g.board.grid[ROWS - 2])
            .any(|c| c.is_some());
        assert!(bottom, "piece must rest at the bottom after a hard drop");
    }

    #[test]
    fn line_clear_awards_points_and_levels() {
        let mut g = Game::new();
        g.level = 1;
        let s0 = g.score;
        let l0 = g.lines;
        g.score_clear(1);
        assert_eq!(g.score, s0 + 100);
        assert_eq!(g.lines, l0 + 1);
        g.score_clear(2);
        assert_eq!(g.score, s0 + 100 + 300);
        assert_eq!(g.lines, l0 + 3);
        g.score_clear(3);
        assert_eq!(g.score, s0 + 100 + 300 + 500);
        g.score_clear(4); // reaches 10 lines -> level 2
        assert_eq!(g.score, s0 + 100 + 300 + 500 + 800);
        assert_eq!(g.lines, l0 + 10);
        assert_eq!(g.level, 2);
    }

    #[test]
    fn blocked_spawn_zone_is_game_over() {
        let mut g = Game::new();
        // Fill the center spawn zone (cols 3-6, rows 0-2) so no piece can spawn/move.
        for y in 0..3 {
            for x in 3..7 {
                g.board.set(x, y, Kind::T);
            }
        }
        g.step();
        assert_eq!(g.phase, Phase::GameOver);
    }

    #[test]
    fn restart_resets_state() {
        let mut g = Game::new();
        g.hard_drop();
        g.phase = Phase::GameOver;
        let pre = g.score;
        let _ = pre; // (may be > 0)
        g.restart();
        assert_eq!(g.score, 0);
        assert_eq!(g.lines, 0);
        assert_eq!(g.level, 1);
        assert_eq!(g.phase, Phase::Playing);
        let filled: usize = g.board.grid.iter().flatten().filter(|c| c.is_some()).count();
        assert_eq!(filled, 0, "board must be empty after restart");
    }

    #[test]
    fn seven_bag_produces_all_kinds_each_cycle() {
        let mut bag = Bag::new(Rng(0xdeadbeef));
        let first_cycle: HashSet<Kind> = (0..7).map(|_| bag.next()).collect();
        assert_eq!(first_cycle.len(), 7, "a fresh 7-bag must contain every piece once");
        // The very next 7 should also be a full permutation.
        let second_cycle: HashSet<Kind> = (0..7).map(|_| bag.next()).collect();
        assert_eq!(second_cycle.len(), 7);
    }

    #[test]
    fn ghost_is_a_valid_rest_position() {
        let g = Game::new();
        let gy = g.ghost_y();
        assert!(gy >= g.current.y);
        let dy = gy - g.current.y;
        assert!(!g.current.collides(&g.board, 0, dy), "rest position must not collide");
        assert!(g.current.collides(&g.board, 0, dy + 1), "one below rest must collide");
    }

    #[test]
    fn pause_and_resume_does_not_drop() {
        let mut g = Game::new();
        let y0 = g.current.y;
        g.phase = Phase::Paused;
        for _ in 0..5 {
            g.step(); // gravity must be ignored while paused
        }
        assert_eq!(g.current.y, y0, "piece must not fall while paused");
        g.phase = Phase::Playing;
    }
}
