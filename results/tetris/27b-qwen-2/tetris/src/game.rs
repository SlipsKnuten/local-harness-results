use std::collections::VecDeque;
use std::time::Duration;

use crate::tetromino::Tetromino;

/// Board dimensions.
pub const COLS: usize = 10;
pub const ROWS: usize = 20;

/// A settled cell on the board.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Cell {
    #[default]
    Empty,
    Filled(Tetromino),
}

/// The active falling piece: its kind plus the top-left position of its
/// rotation box and its current rotation state.
#[derive(Clone, Copy)]
pub struct Piece {
    pub kind: Tetromino,
    pub x: i32,
    pub y: i32,
    pub rotation: u8,
}

/// Small xorshift64 RNG so we don't need an external dependency.
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn next_range(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next_u64() % n }
    }
}

/// A "7-bag" randomizer: all seven pieces are shuffled and dealt out, then a
/// new bag is shuffled. This keeps piece distribution fair.
struct Bag {
    pieces: VecDeque<Tetromino>,
    rng: Rng,
}

impl Bag {
    fn new(seed: u64) -> Self {
        Bag {
            pieces: VecDeque::new(),
            rng: Rng::new(seed),
        }
    }
    fn draw(&mut self) -> Tetromino {
        if self.pieces.is_empty() {
            let mut bag = Tetromino::ALL.to_vec();
            for i in (1..bag.len()).rev() {
                let j = self.rng.next_range(i as u64 + 1) as usize;
                bag.swap(i, j);
            }
            self.pieces = bag.into();
        }
        self.pieces.pop_front().unwrap()
    }
}

/// Full game state plus the rules.
pub struct Game {
    pub board: Vec<Cell>,
    pub current: Piece,
    pub next: Tetromino,
    pub score: u32,
    pub lines: u32,
    pub level: u32,
    pub over: bool,
    bag: Bag,
}

impl Game {
    pub fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1);
        let mut game = Game {
            board: vec![Cell::Empty; ROWS * COLS],
            current: Piece {
                kind: Tetromino::T,
                x: 0,
                y: 0,
                rotation: 0,
            },
            next: Tetromino::T,
            score: 0,
            lines: 0,
            level: 0,
            over: false,
            bag: Bag::new(seed),
        };
        game.next = game.bag.draw();
        game.spawn();
        game
    }

    /// How long the active piece waits before dropping one row.
    pub fn tick_duration(&self) -> Duration {
        let ms = 800u64.saturating_sub((self.level as u64) * 70).clamp(80, 800);
        Duration::from_millis(ms)
    }

    fn spawn(&mut self) {
        let kind = self.next;
        self.next = self.bag.draw();
        self.current = Piece {
            kind,
            x: (COLS / 2) as i32 - 2,
            y: 0,
            rotation: 0,
        };
        if !self.fits(&self.current) {
            self.over = true;
        }
    }

    /// Whether a piece fits entirely within bounds and without overlapping settled cells.
    fn fits(&self, p: &Piece) -> bool {
        for (cx, cy) in p.kind.cells(p.rotation) {
            let bx = p.x + cx;
            let by = p.y + cy;
            if bx < 0 || bx >= COLS as i32 || by >= ROWS as i32 {
                return false;
            }
            if by >= 0 && self.board[by as usize * COLS + bx as usize] != Cell::Empty {
                return false;
            }
        }
        true
    }

    /// The row the active piece would land on if hard-dropped.
    pub fn drop_target_y(&self) -> i32 {
        let mut p = self.current;
        loop {
            let down = Piece { y: p.y + 1, ..p };
            if self.fits(&down) {
                p = down;
            } else {
                return p.y;
            }
        }
    }

    /// One gravity step: move down, or lock if the floor/a piece is hit.
    pub fn tick(&mut self) {
        if self.over {
            return;
        }
        let moved = Piece {
            y: self.current.y + 1,
            ..self.current
        };
        if self.fits(&moved) {
            self.current = moved;
        } else {
            self.lock_piece();
        }
    }

    pub fn move_left(&mut self) {
        if self.over {
            return;
        }
        let moved = Piece {
            x: self.current.x - 1,
            ..self.current
        };
        if self.fits(&moved) {
            self.current = moved;
        }
    }

    pub fn move_right(&mut self) {
        if self.over {
            return;
        }
        let moved = Piece {
            x: self.current.x + 1,
            ..self.current
        };
        if self.fits(&moved) {
            self.current = moved;
        }
    }

    pub fn rotate_cw(&mut self) {
        self.rotate(1);
    }

    pub fn rotate_ccw(&mut self) {
        self.rotate(3);
    }

    fn rotate(&mut self, dir: u8) {
        if self.over {
            return;
        }
        let new_rot = (self.current.rotation + dir) % 4;
        // A small wall-kick table: plain rotation plus shifts to escape walls/floor.
        let kicks: [(i32, i32); 6] = [
            (0, 0),
            (-1, 0),
            (1, 0),
            (0, -1),
            (-2, 0),
            (2, 0),
        ];
        for (dx, dy) in kicks {
            let cand = Piece {
                x: self.current.x + dx,
                y: self.current.y + dy,
                rotation: new_rot,
                ..self.current
            };
            if self.fits(&cand) {
                self.current = cand;
                return;
            }
        }
    }

    pub fn soft_drop(&mut self) {
        if self.over {
            return;
        }
        let moved = Piece {
            y: self.current.y + 1,
            ..self.current
        };
        if self.fits(&moved) {
            self.current = moved;
            self.score += 1;
        } else {
            self.lock_piece();
        }
    }

    pub fn hard_drop(&mut self) {
        if self.over {
            return;
        }
        let target = self.drop_target_y();
        let dist = (target - self.current.y).max(0) as u32;
        self.current.y = target;
        self.score += dist * 2;
        self.lock_piece();
    }

    fn lock_piece(&mut self) {
        for (cx, cy) in self.current.kind.cells(self.current.rotation) {
            let bx = self.current.x + cx;
            let by = self.current.y + cy;
            if by < 0 {
                // Locked above the top of the board -> game over.
                self.over = true;
                continue;
            }
            if bx >= 0 && bx < COLS as i32 && by < ROWS as i32 {
                self.board[by as usize * COLS + bx as usize] = Cell::Filled(self.current.kind);
            }
        }
        self.clear_lines();
        self.spawn();
    }

    fn clear_lines(&mut self) {
        let mut kept: Vec<Cell> = Vec::with_capacity(ROWS * COLS);
        let mut cleared = 0u32;
        for row in 0..ROWS {
            let full = (0..COLS).all(|c| self.board[row * COLS + c] != Cell::Empty);
            if full {
                cleared += 1;
            } else {
                kept.extend_from_slice(&self.board[row * COLS..(row + 1) * COLS]);
            }
        }
        if cleared == 0 {
            return;
        }
        // Prepend empty rows to bring the board back up.
        let empties = vec![Cell::Empty; (cleared as usize) * COLS];
        kept.splice(0..0, empties);
        self.board = kept;

        let points = match cleared {
            1 => 100,
            2 => 300,
            3 => 500,
            _ => 800,
        };
        self.score += points * (self.level + 1);
        self.lines += cleared;
        self.level = self.lines / 10;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_board_fits_a_piece() {
        let g = Game::new();
        let p = Piece {
            kind: Tetromino::O,
            x: 0,
            y: 0,
            rotation: 0,
        };
        assert!(g.fits(&p));
    }

    #[test]
    fn out_of_bounds_is_rejected() {
        let g = Game::new();
        let left = Piece {
            kind: Tetromino::O,
            x: -2,
            y: 0,
            rotation: 0,
        };
        assert!(!g.fits(&left));
        let below = Piece {
            kind: Tetromino::O,
            x: 0,
            y: ROWS as i32,
            rotation: 0,
        };
        assert!(!g.fits(&below));
    }

    #[test]
    fn rotation_keeps_cells_inside_the_box() {
        for t in Tetromino::ALL {
            let n = t.box_size() as i32;
            for rot in 0..4 {
                for (x, y) in t.cells(rot) {
                    assert!(
                        (0..n).contains(&x) && (0..n).contains(&y),
                        "cell ({x},{y}) of {:?} rot {rot} is outside the box",
                        t
                    );
                }
            }
        }
    }

    #[test]
    fn clearing_a_full_bottom_row_scores_and_shifts_down() {
        let mut g = Game::new();
        for c in 0..COLS {
            g.board[(ROWS - 1) * COLS + c] = Cell::Filled(Tetromino::O);
        }
        let len = g.board.len();
        g.clear_lines();
        assert_eq!(g.lines, 1);
        assert_eq!(g.score, 100);
        assert_eq!(g.board.len(), len);
        assert!(g.board.iter().all(|c| matches!(c, Cell::Empty)));
    }

    #[test]
    fn hard_drop_completes_two_rows() {
        let mut g = Game::new();
        g.board = vec![Cell::Empty; ROWS * COLS];
        g.current = Piece {
            kind: Tetromino::O,
            x: 3,
            y: 0,
            rotation: 0,
        };
        // Fill the two bottom rows except the two columns the O piece occupies.
        for r in (ROWS - 2)..ROWS {
            for c in 0..COLS {
                if c != 3 && c != 4 {
                    g.board[r * COLS + c] = Cell::Filled(Tetromino::O);
                }
            }
        }
        g.hard_drop();
        assert_eq!(g.lines, 2);
        assert!(!g.over);
        assert!(g.board.iter().all(|c| matches!(c, Cell::Empty)));
    }

    #[test]
    fn bag_deals_all_seven_pieces_before_repeating() {
        let mut bag = Bag::new(42);
        let mut seen = [false; 7];
        for _ in 0..7 {
            seen[bag.draw() as usize] = true;
        }
        assert!(seen.iter().all(|s| *s), "a piece was missing from the bag: {seen:?}");
    }
}
