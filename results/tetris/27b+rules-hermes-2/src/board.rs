//! The game board: a fixed-height grid of optional piece colors.

pub const WIDTH: usize = 10;
pub const HEIGHT: usize = 20;

/// The seven tetromino colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

impl Kind {
    /// Index into shape tables (must match `tetromino::SHAPES` ordering).
    pub const fn as_idx(self) -> usize {
        match self {
            Kind::I => 0,
            Kind::O => 1,
            Kind::T => 2,
            Kind::S => 3,
            Kind::Z => 4,
            Kind::J => 5,
            Kind::L => 6,
        }
    }

    /// One of each kind, in `as_idx` order.
    pub const fn all() -> [Kind; 7] {
        [
            Kind::I,
            Kind::O,
            Kind::T,
            Kind::S,
            Kind::Z,
            Kind::J,
            Kind::L,
        ]
    }
}

/// A 10x20 field; row 0 is the top.
#[derive(Debug, Default, Clone)]
pub struct Board {
    grid: [[Option<Kind>; WIDTH]; HEIGHT],
}

impl Board {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cell(&self, x: usize, y: usize) -> Option<Kind> {
        self.grid[y][x]
    }

    pub fn set(&mut self, x: usize, y: usize, kind: Kind) {
        self.grid[y][x] = Some(kind);
    }

    /// Whether a falling piece may occupy `(x, y)`: inside the horizontal
    /// bounds, not below the floor, and either above the visible area or on
    /// an empty cell.
    pub fn is_free(&self, x: isize, y: isize) -> bool {
        if x < 0 || x >= WIDTH as isize || y >= HEIGHT as isize {
            return false;
        }
        if y < 0 {
            return true;
        }
        self.grid[y as usize][x as usize].is_none()
    }

    #[cfg(test)]
    pub fn occupied_count(&self) -> usize {
        self.grid
            .iter()
            .flat_map(|row| row.iter())
            .filter(|cell| cell.is_some())
            .count()
    }

    /// Remove every full row, shifting the rows above it down.
    /// Returns the number of rows cleared.
    pub fn clear_lines(&mut self) -> u32 {
        let mut out: [[Option<Kind>; WIDTH]; HEIGHT] = Default::default();
        let mut write = HEIGHT; // index of the next empty slot, walking up from the floor
        let mut cleared = 0;
        for row in self.grid.iter().rev() {
            if row.iter().all(|cell| cell.is_some()) {
                cleared += 1;
            } else {
                write -= 1;
                out[write] = *row;
            }
        }
        self.grid = out;
        cleared
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_board_is_empty() {
        let board = Board::new();
        assert_eq!(board.occupied_count(), 0);
        assert!(board.is_free(0, 0));
        assert!(board.is_free(WIDTH as isize - 1, (HEIGHT - 1) as isize));
    }

    #[test]
    fn is_free_rejects_out_of_bounds() {
        let board = Board::new();
        assert!(!board.is_free(-1, 0));
        assert!(!board.is_free(WIDTH as isize, 0));
        assert!(!board.is_free(0, HEIGHT as isize));
        // Above the visible top is still legal for a falling piece.
        assert!(board.is_free(0, -1));
    }

    #[test]
    fn occupied_cell_blocks() {
        let mut board = Board::new();
        board.set(3, 7, Kind::T);
        assert!(!board.is_free(3, 7));
        assert!(board.is_free(4, 7));
        assert_eq!(board.cell(3, 7), Some(Kind::T));
    }

    #[test]
    fn clear_lines_shifts_rows_down() {
        let mut board = Board::new();
        for x in 0..WIDTH {
            board.set(x, HEIGHT - 1, Kind::I);
        }
        board.set(0, HEIGHT - 2, Kind::S); // marker just above the full row
        assert_eq!(board.clear_lines(), 1);
        assert_eq!(board.occupied_count(), 1);
        assert_eq!(board.cell(0, HEIGHT - 1), Some(Kind::S));
        assert!(board.cell(1, HEIGHT - 1).is_none());
    }

    #[test]
    fn clear_lines_multiple_rows() {
        let mut board = Board::new();
        for y in (HEIGHT - 3)..HEIGHT {
            for x in 0..WIDTH {
                board.set(x, y, Kind::Z);
            }
        }
        assert_eq!(board.clear_lines(), 3);
        assert_eq!(board.occupied_count(), 0);
    }

    #[test]
    fn partial_row_is_not_cleared() {
        let mut board = Board::new();
        for x in 0..WIDTH - 1 {
            board.set(x, HEIGHT - 1, Kind::J);
        }
        assert_eq!(board.clear_lines(), 0);
        assert_eq!(board.occupied_count(), WIDTH - 1);
    }
}
