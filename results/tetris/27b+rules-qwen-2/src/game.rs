//! Core Tetris game logic. Pure and side-effect free (no IO), so it can be
//! unit tested without a terminal.

use std::collections::VecDeque;

pub const COLS: usize = 10;
pub const ROWS: usize = 20;

/// The seven tetrominoes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceType {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl PieceType {
    pub const ALL: [PieceType; 7] = [
        Self::I,
        Self::O,
        Self::T,
        Self::S,
        Self::Z,
        Self::J,
        Self::L,
    ];

    /// Rotation box size (3 for most pieces, 4 for I, 2 for O) and the base
    /// cell offsets `(col, row)` in that box, row 0 = top.
    fn geometry(self) -> (i32, [(i32, i32); 4]) {
        match self {
            Self::I => (4, [(0, 1), (1, 1), (2, 1), (3, 1)]),
            Self::O => (2, [(0, 0), (1, 0), (0, 1), (1, 1)]),
            Self::T => (3, [(1, 0), (0, 1), (1, 1), (2, 1)]),
            Self::S => (3, [(1, 0), (2, 0), (0, 1), (1, 1)]),
            Self::Z => (3, [(0, 0), (1, 0), (1, 1), (2, 1)]),
            Self::J => (3, [(0, 0), (0, 1), (1, 1), (2, 1)]),
            Self::L => (3, [(2, 0), (0, 1), (1, 1), (2, 1)]),
        }
    }

    /// Cell offsets `(col, row)` for the given clockwise rotation.
    pub fn offsets(self, rot: u8) -> [(i32, i32); 4] {
        let n = self.geometry().0;
        self.geometry().1.map(|(mut x, mut y)| {
            for _ in 0..(rot % 4) {
                (x, y) = (n - 1 - y, x);
            }
            (x, y)
        })
    }
}

/// A board cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Empty,
    Filled(PieceType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    Playing,
    Paused,
    Over,
}

/// The piece currently in play, positioned by the top-left corner of its
/// rotation box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivePiece {
    pub shape: PieceType,
    /// Clockwise rotation, 0..4.
    pub rot: u8,
    pub col: i32,
    pub row: i32,
}

impl ActivePiece {
    pub fn cells(&self) -> [(i32, i32); 4] {
        self.shape
            .offsets(self.rot)
            .map(|(dx, dy)| (self.col + dx, self.row + dy))
    }
}

/// SRS wall-kick offsets as `(col_delta, row_delta)` with positive row
/// pointing down.
fn kicks(shape: PieceType, from: u8, to: u8) -> &'static [(i32, i32)] {
    const NONE: [(i32, i32); 1] = [(0, 0)];
    if shape == PieceType::O {
        return &NONE;
    }
    const J01: [(i32, i32); 5] = [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)];
    const J10: [(i32, i32); 5] = [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)];
    const J23: [(i32, i32); 5] = [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)];
    const J32: [(i32, i32); 5] = [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)];
    const I01: [(i32, i32); 5] = [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)];
    const I10: [(i32, i32); 5] = [(0, 0), (2, 0), (-1, 0), (2, -1), (-1, 2)];
    const I12: [(i32, i32); 5] = [(0, 0), (-1, 0), (2, 0), (-1, -2), (2, 1)];
    const I21: [(i32, i32); 5] = [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)];
    match (from, to) {
        (0, 1) => if matches!(shape, PieceType::I) {
            &I01
        } else {
            &J01
        },
        (1, 0) => if matches!(shape, PieceType::I) {
            &I10
        } else {
            &J10
        },
        (1, 2) => if matches!(shape, PieceType::I) {
            &I12
        } else {
            &J10
        },
        (2, 1) => if matches!(shape, PieceType::I) {
            &I21
        } else {
            &J01
        },
        (2, 3) => if matches!(shape, PieceType::I) {
            &I10
        } else {
            &J23
        },
        (3, 2) => if matches!(shape, PieceType::I) {
            &I01
        } else {
            &J32
        },
        (3, 0) => if matches!(shape, PieceType::I) {
            &I21
        } else {
            &J32
        },
        (0, 3) => if matches!(shape, PieceType::I) {
            &I12
        } else {
            &J23
        },
        _ => &NONE,
    }
}

/// A deterministic 64-bit RNG (SplitMix64) so games are reproducible in tests.
#[derive(Debug, Clone)]
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn shuffle(&mut self, v: &mut [PieceType]) {
        for i in (1..v.len()).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            v.swap(i, j);
        }
    }
}

/// A full Tetris game.
pub struct Game {
    pub board: [[Cell; COLS]; ROWS],
    pub current: ActivePiece,
    /// Upcoming pieces, refilled from a 7-bag.
    queue: VecDeque<PieceType>,
    bag: VecDeque<PieceType>,
    pub hold: Option<PieceType>,
    can_hold: bool,
    pub score: u32,
    pub lines: u32,
    pub level: u32,
    pub state: GameState,
    rng: Rng,
}

impl Game {
    /// Start a new game with a deterministic seed.
    pub fn new(seed: u64) -> Self {
        let mut game = Self {
            board: [[Cell::Empty; COLS]; ROWS],
            current: ActivePiece {
                shape: PieceType::O,
                rot: 0,
                col: 4,
                row: 0,
            },
            queue: VecDeque::new(),
            bag: VecDeque::new(),
            hold: None,
            can_hold: true,
            score: 0,
            lines: 0,
            level: 1,
            state: GameState::Playing,
            rng: Rng::new(seed),
        };
        game.refill_queue();
        game.spawn();
        game
    }

    /// Gravity delay in milliseconds for the current level.
    pub fn tick_ms(&self) -> u64 {
        (800 - (self.level - 1) * 60).max(60) as u64
    }

    /// The upcoming pieces, next piece first (at most 3).
    pub fn next(&self) -> Vec<PieceType> {
        self.queue.iter().copied().collect()
    }

    /// Spawn column that centers the piece's rotation box.
    fn spawn_col(shape: PieceType) -> i32 {
        (COLS as i32 - shape.geometry().0) / 2
    }

    fn initial_piece(shape: PieceType) -> ActivePiece {
        ActivePiece {
            shape,
            rot: 0,
            col: Self::spawn_col(shape),
            row: 0,
        }
    }

    fn refill_queue(&mut self) {
        while self.queue.len() < 3 {
            if self.bag.is_empty() {
                let mut bag = PieceType::ALL.to_vec();
                self.rng.shuffle(&mut bag);
                self.bag = bag.into();
            }
            self.queue.push_back(self.bag.pop_front().unwrap());
        }
    }

    fn spawn(&mut self) {
        let shape = self.queue.pop_front().unwrap();
        self.refill_queue();
        let piece = Self::initial_piece(shape);
        if self.collides_at(piece.shape, piece.rot, piece.col, piece.row) {
            self.state = GameState::Over;
        }
        self.current = piece;
    }

    /// Whether the given piece position overlaps a wall, the floor or a
    /// locked cell. Cells above the top edge are always free.
    pub fn collides_at(&self, shape: PieceType, rot: u8, col: i32, row: i32) -> bool {
        for (dx, dy) in shape.offsets(rot) {
            let cx = col + dx;
            let cy = row + dy;
            if cx < 0 || cx >= COLS as i32 || cy >= ROWS as i32 {
                return true;
            }
            if cy >= 0 && self.board[cy as usize][cx as usize] != Cell::Empty {
                return true;
            }
        }
        false
    }

    /// Board row the current piece would land on if dropped instantly.
    pub fn ghost_row(&self) -> i32 {
        let p = self.current;
        let mut row = p.row;
        while !self.collides_at(p.shape, p.rot, p.col, row + 1) {
            row += 1;
        }
        row
    }

    /// Whether a cell of the active piece covers `(r, c)`.
    pub fn is_active_at(&self, r: usize, c: usize) -> bool {
        self.current
            .cells()
            .iter()
            .any(|&(cx, cy)| cx as usize == c && cy as usize == r)
    }

    /// Whether the ghost (landing preview) covers `(r, c)`.
    pub fn is_ghost_at(&self, r: usize, c: usize) -> bool {
        if self.state != GameState::Playing {
            return false;
        }
        let p = self.current;
        let ghost = self.ghost_row();
        if ghost == p.row {
            return false;
        }
        p.shape
            .offsets(p.rot)
            .iter()
            .any(|&(dx, dy)| p.col + dx == c as i32 && ghost + dy == r as i32)
    }

    /// Move the active piece horizontally; ignored when blocked.
    pub fn move_h(&mut self, dx: i32) {
        if self.state != GameState::Playing {
            return;
        }
        let p = self.current;
        if !self.collides_at(p.shape, p.rot, p.col + dx, p.row) {
            self.current.col += dx;
        }
    }

    /// Rotate the active piece with SRS wall kicks. `dir` > 0 is clockwise.
    pub fn rotate(&mut self, dir: i8) {
        if self.state != GameState::Playing {
            return;
        }
        let p = self.current;
        let to = if dir > 0 { (p.rot + 1) % 4 } else { (p.rot + 3) % 4 };
        for &(dc, dr) in kicks(p.shape, p.rot, to) {
            if !self.collides_at(p.shape, to, p.col + dc, p.row + dr) {
                self.current = ActivePiece {
                    rot: to,
                    col: p.col + dc,
                    row: p.row + dr,
                    ..p
                };
                return;
            }
        }
    }

    /// Drop one row, scoring a point; locks the piece when it cannot fall.
    pub fn soft_drop(&mut self) {
        if self.state != GameState::Playing {
            return;
        }
        let p = self.current;
        if !self.collides_at(p.shape, p.rot, p.col, p.row + 1) {
            self.current.row += 1;
            self.score += 1;
        } else {
            self.lock();
        }
    }

    /// Drop the piece to the floor, scoring 2 points per row, then lock it.
    pub fn hard_drop(&mut self) {
        if self.state != GameState::Playing {
            return;
        }
        let p = self.current;
        let mut row = p.row;
        while !self.collides_at(p.shape, p.rot, p.col, row + 1) {
            row += 1;
        }
        self.score += 2 * (row - p.row) as u32;
        self.current.row = row;
        self.lock();
    }

    /// Swap the active piece with the held piece; once per piece.
    pub fn hold(&mut self) {
        if self.state != GameState::Playing || !self.can_hold {
            return;
        }
        let cur = self.current.shape;
        let next = match self.hold {
            Some(held) => {
                self.hold = Some(cur);
                held
            }
            None => {
                self.hold = Some(cur);
                let shape = self.queue.pop_front().unwrap();
                self.refill_queue();
                shape
            }
        };
        self.current = Self::initial_piece(next);
        if self.collides_at(next, 0, self.current.col, self.current.row) {
            self.state = GameState::Over;
        }
        self.can_hold = false;
    }

    /// One gravity step: fall one row, or lock the piece at rest.
    pub fn tick(&mut self) {
        if self.state != GameState::Playing {
            return;
        }
        let p = self.current;
        if !self.collides_at(p.shape, p.rot, p.col, p.row + 1) {
            self.current.row += 1;
        } else {
            self.lock();
        }
    }

    /// Lock the active piece, clear lines, score and spawn the next piece.
    fn lock(&mut self) {
        let p = self.current;
        let mut topped_out = false;
        for (cx, cy) in p.cells() {
            if cy < 0 {
                topped_out = true;
                continue;
            }
            self.board[cy as usize][cx as usize] = Cell::Filled(p.shape);
        }
        self.can_hold = true;
        if topped_out {
            self.state = GameState::Over;
            return;
        }
        let cleared = self.clear_lines();
        self.score += [0, 100, 300, 500, 800][cleared] * self.level;
        self.spawn();
    }

    /// Remove full rows, shift the rest down and update lines/level.
    /// Returns how many rows were cleared.
    fn clear_lines(&mut self) -> usize {
        let kept: Vec<[Cell; COLS]> = self
            .board
            .iter()
            .filter(|row| row.contains(&Cell::Empty))
            .copied()
            .collect();
        let cleared = ROWS - kept.len();
        let mut rows = kept;
        while rows.len() < ROWS {
            rows.push([Cell::Empty; COLS]);
        }
        self.board = rows.try_into().unwrap();
        self.lines += cleared as u32;
        self.level = 1 + self.lines / 10;
        cleared
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_current(game: &mut Game, shape: PieceType, rot: u8, col: i32, row: i32) {
        game.current = ActivePiece { shape, rot, col, row };
    }

    #[test]
    fn spawn_is_centered_and_valid() {
        let game = Game::new(42);
        assert_eq!(game.state, GameState::Playing);
        assert!(!game.collides_at(
            game.current.shape,
            game.current.rot,
            game.current.col,
            game.current.row
        ));
        for (cx, cy) in game.current.cells() {
            assert!((0..COLS as i32).contains(&cx));
            assert!((0..ROWS as i32).contains(&cy));
        }
    }

    #[test]
    fn piece_offsets_rotate_correctly() {
        assert_eq!(
            PieceType::T.offsets(0),
            [(1, 0), (0, 1), (1, 1), (2, 1)]
        );
        assert_eq!(
            PieceType::T.offsets(1),
            [(2, 1), (1, 0), (1, 1), (1, 2)]
        );
        assert_eq!(
            PieceType::T.offsets(2),
            [(1, 2), (2, 1), (1, 1), (0, 1)]
        );
        assert_eq!(
            PieceType::T.offsets(3),
            [(0, 1), (1, 2), (1, 1), (1, 0)]
        );
        assert_eq!(PieceType::T.offsets(4), PieceType::T.offsets(0));

        assert_eq!(
            PieceType::I.offsets(0),
            [(0, 1), (1, 1), (2, 1), (3, 1)]
        );
        assert_eq!(
            PieceType::I.offsets(1),
            [(2, 0), (2, 1), (2, 2), (2, 3)]
        );
        // O's footprint is unaffected by rotation (cell order may permute).
        let o0: std::collections::BTreeSet<(i32, i32)> =
            PieceType::O.offsets(0).iter().copied().collect();
        let o3: std::collections::BTreeSet<(i32, i32)> =
            PieceType::O.offsets(3).iter().copied().collect();
        assert_eq!(o0, o3);
    }

    #[test]
    fn seven_bag_is_fair() {
        let mut game = Game::new(7);
        let mut drawn = vec![game.current.shape];
        while drawn.len() < 28 {
            drawn.push(game.queue.pop_front().unwrap());
            game.refill_queue();
        }
        for chunk in drawn.chunks(7) {
            for piece in PieceType::ALL {
                assert_eq!(
                    chunk.iter().filter(|p| **p == piece).count(),
                    1,
                    "bag {chunk:?} is not a full permutation"
                );
            }
        }
    }

    #[test]
    fn walls_block_horizontal_movement() {
        let mut game = Game::new(1);
        set_current(&mut game, PieceType::O, 0, 0, 0);
        game.move_h(-1);
        assert_eq!(game.current.col, 0);
        game.move_h(1);
        game.move_h(1);
        assert_eq!(game.current.col, 2);
    }

    #[test]
    fn wall_kick_allows_rotation_against_wall() {
        let mut game = Game::new(1);
        // Vertical I in the leftmost column: box col -2, cells in board col 0.
        set_current(&mut game, PieceType::I, 1, -2, 5);
        assert!(!game.collides_at(PieceType::I, 1, -2, 5));
        game.rotate(1);
        // The (2, 0) kick shifts the box to col 0 so the horizontal I fits.
        assert_eq!(game.current.rot, 2);
        assert_eq!(game.current.col, 0);
        assert!(!game.collides_at(PieceType::I, 2, 0, 5));
    }

    #[test]
    fn rotation_is_blocked_without_a_valid_kick() {
        let mut game = Game::new(1);
        set_current(&mut game, PieceType::O, 0, 0, 0);
        game.rotate(1);
        // O has no kicks; a rotation is a no-op for its shape anyway.
        assert_eq!(game.current.rot, 1);
        assert!(!game.collides_at(PieceType::O, 1, 0, 0));
    }

    #[test]
    fn soft_drop_moves_down_and_scores() {
        let mut game = Game::new(1);
        set_current(&mut game, PieceType::O, 0, 4, 0);
        game.soft_drop();
        assert_eq!(game.current.row, 1);
        assert_eq!(game.score, 1);
    }

    #[test]
    fn soft_drop_locks_when_blocked() {
        let mut game = Game::new(1);
        set_current(&mut game, PieceType::O, 0, 4, 18);
        game.soft_drop();
        // Piece locked at rows 18-19 and a new piece spawned.
        let filled = game
            .board[19]
            .iter()
            .filter(|c| matches!(c, Cell::Filled(PieceType::O)))
            .count();
        assert_eq!(filled, 2);
        assert_eq!(game.state, GameState::Playing);
        assert!(game.current.row <= 1);
    }

    #[test]
    fn hard_drop_clears_line_and_scores() {
        let mut game = Game::new(1);
        set_current(&mut game, PieceType::I, 0, 3, 0);
        for c in 0..COLS {
            if !(3..=6).contains(&c) {
                game.board[19][c] = Cell::Filled(PieceType::O);
            }
        }
        game.hard_drop();
        // I falls from board row 1 to row 19 (18 rows x 2 pts) plus 100 for
        // a single line clear at level 1.
        assert_eq!(game.lines, 1);
        assert_eq!(game.score, 136);
        assert!(game.board[19].iter().all(|c| *c == Cell::Empty));
        assert_eq!(game.state, GameState::Playing);
    }

    #[test]
    fn level_ups_every_ten_lines() {
        let mut game = Game::new(1);
        for _ in 0..10 {
            set_current(&mut game, PieceType::I, 0, 3, 0);
            for c in 0..COLS {
                if !(3..=6).contains(&c) {
                    game.board[19][c] = Cell::Filled(PieceType::O);
                }
            }
            game.hard_drop();
        }
        assert_eq!(game.lines, 10);
        assert_eq!(game.level, 2);
        assert!(game.tick_ms() < Game::new(1).tick_ms());
    }

    #[test]
    fn gravity_tick_moves_down_and_locks_at_floor() {
        let mut game = Game::new(1);
        set_current(&mut game, PieceType::O, 0, 4, 0);
        game.tick();
        assert_eq!(game.current.row, 1);
        for _ in 0..30 {
            game.tick();
        }
        let filled = game
            .board[19]
            .iter()
            .filter(|c| matches!(c, Cell::Filled(PieceType::O)))
            .count();
        assert_eq!(filled, 2);
    }

    #[test]
    fn ghost_row_lands_on_floor_or_stack() {
        let mut game = Game::new(1);
        set_current(&mut game, PieceType::T, 0, 4, 0);
        // T's lowest cells sit one row below the box origin.
        assert_eq!(game.ghost_row(), 18);
        game.board[10][4] = Cell::Filled(PieceType::J);
        // The box origin rests one row above the lowest cells, which sit on
        // top of the stack at row 9.
        assert_eq!(game.ghost_row(), 8);
    }

    #[test]
    fn ghost_is_hidden_while_playing() {
        let mut game = Game::new(1);
        set_current(&mut game, PieceType::T, 0, 4, 0);
        assert!(game.is_ghost_at(19, 4));
        assert!(!game.is_ghost_at(0, 9));
    }

    #[test]
    fn hold_once_per_piece() {
        let mut game = Game::new(1);
        let first = game.current.shape;
        game.hold();
        assert_eq!(game.hold, Some(first));
        let second = game.current.shape;
        game.hold();
        assert_eq!(game.current.shape, second, "second hold must be ignored");
        game.hard_drop();
        game.hold();
        assert_eq!(game.current.shape, first, "hold must swap back after locking");
    }

    #[test]
    fn spawning_on_a_blocked_top_is_game_over() {
        let mut game = Game::new(1);
        // Occupy the spawn rows, leaving one column empty so the rows do
        // not clear when the piece locks.
        for c in 0..9 {
            game.board[0][c] = Cell::Filled(PieceType::J);
            game.board[1][c] = Cell::Filled(PieceType::J);
        }
        set_current(&mut game, PieceType::O, 0, 4, 0);
        game.tick();
        assert_eq!(game.state, GameState::Over);
    }

    #[test]
    fn no_input_changes_a_finished_or_paused_game() {
        let mut game = Game::new(1);
        game.state = GameState::Over;
        let piece = game.current;
        let score = game.score;
        game.move_h(1);
        game.rotate(1);
        game.tick();
        game.hard_drop();
        assert_eq!(game.current, piece);
        assert_eq!(game.score, score);
    }
}
