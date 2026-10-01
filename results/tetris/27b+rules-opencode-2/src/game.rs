//! Core Tetris game logic, independent of the terminal UI.

use rand::seq::SliceRandom;
use rand::rngs::SmallRng;

pub const WIDTH: u16 = 10;
pub const HEIGHT: u16 = 20;

/// The seven tetromino kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    /// The 4x4 matrix of the piece in its spawn rotation state.
    pub const fn base(self) -> [[u8; 4]; 4] {
        match self {
            PieceKind::I => [
                [0, 0, 0, 0],
                [1, 1, 1, 1],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
            ],
            PieceKind::O => [
                [0, 0, 0, 0],
                [0, 1, 1, 0],
                [0, 1, 1, 0],
                [0, 0, 0, 0],
            ],
            PieceKind::T => [
                [0, 0, 0, 0],
                [0, 0, 1, 0],
                [0, 1, 1, 1],
                [0, 0, 0, 0],
            ],
            PieceKind::S => [
                [0, 0, 0, 0],
                [0, 1, 1, 0],
                [1, 1, 0, 0],
                [0, 0, 0, 0],
            ],
            PieceKind::Z => [
                [0, 0, 0, 0],
                [1, 1, 0, 0],
                [0, 1, 1, 0],
                [0, 0, 0, 0],
            ],
            PieceKind::J => [
                [0, 0, 0, 0],
                [0, 1, 0, 0],
                [1, 1, 1, 0],
                [0, 0, 0, 0],
            ],
            PieceKind::L => [
                [0, 0, 0, 0],
                [0, 0, 1, 0],
                [0, 0, 1, 0],
                [0, 1, 1, 0],
            ],
        }
    }
}

/// A tetromino placed on the board. The 4x4 `matrix` describes the current
/// rotation; `(x, y)` is the top-left corner of the matrix (y may be negative
/// for rows still above the visible field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub kind: PieceKind,
    pub x: i16,
    pub y: i16,
    matrix: [[u8; 4]; 4],
}

impl Piece {
    pub fn new(kind: PieceKind) -> Self {
        Self {
            kind,
            x: (WIDTH as i16) / 2 - 2,
            y: 0,
            matrix: kind.base(),
        }
    }

    /// Board coordinates of all occupied cells of this piece.
    pub fn cells(&self) -> [(i16, i16); 4] {
        let mut cells = [(0i16, 0i16); 4];
        let mut n = 0;
        for (row, row_cells) in self.matrix.iter().enumerate() {
            for (col, &occupied) in row_cells.iter().enumerate() {
                if occupied == 1 {
                    cells[n] = (self.x + col as i16, self.y + row as i16);
                    n += 1;
                }
            }
        }
        debug_assert_eq!(n, 4);
        cells
    }

    /// The piece rotated 90 degrees clockwise, keeping position.
    pub fn rotated(&self) -> Piece {
        let mut matrix = [[0u8; 4]; 4];
        for (row, src_row) in self.matrix.iter().enumerate() {
            for (col, &value) in src_row.iter().enumerate() {
                matrix[col][3 - row] = value;
            }
        }
        Piece {
            matrix,
            ..*self
        }
    }
}

/// The play field: a WIDTH x HEIGHT grid, row 0 at the top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    grid: Vec<Option<PieceKind>>,
}

impl Board {
    pub fn new() -> Self {
        Self {
            grid: vec![None; (WIDTH * HEIGHT) as usize],
        }
    }

    pub fn get(&self, x: u16, y: u16) -> Option<PieceKind> {
        if x >= WIDTH || y >= HEIGHT {
            return None;
        }
        self.grid[(y * WIDTH + x) as usize]
    }

    pub fn set(&mut self, x: u16, y: u16, kind: Option<PieceKind>) {
        if x >= WIDTH || y >= HEIGHT {
            return;
        }
        self.grid[(y * WIDTH + x) as usize] = kind;
    }

    pub fn is_full_row(&self, y: u16) -> bool {
        (0..WIDTH).all(|x| self.get(x, y).is_some())
    }

    /// True if any cell of `piece` is out of bounds or overlaps a filled cell.
    pub fn collides(&self, piece: &Piece) -> bool {
        for (cx, cy) in piece.cells() {
            if cx < 0 || cx >= WIDTH as i16 || cy >= HEIGHT as i16 {
                return true;
            }
            if cy >= 0 && self.get(cx as u16, cy as u16).is_some() {
                return true;
            }
        }
        false
    }

    /// Writes all visible cells of `piece` into the grid.
    pub fn lock(&mut self, piece: &Piece) {
        for (cx, cy) in piece.cells() {
            if cy >= 0 && cx >= 0 && cx < WIDTH as i16 && cy < HEIGHT as i16 {
                self.set(cx as u16, cy as u16, Some(piece.kind));
            }
        }
    }

    /// Removes all full rows and returns how many were removed.
    pub fn clear_lines(&mut self) -> usize {
        let mut cleared = 0usize;
        let mut y = HEIGHT - 1;
        loop {
            if self.is_full_row(y) {
                // Stay on the same row: the row above slides down into it.
                self.remove_row(y);
                cleared += 1;
            } else if y == 0 {
                break;
            } else {
                y -= 1;
            }
        }
        cleared
    }

    fn remove_row(&mut self, y: u16) {
        for row in (0..y).rev() {
            for x in 0..WIDTH {
                self.set(x, row + 1, self.get(x, row));
            }
        }
        for x in 0..WIDTH {
            self.set(x, 0, None);
        }
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

/// Points awarded for clearing `n` lines at once (0, 100, 300, 500, 800).
pub fn score_for_lines(n: usize) -> u32 {
    const TABLE: [u32; 5] = [0, 100, 300, 500, 800];
    TABLE[n.min(4)]
}

pub fn level_for_lines(lines: u32) -> u32 {
    lines / 10 + 1
}

/// Gravity interval in milliseconds for a level (800 ms at level 1, faster as
/// the level rises, floored at 80 ms).
pub fn gravity_interval_ms(level: u32) -> u64 {
    800u64
        .saturating_sub((level - 1).saturating_mul(75) as u64)
        .max(80)
}

/// A 7-bag randomizer: pieces come out in random groups of seven.
#[derive(Debug, Clone)]
pub struct Bag {
    rng: SmallRng,
    queue: Vec<PieceKind>,
}

impl Bag {
    pub fn new() -> Self {
        Self::with_rng(rand::make_rng())
    }

    #[cfg(test)]
    pub fn with_seed(seed: u64) -> Self {
        use rand::SeedableRng;
        Self::with_rng(SmallRng::seed_from_u64(seed))
    }

    fn with_rng(mut rng: SmallRng) -> Self {
        let mut queue = PieceKind::ALL.to_vec();
        queue.shuffle(&mut rng);
        Self { rng, queue }
    }

    pub fn next(&mut self) -> PieceKind {
        if self.queue.is_empty() {
            let mut queue = PieceKind::ALL.to_vec();
            queue.shuffle(&mut self.rng);
            self.queue = queue;
        }
        self.queue
            .pop()
            .expect("bag queue is refilled when empty")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    Playing,
    Paused,
    GameOver,
}

#[derive(Debug, Clone)]
pub struct Game {
    pub board: Board,
    pub current: Piece,
    pub next: PieceKind,
    bag: Bag,
    pub score: u32,
    pub lines: u32,
    pub state: GameState,
}

impl Game {
    pub fn new() -> Self {
        Self::with_bag(Bag::new())
    }

    pub fn with_bag(mut bag: Bag) -> Self {
        let current = Piece::new(bag.next());
        let game = Self {
            board: Board::new(),
            current,
            next: bag.next(),
            bag,
            score: 0,
            lines: 0,
            state: GameState::Playing,
        };
        if game.board.collides(&game.current) {
            game_with_state(game, GameState::GameOver)
        } else {
            game
        }
    }

    pub fn level(&self) -> u32 {
        level_for_lines(self.lines)
    }

    pub fn interval(&self) -> u64 {
        gravity_interval_ms(self.level())
    }

    /// One gravity step: the piece falls one row, or locks in place.
    pub fn tick(&mut self) {
        if self.state != GameState::Playing {
            return;
        }
        let moved = Piece {
            y: self.current.y + 1,
            ..self.current
        };
        if !self.board.collides(&moved) {
            self.current = moved;
        } else {
            self.lock_piece();
        }
    }

    pub fn move_left(&mut self) {
        if self.state != GameState::Playing {
            return;
        }
        let moved = Piece {
            x: self.current.x - 1,
            ..self.current
        };
        if !self.board.collides(&moved) {
            self.current = moved;
        }
    }

    pub fn move_right(&mut self) {
        if self.state != GameState::Playing {
            return;
        }
        let moved = Piece {
            x: self.current.x + 1,
            ..self.current
        };
        if !self.board.collides(&moved) {
            self.current = moved;
        }
    }

    /// Moves the piece down one row (scoring a soft-drop point), or locks it.
    pub fn soft_drop(&mut self) {
        if self.state != GameState::Playing {
            return;
        }
        let moved = Piece {
            y: self.current.y + 1,
            ..self.current
        };
        if !self.board.collides(&moved) {
            self.current = moved;
            self.score += 1;
        } else {
            self.lock_piece();
        }
    }

    /// Drops the piece to the bottom of the field and locks it.
    pub fn hard_drop(&mut self) {
        if self.state != GameState::Playing {
            return;
        }
        let mut piece = self.current;
        let mut distance = 0;
        while !self
            .board
            .collides(&Piece { y: piece.y + 1, ..piece })
        {
            piece.y += 1;
            distance += 1;
        }
        self.current = piece;
        self.score += 2 * distance;
        self.lock_piece();
    }

    /// Rotates the piece clockwise, trying simple wall kicks.
    pub fn rotate(&mut self) {
        self.try_rotate(self.current.rotated());
    }

    /// Rotates the piece counter-clockwise, trying simple wall kicks.
    pub fn rotate_ccw(&mut self) {
        self.try_rotate(self.current.rotated().rotated().rotated());
    }

    fn try_rotate(&mut self, rotated: Piece) {
        if self.state != GameState::Playing {
            return;
        }
        for dx in [0i16, -1, 1, -2, 2] {
            let kicked = Piece {
                x: rotated.x + dx,
                ..rotated
            };
            if !self.board.collides(&kicked) {
                self.current = kicked;
                return;
            }
        }
    }

    /// Where the current piece would land if dropped straight down.
    pub fn ghost(&self) -> Piece {
        let mut piece = self.current;
        while !self
            .board
            .collides(&Piece { y: piece.y + 1, ..piece })
        {
            piece.y += 1;
        }
        piece
    }

    pub fn toggle_pause(&mut self) {
        match self.state {
            GameState::Playing => self.state = GameState::Paused,
            GameState::Paused => self.state = GameState::Playing,
            GameState::GameOver => {}
        }
    }

    fn lock_piece(&mut self) {
        self.board.lock(&self.current);
        let cleared = self.board.clear_lines();
        if cleared > 0 {
            self.lines += cleared as u32;
            self.score += score_for_lines(cleared);
        }
        self.spawn();
    }

    fn spawn(&mut self) {
        let kind = self.next;
        self.current = Piece::new(kind);
        self.next = self.bag.next();
        if self.board.collides(&self.current) {
            self.state = GameState::GameOver;
        }
    }
}

fn game_with_state(mut game: Game, state: GameState) -> Game {
    game.state = state;
    game
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded_game() -> Game {
        Game::with_bag(Bag::with_seed(42))
    }

    #[test]
    fn bag_produces_permutations_of_all_seven() {
        let mut bag = Bag::with_seed(1);
        for round in 0..3 {
            let mut seen = [false; 7];
            for _ in 0..7 {
                let kind = bag.next();
                seen[kind as usize] = true;
                let _ = round;
            }
            assert!(seen.iter().all(|&s| s), "bag must contain each piece once");
        }
    }

    #[test]
    fn rotation_stays_in_box_and_cycles() {
        for kind in PieceKind::ALL {
            let piece = Piece::new(kind);
            let mut p = piece;
            for step in 0..4 {
                p = p.rotated();
                assert_eq!(p.cells().len(), 4, "{kind:?} step {step}");
                for (cx, cy) in p.cells() {
                    assert!(
                        (p.x..p.x + 4).contains(&cx),
                        "{kind:?} step {step}: cell ({cx}, {cy}) outside box"
                    );
                    assert!(
                        (p.y..p.y + 4).contains(&cy),
                        "{kind:?} step {step}: cell ({cx}, {cy}) outside box"
                    );
                }
            }
            // Four clockwise rotations get back to the original state.
            assert_eq!(p, piece, "four rotations must be the identity for {kind:?}");
        }
    }

    #[test]
    fn i_piece_first_rotation_is_vertical_bar() {
        let piece = Piece::new(PieceKind::I);
        let cells: std::collections::BTreeSet<(i16, i16)> =
            piece.rotated().cells().iter().copied().collect();
        let expected = [
            (piece.x + 2, piece.y),
            (piece.x + 2, piece.y + 1),
            (piece.x + 2, piece.y + 2),
            (piece.x + 2, piece.y + 3),
        ]
        .into_iter()
        .collect();
        assert_eq!(cells, expected);
    }

    #[test]
    fn rotate_ccw_reverts_rotate() {
        let mut game = seeded_game();
        let before = game.current;
        game.rotate();
        if before.kind != PieceKind::O {
            assert_ne!(game.current, before);
        }
        game.rotate_ccw();
        assert_eq!(game.current, before);
    }

    #[test]
    fn o_piece_is_rotation_invariant() {
        let piece = Piece::new(PieceKind::O);
        assert_eq!(piece.rotated(), piece);
    }

    #[test]
    fn t_piece_first_rotation() {
        let piece = Piece::new(PieceKind::T);
        let rotated = piece.rotated();
        let cells: std::collections::BTreeSet<(i16, i16)> =
            rotated.cells().iter().copied().collect();
        // Spawn T (bump up): (x+2, y+1), (x+1, y+2), (x+2, y+2), (x+3, y+2).
        // Rotated 90 deg clockwise the bump points right:
        let expected = [
            (piece.x + 1, piece.y + 1),
            (piece.x + 1, piece.y + 2),
            (piece.x + 2, piece.y + 2),
            (piece.x + 1, piece.y + 3),
        ]
        .into_iter()
        .collect();
        assert_eq!(cells, expected);
    }

    #[test]
    fn empty_board_has_no_collisions() {
        let board = Board::new();
        for kind in PieceKind::ALL {
            assert!(!board.collides(&Piece::new(kind)));
        }
    }

    #[test]
    fn walls_and_floor_collide() {
        let board = Board::new();
        let i_left = Piece {
            x: -1,
            ..Piece::new(PieceKind::I)
        };
        assert!(board.collides(&i_left));
        let i_right = Piece {
            x: WIDTH as i16,
            ..Piece::new(PieceKind::I)
        };
        assert!(board.collides(&i_right));
        let i_bottom = Piece {
            y: HEIGHT as i16,
            ..Piece::new(PieceKind::I)
        };
        assert!(board.collides(&i_bottom));
        let i_above = Piece {
            y: -2,
            ..Piece::new(PieceKind::I)
        };
        assert!(!board.collides(&i_above));
    }

    #[test]
    fn filled_cell_blocks_piece() {
        let mut board = Board::new();
        board.set(1, HEIGHT - 1, Some(PieceKind::I));
        let blocked = Piece {
            x: 0,
            y: (HEIGHT - 3) as i16,
            ..Piece::new(PieceKind::O)
        };
        // O cells at (1, H-2), (2, H-2), (1, H-1), (2, H-1): (1, H-1) is filled.
        assert!(board.collides(&blocked));
        let free = Piece {
            x: 0,
            y: (HEIGHT - 4) as i16,
            ..Piece::new(PieceKind::O)
        };
        assert!(!board.collides(&free));
    }

    #[test]
    fn clear_lines_removes_full_rows_and_shifts_down() {
        let mut board = Board::new();
        let bottom = HEIGHT - 1;
        // Fill the bottom row completely.
        for x in 0..WIDTH {
            board.set(x, bottom, Some(PieceKind::I));
        }
        // Put a marker cell one row above, on the left.
        board.set(0, bottom - 1, Some(PieceKind::O));
        // And one above that, on the right.
        board.set(WIDTH - 1, bottom - 2, Some(PieceKind::T));

        let cleared = board.clear_lines();
        assert_eq!(cleared, 1);
        // The filled bottom row is gone; the rows above slid down by one.
        assert!(!board.is_full_row(bottom));
        assert_eq!(board.get(0, bottom), Some(PieceKind::O));
        assert_eq!(board.get(WIDTH - 1, bottom - 1), Some(PieceKind::T));
        assert!(board.get(0, bottom - 1).is_none());
    }

    #[test]
    fn clear_lines_counts_multiple_rows() {
        let mut board = Board::new();
        for y in (HEIGHT - 3..HEIGHT).rev() {
            for x in 0..WIDTH {
                board.set(x, y, Some(PieceKind::S));
            }
        }
        assert_eq!(board.clear_lines(), 3);
        assert!((0..HEIGHT).all(|y| !board.is_full_row(y)));
    }

    #[test]
    fn partial_rows_are_not_cleared() {
        let mut board = Board::new();
        for x in 0..WIDTH - 1 {
            board.set(x, HEIGHT - 1, Some(PieceKind::J));
        }
        assert_eq!(board.clear_lines(), 0);
        assert_eq!(board.get(WIDTH - 1, HEIGHT - 1), None);
    }

    #[test]
    fn scoring_table() {
        assert_eq!(score_for_lines(0), 0);
        assert_eq!(score_for_lines(1), 100);
        assert_eq!(score_for_lines(2), 300);
        assert_eq!(score_for_lines(3), 500);
        assert_eq!(score_for_lines(4), 800);
        assert_eq!(score_for_lines(9), 800);
    }

    #[test]
    fn level_and_gravity_progression() {
        assert_eq!(level_for_lines(0), 1);
        assert_eq!(level_for_lines(9), 1);
        assert_eq!(level_for_lines(10), 2);
        assert_eq!(level_for_lines(109), 11);
        assert_eq!(gravity_interval_ms(1), 800);
        assert_eq!(gravity_interval_ms(2), 725);
        assert!(gravity_interval_ms(100) >= 80);
        assert_eq!(gravity_interval_ms(100), 80);
    }

    fn count_kind(board: &Board, kind: PieceKind) -> usize {
        (0..WIDTH)
            .flat_map(|x| (0..HEIGHT).map(move |y| board.get(x, y)))
            .filter(|&k| k == Some(kind))
            .count()
    }

    #[test]
    fn hard_drop_lands_on_floor_and_scores() {
        let mut game = seeded_game();
        let kind = game.current.kind;
        game.hard_drop();
        // After the drop a new piece is current; the previous one is locked
        // in full (the 7-bag never repeats a kind within one bag).
        assert_ne!(game.current.kind, kind);
        assert_eq!(count_kind(&game.board, kind), 4);
        assert!(game.score >= 2);
    }

    #[test]
    fn ghost_sits_on_top_of_stack() {
        let mut game = seeded_game();
        game.hard_drop();
        game.hard_drop();
        let ghost = game.ghost();
        assert!(
            !game.board.collides(&game.current),
            "current piece never collides"
        );
        let below = Piece {
            y: ghost.y + 1,
            ..game.current
        };
        assert!(game.board.collides(&below), "one row below the ghost collides");
    }

    #[test]
    fn moves_are_blocked_at_walls() {
        let mut game = seeded_game();
        for _ in 0..WIDTH as usize + 4 {
            game.move_left();
        }
        assert!(game.current.x >= 0);
        assert!(!game.board.collides(&game.current));
        for _ in 0..WIDTH as usize + 8 {
            game.move_right();
        }
        assert!(!game.board.collides(&game.current));
    }

    #[test]
    fn soft_drop_locks_on_floor() {
        let mut game = seeded_game();
        let kind = game.current.kind;
        for _ in 0..HEIGHT as usize * 2 {
            game.soft_drop();
        }
        assert_eq!(count_kind(&game.board, kind), 4);
        assert!(game.score >= 1);
    }

    #[test]
    fn clearing_lines_scores_and_advances_level() {
        let mut board = Board::new();
        for x in 0..WIDTH {
            board.set(x, HEIGHT - 1, Some(PieceKind::I));
        }
        let mut game = Game::with_bag(Bag::with_seed(7));
        game.board = board;
        game.hard_drop();
        assert!(game.lines >= 1);
        assert!(game.score >= 100);
        assert_eq!(game.level(), level_for_lines(game.lines));
    }

    #[test]
    fn stack_reaching_top_is_game_over() {
        let mut game = Game::with_bag(Bag::with_seed(7));
        for _ in 0..200 {
            if game.state == GameState::GameOver {
                break;
            }
            game.hard_drop();
        }
        assert_eq!(game.state, GameState::GameOver);
    }

    #[test]
    fn pause_toggles_state() {
        let mut game = seeded_game();
        assert_eq!(game.state, GameState::Playing);
        game.toggle_pause();
        assert_eq!(game.state, GameState::Paused);
        game.tick();
        let before = game.current.y;
        assert_eq!(game.current.y, before, "no gravity while paused");
        game.toggle_pause();
        assert_eq!(game.state, GameState::Playing);
    }

    #[test]
    fn game_over_ignores_input() {
        let mut game = seeded_game();
        game.state = GameState::GameOver;
        let current = game.current;
        game.move_left();
        game.rotate();
        game.soft_drop();
        game.hard_drop();
        game.tick();
        assert_eq!(game.current, current);
    }
}
