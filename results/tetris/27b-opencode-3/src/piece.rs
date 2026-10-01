use ratatui::style::Color;

/// The seven tetromino types. The variant order matches the index order of
/// `SHAPES` below (I = 0, O = 1, ... L = 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        PieceType::I,
        PieceType::O,
        PieceType::T,
        PieceType::S,
        PieceType::Z,
        PieceType::J,
        PieceType::L,
    ];

    pub fn color(&self) -> Color {
        match self {
            PieceType::I => Color::Cyan,
            PieceType::O => Color::Yellow,
            PieceType::T => Color::Magenta,
            PieceType::S => Color::Green,
            PieceType::Z => Color::Red,
            PieceType::J => Color::Blue,
            PieceType::L => Color::Rgb(255, 140, 0),
        }
    }
}

/// A tetromino with a position on the board and a rotation state (0..4).
#[derive(Debug, Clone, Copy)]
pub struct Piece {
    pub piece_type: PieceType,
    pub x: i16,
    pub y: i16,
    pub rotation: u8,
}

/// Cell offsets (dx, dy) for every piece, for each of its four rotations.
/// `dy` increases downward. Each piece occupies a 4x4 bounding box.
pub const SHAPES: [[[(i16, i16); 4]; 4]; 7] = [
    // I
    [
        [(0, 1), (1, 1), (2, 1), (3, 1)],
        [(2, 0), (2, 1), (2, 2), (2, 3)],
        [(0, 2), (1, 2), (2, 2), (3, 2)],
        [(1, 0), (1, 1), (1, 2), (1, 3)],
    ],
    // O
    [
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
    ],
    // T
    [
        [(1, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (1, 1), (2, 1), (1, 2)],
        [(0, 1), (1, 1), (2, 1), (1, 2)],
        [(1, 0), (0, 1), (1, 1), (1, 2)],
    ],
    // S
    [
        [(1, 0), (2, 0), (0, 1), (1, 1)],
        [(1, 0), (1, 1), (2, 1), (2, 2)],
        [(1, 1), (2, 1), (0, 2), (1, 2)],
        [(0, 0), (0, 1), (1, 1), (1, 2)],
    ],
    // Z
    [
        [(0, 0), (1, 0), (1, 1), (2, 1)],
        [(2, 0), (1, 1), (2, 1), (1, 2)],
        [(0, 1), (1, 1), (1, 2), (2, 2)],
        [(1, 0), (0, 1), (1, 1), (0, 2)],
    ],
    // J
    [
        [(0, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (1, 2)],
        [(0, 1), (1, 1), (2, 1), (2, 2)],
        [(1, 0), (1, 1), (0, 2), (1, 2)],
    ],
    // L
    [
        [(2, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (1, 1), (1, 2), (2, 2)],
        [(0, 1), (1, 1), (2, 1), (0, 2)],
        [(0, 0), (1, 0), (1, 1), (1, 2)],
    ],
];

impl Piece {
    /// The four (dx, dy) cell offsets for this piece's current rotation.
    pub fn cells(&self) -> [(i16, i16); 4] {
        SHAPES[self.piece_type as usize][self.rotation as usize]
    }

    /// The absolute (x, y) board coordinates of this piece's four cells.
    pub fn absolute_cells(&self) -> [(i16, i16); 4] {
        let mut out = [(0, 0); 4];
        let cells = self.cells();
        for i in 0..4 {
            out[i] = (cells[i].0 + self.x, cells[i].1 + self.y);
        }
        out
    }
}
