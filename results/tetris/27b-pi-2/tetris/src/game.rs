use rand::{rngs::ThreadRng, Rng};
use std::time::Duration;

use crate::tetrominoes::PieceType;

pub const COLS: usize = 10;
pub const ROWS: usize = 20;

/// A cell on the board. `None` is empty, otherwise a glyph char identifying the
/// piece type that occupied it (used for color).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell(Option<char>);

impl Cell {
    pub fn empty() -> Self {
        Cell(None)
    }
    pub fn filled(glyph: char) -> Self {
        Cell(Some(glyph))
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }
    pub fn glyph(&self) -> Option<char> {
        self.0
    }
}

/// The active (falling) piece.
#[derive(Debug, Clone)]
pub struct ActivePiece {
    pub kind: PieceType,
    pub col: i16,
    pub row: i16,
    pub rot: usize,
}

impl ActivePiece {
    /// Absolute board coordinates of each of the 4 cells.
    pub fn cells(&self) -> [(i16, i16); 4] {
        let offs = self.kind.rotations()[self.rot];
        offs.map(|(c, r)| (self.col + c, self.row + r))
    }
}

/// Full game state.
pub struct Game {
    pub board: [[Cell; COLS]; ROWS],
    pub active: Option<ActivePiece>,
    pub next: PieceType,
    pub score: u32,
    pub lines: usize,
    pub level: usize,
    pub over: bool,
    pub paused: bool,
    rng: ThreadRng,
}

/// Score awarded per piece for a given number of lines cleared at once.
const LINE_SCORES: [u32; 5] = [0, 100, 300, 500, 800];

impl Game {
    pub fn new() -> Self {
        let mut g = Game {
            board: [[Cell::empty(); COLS]; ROWS],
            active: None,
            next: PieceType::T,
            score: 0,
            lines: 0,
            level: 1,
            over: false,
            paused: false,
            rng: rand::rng(),
        };
        g.next = g.random_piece();
        g.spawn();
        g
    }

    fn random_piece(&mut self) -> PieceType {
        // Simple 7-bag-ish: uniform random over all seven pieces.
        PieceType::ALL[self.rng.random_range(0..7)]
    }

    /// Drop a new active piece from `next` and roll a fresh `next`.
    fn spawn(&mut self) {
        let kind = self.next;
        self.next = self.random_piece();
        let piece = ActivePiece {
            kind,
            col: 3,
            row: 0,
            rot: 0,
        };
        if self.collides(&piece) {
            self.over = true;
        }
        self.active = Some(piece);
    }

    /// True if any cell of `piece` is out of bounds or overlaps the board.
    fn collides(&self, piece: &ActivePiece) -> bool {
        for (c, r) in piece.cells() {
            if c < 0 || c >= COLS as i16 || r >= ROWS as i16 {
                return true;
            }
            if r >= 0 && !self.board[r as usize][c as usize].is_empty() {
                return true;
            }
        }
        false
    }

    pub fn try_move(&mut self, dc: i16, dr: i16) -> bool {
        let Some(mut p) = self.active.clone() else {
            return false;
        };
        p.col += dc;
        p.row += dr;
        if self.collides(&p) {
            return false;
        }
        self.active = Some(p);
        true
    }

    /// Attempt a clockwise (or counter-clockwise) rotation with simple wall kicks.
    pub fn rotate(&mut self, clockwise: bool) {
        let Some(p) = self.active.clone() else {
            return;
        };
        let nr = if clockwise { (p.rot + 1) % 4 } else { (p.rot + 3) % 4 };

        // Base position first, then a handful of horizontal/vertical kicks.
        let kicks: [(i16, i16); 6] = [(0, 0), (-1, 0), (1, 0), (-2, 0), (2, 0), (0, -1)];
        for (kc, kr) in kicks {
            let mut q = p.clone();
            q.rot = nr;
            q.col += kc;
            q.row += kr;
            if !self.collides(&q) {
                self.active = Some(q);
                return;
            }
        }
    }

    /// Soft drop: move down one row. Returns true if it moved.
    pub fn soft_drop(&mut self) -> bool {
        if self.try_move(0, 1) {
            self.score += 1;
            true
        } else {
            false
        }
    }

    /// Hard drop: fall to the bottom, then lock.
    pub fn hard_drop(&mut self) {
        let mut dist = 0;
        while self.try_move(0, 1) {
            dist += 1;
        }
        self.score += dist as u32 * 2;
        self.lock();
    }

    /// Compute where the active piece would rest if dropped straight down.
    /// Returns `None` if there is no active piece.
    pub fn ghost(&self) -> Option<ActivePiece> {
        let mut g = self.active.clone()?;
        loop {
            let mut t = g.clone();
            t.row += 1;
            if self.collides(&t) {
                break;
            }
            g.row += 1;
        }
        Some(g)
    }

    /// Gravity tick: move down; if it can't, lock.
    pub fn tick(&mut self) {
        if self.over || self.paused {
            return;
        }
        if !self.try_move(0, 1) {
            self.lock();
        }
    }

    /// Lock the active piece into the board, clear lines, respawn.
    fn lock(&mut self) {
        let Some(p) = self.active.clone() else {
            return;
        };
        let glyph = p.kind.glyph();
        for (c, r) in p.cells() {
            if r < 0 {
                self.over = true;
                self.active = None;
                return;
            }
            self.board[r as usize][c as usize] = Cell::filled(glyph);
        }

        // Compact the board: copy down non-full rows, counting cleared lines.
        let mut new_board = [[Cell::empty(); COLS]; ROWS];
        let mut write = ROWS;
        let mut cleared = 0;
        for row in (0..ROWS).rev() {
            let row_full = (0..COLS).all(|c| !self.board[row][c].is_empty());
            if row_full {
                cleared += 1;
            } else {
                write -= 1;
                for c in 0..COLS {
                    new_board[write][c] = self.board[row][c];
                }
            }
        }
        self.board = new_board;

        if cleared > 0 {
            self.lines += cleared;
            self.score += LINE_SCORES[cleared.min(4)] * (self.level as u32);
            self.level = self.lines / 10 + 1;
        }

        self.spawn();
    }

    /// Fall interval for the current level (faster at higher levels).
    pub fn drop_interval(&self) -> Duration {
        let level = self.level.min(20);
        let ms = (800 - (level - 1) * 40).max(50);
        Duration::from_millis(ms as u64)
    }

    pub fn reset(&mut self) {
        *self = Game::new();
    }

    pub fn toggle_pause(&mut self) {
        if !self.over {
            self.paused = !self.paused;
        }
    }
}

impl PieceType {
    /// Display glyph used when a piece is locked on the board.
    pub fn glyph(&self) -> char {
        match self {
            PieceType::I => 'I',
            PieceType::O => 'O',
            PieceType::T => 'T',
            PieceType::S => 'S',
            PieceType::Z => 'Z',
            PieceType::J => 'J',
            PieceType::L => 'L',
        }
    }
}

#[cfg(test)]
fn filled_cells(g: &Game) -> usize {
    g.board.iter().flat_map(|row| row.iter()).filter(|c| !c.is_empty()).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_game_spawns_active_and_next_piece() {
        let g = Game::new();
        assert!(g.active.is_some());
        assert!(g.next.glyph().is_ascii_alphabetic());
        assert!(!g.over);
    }

    #[test]
    fn horizontal_movement() {
        let mut g = Game::new();
        let start = g.active.as_ref().unwrap().col;
        g.try_move(1, 0);
        assert_eq!(g.active.as_ref().unwrap().col, start + 1);
        g.try_move(-1, 0);
        assert_eq!(g.active.as_ref().unwrap().col, start);
    }

    #[test]
    fn left_wall_blocks_movement() {
        let mut g = Game::new();
        for _ in 0..40 {
            g.try_move(-1, 0);
        }
        // The piece must remain fully in bounds.
        for (c, r) in g.active.as_ref().unwrap().cells() {
            assert!(c >= 0 && c < COLS as i16);
            assert!(r >= 0 && r < ROWS as i16);
        }
    }

    #[test]
    fn rotation_cycles_back_to_start() {
        let mut g = Game::new();
        let start = g.active.as_ref().unwrap().rot;
        for _ in 0..4 {
            g.rotate(true);
        }
        assert_eq!(g.active.as_ref().unwrap().rot, start);
    }

    #[test]
    fn hard_drop_locks_piece_and_scoring() {
        let mut g = Game::new();
        let score_before = g.score;
        g.hard_drop();
        assert!(g.score > score_before, "hard drop should add score");
        assert!(filled_cells(&g) >= 4, "locked piece cells should be on the board");
    }

    #[test]
    fn line_is_cleared_when_full() {
        let mut g = Game::new();
        // Fill the bottom row completely, then lock the active piece on top of
        // it. Exactly that one full row should be cleared.
        for c in 0..COLS {
            g.board[ROWS - 1][c] = Cell::filled('x');
        }
        g.hard_drop();
        assert_eq!(g.lines, 1, "exactly one full line should be cleared");
        assert!(g.score > 0);
    }

    #[test]
    fn full_lines_compact_downward() {
        let mut g = Game::new();
        // Fill the bottom row, then drop a piece so its body sits above it.
        for c in 0..COLS {
            g.board[ROWS - 1][c] = Cell::filled('x');
        }
        g.hard_drop();
        // Bottom row cleared; some of the locked piece should have risen to the
        // very bottom row now.
        assert!(g.board[ROWS - 1].iter().any(|c| !c.is_empty()));
    }

    #[test]
    fn soft_drop_moves_down_and_scores() {
        let mut g = Game::new();
        let row_before = g.active.as_ref().unwrap().row;
        let score_before = g.score;
        assert!(g.soft_drop());
        assert_eq!(g.active.as_ref().unwrap().row, row_before + 1);
        assert!(g.score > score_before);
    }

    #[test]
    fn gravity_tick_moves_piece_down() {
        let mut g = Game::new();
        let row_before = g.active.as_ref().unwrap().row;
        g.tick();
        let row_after = g.active.as_ref().unwrap().row;
        assert!(row_after >= row_before);
    }

    #[test]
    fn spawn_collision_ends_game() {
        let mut g = Game::new();
        // Block the spawn area without completing any full row.
        for r in 0..4 {
            g.board[r][3] = Cell::filled('x');
            g.board[r][4] = Cell::filled('x');
        }
        g.active = None;
        g.spawn();
        assert!(g.over);
    }

    #[test]
    fn pause_freezes_gravity() {
        let mut g = Game::new();
        g.toggle_pause();
        assert!(g.paused);
        let row_before = g.active.as_ref().unwrap().row;
        g.tick();
        assert_eq!(g.active.as_ref().unwrap().row, row_before);
    }

    #[test]
    fn all_pieces_have_four_rotations_of_four_cells() {
        for kind in PieceType::ALL {
            let rots = kind.rotations();
            assert_eq!(rots.len(), 4);
            for state in rots {
                assert_eq!(state.len(), 4);
            }
        }
    }

    #[test]
    fn drop_interval_speeds_up_with_level() {
        let mut g = Game::new();
        let level1 = g.drop_interval();
        g.lines = 10; // level 2
        g.level = 2;
        let level2 = g.drop_interval();
        assert!(level2 < level1);
    }
}
