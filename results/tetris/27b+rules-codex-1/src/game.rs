//! Core Tetris game logic, independent of the UI.

use std::collections::VecDeque;

pub const COLS: usize = 10;
pub const VISIBLE_ROWS: usize = 20;
pub const HIDDEN_ROWS: usize = 1;
pub const ROWS: usize = HIDDEN_ROWS + VISIBLE_ROWS;

/// A single grid cell: empty or colored by the piece that locked there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cell {
    #[default]
    Empty,
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

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

impl From<PieceType> for Cell {
    fn from(ty: PieceType) -> Self {
        match ty {
            PieceType::I => Cell::I,
            PieceType::O => Cell::O,
            PieceType::T => Cell::T,
            PieceType::S => Cell::S,
            PieceType::Z => Cell::Z,
            PieceType::J => Cell::J,
            PieceType::L => Cell::L,
        }
    }
}

impl PieceType {
    const ALL: [PieceType; 7] = [
        PieceType::I,
        PieceType::O,
        PieceType::T,
        PieceType::S,
        PieceType::Z,
        PieceType::J,
        PieceType::L,
    ];

    /// Side length of the rotation bounding box.
    pub fn size(self) -> usize {
        match self {
            PieceType::I | PieceType::O => 4,
            _ => 3,
        }
    }

    /// Spawn-state grid (SRS), zero-padded to 4x4.
    fn spawn_matrix(self) -> [[u8; 4]; 4] {
        let mut m = [[0u8; 4]; 4];
        match self {
            PieceType::I => m[1] = [1, 1, 1, 1],
            PieceType::O => {
                m[1][1] = 1;
                m[1][2] = 1;
                m[2][1] = 1;
                m[2][2] = 1;
            }
            PieceType::T => {
                m[0][1] = 1;
                m[1][0] = 1;
                m[1][1] = 1;
                m[1][2] = 1;
            }
            PieceType::S => {
                m[0][1] = 1;
                m[0][2] = 1;
                m[1][0] = 1;
                m[1][1] = 1;
            }
            PieceType::Z => {
                m[0][0] = 1;
                m[0][1] = 1;
                m[1][1] = 1;
                m[1][2] = 1;
            }
            PieceType::J => {
                m[0][0] = 1;
                m[1][0] = 1;
                m[1][1] = 1;
                m[1][2] = 1;
            }
            PieceType::L => {
                m[0][2] = 1;
                m[1][0] = 1;
                m[1][1] = 1;
                m[1][2] = 1;
            }
        }
        m
    }

    /// Cell offsets (row, col) within the bounding box for a rotation state.
    pub fn cells(self, rot: u8) -> [(usize, usize); 4] {
        let n = self.size();
        let m = self.spawn_matrix();
        let mut out = [(0usize, 0usize); 4];
        let mut idx = 0;
        for (row, line) in m.iter().enumerate().take(n) {
            for (col, filled) in line.iter().enumerate().take(n) {
                if *filled == 1 {
                    out[idx] = (row, col);
                    idx += 1;
                }
            }
        }
        // Clockwise rotation of an offset inside an n x n box.
        for _ in 0..(rot % 4) {
            for cell in &mut out {
                let (row, col) = *cell;
                *cell = (col, n - 1 - row);
            }
        }
        out
    }
}

/// Deterministic xorshift64* PRNG so games are reproducible from a seed.
#[derive(Debug, Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0x9E3779B97F4A7C15
        } else {
            seed
        })
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.0 = value;
        value.wrapping_mul(0x2545F4914F6CDD1D)
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }
}

#[derive(Debug, Clone, Default)]
pub struct Board {
    cells: [[Cell; COLS]; ROWS],
}

impl Board {
    pub fn is_empty(&self, row: usize, col: usize) -> bool {
        self.cells[row][col] == Cell::Empty
    }

    pub fn at(&self, row: usize, col: usize) -> Cell {
        self.cells[row][col]
    }

    pub fn set(&mut self, row: usize, col: usize, cell: Cell) {
        self.cells[row][col] = cell;
    }

    pub fn is_full_row(&self, row: usize) -> bool {
        self.cells[row].iter().all(|c| *c != Cell::Empty)
    }

    /// Remove all full rows, shift the rest down, return how many were cleared.
    pub fn clear_full_rows(&mut self) -> u32 {
        let mut cleared = 0;
        let mut write = ROWS - 1;
        for row in (0..ROWS).rev() {
            if self.is_full_row(row) {
                cleared += 1;
            } else {
                if write != row {
                    self.cells[write] = self.cells[row];
                }
                write = write.saturating_sub(1);
            }
        }
        for row in 0..cleared as usize {
            self.cells[row] = [Cell::Empty; COLS];
        }
        cleared
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    Playing,
    Paused,
    Over,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationDir {
    Clockwise,
    CounterClockwise,
}

#[derive(Debug)]
pub struct Game {
    board: Board,
    piece: PieceType,
    rot: u8,
    x: i16,
    y: i16,
    next: PieceType,
    bag: VecDeque<PieceType>,
    rng: Rng,
    score: u64,
    lines: u32,
    level: u32,
    state: GameState,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        let mut game = Self {
            board: Board::default(),
            piece: PieceType::I,
            rot: 0,
            x: 3,
            y: 0,
            next: PieceType::I,
            bag: VecDeque::new(),
            rng: Rng::new(seed),
            score: 0,
            lines: 0,
            level: 0,
            state: GameState::Playing,
        };
        game.next = game.draw_from_bag();
        game.spawn();
        game
    }

    pub fn restart(&mut self, seed: u64) {
        *self = Self::new(seed);
    }

    pub fn state(&self) -> GameState {
        self.state
    }

    pub fn score(&self) -> u64 {
        self.score
    }

    pub fn lines(&self) -> u32 {
        self.lines
    }

    pub fn level(&self) -> u32 {
        self.level
    }

    pub fn next(&self) -> PieceType {
        self.next
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    /// Milliseconds per gravity step at the current level.
    pub fn tick_delay_ms(&self) -> u64 {
        (800 - self.level * 60).clamp(60, 800) as u64
    }

    pub fn toggle_pause(&mut self) {
        self.state = match self.state {
            GameState::Playing => GameState::Paused,
            GameState::Paused => GameState::Playing,
            GameState::Over => GameState::Over,
        };
    }

    /// Move the falling piece one gravity step, locking it if it cannot fall.
    pub fn tick(&mut self) {
        if self.state != GameState::Playing {
            return;
        }
        if !self.move_piece(0, 1) {
            self.lock();
        }
    }

    pub fn move_h(&mut self, dx: i16) -> bool {
        self.move_piece(dx, 0)
    }

    pub fn soft_drop(&mut self) -> bool {
        if self.move_piece(0, 1) {
            self.score += 1;
            true
        } else {
            false
        }
    }

    pub fn hard_drop(&mut self) {
        if self.state != GameState::Playing {
            return;
        }
        let mut distance = 0;
        while self.move_piece(0, 1) {
            distance += 1;
        }
        self.score += 2 * distance as u64;
        self.lock();
    }

    pub fn rotate(&mut self, dir: RotationDir) -> bool {
        if self.state != GameState::Playing {
            return false;
        }
        if self.piece == PieceType::O {
            return true;
        }
        let to = (self.rot + match dir {
            RotationDir::Clockwise => 1,
            RotationDir::CounterClockwise => 3,
        }) % 4;
        for (drow, dcol) in kick_table(self.piece, self.rot, to) {
            if !self.collides(self.piece, to, self.x + dcol, self.y + drow) {
                self.rot = to;
                self.x += dcol;
                self.y += drow;
                return true;
            }
        }
        false
    }

    /// Row the piece would rest on if dropped right now.
    pub fn ghost_y(&self) -> i16 {
        let mut y = self.y;
        while !self.collides(self.piece, self.rot, self.x, y + 1) {
            y += 1;
        }
        y
    }

    /// Absolute (row, col, color) cells of the falling piece.
    pub fn active_cells(&self) -> Vec<(usize, usize, Cell)> {
        self.piece
            .cells(self.rot)
            .into_iter()
            .map(|(dr, dc)| (self.y + dr as i16, self.x + dc as i16))
            .filter(|(row, _)| *row >= 0)
            .map(|(row, col)| (row as usize, col as usize, self.piece.into()))
            .collect()
    }

    /// Absolute (row, col) cells of the ghost (drop preview).
    pub fn ghost_cells(&self) -> Vec<(usize, usize)> {
        let ghost_y = self.ghost_y();
        self.piece
            .cells(self.rot)
            .into_iter()
            .map(|(dr, dc)| (ghost_y + dr as i16, self.x + dc as i16))
            .filter(|(row, _)| *row >= 0)
            .map(|(row, col)| (row as usize, col as usize))
            .collect()
    }

    fn move_piece(&mut self, dx: i16, dy: i16) -> bool {
        if self.state != GameState::Playing {
            return false;
        }
        if self.collides(self.piece, self.rot, self.x + dx, self.y + dy) {
            return false;
        }
        self.x += dx;
        self.y += dy;
        true
    }

    fn collides(&self, ty: PieceType, rot: u8, x: i16, y: i16) -> bool {
        for (dr, dc) in ty.cells(rot) {
            let row = y + dr as i16;
            let col = x + dc as i16;
            if col < 0 || col >= COLS as i16 || row >= ROWS as i16 {
                return true;
            }
            if row >= 0 && !self.board.is_empty(row as usize, col as usize) {
                return true;
            }
        }
        false
    }

    fn lock(&mut self) {
        for (dr, dc) in self.piece.cells(self.rot) {
            let row = self.y + dr as i16;
            let col = self.x + dc as i16;
            if row >= 0 {
                self.board.set(row as usize, col as usize, self.piece.into());
            }
        }
        let cleared = self.board.clear_full_rows();
        if cleared > 0 {
            static CLEAR_TABLE: [u64; 5] = [0, 100, 300, 500, 800];
            self.score += CLEAR_TABLE[cleared as usize] * (self.level + 1) as u64;
            self.lines += cleared;
            self.level = self.lines / 10;
        }
        self.spawn();
    }

    fn spawn(&mut self) {
        let ty = self.next;
        self.next = self.draw_from_bag();
        self.piece = ty;
        self.rot = 0;
        self.x = ((COLS - ty.size()) / 2) as i16;
        self.y = 0;
        if self.collides(ty, 0, self.x, self.y) {
            self.state = GameState::Over;
        }
    }

    fn draw_from_bag(&mut self) -> PieceType {
        if self.bag.is_empty() {
            let mut bag = PieceType::ALL.to_vec();
            for i in (1..bag.len()).rev() {
                let j = self.rng.below(i + 1);
                bag.swap(i, j);
            }
            self.bag = bag.into_iter().collect();
        }
        self.bag
            .pop_front()
            .expect("bag was just refilled")
    }

    #[cfg(test)]
    pub fn force_piece(&mut self, ty: PieceType, rot: u8, x: i16, y: i16) {
        self.piece = ty;
        self.rot = rot;
        self.x = x;
        self.y = y;
    }

    #[cfg(test)]
    pub fn force_spawn(&mut self) {
        self.spawn();
    }
}

/// SRS wall-kick offsets as (row, col) deltas with row positive down.
fn kick_table(ty: PieceType, from: u8, to: u8) -> [(i16, i16); 5] {
    match ty {
        PieceType::O => [(0, 0), (0, 0), (0, 0), (0, 0), (0, 0)],
        PieceType::I => match (from, to) {
            (0, 1) => [(0, 0), (0, -2), (0, 1), (1, -2), (-2, 1)],
            (1, 0) => [(0, 0), (0, 2), (0, -1), (-1, 2), (2, -1)],
            (1, 2) => [(0, 0), (0, -1), (0, 2), (-2, -1), (1, 2)],
            (2, 1) => [(0, 0), (0, 1), (0, -2), (2, -1), (-2, 1)],
            (2, 3) => [(0, 0), (0, 2), (0, -1), (-1, 2), (2, -1)],
            (3, 2) => [(0, 0), (0, -2), (0, 1), (1, -2), (-2, 1)],
            (3, 0) => [(0, 0), (0, 1), (0, -2), (2, -1), (-2, 1)],
            (0, 3) => [(0, 0), (0, -1), (0, 2), (-2, -1), (1, 2)],
            _ => unreachable!("rotation has exactly 8 transitions"),
        },
        _ => match (from, to) {
            (0, 1) => [(0, 0), (0, -1), (-1, -1), (2, 0), (2, -1)],
            (1, 0) => [(0, 0), (0, 1), (1, 1), (-2, 0), (-2, 1)],
            (1, 2) => [(0, 0), (0, 1), (1, 1), (-2, 0), (-2, 1)],
            (2, 1) => [(0, 0), (0, -1), (-1, -1), (2, 0), (2, -1)],
            (2, 3) => [(0, 0), (0, 1), (1, 1), (2, 0), (2, -1)],
            (3, 2) => [(0, 0), (0, -1), (-1, -1), (-2, 0), (-2, -1)],
            (3, 0) => [(0, 0), (0, -1), (-1, -1), (-2, 0), (-2, -1)],
            (0, 3) => [(0, 0), (0, 1), (1, 1), (2, 0), (2, -1)],
            _ => unreachable!("rotation has exactly 8 transitions"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn game() -> Game {
        Game::new(42)
    }

    fn piece_index(ty: PieceType) -> usize {
        match ty {
            PieceType::I => 0,
            PieceType::O => 1,
            PieceType::T => 2,
            PieceType::S => 3,
            PieceType::Z => 4,
            PieceType::J => 5,
            PieceType::L => 6,
        }
    }

    fn fill_all_but_col(game: &mut Game, rows: std::ops::Range<usize>, skip_col: usize) {
        for row in rows {
            for col in 0..COLS {
                if col != skip_col {
                    game.board.set(row, col, Cell::I);
                }
            }
        }
    }

    #[test]
    fn every_piece_has_four_unique_cells_in_each_rotation() {
        for ty in PieceType::ALL {
            for rot in 0..4 {
                let mut cells = ty.cells(rot).to_vec();
                cells.sort_unstable();
                cells.dedup();
                assert_eq!(cells.len(), 4, "{ty:?} rot {rot}");
            }
        }
    }

    fn sorted(cells: &[(usize, usize); 4]) -> Vec<(usize, usize)> {
        let mut v = cells.to_vec();
        v.sort_unstable();
        v
    }

    #[test]
    fn o_piece_rotation_is_invariant() {
        let base = sorted(&PieceType::O.cells(0));
        for rot in 1..4 {
            assert_eq!(base, sorted(&PieceType::O.cells(rot)));
        }
    }

    #[test]
    fn new_game_is_playing_with_a_visible_piece() {
        let game = game();
        assert_eq!(game.state(), GameState::Playing);
        let cells = game.active_cells();
        assert!(!cells.is_empty());
        for (row, col, _) in cells {
            assert!(row < ROWS && col < COLS);
        }
    }

    #[test]
    fn bag_deals_each_piece_exactly_once() {
        let mut game = game();
        game.bag.clear();
        let mut seen = [false; 7];
        for _ in 0..7 {
            seen[piece_index(game.draw_from_bag())] = true;
        }
        assert!(seen.iter().all(|s| *s));
    }

    #[test]
    fn piece_cannot_leave_the_board() {
        let mut game = game();
        game.force_piece(PieceType::T, 0, 3, 5);
        for _ in 0..3 {
            assert!(game.move_h(-1));
        }
        assert_eq!(game.x, 0);
        assert!(!game.move_h(-1));
        for _ in 0..COLS - 3 {
            assert!(game.move_h(1));
        }
        assert_eq!(game.x, (COLS - 3) as i16);
        assert!(!game.move_h(1));
    }

    #[test]
    fn gravity_locks_piece_at_floor() {
        let mut game = game();
        game.force_piece(PieceType::T, 0, 3, ROWS as i16 - 2);
        assert!(!game.soft_drop());
        game.tick();
        for (row, col) in [(ROWS - 2, 4), (ROWS - 1, 3), (ROWS - 1, 4), (ROWS - 1, 5)] {
            assert_eq!(game.board.at(row, col), Cell::T);
        }
        // The next piece spawned at the top of the board.
        assert_eq!(game.y, 0);
        assert_eq!(game.state(), GameState::Playing);
    }

    #[test]
    fn hard_drop_scores_and_locks() {
        let mut game = game();
        game.force_piece(PieceType::T, 0, 3, 0);
        game.hard_drop();
        // T bottom sits one box row below its top: 19 steps down from spawn.
        assert_eq!(game.score, 2 * (ROWS - 2) as u64);
        assert_eq!(game.board.at(ROWS - 1, 3), Cell::T);
    }

    #[test]
    fn soft_drop_scores_one_per_step() {
        let mut game = game();
        game.force_piece(PieceType::T, 0, 3, 0);
        assert!(game.soft_drop());
        assert_eq!(game.score, 1);
        assert_eq!(game.y, 1);
    }

    #[test]
    fn single_line_clear_scores_100() {
        let mut game = game();
        fill_all_but_col(&mut game, ROWS - 1..ROWS, 8);
        game.force_piece(PieceType::I, 1, 6, 0);
        game.hard_drop();
        assert_eq!(game.lines, 1);
        assert_eq!(game.score, 100 + 2 * (ROWS - 4) as u64);
        assert_eq!(game.board.at(ROWS - 1, 0), Cell::Empty);
    }

    #[test]
    fn tetris_clear_scores_800() {
        let mut game = game();
        fill_all_but_col(&mut game, (ROWS - 4)..ROWS, 8);
        game.force_piece(PieceType::I, 1, 6, 0);
        game.hard_drop();
        assert_eq!(game.lines, 4);
        assert_eq!(game.score, 800 + 2 * (ROWS - 4) as u64);
    }

    #[test]
    fn level_rises_every_ten_lines() {
        let mut game = game();
        for _ in 0..3 {
            fill_all_but_col(&mut game, (ROWS - 4)..ROWS, 8);
            game.force_piece(PieceType::I, 1, 6, 0);
            game.hard_drop();
        }
        assert_eq!(game.lines, 12);
        assert_eq!(game.level(), 1);
        // Level 1 gravity is faster than the level 0 baseline.
        assert!(game.tick_delay_ms() < 800);
    }

    #[test]
    fn i_piece_wall_kick_at_right_wall() {
        let mut game = game();
        game.force_piece(PieceType::I, 1, 7, 5);
        assert!(game.rotate(RotationDir::Clockwise));
        assert_eq!(game.rot, 2);
        assert_eq!(game.x, 6);
        assert_eq!(game.y, 5);
    }

    #[test]
    fn t_rotation_shape_matches_srs() {
        let mut game = game();
        game.force_piece(PieceType::T, 0, 3, 2);
        assert!(game.rotate(RotationDir::Clockwise));
        let cells: HashSet<(usize, usize)> = game
            .active_cells()
            .into_iter()
            .map(|(row, col, _)| (row, col))
            .collect();
        assert_eq!(cells, [(2, 4), (3, 4), (3, 5), (4, 4)].into_iter().collect());
    }

    #[test]
    fn ghost_sits_on_floor_of_empty_board() {
        let game = game();
        let max_row = game
            .ghost_cells()
            .iter()
            .map(|(row, _)| *row)
            .max()
            .expect("ghost has cells");
        assert_eq!(max_row, ROWS - 1);
    }

    #[test]
    fn blockout_ends_the_game() {
        let mut game = game();
        for row in 0..2 {
            for col in 0..COLS {
                game.board.set(row, col, Cell::J);
            }
        }
        game.force_spawn();
        assert_eq!(game.state(), GameState::Over);
        game.tick();
        game.hard_drop();
        assert_eq!(game.state(), GameState::Over);
    }

    #[test]
    fn pause_freezes_input() {
        let mut game = game();
        game.toggle_pause();
        assert_eq!(game.state(), GameState::Paused);
        assert!(!game.move_h(1));
        assert!(!game.rotate(RotationDir::Clockwise));
        assert!(!game.soft_drop());
        let score = game.score();
        game.tick();
        assert_eq!(game.score(), score);
        game.toggle_pause();
        assert_eq!(game.state(), GameState::Playing);
    }
}
