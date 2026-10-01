use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::time::Duration;

use crate::board::{Board, WIDTH};
use crate::piece::{Piece, PieceType};

/// All mutable game state: the board, the active and next pieces, and
/// scoring.
pub struct Game {
    pub board: Board,
    pub current: Piece,
    pub next: PieceType,
    pub score: u32,
    pub level: u32,
    pub lines: u16,
    pub is_paused: bool,
    pub is_game_over: bool,
    bag: Vec<PieceType>,
    rng: StdRng,
}

impl Game {
    pub fn new() -> Self {
        let mut game = Game {
            board: Board::new(),
            current: Piece {
                piece_type: PieceType::T,
                x: WIDTH / 2 - 2,
                y: 0,
                rotation: 0,
            },
            next: PieceType::T,
            score: 0,
            level: 1,
            lines: 0,
            is_paused: false,
            is_game_over: false,
            bag: Vec::new(),
            rng: StdRng::from_rng(&mut rand::rng()),
        };
        game.next = game.next_from_bag();
        game.spawn_new_piece();
        game
    }

    /// Refill the 7-bag with a shuffled set of all seven pieces.
    fn refill_bag(&mut self) {
        let mut new_bag = PieceType::ALL.to_vec();
        new_bag.shuffle(&mut self.rng);
        self.bag = new_bag;
    }

    /// Pop the next piece from the bag, refilling it when empty.
    fn next_from_bag(&mut self) -> PieceType {
        if self.bag.is_empty() {
            self.refill_bag();
        }
        self.bag.pop().expect("bag to be refilled when empty")
    }

    /// Move `next` into play and draw a fresh next piece.
    fn spawn_new_piece(&mut self) {
        let piece_type = self.next;
        self.next = self.next_from_bag();
        self.current = Piece {
            piece_type,
            x: WIDTH / 2 - 2,
            y: 0,
            rotation: 0,
        };
        if !self.board.is_valid_position(&self.current) {
            self.is_game_over = true;
        }
    }

    fn move_horizontal(&mut self, dx: i16) {
        if self.is_game_over || self.is_paused {
            return;
        }
        let mut p = self.current;
        p.x += dx;
        if self.board.is_valid_position(&p) {
            self.current = p;
        }
    }

    pub fn move_left(&mut self) {
        self.move_horizontal(-1);
    }

    pub fn move_right(&mut self) {
        self.move_horizontal(1);
    }

    /// Rotate clockwise, trying a few horizontal wall-kick offsets.
    pub fn rotate(&mut self) {
        if self.is_game_over || self.is_paused {
            return;
        }
        let base = self.current;
        let kicked = [0i16, -1, 1, -2, 2];
        for dx in kicked {
            let mut p = base;
            p.rotation = (p.rotation + 1) % 4;
            p.x += dx;
            if self.board.is_valid_position(&p) {
                self.current = p;
                return;
            }
        }
    }

    /// Attempt to move the piece down one row. Returns true if it moved.
    pub fn move_down(&mut self, with_score: bool) -> bool {
        if self.is_game_over || self.is_paused {
            return false;
        }
        let mut p = self.current;
        p.y += 1;
        if self.board.is_valid_position(&p) {
            self.current = p;
            if with_score {
                self.score += 1;
            }
            true
        } else {
            false
        }
    }

    /// Gravity step: fall one row or lock the piece when it can't.
    pub fn tick(&mut self) {
        if self.is_game_over || self.is_paused {
            return;
        }
        if !self.move_down(false) {
            self.lock_and_spawn();
        }
    }

    /// Soft drop: move down one row (awarding a point) or lock if grounded.
    pub fn soft_drop(&mut self) {
        if self.is_game_over || self.is_paused {
            return;
        }
        if !self.move_down(true) {
            self.lock_and_spawn();
        }
    }

    /// Drop the piece to the floor instantly, awarding points, then lock it.
    pub fn hard_drop(&mut self) {
        if self.is_game_over || self.is_paused {
            return;
        }
        let mut p = self.current;
        let mut distance = 0i32;
        loop {
            let mut d = p;
            d.y += 1;
            if self.board.is_valid_position(&d) {
                p = d;
                distance += 1;
            } else {
                break;
            }
        }
        self.current = p;
        self.score += distance as u32 * 2;
        self.lock_and_spawn();
    }

    /// The y coordinate where the current piece would rest if dropped.
    pub fn ghost_position(&self) -> i16 {
        let mut y = self.current.y;
        loop {
            let mut p = self.current;
            p.y = y + 1;
            if self.board.is_valid_position(&p) {
                y += 1;
            } else {
                break;
            }
        }
        y
    }

    /// Lock the current piece, clear lines, and score the result.
    fn lock_and_spawn(&mut self) {
        self.board.lock_piece(&self.current);
        let cleared = self.board.clear_lines();
        if cleared > 0 {
            self.lines += cleared;
            let base = match cleared {
                1 => 100,
                2 => 300,
                3 => 500,
                4 => 800,
                _ => 0,
            };
            self.score += base * self.level;
            self.level = (self.lines / 10) as u32 + 1;
        }
        self.spawn_new_piece();
    }

    /// How long to wait before the next gravity step. Faster as level rises.
    pub fn tick_interval(&self) -> Duration {
        let ms = (1000 - (self.level - 1) * 90).max(80);
        Duration::from_millis(ms as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled_count(game: &Game) -> usize {
        let mut n = 0;
        for y in 0..crate::board::HEIGHT {
            for x in 0..crate::board::WIDTH {
                if matches!(game.board.get(x, y), crate::board::Cell::Filled(_)) {
                    n += 1;
                }
            }
        }
        n
    }

    #[test]
    fn new_game_spawns_a_valid_piece() {
        let game = Game::new();
        assert!(!game.is_game_over);
        assert!(game.board.is_valid_position(&game.current));
        assert_eq!(filled_count(&game), 0);
    }

    #[test]
    fn hard_drop_locks_piece_and_scores() {
        let mut game = Game::new();
        let before = filled_count(&game);
        let score_before = game.score;
        game.hard_drop();
        // A single piece lays down exactly four cells and cannot clear a line.
        assert_eq!(filled_count(&game), before + 4);
        assert!(game.score >= score_before);
        assert!(game.board.is_valid_position(&game.current));
    }

    #[test]
    fn rotate_changes_orientation() {
        let mut game = Game::new();
        let start = game.current.rotation;
        game.rotate();
        // At spawn on an empty board a rotation must succeed and stay valid.
        assert_ne!(game.current.rotation, start);
        assert!(game.current.rotation < 4);
        assert!(game.board.is_valid_position(&game.current));
    }

    #[test]
    fn gravity_eventually_locks_pieces() {
        let mut game = Game::new();
        for _ in 0..2000 {
            game.tick();
            if game.is_game_over {
                break;
            }
        }
        assert!(filled_count(&game) > 0);
    }
}
