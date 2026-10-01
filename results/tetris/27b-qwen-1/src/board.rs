//! The static game field: a 10x20 grid of locked cells and line clearing.

use crate::tetromino::Tetromino;

/// Field width in cells (standard Tetris).
pub const WIDTH: usize = 10;
/// Field height in cells (standard Tetris).
pub const HEIGHT: usize = 20;

#[derive(Clone)]
pub struct Board {
    cells: [[Option<Tetromino>; WIDTH]; HEIGHT],
}

impl Board {
    pub fn new() -> Self {
        Board {
            cells: [[None; WIDTH]; HEIGHT],
        }
    }

    /// Whether a board coordinate is free for the falling piece to occupy.
    ///
    /// Out of bounds horizontally or below the bottom: not free.
    /// Above the top (`r < 0`): free (pieces may start partly above the visible field).
    /// Otherwise: free only if the cell is empty.
    pub fn is_free(&self, r: i32, c: i32) -> bool {
        if c < 0 || c >= WIDTH as i32 {
            return false;
        }
        if r >= HEIGHT as i32 {
            return false;
        }
        if r < 0 {
            return true;
        }
        self.cells[r as usize][c as usize].is_none()
    }

    /// The tetromino locked at `(r, c)`, if any.
    pub fn cell(&self, r: usize, c: usize) -> Option<Tetromino> {
        self.cells[r][c]
    }

    /// Lock a cell.
    pub fn set(&mut self, r: usize, c: usize, t: Tetromino) {
        self.cells[r][c] = Some(t);
    }

    /// Remove every full row, shifting the remaining rows down to fill the gaps.
    /// Returns the number of rows cleared.
    pub fn clear_full_rows(&mut self) -> usize {
        let mut full = [false; HEIGHT];
        let mut cleared = 0usize;
        for (r, row) in self.cells.iter().enumerate() {
            if row.iter().all(|c| c.is_some()) {
                full[r] = true;
                cleared += 1;
            }
        }
        if cleared == 0 {
            return 0;
        }

        // Copy the surviving rows to the bottom, preserving their vertical order.
        let mut next: [[Option<Tetromino>; WIDTH]; HEIGHT] = [[None; WIDTH]; HEIGHT];
        let mut write = HEIGHT;
        for (r, row) in self.cells.iter().enumerate().rev() {
            if !full[r] {
                write -= 1;
                next[write] = *row;
            }
        }
        self.cells = next;
        cleared
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_free_respects_bounds() {
        let b = Board::new();
        assert!(b.is_free(0, 0));
        assert!(!b.is_free(0, -1));
        assert!(!b.is_free(0, WIDTH as i32));
        assert!(!b.is_free(HEIGHT as i32, 0));
        assert!(b.is_free(-1, 0), "above the top is free");
    }

    #[test]
    fn clears_a_full_bottom_row_and_shifts_down() {
        let mut b = Board::new();
        for c in 0..WIDTH {
            b.set(HEIGHT - 1, c, Tetromino::I);
        }
        // A marker cell just above the bottom row, on the left.
        b.set(HEIGHT - 2, 0, Tetromino::T);

        let cleared = b.clear_full_rows();
        assert_eq!(cleared, 1);
        // The marker shifted down to the bottom.
        assert_eq!(b.cell(HEIGHT - 1, 0), Some(Tetromino::T));
        // The top row is empty.
        assert!(b.cell(0, 0).is_none());
    }

    #[test]
    fn clears_multiple_rows() {
        let mut b = Board::new();
        for r in (HEIGHT - 3)..HEIGHT {
            for c in 0..WIDTH {
                b.set(r, c, Tetromino::O);
            }
        }
        assert_eq!(b.clear_full_rows(), 3);
        // Everything is now empty.
        for r in 0..HEIGHT {
            for c in 0..WIDTH {
                assert!(b.cell(r, c).is_none());
            }
        }
    }

    #[test]
    fn does_not_clear_an_incomplete_row() {
        let mut b = Board::new();
        for c in 0..WIDTH - 1 {
            b.set(HEIGHT - 1, c, Tetromino::I);
        }
        assert_eq!(b.clear_full_rows(), 0);
        assert_eq!(b.cell(HEIGHT - 1, 0), Some(Tetromino::I));
    }
}
