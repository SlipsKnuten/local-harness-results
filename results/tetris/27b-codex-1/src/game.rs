//! Pure Tetris game state and rules: board, movement, rotation, gravity,
//! line clearing, scoring, hold and the 7-bag randomizer. No I/O lives here,
//! so it is easy to unit test.

use crate::piece::Tetromino;
use std::time::{SystemTime, UNIX_EPOCH};

/// Playfield width in cells.
pub const COLS: usize = 10;
/// Rows actually visible to the player.
pub const VISIBLE_ROWS: usize = 20;
/// Extra rows above the visible area where pieces spawn (hidden buffer).
pub const HIDDEN_ROWS: usize = 20;
/// Total board height = hidden + visible.
pub const BOARD_ROWS: usize = VISIBLE_ROWS + HIDDEN_ROWS;
/// Board row index of the topmost visible row.
pub const HIDDEN: usize = HIDDEN_ROWS;

/// Candidate wall-kick offsets (screen coords: +x right, +y down) tried in
/// order when a rotation would otherwise collide.
const KICKS: [(i32, i32); 6] = [(0, 0), (-1, 0), (1, 0), (0, -1), (-2, 0), (2, 0)];

/// A piece in flight: its kind, the four filled cells for the current
/// rotation, and the box position `(x, y)` on the board.
#[derive(Clone, Debug)]
pub struct Piece {
    pub kind: Tetromino,
    pub cells: [(i8, i8); 4],
    pub x: i32,
    pub y: i32,
}

impl Piece {
    /// A freshly spawned piece centered horizontally at the top of the board.
    pub fn spawn(kind: Tetromino) -> Self {
        let size = kind.size() as i32;
        Piece {
            kind,
            cells: kind.base_cells(),
            x: (COLS as i32 - size) / 2,
            y: 0,
        }
    }
}

/// Rotate `cells` 90 degrees around the `kind`'s bounding box.
fn rotated(kind: Tetromino, cells: &[(i8, i8); 4], clockwise: bool) -> [(i8, i8); 4] {
    let n = kind.size() as i8;
    let mut out = [(0i8, 0i8); 4];
    for i in 0..4 {
        let (r, c) = cells[i];
        out[i] = if clockwise { (c, n - 1 - r) } else { (n - 1 - c, r) };
    }
    out
}

/// The settled stack. `0` is empty; `1..=7` encodes the tetromino kind
/// (`kind.idx() + 1`) for later coloring.
#[derive(Clone, Debug)]
pub struct Board {
    cells: Vec<u8>,
}

impl Board {
    fn new() -> Self {
        Self { cells: vec![0; BOARD_ROWS * COLS] }
    }

    /// Stored value at a valid `(row, col)`; `0` when empty.
    pub fn filled(&self, row: usize, col: usize) -> u8 {
        self.cells[row * COLS + col]
    }

    fn set(&mut self, row: usize, col: usize, value: u8) {
        self.cells[row * COLS + col] = value;
    }

    fn full(&self, row: usize) -> bool {
        for col in 0..COLS {
            if self.cells[row * COLS + col] == 0 {
                return false;
            }
        }
        true
    }

    /// Probe a board coordinate. Out-of-bounds columns and the floor read as a
    /// solid wall (`1`); above the top reads as empty.
    fn at(&self, row: i32, col: i32) -> u8 {
        if col < 0 || col >= COLS as i32 {
            return 1;
        }
        if row < 0 {
            return 0;
        }
        if row >= BOARD_ROWS as i32 {
            return 1;
        }
        self.cells[row as usize * COLS + col as usize]
    }

    /// Remove every full row and let the rest fall. Returns rows cleared.
    fn clear_lines(&mut self) -> u32 {
        let mut kept: Vec<u8> = Vec::with_capacity(BOARD_ROWS);
        for row in (0..BOARD_ROWS).rev() {
            if self.full(row) {
                continue;
            }
            for col in 0..COLS {
                kept.push(self.cells[row * COLS + col]);
            }
        }
        let kept_rows = kept.len() / COLS;
        let cleared = (BOARD_ROWS - kept_rows) as u32;
        let mut fresh = vec![0u8; BOARD_ROWS * COLS];
        for (i, chunk) in kept.chunks(COLS).enumerate() {
            let row = BOARD_ROWS - 1 - i;
            for (col, &value) in chunk.iter().enumerate() {
                fresh[row * COLS + col] = value;
            }
        }
        self.cells = fresh;
        cleared
    }
}

/// The whole game: current board, active piece, queues, hold, and stats.
pub struct Game {
    pub board: Board,
    pub current: Piece,
    pub hold: Option<Tetromino>,
    pub score: u64,
    pub lines: u32,
    pub level: u32,
    pub over: bool,
    pub paused: bool,
    queue: Vec<Tetromino>,
    bag: Vec<Tetromino>,
    can_hold: bool,
    rng: u64,
}

impl Game {
    /// A new, empty game with a seeded 7-bag and the first piece active.
    pub fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9e37_79b9_7f4a_7c15)
            | 1; // xorshift needs a non-zero seed
        let mut game = Game {
            board: Board::new(),
            current: Piece::spawn(Tetromino::I),
            hold: None,
            score: 0,
            lines: 0,
            level: 0,
            over: false,
            paused: false,
            queue: Vec::new(),
            bag: Vec::new(),
            can_hold: true,
            rng: seed,
        };
        game.ensure_queue();
        let kind = game.take_next();
        game.spawn_current(kind);
        game
    }

    /// The next pieces, front first (always at least three).
    pub fn queue(&self) -> &[Tetromino] {
        &self.queue
    }

    /// Milliseconds between gravity steps for the current level.
    pub fn gravity_interval_ms(&self) -> u128 {
        let ms = 800.0 * 0.84f64.powf(self.level as f64);
        ms.max(60.0) as u128
    }

    /// How far the current piece can fall before it rests (ghost piece).
    pub fn drop_distance(&self) -> i32 {
        let mut d = 0;
        while self.fits(&self.current.cells, self.current.x, self.current.y + 1 + d) {
            d += 1;
        }
        d
    }

    // --- input actions ---------------------------------------------------

    pub fn move_h(&mut self, dx: i32) {
        if self.over || self.paused {
            return;
        }
        let x = self.current.x + dx;
        if self.fits(&self.current.cells, x, self.current.y) {
            self.current.x = x;
        }
    }

    pub fn rotate(&mut self, clockwise: bool) {
        if self.over || self.paused {
            return;
        }
        let cells = rotated(self.current.kind, &self.current.cells, clockwise);
        for (dx, dy) in KICKS {
            let x = self.current.x + dx;
            let y = self.current.y + dy;
            if self.fits(&cells, x, y) {
                self.current.cells = cells;
                self.current.x = x;
                self.current.y = y;
                return;
            }
        }
    }

    /// One gravity step: fall a row, or lock if resting.
    pub fn step(&mut self) {
        if self.over || self.paused {
            return;
        }
        let y = self.current.y + 1;
        if self.fits(&self.current.cells, self.current.x, y) {
            self.current.y = y;
        } else {
            self.lock();
        }
    }

    /// Manual soft drop: fall a row (+1 point) or lock if resting.
    pub fn soft_drop(&mut self) {
        if self.over || self.paused {
            return;
        }
        let y = self.current.y + 1;
        if self.fits(&self.current.cells, self.current.x, y) {
            self.current.y = y;
            self.score += 1;
        } else {
            self.lock();
        }
    }

    /// Instantly drop to the stack (+2 points per row) and lock.
    pub fn hard_drop(&mut self) {
        if self.over || self.paused {
            return;
        }
        let mut d = 0;
        while self.fits(&self.current.cells, self.current.x, self.current.y + 1 + d) {
            d += 1;
        }
        self.current.y += d;
        self.score += 2 * d as u64;
        self.lock();
    }

    /// Swap the active piece with the hold slot (once per piece).
    pub fn hold(&mut self) {
        if self.over || self.paused || !self.can_hold {
            return;
        }
        self.can_hold = false;
        let active = self.current.kind;
        let swapped = self.hold.take();
        self.hold = Some(active);
        let kind = match swapped {
            Some(h) => h,
            None => self.take_next(),
        };
        self.spawn_current(kind);
    }

    // --- internals -------------------------------------------------------

    /// xorshift64 RNG.
    fn next(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }

    fn refill_bag(&mut self) {
        self.bag = Tetromino::ALL.to_vec();
        for i in (1..self.bag.len()).rev() {
            let j = (self.next() % (i as u64 + 1)) as usize;
            self.bag.swap(i, j);
        }
    }

    fn ensure_queue(&mut self) {
        while self.queue.len() < 3 {
            if self.bag.is_empty() {
                self.refill_bag();
            }
            if let Some(kind) = self.bag.pop() {
                self.queue.push(kind);
            }
        }
    }

    fn take_next(&mut self) -> Tetromino {
        self.ensure_queue();
        let kind = self.queue.remove(0);
        self.ensure_queue();
        kind
    }

    fn spawn_current(&mut self, kind: Tetromino) {
        let piece = Piece::spawn(kind);
        if !self.fits(&piece.cells, piece.x, piece.y) {
            self.over = true; // no room to spawn: top out
        }
        self.current = piece;
    }

    /// Would the given cells at `(x, y)` collide with walls/floor/stack?
    fn fits(&self, cells: &[(i8, i8); 4], x: i32, y: i32) -> bool {
        for &(r, c) in cells {
            let br = y + r as i32;
            let bc = x + c as i32;
            if bc < 0 || bc >= COLS as i32 {
                return false;
            }
            if br >= BOARD_ROWS as i32 {
                return false;
            }
            if br >= 0 && self.board.at(br, bc) != 0 {
                return false;
            }
        }
        true
    }

    /// Freeze the active piece into the stack, clear lines, and spawn next.
    fn lock(&mut self) {
        let mut all_hidden = true;
        for &(r, c) in &self.current.cells {
            let br = self.current.y + r as i32;
            let bc = self.current.x + c as i32;
            if br < 0 {
                continue;
            }
            if br >= 0 && br < BOARD_ROWS as i32 && bc >= 0 && bc < COLS as i32 {
                self.board.set(br as usize, bc as usize, self.current.kind.idx() as u8 + 1);
            }
            if br >= HIDDEN as i32 {
                all_hidden = false;
            }
        }
        // Locked entirely in the hidden buffer: the stack reached the top.
        if all_hidden {
            self.over = true;
        }

        let cleared = self.board.clear_lines();
        if cleared > 0 {
            self.score += [0, 100, 300, 500, 800][cleared as usize] as u64;
            self.lines += cleared;
            let level = self.lines / 10;
            if level > self.level {
                self.level = level;
            }
        }

        if !self.over {
            let kind = self.take_next();
            self.spawn_current(kind);
            self.can_hold = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_game_is_sane() {
        let game = Game::new();
        assert!(!game.over);
        assert_eq!(game.score, 0);
        assert_eq!(game.lines, 0);
        assert_eq!(game.level, 0);
        assert!(game.queue().len() >= 3);
        assert!(game.gravity_interval_ms() >= 60);
    }

    #[test]
    fn rotation_round_trips_to_base() {
        for kind in Tetromino::ALL {
            let mut cells = kind.base_cells();
            for _ in 0..4 {
                cells = rotated(kind, &cells, true);
            }
            assert_eq!(cells, kind.base_cells(), "kind {kind:?}");
        }
    }

    #[test]
    fn cw_then_ccw_is_identity() {
        for kind in Tetromino::ALL {
            let cells = kind.base_cells();
            let back = rotated(kind, &rotated(kind, &cells, true), false);
            assert_eq!(back, cells, "kind {kind:?}");
        }
    }

    #[test]
    fn hard_drop_fills_board_and_spawns_next() {
        let mut game = Game::new();
        game.hard_drop();
        let filled = (0..BOARD_ROWS)
            .flat_map(|r| (0..COLS).map(move |c| (r, c)))
            .filter(|&(r, c)| game.board.filled(r, c) != 0)
            .count();
        assert!(filled >= 4, "expected the dropped piece to be on the board");
        assert!(!game.over, "a single drop on an empty board cannot end the game");
    }

    #[test]
    fn gravity_step_falls_and_eventually_locks() {
        let mut game = Game::new();
        let start = game.current.y;
        game.step();
        assert_eq!(game.current.y, start + 1);
        // Keep stepping until it locks; a new piece must appear.
        for _ in 0..BOARD_ROWS {
            if game.over {
                break;
            }
            game.step();
        }
        assert!(!game.over);
    }

    #[test]
    fn wall_kick_allows_rotation_near_side() {
        let mut game = Game::new();
        // Force the I piece against the left wall, then rotate; a kick must
        // keep it inside the board rather than being rejected outright.
        game.current = Piece::spawn(Tetromino::I);
        while game.fits(&game.current.cells, game.current.x - 1, game.current.y) {
            game.current.x -= 1;
        }
        let before = (game.current.x, game.current.y, game.current.cells);
        game.rotate(true);
        assert_ne!(before, (game.current.x, game.current.y, game.current.cells));
    }

    #[test]
    fn clear_a_full_row() {
        let mut board = Board::new();
        let row = HIDDEN;
        for col in 0..COLS {
            board.set(row, col, 5);
        }
        assert!(board.full(row));
        assert_eq!(board.clear_lines(), 1);
        assert!(!board.full(row));
    }

    #[test]
    fn clearing_two_rows_shifts_block_down() {
        let mut board = Board::new();
        let r1 = HIDDEN; // 20
        let r2 = HIDDEN + 1; // 21
        for col in 0..COLS {
            board.set(r1, col, 3);
            board.set(r2, col, 3);
        }
        board.set(r1 - 1, 0, 2); // a lone cell just above the two full rows
        assert_eq!(board.clear_lines(), 2);
        assert_eq!(board.filled(r2, 0), 2); // fell two rows
        assert_eq!(board.filled(r2, 1), 0); // rest of that row stayed empty
    }

    #[test]
    fn one_bag_is_a_permutation_of_all_kinds() {
        let mut game = Game::new();
        game.queue = Vec::new();
        game.bag = Vec::new();
        game.refill_bag();
        let mut seen = [false; 7];
        for _ in 0..7 {
            let kind = game.take_next();
            seen[kind.idx()] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn hold_swaps_and_blocks_double_hold() {
        let mut game = Game::new();
        let first = game.current.kind;
        game.hold();
        assert_eq!(game.hold, Some(first));
        let second = game.current.kind;
        game.hold(); // not allowed: same piece already held this drop
        assert_eq!(game.current.kind, second);
    }

    #[test]
    fn auto_play_eventually_tops_out() {
        // Straight hard-drops with no strategy must build a stack and trigger
        // game over (top-out) without ever panicking.
        let mut game = Game::new();
        let mut drops = 0;
        while !game.over && drops < 1000 {
            game.hard_drop();
            drops += 1;
        }
        assert!(game.over, "expected top-out after {drops} straight drops");
        assert!(drops < 1000);
    }
}
