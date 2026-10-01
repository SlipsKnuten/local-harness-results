//! Tetromino shapes (SRS rotation states) and the live piece.

use crate::board::Kind;

/// A piece on the board: its kind, rotation (0-3) and the top-left corner
/// `(x, y)` of its bounding box, in board coordinates.
#[derive(Debug, Clone, Copy)]
pub struct Piece {
    pub kind: Kind,
    pub rot: usize,
    pub x: isize,
    pub y: isize,
}

/// SRS rotation states: `SHAPES[kind][rot]` is the four block offsets inside
/// the piece's bounding box (4x4 for I, 2x2 for O, 3x3 for the rest).
const SHAPES: [[[(isize, isize); 4]; 4]; 7] = [
    // I
    [
        [(0, 1), (1, 1), (2, 1), (3, 1)],
        [(2, 0), (2, 1), (2, 2), (2, 3)],
        [(0, 2), (1, 2), (2, 2), (3, 2)],
        [(1, 0), (1, 1), (1, 2), (1, 3)],
    ],
    // O
    [[(0, 0), (1, 0), (0, 1), (1, 1)]; 4],
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
    /// Spawn a piece in the top-center of the board.
    pub fn spawn(kind: Kind) -> Self {
        let x = match kind {
            Kind::I => 3,
            Kind::O => 4,
            _ => 3,
        };
        Self {
            kind,
            rot: 0,
            x,
            y: 0,
        }
    }

    /// The absolute board coordinates of the piece's four blocks.
    pub fn cells(&self) -> [(isize, isize); 4] {
        let offs = SHAPES[self.kind.as_idx()][self.rot];
        let mut out = [(0, 0); 4];
        for i in 0..4 {
            out[i] = (self.x + offs[i].0, self.y + offs[i].1);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;

    #[test]
    fn every_rotation_has_four_distinct_cells() {
        for kind in Kind::all() {
            for rot in 0..4 {
                let piece = Piece {
                    kind,
                    rot,
                    x: 0,
                    y: 0,
                };
                let cells = piece.cells();
                for i in 0..4 {
                    for j in (i + 1)..4 {
                        assert_ne!(
                            cells[i], cells[j],
                            "duplicate cell for {:?} in rotation {rot}",
                            kind
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn all_spawns_fit_on_an_empty_board() {
        let board = Board::new();
        for kind in Kind::all() {
            let piece = Piece::spawn(kind);
            for (x, y) in piece.cells() {
                assert!(
                    board.is_free(x, y),
                    "{kind:?} spawn cell ({x}, {y}) is not free"
                );
            }
        }
    }

    #[test]
    fn rotation_cycles_every_four_steps() {
        for kind in Kind::all() {
            let mut rot = 0;
            for _ in 0..4 {
                rot = (rot + 1) % 4;
            }
            let mut p = Piece::spawn(kind);
            p.rot = rot;
            assert_eq!(p.rot, 0);
        }
    }

    #[test]
    fn i_piece_rotation_uses_full_column() {
        let mut piece = Piece::spawn(Kind::I);
        piece.rot = 1;
        let cells = piece.cells();
        let xs: std::collections::HashSet<isize> = cells.iter().map(|&(x, _)| x).collect();
        let ys: std::collections::BTreeSet<isize> = cells.iter().map(|&(_, y)| y).collect();
        assert_eq!(
            xs.len(),
            1,
            "vertical I should occupy one column: {cells:?}"
        );
        assert_eq!(ys.len(), 4, "vertical I should span four rows: {cells:?}");
    }
}
