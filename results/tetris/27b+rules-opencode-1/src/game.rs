//! Core Tetris game logic, independent of any terminal rendering.

use rand::Rng;
use rand::RngExt;

pub const WIDTH: usize = 10;
pub const HEIGHT: usize = 20;

/// The seven tetromino shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl Shape {
    pub const ALL: [Shape; 7] = [Shape::I, Shape::O, Shape::T, Shape::S, Shape::Z, Shape::J, Shape::L];

    /// Relative cell offsets for each of the four rotation states.
    /// x grows to the right, y grows downward.
    pub fn cells(self, rotation: usize) -> &'static [(i32, i32)] {
        const ROTATIONS: [[&[(i32, i32)]; 4]; 7] = [
            [
                &[(0, 1), (1, 1), (2, 1), (3, 1)],
                &[(2, 0), (2, 1), (2, 2), (2, 3)],
                &[(0, 2), (1, 2), (2, 2), (3, 2)],
                &[(1, 0), (1, 1), (1, 2), (1, 3)],
            ],
            [
                &[(0, 0), (1, 0), (0, 1), (1, 1)],
                &[(0, 0), (1, 0), (0, 1), (1, 1)],
                &[(0, 0), (1, 0), (0, 1), (1, 1)],
                &[(0, 0), (1, 0), (0, 1), (1, 1)],
            ],
            [
                &[(0, 1), (1, 1), (2, 1), (1, 0)],
                &[(1, 0), (1, 1), (2, 1), (1, 2)],
                &[(0, 1), (1, 1), (2, 1), (1, 2)],
                &[(1, 0), (1, 1), (0, 1), (1, 2)],
            ],
            [
                &[(1, 0), (2, 0), (0, 1), (1, 1)],
                &[(1, 0), (1, 1), (2, 1), (2, 2)],
                &[(1, 1), (2, 1), (0, 2), (1, 2)],
                &[(0, 0), (0, 1), (1, 1), (1, 2)],
            ],
            [
                &[(0, 0), (1, 0), (1, 1), (2, 1)],
                &[(2, 0), (1, 1), (2, 1), (1, 2)],
                &[(0, 1), (1, 1), (1, 2), (2, 2)],
                &[(1, 0), (1, 1), (0, 1), (0, 2)],
            ],
            [
                &[(0, 0), (0, 1), (1, 1), (2, 1)],
                &[(1, 0), (2, 0), (1, 1), (1, 2)],
                &[(0, 1), (1, 1), (2, 1), (2, 2)],
                &[(1, 0), (1, 1), (1, 2), (0, 2)],
            ],
            [
                &[(2, 0), (0, 1), (1, 1), (2, 1)],
                &[(1, 0), (1, 1), (1, 2), (2, 2)],
                &[(0, 1), (1, 1), (2, 1), (0, 2)],
                &[(0, 0), (1, 0), (1, 1), (1, 2)],
            ],
        ];
        ROTATIONS[self as usize][rotation % 4]
    }
}

/// A piece falling on the board: shape, rotation state and position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub shape: Shape,
    pub rotation: usize,
    pub x: i32,
    pub y: i32,
}

/// The grid of locked cells.
#[derive(Debug, Clone)]
pub struct Board {
    grid: Vec<Option<Shape>>,
}

impl Board {
    pub fn new() -> Self {
        Self { grid: vec![None; WIDTH * HEIGHT] }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<Shape> {
        self.grid.get(y * WIDTH + x).copied().flatten()
    }

    pub fn set(&mut self, x: usize, y: usize, shape: Shape) {
        self.grid[y * WIDTH + x] = Some(shape);
    }

    fn row_full(&self, y: usize) -> bool {
        (0..WIDTH).all(|x| self.get(x, y).is_some())
    }

    /// Remove all full rows, adding empty rows on top. Returns rows cleared.
    pub fn clear_full_rows(&mut self) -> usize {
        let mut cleared = 0;
        let mut y = 0;
        while y < HEIGHT {
            if self.row_full(y) {
                self.grid.drain(y * WIDTH..(y + 1) * WIDTH);
                self.grid.splice(0..0, std::iter::repeat_n(None, WIDTH));
                cleared += 1;
            } else {
                y += 1;
            }
        }
        cleared
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

/// Wall-kick offsets tried when a rotation would collide.
const KICKS: [(i32, i32); 6] = [(0, 0), (-1, 0), (1, 0), (-2, 0), (2, 0), (0, -1)];

/// Points awarded for clearing 1..=4 lines at once.
const LINE_POINTS: [u32; 5] = [0, 100, 300, 500, 800];

/// The whole game state.
pub struct Game<R: Rng> {
    pub board: Board,
    pub current: Piece,
    pub next: Shape,
    pub score: u32,
    pub level: u32,
    pub lines: u32,
    pub over: bool,
    bag: Vec<Shape>,
    rng: R,
}

impl<R: Rng> Game<R> {
    pub fn new(rng: R) -> Self {
        let mut game = Self {
            board: Board::new(),
            current: Piece { shape: Shape::I, rotation: 0, x: 3, y: 0 },
            next: Shape::I,
            score: 0,
            level: 1,
            lines: 0,
            over: false,
            bag: Vec::new(),
            rng,
        };
        game.next = game.draw_from_bag();
        game.spawn();
        game
    }

    /// Draw the next shape from a shuffled 7-bag.
    fn draw_from_bag(&mut self) -> Shape {
        if self.bag.is_empty() {
            self.bag = Shape::ALL.to_vec();
            for i in (1..self.bag.len()).rev() {
                let j = self.rng.random_range(0..=i);
                self.bag.swap(i, j);
            }
        }
        self.bag.pop().expect("bag was just refilled")
    }

    /// Absolute board coordinates of the four cells of the piece.
    pub fn piece_cells(&self, piece: &Piece) -> [(i32, i32); 4] {
        let mut out = [(0, 0); 4];
        for (i, (dx, dy)) in piece.shape.cells(piece.rotation).iter().enumerate() {
            out[i] = (piece.x + dx, piece.y + dy);
        }
        out
    }

    /// Whether the piece overlaps a wall, the floor or a locked cell.
    /// Cells above the top of the board (y < 0) are allowed.
    pub fn collides(&self, piece: &Piece) -> bool {
        for (x, y) in self.piece_cells(piece) {
            if x < 0 || x >= WIDTH as i32 || y >= HEIGHT as i32 {
                return true;
            }
            if y >= 0 && self.board.get(x as usize, y as usize).is_some() {
                return true;
            }
        }
        false
    }

    /// Spawn the queued piece at the top center. Sets `over` if it collides.
    fn spawn(&mut self) {
        let shape = self.next;
        self.next = self.draw_from_bag();
        self.current = Piece { shape, rotation: 0, x: 3, y: 0 };
        if self.collides(&self.current) {
            self.over = true;
        }
    }

    fn lock(&mut self) {
        let mut locked_above_top = false;
        for (x, y) in self.piece_cells(&self.current) {
            if y < 0 {
                locked_above_top = true;
                continue;
            }
            self.board.set(x as usize, y as usize, self.current.shape);
        }
        let cleared = self.board.clear_full_rows();
        self.lines += cleared as u32;
        self.score += LINE_POINTS[cleared] * self.level;
        self.level = self.lines / 10 + 1;
        if locked_above_top {
            self.over = true;
            return;
        }
        self.spawn();
    }

    pub fn move_left(&mut self) -> bool {
        self.try_move(-1, 0)
    }

    pub fn move_right(&mut self) -> bool {
        self.try_move(1, 0)
    }

    pub fn soft_drop(&mut self) -> bool {
        if self.try_move(0, 1) {
            self.score += 1;
            true
        } else {
            false
        }
    }

    pub fn rotate(&mut self, direction: i32) {
        if self.over {
            return;
        }
        let new_rotation = ((self.current.rotation as i32 + direction + 4) % 4) as usize;
        for (kx, ky) in KICKS {
            let candidate = Piece {
                shape: self.current.shape,
                rotation: new_rotation,
                x: self.current.x + kx,
                y: self.current.y + ky,
            };
            if !self.collides(&candidate) {
                self.current = candidate;
                return;
            }
        }
    }

    /// Y coordinate where a hard drop would land.
    pub fn ghost_y(&self) -> i32 {
        let mut y = self.current.y;
        while !self.collides(&Piece { y: y + 1, ..self.current }) {
            y += 1;
        }
        y
    }

    pub fn hard_drop(&mut self) {
        if self.over {
            return;
        }
        let mut distance = 0;
        while self.try_move(0, 1) {
            distance += 1;
        }
        self.score += distance as u32 * 2;
        self.lock();
    }

    /// One gravity step: fall one row, or lock and spawn if blocked.
    pub fn tick(&mut self) {
        if self.over {
            return;
        }
        if !self.try_move(0, 1) {
            self.lock();
        }
    }

    pub fn try_move(&mut self, dx: i32, dy: i32) -> bool {
        if self.over {
            return false;
        }
        let candidate = Piece { x: self.current.x + dx, y: self.current.y + dy, ..self.current };
        if self.collides(&candidate) {
            return false;
        }
        self.current = candidate;
        true
    }

    /// Delay in milliseconds between gravity ticks, decreasing with level.
    pub fn delay_ms(&self) -> u64 {
        let speedup = (self.level.saturating_sub(1) as u64) * 40;
        500u64.saturating_sub(speedup).max(80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn game(seed: u64) -> Game<StdRng> {
        Game::new(StdRng::seed_from_u64(seed))
    }

    /// Cells relative to the SRS rotation pivot, doubled to avoid fractions.
    fn scaled(cells: &[(i32, i32)], px2: i32, py2: i32) -> [(i32, i32); 4] {
        let mut out = [(0, 0); 4];
        for (i, &(x, y)) in cells.iter().enumerate() {
            out[i] = (2 * x - px2, 2 * y - py2);
        }
        out.sort_unstable();
        out
    }

    fn rotated_cw(cells: &[(i32, i32)], px2: i32, py2: i32) -> [(i32, i32); 4] {
        let rel = scaled(cells, px2, py2);
        let mut out = [(0, 0); 4];
        for (i, &(rx, ry)) in rel.iter().enumerate() {
            out[i] = (-ry, rx);
        }
        out.sort_unstable();
        out
    }

    #[test]
    fn rotations_are_consistent_rotations() {
        for shape in Shape::ALL {
            if shape == Shape::O {
                // The O piece is a square: every rotation state is identical.
                let base = shape.cells(0).to_vec();
                for r in 1..4 {
                    assert_eq!(shape.cells(r).to_vec(), base, "O state {r} should equal state 0");
                }
                continue;
            }
            let (px2, py2) = if shape == Shape::I { (3, 3) } else { (2, 2) };
            for r in 0..4 {
                let cur = shape.cells(r);
                let next = shape.cells(r + 1);
                assert_eq!(
                    scaled(next, px2, py2),
                    rotated_cw(cur, px2, py2),
                    "{shape:?} state {r} is not a CW rotation of the previous state"
                );
            }
        }
    }

    #[test]
    fn every_state_has_four_unique_cells() {
        for shape in Shape::ALL {
            for r in 0..4 {
                let cells = shape.cells(r);
                assert_eq!(cells.len(), 4);
                let mut sorted = cells.to_vec();
                sorted.sort_unstable();
                sorted.dedup();
                assert_eq!(sorted.len(), 4, "{shape:?} state {r} has duplicate cells");
            }
        }
    }

    #[test]
    fn bag_yields_permutations_of_all_shapes() {
        let mut g = game(1);
        // Game::new already drew 2 shapes (next + current); draw 5 to exhaust bag one.
        for _ in 0..5 {
            g.draw_from_bag();
        }
        for _ in 0..3 {
            let mut batch: Vec<u8> = (0..7).map(|_| g.draw_from_bag() as u8).collect();
            batch.sort_unstable();
            assert_eq!(batch, vec![0, 1, 2, 3, 4, 5, 6]);
        }
    }

    #[test]
    fn collides_with_walls_and_floor() {
        let g = game(2);
        let t = Piece { shape: Shape::T, rotation: 0, x: 0, y: 0 };
        assert!(!g.collides(&t));
        assert!(g.collides(&Piece { x: -1, ..t }));
        assert!(g.collides(&Piece { x: 8, ..t }));
        assert!(!g.collides(&Piece { y: (HEIGHT - 2) as i32, ..t }));
        assert!(g.collides(&Piece { y: (HEIGHT - 1) as i32, ..t }));
        // Cells above the top are allowed.
        assert!(!g.collides(&Piece { y: -1, ..t }));
    }

    #[test]
    fn collides_with_locked_cells() {
        let mut g = game(3);
        g.board.set(5, 5, Shape::O);
        // T at (4,5) has a cell at (5,5).
        assert!(g.collides(&Piece { shape: Shape::T, rotation: 0, x: 4, y: 5 }));
        assert!(!g.collides(&Piece { shape: Shape::T, rotation: 0, x: 3, y: 5 }));
    }

    #[test]
    fn piece_cells_are_absolute() {
        let g = game(15);
        let p = Piece { shape: Shape::O, rotation: 0, x: 3, y: 4 };
        assert_eq!(g.piece_cells(&p), [(3, 4), (4, 4), (3, 5), (4, 5)]);
    }

    #[test]
    fn ghost_y_is_landing_row() {
        let mut g = game(16);
        g.current = Piece { shape: Shape::I, rotation: 0, x: 0, y: 0 };
        assert_eq!(g.ghost_y(), (HEIGHT - 2) as i32);
        // I (rot 0) occupies row y+1; with a block at row 17 it must land on row 16.
        g.board.set(2, HEIGHT - 3, Shape::O);
        assert_eq!(g.ghost_y(), (HEIGHT - 5) as i32);
    }

    #[test]
    fn hard_drop_clears_line_and_scores() {
        let mut g = game(4);
        for x in 4..WIDTH {
            g.board.set(x, HEIGHT - 1, Shape::O);
        }
        g.current = Piece { shape: Shape::I, rotation: 0, x: 0, y: 0 };
        g.hard_drop();
        assert_eq!(g.lines, 1);
        assert_eq!(g.score, 18 * 2 + 100);
        assert!(g.board.get(0, HEIGHT - 1).is_none());
        assert_eq!(g.current.y, 0);
    }

    #[test]
    fn tetris_clear_scores_800() {
        let mut g = game(5);
        for y in (HEIGHT - 4)..HEIGHT {
            for x in 0..WIDTH {
                if x != 2 {
                    g.board.set(x, y, Shape::O);
                }
            }
        }
        g.current = Piece { shape: Shape::I, rotation: 1, x: 0, y: 0 };
        g.hard_drop();
        assert_eq!(g.lines, 4);
        assert_eq!(g.score, 16 * 2 + 800);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                assert!(g.board.get(x, y).is_none(), "board should be empty at ({x},{y})");
            }
        }
    }

    #[test]
    fn soft_drop_scores_and_locks_at_floor() {
        let mut g = game(6);
        g.current = Piece { shape: Shape::I, rotation: 0, x: 0, y: 0 };
        let mut steps = 0;
        while g.soft_drop() {
            steps += 1;
        }
        assert_eq!(steps, HEIGHT - 2);
        assert_eq!(g.score, steps as u32);
        assert!(!g.soft_drop());
        g.tick();
        assert!(g.board.get(0, HEIGHT - 1).is_some());
        assert_eq!(g.current.y, 0);
    }

    #[test]
    fn tick_locks_piece_at_floor_and_spawns() {
        let mut g = game(13);
        let next_before = g.next;
        g.current = Piece { shape: Shape::O, rotation: 0, x: 0, y: (HEIGHT - 2) as i32 };
        g.tick();
        assert!(g.board.get(0, HEIGHT - 1).is_some());
        assert_eq!(g.current.shape, next_before);
        assert_eq!(g.current.y, 0);
    }

    #[test]
    fn game_over_when_spawn_collides() {
        let mut g = game(7);
        for y in 0..3 {
            for x in 0..WIDTH {
                g.board.set(x, y, Shape::O);
            }
        }
        g.over = false;
        g.spawn();
        assert!(g.over);
    }

    #[test]
    fn level_up_after_10_lines() {
        let mut g = game(8);
        g.lines = 9;
        for x in 4..WIDTH {
            g.board.set(x, HEIGHT - 1, Shape::O);
        }
        g.current = Piece { shape: Shape::I, rotation: 0, x: 0, y: 0 };
        g.hard_drop();
        assert_eq!(g.lines, 10);
        assert_eq!(g.level, 2);
    }

    #[test]
    fn delay_decreases_with_level() {
        let mut g = game(9);
        g.level = 1;
        assert_eq!(g.delay_ms(), 500);
        g.level = 10;
        assert_eq!(g.delay_ms(), 140);
        g.level = 11;
        assert_eq!(g.delay_ms(), 100);
        g.level = 12;
        assert_eq!(g.delay_ms(), 80);
        g.level = 100;
        assert_eq!(g.delay_ms(), 80);
    }

    #[test]
    fn rotation_uses_wall_kick_at_right_wall() {
        let mut g = game(10);
        g.current = Piece { shape: Shape::I, rotation: 1, x: 7, y: 0 };
        g.rotate(1);
        assert_eq!(g.current.rotation, 2);
        assert_eq!(g.current.x, 6);
    }

    #[test]
    fn rotation_uses_floor_kick() {
        let mut g = game(12);
        g.current = Piece { shape: Shape::I, rotation: 2, x: 0, y: (HEIGHT - 3) as i32 };
        g.rotate(1);
        assert_eq!(g.current.rotation, 3);
        assert_eq!(g.current.y, (HEIGHT - 4) as i32);
    }

    #[test]
    fn movement_is_blocked_by_walls() {
        let mut g = game(17);
        g.current = Piece { shape: Shape::T, rotation: 0, x: 0, y: 0 };
        assert!(!g.move_left());
        assert_eq!(g.current.x, 0);
        assert!(g.move_right());
        assert_eq!(g.current.x, 1);
        assert!(g.move_left());
        assert_eq!(g.current.x, 0);
    }
}
