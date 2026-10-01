use std::time::Duration;

use rand::prelude::*;
use rand::rngs::StdRng;

pub const COLS: usize = 10;
pub const ROWS: usize = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PieceKind {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl PieceKind {
    pub fn idx(self) -> usize {
        match self {
            PieceKind::I => 0,
            PieceKind::O => 1,
            PieceKind::T => 2,
            PieceKind::S => 3,
            PieceKind::Z => 4,
            PieceKind::J => 5,
            PieceKind::L => 6,
        }
    }

    /// Bounding-box size and spawn-state cells of a piece.
    pub fn shape(self) -> (usize, [(usize, usize); 4]) {
        match self {
            PieceKind::I => (4, [(0, 1), (1, 1), (2, 1), (3, 1)]),
            PieceKind::O => (2, [(0, 0), (1, 0), (0, 1), (1, 1)]),
            PieceKind::T => (3, [(1, 0), (0, 1), (1, 1), (2, 1)]),
            PieceKind::S => (3, [(1, 0), (2, 0), (0, 1), (1, 1)]),
            PieceKind::Z => (3, [(0, 0), (1, 0), (1, 1), (2, 1)]),
            PieceKind::J => (3, [(0, 0), (0, 1), (1, 1), (2, 1)]),
            PieceKind::L => (3, [(2, 0), (0, 1), (1, 1), (2, 1)]),
        }
    }
}

fn rotated(cell: (usize, usize), size: usize, times: u8) -> (usize, usize) {
    let (mut x, mut y) = cell;
    for _ in 0..times {
        let nx = size - 1 - y;
        let ny = x;
        (x, y) = (nx, ny);
    }
    (x, y)
}

pub struct Game {
    pub grid: [u8; ROWS * COLS],
    pub kind: PieceKind,
    pub next: PieceKind,
    pub x: isize,
    pub y: isize,
    pub rot: u8,
    pub score: u32,
    pub lines: u32,
    pub level: u32,
    pub over: bool,
    bag: Vec<PieceKind>,
    rng: StdRng,
}

impl Game {
    pub fn new() -> Self {
        let mut game = Game {
            grid: [0; ROWS * COLS],
            kind: PieceKind::I,
            next: PieceKind::O,
            x: 0,
            y: 0,
            rot: 0,
            score: 0,
            lines: 0,
            level: 1,
            over: false,
            bag: Vec::new(),
            rng: StdRng::seed_from_u64(rand::random()),
        };
        game.spawn();
        game
    }

    pub fn reset(&mut self) {
        *self = Game::new();
    }

    pub fn game_over(&self) -> bool {
        self.over
    }

    /// Guideline-style gravity interval per level, clamped to a playable range.
    pub fn gravity_interval(&self) -> Duration {
        let level = self.level as f64;
        let ms = (0.7_f64 - (level - 1.0) * 0.007).powf(level - 1.0) * 1000.0;
        Duration::from_millis(ms.clamp(40.0, 1000.0) as u64)
    }

    pub fn cells(&self) -> [(isize, isize); 4] {
        self.cells_at(self.x, self.y, self.rot)
    }

    pub fn cells_at(&self, x: isize, y: isize, rot: u8) -> [(isize, isize); 4] {
        let (size, base) = self.kind.shape();
        base.map(|(cx, cy)| {
            let (rx, ry) = rotated((cx, cy), size, rot);
            (x + rx as isize, y + ry as isize)
        })
    }

    /// Row the falling piece would land on if dropped immediately.
    pub fn ghost_y(&self) -> isize {
        let mut gy = self.y;
        while !self.collides(self.cells_at(self.x, gy + 1, self.rot)) {
            gy += 1;
        }
        gy
    }

    pub fn move_h(&mut self, dx: isize) {
        if self.over {
            return;
        }
        let cells = self.cells().map(|(x, y)| (x + dx, y));
        if !self.collides(cells) {
            self.x += dx;
        }
    }

    pub fn move_down(&mut self) -> bool {
        if self.over {
            return false;
        }
        let cells = self.cells().map(|(x, y)| (x, y + 1));
        if self.collides(cells) {
            return false;
        }
        self.y += 1;
        true
    }

    pub fn move_down_possible(&self) -> bool {
        !self.collides(self.cells().map(|(x, y)| (x, y + 1)))
    }

    pub fn soft_drop(&mut self) {
        if self.move_down() {
            self.score += 1;
        }
    }

    pub fn hard_drop(&mut self) {
        if self.over {
            return;
        }
        let ghost = self.ghost_y();
        self.score += ((ghost - self.y) * 2) as u32;
        self.y = ghost;
        self.lock();
    }

    pub fn rotate_cw(&mut self) {
        if self.over || self.kind == PieceKind::O {
            return;
        }
        self.try_rotate((self.rot + 1) % 4);
    }

    pub fn rotate_ccw(&mut self) {
        if self.over || self.kind == PieceKind::O {
            return;
        }
        self.try_rotate((self.rot + 3) % 4);
    }

    /// Lock the current piece into the grid, clear lines, and spawn the next one.
    pub fn lock(&mut self) {
        let mut topped_out = false;
        for (cx, cy) in self.cells() {
            if cy < 0 {
                topped_out = true;
                continue;
            }
            self.grid[cy as usize * COLS + cx as usize] = self.kind.idx() as u8 + 1;
        }
        if topped_out {
            self.over = true;
            return;
        }

        let cleared = self.clear_lines();
        let points = [0, 100, 300, 500, 800][cleared] * self.level;
        self.score += points;
        self.lines += cleared as u32;
        self.level = self.lines / 10 + 1;
        self.spawn();
    }

    fn try_rotate(&mut self, new_rot: u8) {
        const KICKS: [(isize, isize); 6] = [(0, 0), (-1, 0), (1, 0), (-2, 0), (2, 0), (0, -1)];
        let old_rot = self.rot;
        for (kx, ky) in KICKS {
            self.rot = new_rot;
            let cells = self
                .cells_at(self.x + kx, self.y + ky, new_rot);
            if !self.collides(cells) {
                self.x += kx;
                self.y += ky;
                return;
            }
        }
        self.rot = old_rot;
    }

    fn collides(&self, cells: [(isize, isize); 4]) -> bool {
        cells.iter().any(|&(cx, cy)| {
            if cx < 0 || cx >= COLS as isize || cy >= ROWS as isize {
                return true;
            }
            if cy >= 0 && self.grid[cy as usize * COLS + cx as usize] != 0 {
                return true;
            }
            false
        })
    }

    fn clear_lines(&mut self) -> usize {
        let mut write_row = ROWS;
        for read_row in (0..ROWS).rev() {
            let full = (0..COLS).all(|c| self.grid[read_row * COLS + c] != 0);
            if full {
                continue;
            }
            write_row -= 1;
            if write_row != read_row {
                let row = self.grid[read_row * COLS..read_row * COLS + COLS].to_vec();
                self.grid[write_row * COLS..write_row * COLS + COLS].copy_from_slice(&row);
            }
        }
        for row in 0..write_row {
            self.grid[row * COLS..(row + 1) * COLS].fill(0);
        }
        write_row
    }

    fn draw_piece(&mut self) -> PieceKind {
        if self.bag.is_empty() {
            self.bag = [
                PieceKind::I,
                PieceKind::O,
                PieceKind::T,
                PieceKind::S,
                PieceKind::Z,
                PieceKind::J,
                PieceKind::L,
            ]
            .to_vec();
            self.bag.shuffle(&mut self.rng);
        }
        self.bag.pop().expect("bag is refilled when empty")
    }

    fn spawn(&mut self) {
        self.kind = self.next;
        self.next = self.draw_piece();
        let (size, _) = self.kind.shape();
        self.x = (COLS as isize - size as isize) / 2;
        self.y = 0;
        self.rot = 0;
        if self.collides(self.cells()) {
            self.over = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seven_bag_contains_every_piece_once() {
        let mut game = Game::new();
        // The initial spawn already consumed one bag draw (`game.next`),
        // so the first full bag is next + the next 6 draws.
        let mut window = vec![game.next];
        for _ in 0..14 {
            window.push(game.draw_piece());
        }
        for chunk in window.chunks(7).take(2) {
            let mut seen = [false; 7];
            for kind in chunk {
                seen[kind.idx()] = true;
            }
            assert!(seen.iter().all(|&s| s));
        }
    }

    #[test]
    fn lock_clears_full_line_and_scores() {
        let mut game = Game::new();
        for col in 0..COLS {
            game.grid[(ROWS - 1) * COLS + col] = 1;
        }
        game.kind = PieceKind::O;
        game.rot = 0;
        game.x = 0;
        game.y = (ROWS - 2) as isize;
        let score_before = game.score;
        game.lock();
        assert_eq!(game.lines, 1);
        assert_eq!(game.score, score_before + 100);
        // The row above the cleared line falls down.
        assert_eq!(game.grid[(ROWS - 1) * COLS + 0], 2);
        assert_eq!(game.grid[(ROWS - 1) * COLS + 1], 2);
        assert_eq!(game.grid[(ROWS - 1) * COLS + 2], 0);
    }

    #[test]
    fn hard_drop_locks_piece_on_floor() {
        let mut game = Game::new();
        let value = game.kind.idx() as u8 + 1;
        game.hard_drop();
        assert_eq!(game.grid.iter().filter(|&&c| c == value).count(), 4);
        assert!(!game.game_over());
    }

    #[test]
    fn rotation_near_wall_stays_in_bounds() {
        let mut game = Game::new();
        game.kind = PieceKind::T;
        game.rot = 0;
        game.x = 0;
        game.y = 0;
        for _ in 0..8 {
            game.rotate_cw();
            for (cx, cy) in game.cells() {
                assert!(cx >= 0 && cx < COLS as isize);
                assert!(cy < ROWS as isize);
            }
        }
    }

    #[test]
    fn ghost_lands_on_stack() {
        let mut game = Game::new();
        game.kind = PieceKind::O;
        game.x = 0;
        game.y = 0;
        game.grid[(ROWS - 1) * COLS + 0] = 5;
        game.grid[(ROWS - 1) * COLS + 1] = 5;
        assert_eq!(game.ghost_y(), (ROWS - 3) as isize);
    }

    #[test]
    fn gravity_speeds_up_with_level() {
        let mut game = Game::new();
        let first = game.gravity_interval();
        game.level = 10;
        assert!(game.gravity_interval() < first);
        game.level = 50;
        assert!(game.gravity_interval() >= Duration::from_millis(40));
    }
}
