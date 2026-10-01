//! Core Tetris game logic, free of any UI dependencies so it can be unit-tested.

use std::collections::VecDeque;

pub const W: usize = 10;
pub const H: usize = 20;

/// The seven tetromino types.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
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

    /// Non-zero id stored in `Game::board` for a locked piece (1 = I .. 7 = L).
    pub fn id(self) -> u8 {
        match self {
            Self::I => 1,
            Self::O => 2,
            Self::T => 3,
            Self::S => 4,
            Self::Z => 5,
            Self::J => 6,
            Self::L => 7,
        }
    }
}

/// The four cells (x, y) of `ty` in rotation `rot`, relative to the top-left
/// corner of a 4x4 bounding box.
pub fn piece_cells(ty: PieceType, rot: u8) -> [(i16, i16); 4] {
    let shape = match ty {
        PieceType::I => [
            [(0, 1), (1, 1), (2, 1), (3, 1)],
            [(2, 0), (2, 1), (2, 2), (2, 3)],
            [(0, 2), (1, 2), (2, 2), (3, 2)],
            [(1, 0), (1, 1), (1, 2), (1, 3)],
        ],
        PieceType::O => {
            let s = [(1, 0), (2, 0), (1, 1), (2, 1)];
            [s, s, s, s]
        }
        PieceType::T => [
            [(1, 0), (0, 1), (1, 1), (2, 1)],
            [(1, 0), (1, 1), (2, 1), (1, 2)],
            [(0, 1), (1, 1), (2, 1), (1, 2)],
            [(1, 0), (0, 1), (1, 1), (1, 2)],
        ],
        PieceType::S => [
            [(1, 0), (2, 0), (0, 1), (1, 1)],
            [(1, 0), (1, 1), (2, 1), (2, 2)],
            [(1, 1), (2, 1), (0, 2), (1, 2)],
            [(0, 0), (0, 1), (1, 1), (1, 2)],
        ],
        PieceType::Z => [
            [(0, 0), (1, 0), (1, 1), (2, 1)],
            [(2, 0), (1, 1), (2, 1), (1, 2)],
            [(0, 1), (1, 1), (1, 2), (2, 2)],
            [(1, 0), (0, 1), (1, 1), (0, 2)],
        ],
        PieceType::J => [
            [(0, 0), (0, 1), (1, 1), (2, 1)],
            [(1, 0), (2, 0), (1, 1), (1, 2)],
            [(0, 1), (1, 1), (2, 1), (2, 2)],
            [(1, 0), (1, 1), (0, 2), (1, 2)],
        ],
        PieceType::L => [
            [(2, 0), (0, 1), (1, 1), (2, 1)],
            [(1, 0), (1, 1), (1, 2), (2, 2)],
            [(0, 1), (1, 1), (2, 1), (0, 2)],
            [(0, 0), (1, 0), (1, 1), (1, 2)],
        ],
    };
    shape[rot as usize % 4]
}

/// The currently falling piece.
#[derive(Clone, Copy, Debug)]
pub struct Piece {
    pub ty: PieceType,
    pub rot: u8,
    pub x: u8,
    pub y: u8,
}

impl Piece {
    pub fn cells(&self) -> [(i16, i16); 4] {
        piece_cells(self.ty, self.rot).map(|(cx, cy)| (self.x as i16 + cx, self.y as i16 + cy))
    }
}

/// A full game: board, active/next pieces, 7-bag randomizer and scoring.
pub struct Game {
    /// `board[y][x]` is 0 (empty) or `PieceType::id()` of a locked cell.
    pub board: [[u8; W]; H],
    pub piece: Piece,
    pub next: PieceType,
    pub score: u32,
    pub lines: u32,
    pub level: u32,
    pub over: bool,
    pub paused: bool,
    bag: VecDeque<PieceType>,
    rng: u64,
}

impl Game {
    /// Start a new game. The first next piece is already drawn from the bag.
    pub fn new(seed: u64) -> Self {
        let mut g = Game {
            board: [[0; W]; H],
            piece: Piece {
                ty: PieceType::T,
                rot: 0,
                x: 3,
                y: 0,
            },
            next: PieceType::T,
            score: 0,
            lines: 0,
            level: 1,
            over: false,
            paused: false,
            bag: VecDeque::new(),
            rng: seed,
        };
        g.next = g.draw();
        g
    }

    /// SplitMix64: small, dependency-free, good enough for a 7-bag shuffle.
    fn rng_next(&mut self) -> u64 {
        self.rng = self.rng.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.rng;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Draw the next piece, refilling the 7-bag when empty.
    pub fn draw(&mut self) -> PieceType {
        if self.bag.is_empty() {
            let mut bag = PieceType::ALL.to_vec();
            for i in (1..bag.len()).rev() {
                let j = (self.rng_next() % (i + 1) as u64) as usize;
                bag.swap(i, j);
            }
            self.bag = bag.into();
        }
        self.bag.pop_front().expect("bag just refilled")
    }

    /// Would a piece of `ty` in rotation `rot` with box top-left at (x, y)
    /// stick out of the board or overlap a locked cell?
    pub fn collides(&self, x: i16, y: i16, ty: PieceType, rot: u8) -> bool {
        for &(cx, cy) in piece_cells(ty, rot).iter() {
            let bx = x + cx;
            let by = y + cy;
            if bx < 0 || bx >= W as i16 || by < 0 || by >= H as i16 {
                return true;
            }
            if self.board[by as usize][bx as usize] != 0 {
                return true;
            }
        }
        false
    }

    pub fn move_left(&mut self) {
        if self.over || self.paused {
            return;
        }
        if !self.collides(
            self.piece.x as i16 - 1,
            self.piece.y as i16,
            self.piece.ty,
            self.piece.rot,
        ) {
            self.piece.x -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.over || self.paused {
            return;
        }
        if !self.collides(
            self.piece.x as i16 + 1,
            self.piece.y as i16,
            self.piece.ty,
            self.piece.rot,
        ) {
            self.piece.x += 1;
        }
    }

    fn rotate(&mut self, dir: i8) {
        if self.over || self.paused {
            return;
        }
        let nr = ((self.piece.rot as i16 + dir as i16 + 4) % 4) as u8;
        // Try the current column first, then simple wall kicks.
        for dx in [0i16, -1, 1, -2, 2] {
            if !self.collides(
                self.piece.x as i16 + dx,
                self.piece.y as i16,
                self.piece.ty,
                nr,
            ) {
                self.piece.x = (self.piece.x as i16 + dx) as u8;
                self.piece.rot = nr;
                return;
            }
        }
    }

    pub fn rotate_cw(&mut self) {
        self.rotate(1);
    }

    pub fn rotate_ccw(&mut self) {
        self.rotate(-1);
    }

    /// Move the piece down one row (soft drop). Returns true if it moved.
    /// A successful move scores 1 point.
    pub fn soft_drop(&mut self) -> bool {
        if self.over || self.paused {
            return false;
        }
        if !self.collides(
            self.piece.x as i16,
            self.piece.y as i16 + 1,
            self.piece.ty,
            self.piece.rot,
        ) {
            self.piece.y += 1;
            self.score += 1;
            true
        } else {
            false
        }
    }

    /// Drop the piece to the bottom/floor, score 2 points per row fallen,
    /// and lock it. Returns the number of rows fallen.
    pub fn hard_drop(&mut self) -> u32 {
        if self.over || self.paused {
            return 0;
        }
        let mut dist = 0;
        while !self.collides(
            self.piece.x as i16,
            self.piece.y as i16 + dist as i16 + 1,
            self.piece.ty,
            self.piece.rot,
        ) {
            dist += 1;
        }
        self.piece.y += dist as u8;
        self.score += (dist as u32) * 2;
        self.lock();
        dist as u32
    }

    /// One gravity step: fall one row, or lock the piece if it cannot.
    pub fn tick(&mut self) {
        if self.over || self.paused {
            return;
        }
        if !self.collides(
            self.piece.x as i16,
            self.piece.y as i16 + 1,
            self.piece.ty,
            self.piece.rot,
        ) {
            self.piece.y += 1;
        } else {
            self.lock();
        }
    }

    /// Merge the active piece into the board, clear full lines, update the
    /// score/level, and spawn the next piece (game over on a blocked spawn).
    fn lock(&mut self) {
        for &(cx, cy) in self.piece.cells().iter() {
            if (0..H as i16).contains(&cy) && (0..W as i16).contains(&cx) {
                self.board[cy as usize][cx as usize] = self.piece.ty.id();
            }
        }
        let cleared = self.clear_lines();
        if cleared > 0 {
            self.lines += cleared as u32;
            self.score += [0, 100, 300, 500, 800][cleared] * self.level;
            self.level = self.lines / 10 + 1;
        }
        let spawn = self.next;
        self.next = self.draw();
        self.piece = Piece {
            ty: spawn,
            rot: 0,
            x: 3,
            y: 0,
        };
        if self.collides(3, 0, spawn, 0) {
            self.over = true;
        }
    }

    /// Remove full rows and drop the rest. Returns how many rows cleared.
    fn clear_lines(&mut self) -> usize {
        // Surviving rows, bottom to top.
        let survivors: Vec<[u8; W]> = self
            .board
            .iter()
            .rev()
            .filter(|row| row.contains(&0))
            .cloned()
            .collect();
        let cleared = H - survivors.len();
        for (i, row) in self.board.iter_mut().rev().enumerate() {
            if i < survivors.len() {
                *row = survivors[i];
            } else {
                row.fill(0);
            }
        }
        cleared
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_bag_contains_all_seven_pieces() {
        let mut g = Game::new(1234);
        let mut bag = vec![g.next];
        for _ in 0..6 {
            bag.push(g.draw());
        }
        bag.sort();
        assert_eq!(bag, PieceType::ALL.to_vec());
    }

    #[test]
    fn every_piece_spawns_inside_the_board() {
        for ty in PieceType::ALL {
            for &(cx, cy) in piece_cells(ty, 0).iter() {
                let x = 3 + cx;
                assert!(
                    x >= 0 && x < W as i16 && cy >= 0 && cy < H as i16,
                    "{ty:?} cell ({cx}, {cy}) out of bounds"
                );
            }
        }
    }

    #[test]
    fn each_rotation_keeps_four_cells_in_box() {
        for ty in PieceType::ALL {
            for rot in 0..4 {
                let cells = piece_cells(ty, rot);
                assert_eq!(cells.len(), 4);
                for &(cx, cy) in cells.iter() {
                    assert!(
                        (0..4).contains(&cx) && (0..4).contains(&cy),
                        "{ty:?} rot {rot}"
                    );
                }
            }
        }
    }

    #[test]
    fn left_wall_stops_movement() {
        let mut g = Game::new(1);
        for _ in 0..20 {
            g.move_left();
        }
        for &(cx, _) in g.piece.cells().iter() {
            assert!(cx >= 0);
        }
    }

    #[test]
    fn right_wall_stops_movement() {
        let mut g = Game::new(1);
        for _ in 0..20 {
            g.move_right();
        }
        for &(cx, _) in g.piece.cells().iter() {
            assert!(cx < W as i16);
        }
    }

    #[test]
    fn rotation_changes_state_and_stays_in_bounds() {
        let mut g = Game::new(1);
        g.rotate_cw();
        assert_eq!(g.piece.rot, 1);
        for &(cx, cy) in g.piece.cells().iter() {
            assert!(cx >= 0 && cx < W as i16 && cy >= 0 && cy < H as i16);
        }
    }

    #[test]
    fn soft_drop_scores_and_moves() {
        let mut g = Game::new(1);
        let y = g.piece.y;
        assert!(g.soft_drop());
        assert_eq!(g.piece.y, y + 1);
        assert_eq!(g.score, 1);
    }

    #[test]
    fn hard_drop_lands_on_the_floor() {
        let mut g = Game::new(1);
        g.piece = Piece {
            ty: PieceType::I,
            rot: 0,
            x: 0,
            y: 0,
        };
        g.hard_drop();
        assert!(g.board[H - 1].contains(&1));
        assert!(g.board[H - 2].iter().all(|&c| c == 0));
        assert!(!g.over);
        // Fell 18 rows => 36 points.
        assert_eq!(g.score, 36);
    }

    #[test]
    fn tick_locks_piece_when_stuck() {
        let mut g = Game::new(1);
        g.piece = Piece {
            ty: PieceType::I,
            rot: 0,
            x: 0,
            y: 18,
        };
        g.tick(); // cannot fall further => locks
        assert!(g.board[H - 1].contains(&1));
        assert!(g.piece.y <= 1); // fresh piece spawned at the top
    }

    #[test]
    fn full_row_clears_and_scores() {
        let mut g = Game::new(1);
        for x in 0..W {
            g.board[H - 1][x] = 1;
        }
        g.hard_drop();
        assert_eq!(g.lines, 1);
        assert!(g.score >= 100); // 100 for the row + hard-drop points
        assert!(g.board[H - 1].contains(&0));
    }

    #[test]
    fn tetris_clears_four_rows() {
        let mut g = Game::new(1);
        for y in (H - 4)..H {
            for x in 0..W {
                g.board[y][x] = 1;
            }
        }
        g.hard_drop();
        assert_eq!(g.lines, 4);
        assert!(g.score >= 800); // 800 for the tetris + hard-drop points
    }

    #[test]
    fn level_goes_up_every_ten_lines() {
        let mut g = Game::new(1);
        g.lines = 9;
        // Clear one more row through the real code path.
        for x in 0..W {
            g.board[H - 1][x] = 1;
        }
        g.hard_drop();
        assert_eq!(g.lines, 10);
        assert_eq!(g.level, 2);
    }

    #[test]
    fn blocked_spawn_ends_the_game() {
        let mut g = Game::new(42);
        // Fill the spawn area, leaving column 0 empty so no row is "full"
        // (nothing would be cleared by the lock below).
        for y in 0..=1 {
            for x in 1..W {
                g.board[y][x] = 3;
            }
        }
        // Current piece parked at the bottom, away from the filled top.
        g.piece = Piece {
            ty: PieceType::T,
            rot: 0,
            x: 0,
            y: 18,
        };
        g.hard_drop();
        assert!(g.over);
    }

    #[test]
    fn paused_game_ignores_moves() {
        let mut g = Game::new(1);
        g.paused = true;
        let (x, y, rot, score) = (g.piece.x, g.piece.y, g.piece.rot, g.score);
        g.move_left();
        g.move_right();
        g.rotate_cw();
        g.soft_drop();
        g.hard_drop();
        g.tick();
        assert_eq!(
            (g.piece.x, g.piece.y, g.piece.rot, g.score),
            (x, y, rot, score)
        );
    }
}
