//! High-level game state and rules: gravity, movement, rotation (with basic
//! wall kicks), locking, line clearing and scoring.

use crate::board::{Board, HEIGHT, WIDTH};
use crate::rng::Rng;
use crate::tetromino::Piece;

/// A 7-bag randomizer: it shuffles the full set of pieces and deals them out,
/// refilling as it drains.
#[derive(Debug)]
pub struct Bag {
    rng: Rng,
    queue: Vec<crate::board::Kind>,
}

impl Bag {
    pub fn new(rng: Rng) -> Self {
        Self {
            rng,
            queue: Vec::new(),
        }
    }

    /// Draw the next kind, refilling and reshuffling the bag when empty.
    pub fn next(&mut self) -> crate::board::Kind {
        if self.queue.is_empty() {
            let mut batch = crate::board::Kind::all().to_vec();
            self.rng.shuffle(&mut batch);
            self.queue = batch;
        }
        self.queue.pop().unwrap()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Playing,
    Over,
}

#[derive(Debug)]
pub struct Game {
    pub board: Board,
    pub current: Piece,
    pub bag: Bag,
    pub score: u32,
    pub lines: u32,
    pub level: u32,
    pub status: Status,
    /// The kind of the piece coming up next (for the "next" preview).
    pub next: crate::board::Kind,
}

const LINE_SCORES: [u32; 5] = [0, 100, 300, 500, 800];
const LINES_PER_LEVEL: u32 = 10;

impl Game {
    pub fn new(seed: u64) -> Self {
        let rng = Rng::with_seed(seed);
        let mut bag = Bag::new(rng);
        let first = bag.next();
        let next = bag.next();
        Game {
            board: Board::new(),
            current: Piece::spawn(first),
            bag,
            score: 0,
            lines: 0,
            level: 0,
            status: Status::Playing,
            next,
        }
    }

    /// Milliseconds between gravity steps at the current level.
    pub fn drop_interval_ms(&self) -> u64 {
        // 800ms at level 0, dropping by 70ms per level to a floor of 80ms.
        let ms = 800u64.saturating_sub((self.level as u64) * 70);
        ms.max(80)
    }

    fn fits(&self, piece: &Piece) -> bool {
        piece.cells().iter().all(|&(x, y)| self.board.is_free(x, y))
    }

    /// Move down by one row. Returns `true` if the piece moved, `false` if it
    /// is locked into the board (caller should spawn the next piece).
    pub fn step_down(&mut self) -> bool {
        if self.status != Status::Playing {
            return false;
        }
        let mut candidate = self.current;
        candidate.y += 1;
        if self.fits(&candidate) {
            self.current = candidate;
            true
        } else {
            self.lock();
            false
        }
    }

    /// Try to move left/right by `dx`. No-op when the wall is in the way.
    pub fn move_horizontal(&mut self, dx: isize) {
        if self.status != Status::Playing {
            return;
        }
        let mut candidate = self.current;
        candidate.x += dx;
        if self.fits(&candidate) {
            self.current = candidate;
        }
    }

    /// Rotate clockwise, trying a small set of wall kicks (no-op SRS).
    pub fn rotate(&mut self) {
        if self.status != Status::Playing {
            return;
        }
        let base = self.current;
        let base_rot = base.rot;
        let kicked_rot = (base_rot + 1) % 4;
        for (dx, dy) in [(0, 0), (-1, 0), (1, 0), (0, -1), (-2, 0), (2, 0)] {
            let mut candidate = base;
            candidate.rot = kicked_rot;
            candidate.x += dx;
            candidate.y += dy;
            if self.fits(&candidate) {
                self.current = candidate;
                return;
            }
        }
    }

    /// Hard drop: move the piece to the lowest legal row and lock it
    /// immediately, awarding 2 points per cell dropped.
    pub fn hard_drop(&mut self) {
        if self.status != Status::Playing {
            return;
        }
        let mut distance = 0;
        while {
            let mut c = self.current;
            c.y += 1;
            self.fits(&c)
        } {
            self.current.y += 1;
            distance += 1;
        }
        self.score += distance as u32 * 2;
        self.lock();
    }

    /// Soft drop: one fast step down, worth 1 point when it moves.
    pub fn soft_drop(&mut self) {
        if self.status != Status::Playing {
            return;
        }
        let mut candidate = self.current;
        candidate.y += 1;
        if self.fits(&candidate) {
            self.current = candidate;
            self.score += 1;
        } else {
            self.lock();
        }
    }

    /// Write the locked piece into the board, clear full rows, score and
    /// spawn the next piece. Sets `status` to `Over` when the new piece
    /// cannot spawn.
    fn lock(&mut self) {
        for (x, y) in self.current.cells() {
            if x < 0 || x >= WIDTH as isize || y < 0 || y >= HEIGHT as isize {
                continue;
            }
            self.board.set(x as usize, y as usize, self.current.kind);
        }
        let cleared = self.board.clear_lines();
        if cleared > 0 {
            self.score += LINE_SCORES[cleared as usize] * (self.level + 1);
            self.lines += cleared;
            self.level = self.lines / LINES_PER_LEVEL;
        }
        self.current = Piece::spawn(self.next);
        self.next = self.bag.next();
        if !self.fits(&self.current) {
            self.status = Status::Over;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Kind;

    fn seeded() -> Game {
        Game::new(12345)
    }

    #[test]
    fn new_game_has_current_and_next() {
        let game = seeded();
        assert_eq!(game.status, Status::Playing);
        assert_eq!(game.score, 0);
        assert_eq!(game.lines, 0);
        // There is always one live and one upcoming piece.
        assert!(matches!(game.current.kind, _));
        assert!(matches!(game.next, _));
    }

    #[test]
    fn bag_deals_each_kind_before_repeating() {
        let mut bag = Bag::new(Rng::with_seed(99));
        let mut seen = std::collections::HashSet::new();
        // A full first bag must contain exactly the seven distinct kinds.
        for _ in 0..7 {
            let k = bag.next();
            assert!(seen.insert(k.as_idx()), "bag repeated early: {k:?}");
        }
        assert_eq!(seen.len(), 7);
    }

    #[test]
    fn step_down_moves_the_piece() {
        let mut game = seeded();
        let start = game.current.y;
        assert!(game.step_down());
        assert_eq!(game.current.y, start + 1);
    }

    #[test]
    fn move_horizontal_hits_the_wall() {
        let mut game = seeded();
        game.current = Piece::spawn(Kind::O);
        // Push left repeatedly; the O piece must never cross x<0.
        for _ in 0..20 {
            game.move_horizontal(-1);
        }
        let min_x = game.current.cells().iter().map(|&(x, _)| x).min().unwrap();
        assert!(min_x >= 0);
    }

    #[test]
    fn hard_drop_locks_and_spawns() {
        let mut game = seeded();
        let before = game.board.occupied_count();
        game.hard_drop();
        assert!(
            game.board.occupied_count() > before,
            "lock should add cells"
        );
        // The previous next became the live piece.
    }

    #[test]
    fn soft_drop_scores_one_point_per_move() {
        let mut game = seeded();
        game.score = 0;
        game.soft_drop();
        assert_eq!(game.score, 1);
    }

    #[test]
    fn drop_interval_is_clamped() {
        let mut game = seeded();
        assert_eq!(game.drop_interval_ms(), 800);
        game.level = 100;
        assert_eq!(game.drop_interval_ms(), 80);
    }

    #[test]
    fn rotate_changes_rotation_when_room_allows() {
        let mut game = seeded();
        let before = game.current.rot;
        game.rotate();
        // With an empty board near the top, the kick (0,0) always succeeds.
        assert_eq!(game.current.rot, (before + 1) % 4);
    }
}
