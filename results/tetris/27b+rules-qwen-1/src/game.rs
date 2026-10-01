//! Pure Tetris game logic, independent of rendering.

use std::time::Duration;

use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};

/// Board width in cells.
pub const COLS: usize = 10;
/// Visible board height in cells.
pub const ROWS: usize = 20;

/// The seven tetromino kinds, in bag order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl Kind {
    pub const ALL: [Kind; 7] = [Kind::I, Kind::O, Kind::T, Kind::S, Kind::Z, Kind::J, Kind::L];

    pub const fn from_idx(i: usize) -> Kind {
        Self::ALL[i % Self::ALL.len()]
    }

    /// Side length of the bounding square that contains the piece.
    pub const fn box_size(self) -> usize {
        match self {
            Kind::I => 4,
            Kind::O => 2,
            _ => 3,
        }
    }

    /// Cell offsets `(x, y)` for rotation state `rot % 4`, relative to the
    /// top-left of the bounding square.
    pub const fn shape_cells(self, rot: usize) -> [(usize, usize); 4] {
        const I: [[(usize, usize); 4]; 4] = [
            [(0, 1), (1, 1), (2, 1), (3, 1)],
            [(2, 0), (2, 1), (2, 2), (2, 3)],
            [(0, 2), (1, 2), (2, 2), (3, 2)],
            [(1, 0), (1, 1), (1, 2), (1, 3)],
        ];
        const O: [[(usize, usize); 4]; 4] = [
            [(0, 0), (1, 0), (0, 1), (1, 1)],
            [(0, 0), (1, 0), (0, 1), (1, 1)],
            [(0, 0), (1, 0), (0, 1), (1, 1)],
            [(0, 0), (1, 0), (0, 1), (1, 1)],
        ];
        const T: [[(usize, usize); 4]; 4] = [
            [(1, 0), (0, 1), (1, 1), (2, 1)],
            [(1, 0), (1, 1), (2, 1), (1, 2)],
            [(0, 1), (1, 1), (2, 1), (1, 2)],
            [(1, 0), (0, 1), (1, 1), (1, 2)],
        ];
        const S: [[(usize, usize); 4]; 4] = [
            [(1, 0), (2, 0), (0, 1), (1, 1)],
            [(1, 0), (1, 1), (2, 1), (2, 2)],
            [(1, 1), (2, 1), (0, 2), (1, 2)],
            [(0, 0), (0, 1), (1, 1), (1, 2)],
        ];
        const Z: [[(usize, usize); 4]; 4] = [
            [(0, 0), (1, 0), (1, 1), (2, 1)],
            [(2, 0), (1, 1), (2, 1), (1, 2)],
            [(0, 1), (1, 1), (1, 2), (2, 2)],
            [(1, 0), (0, 1), (1, 1), (0, 2)],
        ];
        const J: [[(usize, usize); 4]; 4] = [
            [(0, 0), (0, 1), (1, 1), (2, 1)],
            [(1, 0), (2, 0), (1, 1), (1, 2)],
            [(0, 1), (1, 1), (2, 1), (2, 2)],
            [(1, 0), (1, 1), (0, 2), (1, 2)],
        ];
        const L: [[(usize, usize); 4]; 4] = [
            [(2, 0), (0, 1), (1, 1), (2, 1)],
            [(1, 0), (1, 1), (1, 2), (2, 2)],
            [(0, 1), (1, 1), (2, 1), (0, 2)],
            [(0, 0), (1, 0), (1, 1), (1, 2)],
        ];
        let states = match self {
            Kind::I => I,
            Kind::O => O,
            Kind::T => T,
            Kind::S => S,
            Kind::Z => Z,
            Kind::J => J,
            Kind::L => L,
        };
        states[rot % 4]
    }
}

/// A falling piece: a kind at a rotation state and board position.
/// `x`/`y` are the top-left of the bounding square; `y` may be negative
/// (the hidden spawn zone above the visible board).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub kind: Kind,
    pub rot: usize,
    pub x: isize,
    pub y: isize,
}

impl Piece {
    /// Spawn position for a fresh piece.
    pub fn new(kind: Kind) -> Self {
        Self {
            kind,
            rot: 0,
            x: ((COLS as isize) - kind.box_size() as isize) / 2,
            y: -1,
        }
    }

    /// Absolute cell positions as `(col, row)`; row may be negative.
    pub fn cells(&self) -> [(isize, isize); 4] {
        self.kind
            .shape_cells(self.rot)
            .map(|(cx, cy)| (self.x + cx as isize, self.y + cy as isize))
    }

    pub fn moved(&self, dx: isize, dy: isize) -> Self {
        let mut p = *self;
        p.x += dx;
        p.y += dy;
        p
    }

    pub fn rotated(&self, clockwise: bool) -> Self {
        let mut p = *self;
        p.rot = if clockwise {
            (p.rot + 1) % 4
        } else {
            (p.rot + 3) % 4
        };
        p
    }
}

/// The 10x20 field. Cell values: `0` is empty, `1..=7` is `Kind as u8 + 1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    grid: [[u8; COLS]; ROWS],
}

impl Board {
    pub fn new() -> Self {
        Self {
            grid: [[0; COLS]; ROWS],
        }
    }

    #[cfg(test)]
    pub fn cell(&self, row: usize, col: usize) -> u8 {
        self.grid[row][col]
    }

    pub fn rows(&self) -> &[[u8; COLS]] {
        &self.grid
    }

    #[cfg(test)]
    pub fn filled_cells(&self) -> usize {
        self.grid.iter().flatten().filter(|&&v| v != 0).count()
    }

    /// Whether a cell is empty. Cells above the visible field (negative row)
    /// are always free; out-of-bounds cells are not.
    pub fn is_free(&self, row: isize, col: isize) -> bool {
        if col < 0 || col >= COLS as isize || row >= ROWS as isize {
            return false;
        }
        if row < 0 {
            return true;
        }
        self.grid[row as usize][col as usize] == 0
    }

    pub fn fits(&self, piece: &Piece) -> bool {
        piece.cells().iter().all(|&(c, r)| self.is_free(r, c))
    }

    /// Stamp the piece into the board. Returns `false` if any cell ended up
    /// in the hidden zone above the board (top-out).
    pub fn lock(&mut self, piece: &Piece) -> bool {
        let mut top_out = false;
        for &(c, r) in &piece.cells() {
            if r < 0 {
                top_out = true;
                continue;
            }
            self.grid[r as usize][c as usize] = piece.kind as u8 + 1;
        }
        !top_out
    }

    /// Remove all complete rows and return how many were cleared.
    pub fn clear_lines(&mut self) -> usize {
        let mut kept: Vec<[u8; COLS]> = Vec::new();
        let mut cleared = 0;
        for row in &self.grid {
            if row.iter().all(|&v| v != 0) {
                cleared += 1;
            } else {
                kept.push(*row);
            }
        }
        self.grid[cleared..].copy_from_slice(&kept);
        for row in &mut self.grid[..cleared] {
            *row = [0; COLS];
        }
        cleared
    }

    #[cfg(test)]
    pub fn set_cell(&mut self, row: usize, col: usize, kind: Option<Kind>) {
        self.grid[row][col] = kind.map_or(0, |k| k as u8 + 1);
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Playing,
    Paused,
    Over,
}

/// Scores for clearing 1..=4 lines at once, before the level multiplier.
const LINE_SCORES: [u64; 5] = [0, 100, 300, 500, 800];

/// Wall-kick offsets tried when rotating.
const KICKS: [(isize, isize); 6] = [(0, 0), (-1, 0), (1, 0), (0, -1), (-2, 0), (2, 0)];

pub struct Game {
    board: Board,
    piece: Option<Piece>,
    queue: Vec<Kind>,
    bag: Vec<Kind>,
    score: u64,
    lines: u32,
    level: u32,
    status: Status,
    rng: StdRng,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        let mut game = Self {
            board: Board::new(),
            piece: None,
            queue: Vec::new(),
            bag: Vec::new(),
            score: 0,
            lines: 0,
            level: 1,
            status: Status::Playing,
            rng: StdRng::seed_from_u64(seed),
        };
        let first = game.take_from_bag();
        game.queue.push(first);
        game.spawn();
        debug_assert!(game.piece.is_some());
        game
    }

    // ----- queries ---------------------------------------------------------

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn current(&self) -> Option<Piece> {
        self.piece
    }

    pub fn next(&self) -> Option<Kind> {
        self.queue.first().copied()
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

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn is_over(&self) -> bool {
        self.status == Status::Over
    }

    pub fn is_paused(&self) -> bool {
        self.status == Status::Paused
    }

    /// Rows the current piece can still fall before it would collide on the
    /// next row (the resting offset for a hard drop / ghost).
    pub fn drop_distance(&self) -> usize {
        let Some(piece) = self.piece else {
            return 0;
        };
        let mut d = 0;
        while self.board.fits(&piece.moved(0, (d + 1) as isize)) {
            d += 1;
        }
        d
    }

    /// Time the current piece stays in one row before gravity pulls it down.
    pub fn gravity_interval(&self) -> Duration {
        let ms = 800u64.saturating_sub((self.level - 1) as u64 * 70).max(50);
        Duration::from_millis(ms)
    }

    /// Level derived from total cleared lines (one level per 10 lines).
    pub const fn level_for_lines(lines: u32) -> u32 {
        lines / 10 + 1
    }

    // ----- input -----------------------------------------------------------

    pub fn toggle_pause(&mut self) {
        self.status = match self.status {
            Status::Playing => Status::Paused,
            Status::Paused => Status::Playing,
            Status::Over => Status::Over,
        };
    }

    pub fn move_left(&mut self) {
        self.try_move(-1, 0);
    }

    pub fn move_right(&mut self) {
        self.try_move(1, 0);
    }

    pub fn rotate_cw(&mut self) {
        self.rotate(true);
    }

    pub fn rotate_ccw(&mut self) {
        self.rotate(false);
    }

    /// Move one row down, scoring 1 point; locks the piece if it can't move.
    pub fn soft_drop(&mut self) {
        if !self.can_act() {
            return;
        }
        if !self.try_move(0, 1) {
            self.lock_piece();
        } else {
            self.score += 1;
        }
    }

    /// Drop the piece to the floor immediately, scoring 2 points per row.
    pub fn hard_drop(&mut self) {
        if !self.can_act() {
            return;
        }
        let dist = self.drop_distance();
        let piece = self.piece.expect("can_act checked");
        self.piece = Some(piece.moved(0, dist as isize));
        self.score += 2 * dist as u64;
        self.lock_piece();
    }

    /// One gravity tick: the piece falls one row or locks.
    pub fn step(&mut self) {
        if self.status != Status::Playing {
            return;
        }
        if !self.try_move(0, 1) {
            self.lock_piece();
        }
    }

    // ----- internals -------------------------------------------------------

    fn can_act(&self) -> bool {
        self.status == Status::Playing && self.piece.is_some()
    }

    fn try_move(&mut self, dx: isize, dy: isize) -> bool {
        if !self.can_act() {
            return false;
        }
        let piece = self.piece.expect("can_act checked");
        let moved = piece.moved(dx, dy);
        if self.board.fits(&moved) {
            self.piece = Some(moved);
            true
        } else {
            false
        }
    }

    fn rotate(&mut self, clockwise: bool) {
        if !self.can_act() {
            return;
        }
        let piece = self.piece.expect("can_act checked");
        for (dx, dy) in KICKS {
            let candidate = piece.rotated(clockwise).moved(dx, dy);
            if self.board.fits(&candidate) {
                self.piece = Some(candidate);
                return;
            }
        }
    }

    fn lock_piece(&mut self) {
        let Some(piece) = self.piece else {
            return;
        };
        if !self.board.lock(&piece) {
            self.piece = None;
            self.status = Status::Over;
            return;
        }
        let cleared = self.board.clear_lines();
        if cleared > 0 {
            self.score += LINE_SCORES[cleared] * self.level as u64;
            self.lines += cleared as u32;
            self.level = Self::level_for_lines(self.lines);
        }
        self.spawn();
    }

    fn take_from_bag(&mut self) -> Kind {
        if self.bag.is_empty() {
            self.bag = Kind::ALL.to_vec();
            self.bag.shuffle(&mut self.rng);
        }
        self.bag.pop().expect("bag just refilled")
    }

    /// Take the next kind from the queue and spawn it; the game ends if the
    /// spawn position is occupied.
    fn spawn(&mut self) {
        let kind = self.queue.remove(0);
        let replacement = self.take_from_bag();
        self.queue.push(replacement);
        let piece = Piece::new(kind);
        if self.board.fits(&piece) {
            self.piece = Some(piece);
        } else {
            self.piece = None;
            self.status = Status::Over;
        }
    }

    #[cfg(test)]
    pub fn debug_set_piece(&mut self, piece: Piece) {
        if self.board.fits(&piece) {
            self.piece = Some(piece);
            self.status = Status::Playing;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rotation_state_has_four_distinct_cells() {
        for kind in Kind::ALL {
            let box_size = kind.box_size();
            for rot in 0..4 {
                let mut seen = std::collections::BTreeSet::new();
                for (x, y) in kind.shape_cells(rot) {
                    assert!(x < box_size && y < box_size, "{kind:?} rot {rot}");
                    assert!(seen.insert((x, y)), "duplicate cell for {kind:?} rot {rot}");
                }
                assert_eq!(seen.len(), 4);
            }
        }
    }

    #[test]
    fn bag_contains_all_seven_kinds_per_refill() {
        let mut game = Game::new(7);
        for _ in 0..3 {
            game.bag.clear();
            let mut got: Vec<Kind> = (0..7).map(|_| game.take_from_bag()).collect();
            got.sort();
            assert_eq!(got, Kind::ALL.to_vec());
        }
    }

    #[test]
    fn spawn_position_is_centered_and_fits() {
        for kind in Kind::ALL {
            let game = Game::new(1);
            let piece = Piece::new(kind);
            assert!(game.board.fits(&piece), "{kind:?}");
            let cells = piece.cells();
            let min_c = cells.iter().map(|&(c, _)| c).min().unwrap();
            let max_c = cells.iter().map(|&(c, _)| c).max().unwrap();
            assert!(min_c >= 0 && max_c < COLS as isize, "{kind:?}");
        }
    }

    #[test]
    fn piece_cannot_leave_board_horizontally() {
        let mut game = Game::new(1);
        for _ in 0..COLS {
            game.move_left();
        }
        let stuck = game.current().unwrap();
        game.move_left();
        assert_eq!(game.current().unwrap().x, stuck.x);

        // same on the right wall
        let mut game = Game::new(1);
        for _ in 0..COLS {
            game.move_right();
        }
        let stuck = game.current().unwrap();
        game.move_right();
        assert_eq!(game.current().unwrap().x, stuck.x);
    }

    #[test]
    fn gravity_step_moves_piece_down() {
        let mut game = Game::new(1);
        let before = game.current().unwrap();
        game.step();
        let after = game.current().unwrap();
        assert_eq!(after.y, before.y + 1);
        assert_eq!(after.x, before.x);
    }

    #[test]
    fn soft_drop_scores_one_point_per_row() {
        let mut game = Game::new(1);
        let before = game.current().unwrap();
        let score_before = game.score();
        game.soft_drop();
        let after = game.current().unwrap();
        assert_eq!(after.y, before.y + 1);
        assert_eq!(game.score(), score_before + 1);
    }

    #[test]
    fn hard_drop_lands_piece_on_the_bottom_row() {
        let mut game = Game::new(1);
        game.hard_drop();
        assert_eq!(game.board().filled_cells(), 4);
        let bottom_filled = (0..COLS)
            .filter(|&c| game.board().cell(ROWS - 1, c) != 0)
            .count();
        assert!(bottom_filled >= 1, "no locked cell on the bottom row");
        assert!(game.score() > 0, "hard drop should score");
        assert!(game.current().is_some(), "a fresh piece spawns after lock");
    }

    #[test]
    fn clearing_a_row_awards_points_and_shifts_the_board() {
        let mut game = Game::new(1);
        for c in 0..COLS {
            if c != 4 && c != 5 {
                game.board.set_cell(ROWS - 1, c, Some(Kind::O));
            }
        }
        let piece = Piece {
            kind: Kind::O,
            rot: 0,
            x: 4,
            y: (ROWS - 2) as isize,
        };
        game.debug_set_piece(piece);
        assert!(game.board.fits(&piece));
        game.hard_drop();
        assert_eq!(game.lines(), 1);
        assert_eq!(game.score(), 100, "one line at level 1 scores 100");
        assert_eq!(game.board().filled_cells(), 2, "O piece now rests on the bottom row");
        assert!(game.board().cell(ROWS - 1, 4) != 0);
        assert!(game.board().cell(ROWS - 1, 5) != 0);
    }

    #[test]
    fn clearing_four_rows_is_a_tetris() {
        let mut game = Game::new(1);
        for r in (ROWS - 4)..ROWS {
            for c in 1..COLS {
                game.board.set_cell(r, c, Some(Kind::O));
            }
        }
        // Vertical I in column 0: state 1 cells are at (x=2, y=0..3).
        let piece = Piece {
            kind: Kind::I,
            rot: 1,
            x: -2,
            y: -1,
        };
        game.debug_set_piece(piece);
        game.hard_drop();
        assert_eq!(game.lines(), 4);
        assert!(game.score() >= 800, "tetris scores 800 at level 1, got {}", game.score());
        assert_eq!(
            game.board().filled_cells(),
            0,
            "the I's cells completed the rows and were cleared with them"
        );
    }

    #[test]
    fn level_rises_every_ten_lines() {
        assert_eq!(Game::level_for_lines(0), 1);
        assert_eq!(Game::level_for_lines(9), 1);
        assert_eq!(Game::level_for_lines(10), 2);
        assert_eq!(Game::level_for_lines(29), 3);
        assert_eq!(Game::level_for_lines(30), 4);
    }

    #[test]
    fn gravity_speeds_up_with_level() {
        let mut game = Game::new(1);
        let base = game.gravity_interval();
        game.level = 10;
        assert!(game.gravity_interval() < base, "higher level must fall faster");
        assert!(game.gravity_interval() >= Duration::from_millis(50), "speed cap");
    }

    #[test]
    fn pause_freezes_gravity_and_replay_resumes() {
        let mut game = Game::new(1);
        game.toggle_pause();
        assert!(game.is_paused());
        let before = game.current().unwrap();
        game.step();
        assert_eq!(game.current().unwrap(), before);
        game.toggle_pause();
        assert!(!game.is_paused());
        game.step();
        assert_eq!(game.current().unwrap().y, before.y + 1);
    }

    #[test]
    fn input_is_ignored_after_game_over() {
        let mut game = Game::new(1);
        for r in 0..4 {
            for c in 0..COLS {
                game.board.set_cell(r, c, Some(Kind::O));
            }
        }
        game.spawn();
        assert!(game.is_over());
        assert!(game.current().is_none());
        let status = game.status();
        game.move_left();
        game.rotate_cw();
        game.soft_drop();
        game.hard_drop();
        game.step();
        assert_eq!(game.status(), status);
    }

    #[test]
    fn rotation_near_wall_uses_a_kick() {
        // T piece against the left wall: a plain CW rotation would push
        // cells into column -1, so a kick offset must make it legal.
        let mut game = Game::new(1);
        let piece = Piece {
            kind: Kind::T,
            rot: 0,
            x: -1,
            y: 5,
        };
        game.debug_set_piece(piece);
        game.rotate_cw();
        let rotated = game.current().unwrap();
        assert_eq!(rotated.rot, 1, "rotation should have applied with a kick");
        for (c, _r) in rotated.cells() {
            assert!(c >= 0 && c < COLS as isize);
        }
    }

    #[test]
    fn rotation_states_are_consistent_round_trip() {
        for kind in Kind::ALL {
            let base = Piece {
                kind,
                rot: 0,
                x: 5,
                y: 5,
            };
            let back = base.rotated(true).rotated(true).rotated(true).rotated(true);
            assert_eq!(back.cells(), base.cells());
            let back_ccw = base.rotated(false).rotated(false).rotated(false).rotated(false);
            assert_eq!(back_ccw.cells(), base.cells());
        }
    }

}
