// Tetromino definitions using the Super Rotation System (SRS).
// Cell coordinates are (x, y) offsets inside the piece's bounding box, with
// y increasing DOWNWARD. Kick tables are the canonical tetris.wiki tables
// with the y component negated (wiki convention is y-up).

#[derive(Copy, Clone)]
pub enum PieceType { I, J, S, T, O, L, Z }

pub fn ordinal(t: PieceType) -> usize {
    match t {
        PieceType::I => 0,
        PieceType::J => 1,
        PieceType::S => 2,
        PieceType::T => 3,
        PieceType::O => 4,
        PieceType::L => 5,
        PieceType::Z => 6,
    }
}

pub fn from_ordinal(n: usize) -> PieceType {
    match n {
        0 => PieceType::I,
        1 => PieceType::J,
        2 => PieceType::S,
        3 => PieceType::T,
        4 => PieceType::O,
        5 => PieceType::L,
        _ => PieceType::Z,
    }
}

// SHAPES[ordinal][rotation state] -> the 4 cell offsets of the piece.
pub const SHAPES: [[[(usize, usize); 4]; 4]; 7] = [
    // I (4x4 box)
    [
        [(0, 1), (1, 1), (2, 1), (3, 1)],
        [(2, 0), (2, 1), (2, 2), (2, 3)],
        [(0, 2), (1, 2), (2, 2), (3, 2)],
        [(1, 0), (1, 1), (1, 2), (1, 3)],
    ],
    // J (3x3 box)
    [
        [(0, 0), (0, 1), (1, 1), (2, 1)],
        [(2, 0), (1, 0), (1, 1), (1, 2)],
        [(2, 2), (2, 1), (1, 1), (0, 1)],
        [(0, 2), (1, 2), (1, 1), (1, 0)],
    ],
    // S (3x3 box)
    [
        [(1, 0), (2, 0), (0, 1), (1, 1)],
        [(2, 1), (2, 2), (1, 0), (1, 1)],
        [(1, 2), (0, 2), (2, 1), (1, 1)],
        [(0, 1), (0, 0), (1, 2), (1, 1)],
    ],
    // T (3x3 box)
    [
        [(1, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (1, 1), (2, 1), (1, 2)],
        [(1, 2), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (0, 1), (1, 1), (1, 2)],
    ],
    // O (3x3 box, rotation invariant)
    [
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
    ],
    // L (3x3 box)
    [
        [(2, 0), (0, 1), (1, 1), (2, 1)],
        [(2, 2), (1, 0), (1, 1), (1, 2)],
        [(0, 2), (2, 1), (1, 1), (0, 1)],
        [(0, 0), (1, 2), (1, 1), (1, 0)],
    ],
    // Z (3x3 box)
    [
        [(0, 0), (1, 0), (1, 1), (2, 1)],
        [(2, 0), (2, 1), (1, 1), (1, 2)],
        [(2, 2), (1, 2), (1, 1), (0, 1)],
        [(0, 2), (0, 1), (1, 1), (1, 0)],
    ],
];

// Wall kick table rows are ordered: 0->R, R->0, R->2, 2->R, 2->L, L->2, L->0, 0->L.
const KICK_JLSTZ: [[(i32, i32); 5]; 8] = [
    [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
    [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
    [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
    [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
    [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
    [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
    [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
    [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
];

const KICK_I: [[(i32, i32); 5]; 8] = [
    [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)],
    [(0, 0), (2, 0), (-1, 0), (2, -1), (-1, 2)],
    [(0, 0), (-1, 0), (2, 0), (-1, -2), (2, 1)],
    [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)],
    [(0, 0), (2, 0), (-1, 0), (2, -1), (-1, 2)],
    [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)],
    [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)],
    [(0, 0), (-1, 0), (2, 0), (-1, -2), (2, 1)],
];

fn kick_row(from: usize, to: usize) -> usize {
    match (from, to) {
        (0, 1) => 0,
        (1, 0) => 1,
        (1, 2) => 2,
        (2, 1) => 3,
        (2, 3) => 4,
        (3, 2) => 5,
        (3, 0) => 6,
        (0, 3) => 7,
        _ => 0,
    }
}

pub fn is_i_piece(kind: PieceType) -> bool {
    match kind {
        PieceType::I => true,
        _ => false,
    }
}

// The five offsets to try when rotating `kind` from rotation `from` to `to`.
pub fn kick_offsets(kind: PieceType, from: usize, to: usize) -> [(i32, i32); 5] {
    let row = kick_row(from, to);
    if is_i_piece(kind) {
        KICK_I[row]
    } else {
        KICK_JLSTZ[row]
    }
}
