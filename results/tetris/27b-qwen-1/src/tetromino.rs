//! The seven tetrominoes, their rotations and their colours.

use ratatui::style::Color;

/// A single clockwise rotation of a cell inside an `n x n` box: `(r, c) -> (c, n - 1 - r)`.
fn cw(cell: (i32, i32), n: i32) -> (i32, i32) {
    let (r, c) = cell;
    (c, n - 1 - r)
}

/// The seven Tetris tetrominoes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// All seven pieces, used to build a shuffled random "bag".
    pub const ALL: [Tetromino; 7] = [
        Tetromino::I,
        Tetromino::O,
        Tetromino::T,
        Tetromino::S,
        Tetromino::Z,
        Tetromino::J,
        Tetromino::L,
    ];

    /// Size of the square bounding box the piece rotates in.
    pub const fn size(self) -> usize {
        match self {
            Tetromino::I => 4,
            Tetromino::O => 2,
            _ => 3,
        }
    }

    /// Occupied cells `(row, col)` in the piece's own coordinates at rotation 0.
    pub const fn base_cells(self) -> [(i32, i32); 4] {
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

    /// Occupied cells for a given rotation `rot` (0..=3), rotating clockwise.
    pub fn cells(self, rot: u8) -> [(i32, i32); 4] {
        let n = self.size() as i32;
        let mut out = self.base_cells();
        for _ in 0..(rot % 4) {
            let (a, b, c, d) = (out[0], out[1], out[2], out[3]);
            out = [cw(a, n), cw(b, n), cw(c, n), cw(d, n)];
        }
        out
    }

    /// A distinct terminal colour for each piece.
    pub const fn color(self) -> Color {
        match self {
            Tetromino::I => Color::Rgb(0, 195, 215),
            Tetromino::O => Color::Rgb(238, 200, 42),
            Tetromino::T => Color::Rgb(195, 92, 222),
            Tetromino::S => Color::Rgb(92, 202, 92),
            Tetromino::Z => Color::Rgb(232, 72, 72),
            Tetromino::J => Color::Rgb(72, 122, 236),
            Tetromino::L => Color::Rgb(242, 152, 42),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Tetromino;

    #[test]
    fn every_piece_has_four_cells() {
        for t in Tetromino::ALL {
            assert_eq!(t.cells(0).len(), 4, "{t:?} should have 4 cells");
        }
    }

    #[test]
    fn rotation_round_trip_returns_to_base() {
        for t in Tetromino::ALL {
            assert_eq!(t.cells(0), t.cells(4), "{t:?}: 4 rotations should be the identity");
            assert_eq!(t.cells(0), t.cells(8));
        }
    }

    fn cell_set(t: Tetromino, r: u8) -> std::collections::BTreeSet<(i32, i32)> {
        t.cells(r).into_iter().collect()
    }

    #[test]
    fn o_piece_rotation_is_stable() {
        let base = cell_set(Tetromino::O, 0);
        for r in 0..4u8 {
            assert_eq!(base, cell_set(Tetromino::O, r), "O should occupy the same cells at rotation {r}");
        }
    }

    #[test]
    fn rotation_stays_within_bounds_and_is_unique() {
        for t in Tetromino::ALL {
            let n = t.size() as i32;
            let mut seen = std::collections::BTreeSet::new();
            for r in 0..4u8 {
                for (x, y) in t.cells(r) {
                    assert!((0..n).contains(&x), "{t:?} rot {r}: x {x} out of bounds");
                    assert!((0..n).contains(&y), "{t:?} rot {r}: y {y} out of bounds");
                }
                let cells = cell_set(t, r);
                // The O is symmetric, so its rotations repeat; the other six pieces have 4 distinct states.
                if t != Tetromino::O {
                    assert!(!seen.contains(&cells), "{t:?}: duplicate rotation state at rot {r}");
                }
                seen.insert(cells);
            }
            if t == Tetromino::O {
                assert_eq!(seen.len(), 1, "O should have a single rotation state");
            } else {
                assert_eq!(seen.len(), 4, "{t:?} should have 4 distinct rotation states");
            }
        }
    }
}
