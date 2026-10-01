use ratatui::style::Color;

/// The seven standard tetrominoes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tetromino {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl Tetromino {
    /// All seven pieces, used to seed the 7-bag randomizer.
    pub const ALL: [Tetromino; 7] = [
        Tetromino::I,
        Tetromino::O,
        Tetromino::T,
        Tetromino::S,
        Tetromino::Z,
        Tetromino::J,
        Tetromino::L,
    ];

    /// Side length of the square box the piece rotates inside (SRS-style).
    pub fn box_size(&self) -> u8 {
        match self {
            Tetromino::I => 4,
            Tetromino::O => 2,
            _ => 3,
        }
    }

    /// Occupied cells in the spawn (rotation 0) orientation as `(x, y)` within the box.
    fn base_cells(&self) -> [(u8, u8); 4] {
        match self {
            Tetromino::I => [(0, 1), (1, 1), (2, 1), (3, 1)],
            Tetromino::O => [(0, 0), (1, 0), (0, 1), (1, 1)],
            Tetromino::T => [(0, 1), (1, 1), (2, 1), (1, 0)],
            Tetromino::S => [(1, 0), (2, 0), (0, 1), (1, 1)],
            Tetromino::Z => [(0, 0), (1, 0), (1, 1), (2, 1)],
            Tetromino::J => [(0, 0), (0, 1), (1, 1), (2, 1)],
            Tetromino::L => [(2, 0), (0, 1), (1, 1), (2, 1)],
        }
    }

    /// Occupied cells for a given rotation (0..=3, clockwise from spawn).
    ///
    /// Rotation is performed by rotating the base cells clockwise inside the
    /// bounding box: `(x, y) -> (n - 1 - y, x)`.
    pub fn cells(&self, rotation: u8) -> [(i32, i32); 4] {
        let n = self.box_size();
        let mut cells = self.base_cells();
        for _ in 0..(rotation % 4) {
            cells = cells.map(|(x, y)| (n - 1 - y, x));
        }
        cells.map(|(x, y)| (x as i32, y as i32))
    }

    /// Standard guideline color for the piece.
    pub fn color(&self) -> Color {
        match self {
            Tetromino::I => Color::Cyan,
            Tetromino::O => Color::Yellow,
            Tetromino::T => Color::Magenta,
            Tetromino::S => Color::Green,
            Tetromino::Z => Color::Red,
            Tetromino::J => Color::Blue,
            Tetromino::L => Color::Rgb(255, 140, 0), // orange
        }
    }
}
