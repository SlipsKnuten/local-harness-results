//! Game state and rules: piece movement, rotation, locking, scoring and level progression.

use std::collections::VecDeque;

use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::board::{Board, WIDTH};
use crate::input::Action;
use crate::tetromino::Tetromino;

/// High-level state machine for the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    Playing,
    Paused,
    GameOver,
}

/// The currently falling piece: its type, rotation and the board offset of its origin cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Piece {
    pub kind: Tetromino,
    pub rot: u8,
    pub x: i32,
    pub y: i32,
}

/// Build a freshly spawned piece of `kind`, centred horizontally at the top of the field.
fn make_piece(kind: Tetromino) -> Piece {
    Piece {
        kind,
        rot: 0,
        x: (WIDTH as i32 - kind.size() as i32) / 2,
        y: 0,
    }
}

pub struct Game {
    pub board: Board,
    pub current: Piece,
    /// Upcoming pieces (the 7-bag stream); the front is the next piece.
    queue: VecDeque<Tetromino>,
    pub hold: Option<Tetromino>,
    pub can_hold: bool,
    pub score: u32,
    pub high: u32,
    pub level: u32,
    pub lines: u32,
    pub state: GameState,
    rng: SmallRng,
}

impl Game {
    /// Start a fresh game, seeded with the persisted high score.
    pub fn new(high: u32) -> Self {
        let mut game = Game {
            board: Board::new(),
            current: Piece {
                kind: Tetromino::T,
                rot: 0,
                x: 0,
                y: 0,
            },
            queue: VecDeque::new(),
            hold: None,
            can_hold: true,
            score: 0,
            high,
            level: 1,
            lines: 0,
            state: GameState::Playing,
            rng: SmallRng::from_os_rng(),
        };
        game.fill_queue();
        let kind = game.queue.pop_front().unwrap();
        game.current = make_piece(kind);
        game.state = if game.collides(&game.current) {
            GameState::GameOver
        } else {
            GameState::Playing
        };
        game
    }

    /// Run a user action. Returns `true` if the action changed the active piece
    /// (a lock, hold or restart), so the caller can reset the gravity timer.
    pub fn apply(&mut self, action: Action) -> bool {
        use Action::*;
        match action {
            Quit => false,
            Pause => {
                self.toggle_pause();
                false
            }
            Restart => {
                let high = self.high;
                *self = Game::new(high);
                true
            }
            Move(dx) => {
                self.move_h(dx);
                false
            }
            SoftDrop => {
                self.soft_drop();
                false
            }
            HardDrop => {
                self.hard_drop();
                true
            }
            RotateCw => {
                self.rotate(false);
                false
            }
            RotateCcw => {
                self.rotate(true);
                false
            }
            Hold => {
                self.hold();
                true
            }
        }
    }

    /// Advance gravity by one row, or lock the piece if it cannot move down.
    /// Returns `true` if the piece locked.
    pub fn tick(&mut self) -> bool {
        if self.state != GameState::Playing {
            return false;
        }
        let p = self.offset(&self.current, 0, 1);
        if self.collides(&p) {
            self.lock();
            true
        } else {
            self.current = p;
            false
        }
    }

    /// The gravity interval in milliseconds for the current level (falls faster as you level up).
    pub fn gravity_delay(&self) -> u64 {
        let l = (self.level - 1) as u64;
        1000u64.saturating_sub(l * 90).max(100)
    }

    /// The active piece dropped to the lowest legal position (for the ghost piece).
    pub fn ghost_piece(&self) -> Piece {
        let mut p = self.current;
        while !self.collides(&self.offset(&p, 0, 1)) {
            p.y += 1;
        }
        p
    }

    /// The next `n` pieces (for the preview), in arrival order.
    pub fn next_pieces(&self, n: usize) -> Vec<Tetromino> {
        self.queue.iter().take(n).copied().collect()
    }

    // --- Internals -----------------------------------------------------------

    /// Keep the piece queue topped up with full shuffled 7-bags.
    fn fill_queue(&mut self) {
        while self.queue.len() < 7 {
            let mut bag = Tetromino::ALL.to_vec();
            bag.shuffle(&mut self.rng);
            for t in bag {
                self.queue.push_back(t);
            }
        }
    }

    /// Absolute board coordinates of the four cells occupied by `p`.
    fn piece_cells(&self, p: &Piece) -> [(i32, i32); 4] {
        p.kind
            .cells(p.rot)
            .map(|(r, c)| (p.y + r, p.x + c))
    }

    fn collides(&self, p: &Piece) -> bool {
        self.piece_cells(p)
            .iter()
            .any(|&(r, c)| !self.board.is_free(r, c))
    }

    fn offset(&self, p: &Piece, dx: i32, dy: i32) -> Piece {
        Piece {
            kind: p.kind,
            rot: p.rot,
            x: p.x + dx,
            y: p.y + dy,
        }
    }

    fn move_h(&mut self, dx: i32) {
        if self.state != GameState::Playing {
            return;
        }
        let p = self.offset(&self.current, dx, 0);
        if !self.collides(&p) {
            self.current = p;
        }
    }

    fn soft_drop(&mut self) {
        if self.state != GameState::Playing {
            return;
        }
        let p = self.offset(&self.current, 0, 1);
        if !self.collides(&p) {
            self.current = p;
            self.score += 1;
        }
    }

    fn hard_drop(&mut self) {
        if self.state != GameState::Playing {
            return;
        }
        let mut p = self.current;
        let mut dist = 0;
        while !self.collides(&self.offset(&p, 0, 1)) {
            p.y += 1;
            dist += 1;
        }
        self.score += dist * 2;
        self.current = p;
        self.lock();
    }

    /// Rotate, trying a small set of wall kicks if the in-place rotation is blocked.
    fn rotate(&mut self, ccw: bool) {
        if self.state != GameState::Playing {
            return;
        }
        let delta: i32 = if ccw { -1 } else { 1 };
        let new_rot = (self.current.rot as i32 + delta).rem_euclid(4) as u8;

        let kicks: [(i32, i32); 8] = [
            (0, 0),
            (-1, 0),
            (1, 0),
            (0, -1),
            (-1, -1),
            (1, -1),
            (2, 0),
            (-2, 0),
        ];
        for (dx, dy) in kicks {
            let p = Piece {
                kind: self.current.kind,
                rot: new_rot,
                x: self.current.x + dx,
                y: self.current.y + dy,
            };
            if !self.collides(&p) {
                self.current = p;
                return;
            }
        }
    }

    fn hold(&mut self) {
        if self.state != GameState::Playing || !self.can_hold {
            return;
        }
        let current_kind = self.current.kind;
        match self.hold.take() {
            Some(h) => {
                let p = make_piece(h);
                self.hold = Some(current_kind);
                self.current = p;
                if self.collides(&p) {
                    self.game_over();
                }
            }
            None => {
                self.hold = Some(current_kind);
                self.spawn();
            }
        }
        self.can_hold = false;
    }

    /// Place the front of the queue as the new active piece.
    fn spawn(&mut self) {
        let kind = match self.queue.pop_front() {
            Some(k) => k,
            None => {
                self.fill_queue();
                self.queue.pop_front().expect("queue is refilled before popping")
            }
        };
        self.fill_queue();

        let p = make_piece(kind);
        self.current = p;
        self.can_hold = true;
        if self.collides(&p) {
            self.game_over();
        }
    }

    /// Lock the active piece into the board, clear lines, score and spawn the next piece.
    fn lock(&mut self) {
        let p = self.current;
        let mut top_out = false;
        for (r, c) in self.piece_cells(&p) {
            if r < 0 {
                top_out = true;
            } else {
                self.board.set(r as usize, c as usize, p.kind);
            }
        }

        let cleared = self.board.clear_full_rows();
        if cleared > 0 {
            let base = [0u32, 100, 300, 500, 800];
            self.score += base[cleared] * self.level;
            self.lines += cleared as u32;
            self.level = self.lines / 10 + 1;
        }

        if top_out {
            self.game_over();
            return;
        }
        self.spawn();
    }

    fn toggle_pause(&mut self) {
        match self.state {
            GameState::Playing => self.state = GameState::Paused,
            GameState::Paused => self.state = GameState::Playing,
            GameState::GameOver => {}
        }
    }

    fn game_over(&mut self) {
        if self.score > self.high {
            self.high = self.score;
        }
        self.state = GameState::GameOver;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::HEIGHT;

    fn o_at(x: i32, y: i32) -> Piece {
        Piece { kind: Tetromino::O, rot: 0, x, y }
    }

    #[test]
    fn new_game_is_playing_with_next_pieces() {
        let g = Game::new(0);
        assert_eq!(g.state, GameState::Playing);
        assert!(g.next_pieces(5).len() >= 3);
        assert!(g.can_hold);
    }

    #[test]
    fn double_clear_scores_300_at_level_one() {
        let mut g = Game::new(0);
        g.level = 1;
        g.score = 0;
        g.lines = 0;
        // Fill the two bottom rows except columns 4-5 (where the O will land).
        for r in (HEIGHT - 2)..HEIGHT {
            for c in 0..WIDTH {
                if c != 4 && c != 5 {
                    g.board.set(r, c, Tetromino::I);
                }
            }
        }
        let mut p = make_piece(Tetromino::O);
        p.y = HEIGHT as i32 - 2; // already at the landing row, so the hard drop adds no drop points
        g.current = p;
        assert_eq!(g.current.x, 4);
        g.hard_drop();
        assert_eq!(g.lines, 2);
        assert_eq!(g.score, 300);
    }

    #[test]
    fn tetris_four_line_clear_scores_800() {
        let mut g = Game::new(0);
        g.level = 1;
        g.score = 0;
        g.lines = 0;
        // Fill the bottom four rows except column 5 (the vertical I's column).
        for r in (HEIGHT - 4)..HEIGHT {
            for c in 0..WIDTH {
                if c != 5 {
                    g.board.set(r, c, Tetromino::O);
                }
            }
        }
        // Vertical I in the 4x4 box occupies box-column 2, so x=3 -> board column 5.
        // Start it on its landing row so the hard drop adds no drop points.
        g.current = Piece { kind: Tetromino::I, rot: 1, x: 3, y: HEIGHT as i32 - 4 };
        g.hard_drop();
        assert_eq!(g.lines, 4);
        assert_eq!(g.score, 800);
    }

    #[test]
    fn gravity_gets_faster_and_clamps_with_level() {
        let mut g = Game::new(0);
        g.level = 1;
        let d1 = g.gravity_delay();
        g.level = 5;
        let d5 = g.gravity_delay();
        assert!(d5 < d1);
        g.level = 999;
        assert_eq!(g.gravity_delay(), 100, "delay should clamp at 100ms");
    }

    #[test]
    fn moving_into_the_wall_is_blocked() {
        let mut g = Game::new(0);
        g.current = o_at(0, 0);
        g.move_h(-1);
        assert_eq!(g.current.x, 0, "cannot move past the left wall");
        g.move_h(1);
        assert_eq!(g.current.x, 1);
    }

    #[test]
    fn rotation_is_reversible_in_open_space() {
        let mut g = Game::new(0);
        g.current = Piece { kind: Tetromino::T, rot: 0, x: 3, y: 5 };
        g.rotate(false);
        assert_eq!(g.current.rot, 1);
        g.rotate(true);
        assert_eq!(g.current.rot, 0);
    }

    #[test]
    fn hold_stores_piece_and_disables_until_next() {
        let mut g = Game::new(0);
        g.current = Piece { kind: Tetromino::I, rot: 0, x: 3, y: 0 };
        g.hold();
        assert_eq!(g.hold, Some(Tetromino::I));
        assert!(!g.can_hold);
        // A second hold in the same piece turn is a no-op.
        let before = g.current;
        g.hold();
        assert_eq!(g.current, before);
    }

    #[test]
    fn hold_swaps_with_existing_slot() {
        let mut g = Game::new(0);
        g.hold = Some(Tetromino::T);
        g.can_hold = true;
        g.current = Piece { kind: Tetromino::I, rot: 0, x: 3, y: 0 };
        g.hold();
        assert_eq!(g.hold, Some(Tetromino::I));
        assert_eq!(g.current.kind, Tetromino::T);
        assert!(!g.can_hold);
    }

    #[test]
    fn game_over_when_spawned_piece_collides() {
        let mut g = Game::new(0);
        for r in 0..5 {
            for c in 0..WIDTH {
                g.board.set(r, c, Tetromino::I);
            }
        }
        g.spawn();
        assert_eq!(g.state, GameState::GameOver);
    }

    #[test]
    fn game_over_when_piece_locks_above_the_top() {
        let mut g = Game::new(0);
        g.current = Piece { kind: Tetromino::I, rot: 1, x: 3, y: -2 };
        g.lock();
        assert_eq!(g.state, GameState::GameOver);
        assert_eq!(g.high, g.score, "high score should be updated on game over");
    }

    #[test]
    fn high_score_survives_restarting() {
        let mut g = Game::new(0);
        g.score = 500;
        g.state = GameState::GameOver;
        g.high = 500;
        g = Game::new(g.high);
        assert_eq!(g.high, 500);
        assert_eq!(g.score, 0);
    }
}
