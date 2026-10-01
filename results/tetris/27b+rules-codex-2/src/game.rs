//! Core Tetris game logic, fully decoupled from rendering.

use std::time::Duration;

use ratatui::style::Color;

pub const WIDTH: usize = 10;
pub const HEIGHT: usize = 20;
const CELLS: usize = WIDTH * HEIGHT;

/// Score per cleared-line count (index = number of lines cleared).
const LINE_SCORES: [u64; 5] = [0, 100, 300, 500, 800];

/// Simplified wall-kick offsets tried (in order) after a rotation.
const KICKS: [(i16, i16); 6] = [(0, 0), (-1, 0), (1, 0), (0, -1), (-2, 0), (2, 0)];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub const ALL: [PieceKind; 7] = [
        PieceKind::I,
        PieceKind::O,
        PieceKind::T,
        PieceKind::S,
        PieceKind::Z,
        PieceKind::J,
        PieceKind::L,
    ];

    pub fn color(self) -> Color {
        match self {
            PieceKind::I => Color::Cyan,
            PieceKind::O => Color::Yellow,
            PieceKind::T => Color::Magenta,
            PieceKind::S => Color::Green,
            PieceKind::Z => Color::Red,
            PieceKind::J => Color::Blue,
            PieceKind::L => Color::Rgb(230, 126, 34),
        }
    }

    /// The four SRS rotation states. Each state lists the four occupied cells
    /// as (row, col) offsets inside the piece's bounding box.
    fn rotation_states(self) -> &'static [[(i16, i16); 4]; 4] {
        static I: [[(i16, i16); 4]; 4] = [
            [(1, 0), (1, 1), (1, 2), (1, 3)],
            [(0, 2), (1, 2), (2, 2), (3, 2)],
            [(2, 0), (2, 1), (2, 2), (2, 3)],
            [(0, 1), (1, 1), (2, 1), (3, 1)],
        ];
        static O: [[(i16, i16); 4]; 4] = [
            [(0, 1), (0, 2), (1, 1), (1, 2)],
            [(0, 1), (0, 2), (1, 1), (1, 2)],
            [(0, 1), (0, 2), (1, 1), (1, 2)],
            [(0, 1), (0, 2), (1, 1), (1, 2)],
        ];
        static T: [[(i16, i16); 4]; 4] = [
            [(0, 1), (1, 0), (1, 1), (1, 2)],
            [(0, 1), (1, 1), (1, 2), (2, 1)],
            [(1, 0), (1, 1), (1, 2), (2, 1)],
            [(0, 1), (1, 0), (1, 1), (2, 1)],
        ];
        static S: [[(i16, i16); 4]; 4] = [
            [(0, 1), (0, 2), (1, 0), (1, 1)],
            [(0, 1), (1, 1), (1, 2), (2, 2)],
            [(1, 1), (1, 2), (2, 0), (2, 1)],
            [(0, 0), (1, 0), (1, 1), (2, 1)],
        ];
        static Z: [[(i16, i16); 4]; 4] = [
            [(0, 0), (0, 1), (1, 1), (1, 2)],
            [(0, 2), (1, 1), (1, 2), (2, 1)],
            [(1, 0), (1, 1), (2, 1), (2, 2)],
            [(0, 1), (1, 0), (1, 1), (2, 0)],
        ];
        static J: [[(i16, i16); 4]; 4] = [
            [(0, 0), (1, 0), (1, 1), (1, 2)],
            [(0, 1), (0, 2), (1, 1), (2, 1)],
            [(1, 0), (1, 1), (1, 2), (2, 2)],
            [(0, 1), (1, 1), (2, 0), (2, 1)],
        ];
        static L: [[(i16, i16); 4]; 4] = [
            [(0, 2), (1, 0), (1, 1), (1, 2)],
            [(0, 1), (1, 1), (2, 1), (2, 2)],
            [(1, 0), (1, 1), (1, 2), (2, 0)],
            [(0, 0), (0, 1), (1, 1), (2, 1)],
        ];
        match self {
            PieceKind::I => &I,
            PieceKind::O => &O,
            PieceKind::T => &T,
            PieceKind::S => &S,
            PieceKind::Z => &Z,
            PieceKind::J => &J,
            PieceKind::L => &L,
        }
    }

    /// The spawn (state 0) cells, used for the "next piece" preview.
    pub fn spawn_state(self) -> [(i16, i16); 4] {
        self.rotation_states()[0]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotateDir {
    Clockwise,
    CounterClockwise,
}

/// A falling tetromino: kind, rotation state, and position of the bounding box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub kind: PieceKind,
    pub rotation: usize,
    pub x: i16,
    pub y: i16,
}

impl Piece {
    /// Spawn position: centered horizontally, top of the field.
    pub fn spawn(kind: PieceKind) -> Self {
        Self { kind, rotation: 0, x: 3, y: 0 }
    }

    /// Absolute board coordinates of the four occupied cells, as (x, y).
    pub fn cells(&self) -> [(i16, i16); 4] {
        let state = self.kind.rotation_states()[self.rotation];
        state.map(|(row, col)| (self.x + col, self.y + row))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    cells: [Option<PieceKind>; CELLS],
}

impl Board {
    pub fn new() -> Self {
        Self { cells: [None; CELLS] }
    }

    /// The piece kind locked at (x, y), if any.
    pub fn get(&self, x: i16, y: i16) -> Option<PieceKind> {
        if x < 0 || x >= WIDTH as i16 || y < 0 || y >= HEIGHT as i16 {
            return None;
        }
        self.cells[y as usize * WIDTH + x as usize]
    }

    /// Lock a piece into the board (all cells must be in bounds).
    pub fn set(&mut self, x: i16, y: i16, kind: PieceKind) {
        if x < 0 || x >= WIDTH as i16 || y < 0 || y >= HEIGHT as i16 {
            return;
        }
        self.cells[y as usize * WIDTH + x as usize] = Some(kind);
    }

    pub fn lock(&mut self, piece: &Piece) {
        for (x, y) in piece.cells() {
            self.set(x, y, piece.kind);
        }
    }

    /// True if any cell of `piece` is out of bounds or overlaps a locked cell.
    pub fn collides(&self, piece: &Piece) -> bool {
        piece.cells().iter().any(|&(x, y)| {
            if x < 0 || x >= WIDTH as i16 || y < 0 || y >= HEIGHT as i16 {
                return true;
            }
            self.cells[y as usize * WIDTH + x as usize].is_some()
        })
    }

    #[cfg(test)]
    pub fn occupied_count(&self) -> usize {
        self.cells.iter().filter(|c| c.is_some()).count()
    }

    /// Remove all full rows, dropping the rows above down. Returns the count.
    pub fn clear_full_rows(&mut self) -> usize {
        let mut write_row = HEIGHT;
        for read_row in (0..HEIGHT).rev() {
            let full = (0..WIDTH)
                .all(|x| self.cells[read_row * WIDTH + x].is_some());
            if full {
                continue;
            }
            if read_row != write_row - 1 {
                self.cells.copy_within(
                    read_row * WIDTH..(read_row + 1) * WIDTH,
                    (write_row - 1) * WIDTH,
                );
            }
            write_row -= 1;
        }
        for cell in &mut self.cells[..write_row * WIDTH] {
            *cell = None;
        }
        write_row
    }
}

pub struct Game {
    board: Board,
    piece: Piece,
    next: PieceKind,
    bag: Vec<PieceKind>,
    rng: u64,
    score: u64,
    lines: u32,
    level: u32,
    over: bool,
}

impl Game {
    /// Start a new game seeded from the current time.
    pub fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xC0FFEE);
        Self::with_seed(seed)
    }

    /// Start a game with a deterministic piece sequence (for tests).
    pub fn with_seed(seed: u64) -> Self {
        let mut game = Game {
            board: Board::new(),
            piece: Piece::spawn(PieceKind::I),
            next: PieceKind::I,
            bag: Vec::new(),
            rng: seed,
            score: 0,
            lines: 0,
            level: 1,
            over: false,
        };
        game.next = game.next_kind();
        game.spawn_piece();
        game
    }

    pub fn restart(&mut self) {
        *self = Self::new();
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    /// Mutable board access, used by the unit tests.
    #[cfg(test)]
    pub fn board_mut(&mut self) -> &mut Board {
        &mut self.board
    }

    pub fn piece(&self) -> &Piece {
        &self.piece
    }

    pub fn preview_kind(&self) -> PieceKind {
        self.next
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

    pub fn is_over(&self) -> bool {
        self.over
    }

    /// Time between gravity steps; shorter at higher levels.
    pub fn gravity_interval(&self) -> Duration {
        let millis = 600u64.saturating_sub(u64::from(self.level - 1) * 50).max(80);
        Duration::from_millis(millis)
    }

    /// A copy of the current piece dropped to the floor (for ghost rendering).
    pub fn ghost_piece(&self) -> Piece {
        let mut ghost = self.piece;
        while !self.collides_moved_down(&ghost) {
            ghost.y += 1;
        }
        ghost
    }

    pub fn move_left(&mut self) -> bool {
        if self.over {
            return false;
        }
        let mut piece = self.piece;
        piece.x -= 1;
        if self.board.collides(&piece) {
            return false;
        }
        self.piece = piece;
        true
    }

    pub fn move_right(&mut self) -> bool {
        if self.over {
            return false;
        }
        let mut piece = self.piece;
        piece.x += 1;
        if self.board.collides(&piece) {
            return false;
        }
        self.piece = piece;
        true
    }

    pub fn rotate(&mut self, dir: RotateDir) -> bool {
        if self.over {
            return false;
        }
        let to = match dir {
            RotateDir::Clockwise => (self.piece.rotation + 1) % 4,
            RotateDir::CounterClockwise => (self.piece.rotation + 3) % 4,
        };
        for (dx, dy) in KICKS {
            let mut piece = self.piece;
            piece.rotation = to;
            piece.x += dx;
            piece.y += dy;
            if !self.board.collides(&piece) {
                self.piece = piece;
                return true;
            }
        }
        false
    }

    /// Move down one row. Returns false when the piece is already grounded.
    pub fn soft_drop(&mut self) -> bool {
        if self.over || self.is_grounded() {
            return false;
        }
        self.piece.y += 1;
        self.score += 1;
        true
    }

    /// Drop to the floor immediately and lock. Returns the distance fallen.
    pub fn hard_drop(&mut self) -> u16 {
        if self.over {
            return 0;
        }
        let mut distance = 0u16;
        while !self.is_grounded() {
            self.piece.y += 1;
            distance += 1;
        }
        self.score += 2 * u64::from(distance);
        self.lock_piece();
        distance
    }

    /// One gravity step: fall one row, or lock when grounded.
    pub fn tick(&mut self) {
        if self.over {
            return;
        }
        if self.is_grounded() {
            self.lock_piece();
        } else {
            self.piece.y += 1;
        }
    }

    fn is_grounded(&self) -> bool {
        let mut down = self.piece;
        down.y += 1;
        self.board.collides(&down)
    }

    fn collides_moved_down(&self, piece: &Piece) -> bool {
        let mut down = *piece;
        down.y += 1;
        self.board.collides(&down)
    }

    fn lock_piece(&mut self) {
        self.board.lock(&self.piece);
        let cleared = self.board.clear_full_rows();
        if cleared > 0 {
            self.score += LINE_SCORES[cleared] * u64::from(self.level);
            self.lines += cleared as u32;
            self.level = self.lines / 10 + 1;
        }
        self.spawn_piece();
    }

    fn spawn_piece(&mut self) {
        let kind = self.next;
        self.next = self.next_kind();
        self.piece = Piece::spawn(kind);
        if self.board.collides(&self.piece) {
            self.over = true;
        }
    }

    /// Next piece from the 7-bag randomizer.
    fn next_kind(&mut self) -> PieceKind {
        if self.bag.is_empty() {
            self.bag = self.fresh_bag();
        }
        self.bag.pop().unwrap()
    }

    fn fresh_bag(&mut self) -> Vec<PieceKind> {
        let mut bag = PieceKind::ALL.to_vec();
        for index in (1..bag.len()).rev() {
            let swap = (self.lcg() % (index + 1) as u64) as usize;
            bag.swap(index, swap);
        }
        bag
    }

    fn lcg(&mut self) -> u64 {
        self.rng = self
            .rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.rng >> 33
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game_with_piece(kind: PieceKind) -> Game {
        let mut game = Game::with_seed(0);
        game.piece = Piece::spawn(kind);
        game
    }

    #[test]
    fn new_game_starts_playable() {
        let game = Game::with_seed(42);
        assert!(!game.is_over());
        assert_eq!(game.score(), 0);
        assert_eq!(game.lines(), 0);
        assert_eq!(game.level(), 1);
        assert!(!game.board().collides(game.piece()));
    }

    #[test]
    fn every_piece_spawns_inside_the_board() {
        for kind in PieceKind::ALL {
            let piece = Piece::spawn(kind);
            for (x, y) in piece.cells() {
                assert!(
                    (0..WIDTH as i16).contains(&x) && (0..HEIGHT as i16).contains(&y),
                    "{:?} spawns out of bounds: {x},{y}",
                    kind
                );
            }
        }
    }

    #[test]
    fn walls_stop_horizontal_movement() {
        for kind in PieceKind::ALL {
            let mut game = game_with_piece(kind);
            let mut moved = 0usize;
            while game.move_left() {
                moved += 1;
            }
            assert!(
                game.piece().cells().iter().all(|&(x, _)| x >= 0),
                "{:?} leaked past the left wall",
                kind
            );
            assert!(moved > 0);

            let mut game = game_with_piece(kind);
            let mut moved = 0usize;
            while game.move_right() {
                moved += 1;
            }
            assert!(
                game.piece().cells().iter().all(|&(x, _)| x < WIDTH as i16),
                "{:?} leaked past the right wall",
                kind
            );
            assert!(moved > 0);
        }
    }

    #[test]
    fn rotation_changes_shape_and_round_trips() {
        let mut game = game_with_piece(PieceKind::T);
        let start = game.piece().cells();
        for _ in 0..4 {
            assert!(game.rotate(RotateDir::Clockwise));
        }
        assert_eq!(game.piece().cells(), start);
        assert!(game.rotate(RotateDir::CounterClockwise));
        assert_ne!(game.piece().cells(), start);
    }

    #[test]
    fn rotation_uses_wall_kicks() {
        let mut game = game_with_piece(PieceKind::I);
        assert!(game.rotate(RotateDir::Clockwise)); // vertical, col 5
        for _ in 0..4 {
            assert!(game.move_right());
        }
        assert!(!game.move_right()); // vertical col now at 9
        assert_eq!(game.piece().x, 7);
        // State 2 would span cols 7..10 -> kicked left by one.
        assert!(game.rotate(RotateDir::Clockwise));
        assert_eq!(game.piece().x, 6);
        assert!(game.piece().cells().iter().all(|&(x, _)| x < WIDTH as i16));
    }

    #[test]
    fn gravity_locks_piece_on_floor() {
        let mut game = game_with_piece(PieceKind::O);
        for _ in 0..HEIGHT + 2 {
            game.tick();
        }
        assert_eq!(game.board().occupied_count(), 4);
        assert!(!game.is_over());
    }

    #[test]
    fn piece_locks_on_gravity_tick_at_floor() {
        let mut game = game_with_piece(PieceKind::O);
        for _ in 0..18 {
            assert!(game.soft_drop());
        }
        assert!(!game.soft_drop()); // grounded
        assert!(game.move_left()); // sliding on the floor does not lock
        assert_eq!(game.board().occupied_count(), 0);
        game.tick();
        assert_eq!(game.board().occupied_count(), 4);
    }

    #[test]
    fn soft_drop_scores_one_point_per_row() {
        let mut game = game_with_piece(PieceKind::O);
        assert!(game.soft_drop());
        assert_eq!(game.score(), 1);
        assert_eq!(game.piece().y, 1);
    }

    #[test]
    fn hard_drop_lands_scores_and_locks() {
        let mut game = game_with_piece(PieceKind::O);
        let distance = game.hard_drop();
        assert_eq!(distance, 18);
        assert_eq!(game.score(), 36);
        assert_eq!(game.board().occupied_count(), 4);
        assert_eq!(game.lines(), 0);
    }

    #[test]
    fn completing_one_line_clears_and_scores() {
        let mut game = game_with_piece(PieceKind::O);
        let bottom = HEIGHT as i16 - 1;
        for x in 0..WIDTH as i16 {
            if x != 4 && x != 5 {
                game.board_mut().set(x, bottom, PieceKind::I);
            }
        }
        game.hard_drop();
        assert_eq!(game.lines(), 1);
        assert_eq!(game.score(), 100 + 36); // line + hard drop
        assert_eq!(game.board().occupied_count(), 2); // top half of O survived
        assert_eq!(game.board().get(4, bottom), Some(PieceKind::O));
    }

    #[test]
    fn clearing_four_lines_scores_tetris() {
        let mut game = game_with_piece(PieceKind::I);
        for y in 16..HEIGHT as i16 {
            for x in 0..WIDTH as i16 {
                if x != 4 {
                    game.board_mut().set(x, y, PieceKind::I);
                }
            }
        }
        assert!(game.rotate(RotateDir::Clockwise)); // vertical, col 5
        assert!(game.move_left()); // vertical, col 4
        let distance = game.hard_drop();
        assert_eq!(distance, 16);
        assert_eq!(game.lines(), 4);
        assert_eq!(game.level(), 1);
        assert_eq!(game.score(), 800 + 32);
        assert_eq!(game.board().occupied_count(), 0);
    }

    #[test]
    fn level_up_every_ten_lines() {
        let mut game = game_with_piece(PieceKind::O);
        for _ in 0..10 {
            let bottom = HEIGHT as i16 - 1;
            for x in 0..WIDTH as i16 {
                if x != 4 && x != 5 {
                    game.board_mut().set(x, bottom, PieceKind::I);
                }
            }
            game.hard_drop();
            game.piece = Piece::spawn(PieceKind::O);
        }
        assert_eq!(game.lines(), 10);
        assert_eq!(game.level(), 2);
        assert!(game.gravity_interval() < Duration::from_millis(600));
    }

    #[test]
    fn spawn_blocking_stack_ends_game() {
        let mut game = game_with_piece(PieceKind::O);
        for y in 0..4i16 {
            for x in 0..WIDTH as i16 {
                if x != 9 {
                    game.board_mut().set(x, y, PieceKind::I);
                }
            }
        }
        assert!(!game.is_over());
        game.tick(); // locks on the stack; next spawn collides
        assert!(game.is_over());
        game.tick(); // ticks are ignored after game over
        assert!(game.is_over());
    }

    #[test]
    fn seven_bag_contains_each_piece_once() {
        let mut game = Game::with_seed(7);
        // Current piece + preview + five more = the first full 7-bag.
        let mut kinds = vec![game.piece().kind, game.preview_kind()];
        for _ in 0..5 {
            kinds.push(game.next_kind());
        }
        for kind in PieceKind::ALL {
            assert_eq!(
                kinds.iter().filter(|k| **k == kind).count(),
                1,
                "bag must contain each piece exactly once"
            );
        }
    }

    #[test]
    fn ghost_piece_sits_on_the_floor() {
        let game = game_with_piece(PieceKind::O);
        let ghost = game.ghost_piece();
        assert_eq!(ghost.y, 18);
        let mut down = ghost;
        down.y += 1;
        assert!(game.board().collides(&down));
    }

    #[test]
    fn ghost_piece_equals_piece_when_grounded() {
        let mut game = game_with_piece(PieceKind::O);
        for _ in 0..18 {
            game.soft_drop();
        }
        assert_eq!(game.ghost_piece(), *game.piece());
    }

    #[test]
    fn gravity_interval_is_valid_and_decreases() {
        let mut game = game_with_piece(PieceKind::O);
        let first = game.gravity_interval();
        assert!(first <= Duration::from_millis(600));
        // force a level change
        game.lines = 10;
        game.level = 2;
        assert!(game.gravity_interval() < first);
    }
}
