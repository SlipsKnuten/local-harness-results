//! The seven tetrominoes: spawn shapes and per-kind metadata.

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
    /// All kinds, in a stable index order (index == [`Tetromino::idx`]).
    pub const ALL: [Tetromino; 7] = [
        Tetromino::I,
        Tetromino::O,
        Tetromino::T,
        Tetromino::S,
        Tetromino::Z,
        Tetromino::J,
        Tetromino::L,
    ];

    /// Zero-based index into [`Tetromino::ALL`].
    pub const fn idx(self) -> usize {
        match self {
            Tetromino::I => 0,
            Tetromino::O => 1,
            Tetromino::T => 2,
            Tetromino::S => 3,
            Tetromino::Z => 4,
            Tetromino::J => 5,
            Tetromino::L => 6,
        }
    }

    /// Side length of the square bounding box the piece rotates inside.
    pub const fn size(self) -> u8 {
        match self {
            Tetromino::I => 4,
            Tetromino::O => 2,
            _ => 3,
        }
    }

    /// The four filled cells `(row, col)` in the spawn orientation, within the
    /// `size` x `size` box.
    pub const fn base_cells(self) -> [(i8, i8); 4] {
        match self {
            Tetromino::I => [(1, 0), (1, 1), (1, 2), (1, 3)],
            Tetromino::O => [(0, 0), (0, 1), (1, 0), (1, 1)],
            Tetromino::T => [(0, 1), (1, 0), (1, 1), (1, 2)],
            Tetromino::S => [(0, 1), (0, 2), (1, 0), (1, 1)],
            Tetromino::Z => [(0, 0), (0, 1), (1, 1), (1, 2)],
            Tetromino::J => [(0, 0), (1, 0), (1, 1), (1, 2)],
            Tetromino::L => [(0, 2), (1, 0), (1, 1), (1, 2)],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_exactly_four_cells() {
        for kind in Tetromino::ALL {
            assert_eq!(kind.base_cells().len(), 4, "kind {kind:?}");
        }
    }

    #[test]
    fn idx_is_consistent_with_all() {
        for (i, kind) in Tetromino::ALL.iter().enumerate() {
            assert_eq!(kind.idx(), i);
        }
    }

    #[test]
    fn base_cells_fit_inside_box() {
        for kind in Tetromino::ALL {
            let n = kind.size() as i8;
            for (r, c) in kind.base_cells() {
                assert!(r >= 0 && r < n, "kind {kind:?} row {r}");
                assert!(c >= 0 && c < n, "kind {kind:?} col {c}");
            }
        }
    }
}
