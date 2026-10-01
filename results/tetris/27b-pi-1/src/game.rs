//! Core game state and rules: spawning, movement, rotation, gravity, scoring.

use crate::board::Board;
use crate::piece::{Piece, PieceType};
use std::time::{SystemTime, UNIX_EPOCH};

/// A tiny xorshift64 RNG so we don't need an external dependency.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        // Ensure a non-zero seed.
        Rng(if seed == 0 { 0x9e3779b97f4a7c15 } else { seed })
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn range(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

pub struct Game {
    pub board: Board,
    pub current: Piece,
    pub next: PieceType,
    pub score: i32,
    pub lines: i32,
    pub level: i32,
    pub game_over: bool,
    pub paused: bool,
    rng: Rng,
}

impl Game {
    pub fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xdeadbeef);
        let rng = Rng::new(seed);
        let game = Game {
            board: Board::new(),
            current: Piece::new(PieceType::I),
            next: PieceType::O,
            score: 0,
            lines: 0,
            level: 1,
            game_over: false,
            paused: false,
            rng,
        };
        let mut game = game;
        // Prime the "next" value and spawn the first piece.
        game.next = game.random_shape();
        game.spawn();
        game
    }

    fn random_shape(&mut self) -> PieceType {
        PieceType::ALL[self.rng.range(PieceType::ALL.len())]
    }

    /// Spawn the queued piece and roll a new one for the preview.
    fn spawn(&mut self) {
        let shape = self.next;
        self.next = self.random_shape();
        let mut p = Piece::new(shape);
        p.x = 3; // centres the (up to) 4-wide bounding box on the 10-wide field
        p.y = 0;
        if self.board.collides(&p) {
            // No room to spawn: top-out.
            self.game_over = true;
        }
        self.current = p;
    }

    pub fn toggle_pause(&mut self) {
        if !self.game_over {
            self.paused = !self.paused;
        }
    }

    pub fn restart(&mut self) {
        *self = Game::new();
    }

    // ----- Player actions -----

    pub fn move_left(&mut self) {
        if self.game_over || self.paused {
            return;
        }
        let p = self.current.moved(-1, 0);
        if !self.board.collides(&p) {
            self.current = p;
        }
    }

    pub fn move_right(&mut self) {
        if self.game_over || self.paused {
            return;
        }
        let p = self.current.moved(1, 0);
        if !self.board.collides(&p) {
            self.current = p;
        }
    }

    pub fn rotate(&mut self, dir: i8) {
        if self.game_over || self.paused {
            return;
        }
        let new_rot = (((self.current.rot as i16) + (dir as i16) + 4) % 4) as u8;
        // Try the rotation with a few simple wall-kicks.
        for &dx in &[0i32, -1, 1, -2, 2] {
            let mut p = self.current.moved(dx, 0);
            p.rot = new_rot;
            if !self.board.collides(&p) {
                self.current = p;
                return;
            }
        }
    }

    pub fn soft_drop(&mut self) {
        if self.game_over || self.paused {
            return;
        }
        let p = self.current.moved(0, 1);
        if !self.board.collides(&p) {
            self.current = p;
            self.score += 1;
        } else {
            self.lock_piece();
        }
    }

    pub fn hard_drop(&mut self) {
        if self.game_over || self.paused {
            return;
        }
        while !self.board.collides(&self.current.moved(0, 1)) {
            self.current = self.current.moved(0, 1);
            self.score += 2;
        }
        self.lock_piece();
    }

    // ----- Engine -----

    /// One gravity step, driven by the main loop's tick timer.
    pub fn tick(&mut self) {
        if self.game_over || self.paused {
            return;
        }
        let p = self.current.moved(0, 1);
        if !self.board.collides(&p) {
            self.current = p;
        } else {
            self.lock_piece();
        }
    }

    fn lock_piece(&mut self) {
        self.board.lock(&self.current);
        let cleared = self.board.clear_lines();
        if cleared > 0 {
            let base = [0i32, 100, 300, 500, 800][cleared.min(4)];
            self.score += base * self.level;
            self.lines += cleared as i32;
            self.level = self.lines / 10 + 1;
        }
        self.spawn();
    }

    /// Milliseconds per gravity step for the current level.
    pub fn tick_ms(&self) -> u64 {
        (450 - (self.level - 1) * 40).max(80) as u64
    }
}
