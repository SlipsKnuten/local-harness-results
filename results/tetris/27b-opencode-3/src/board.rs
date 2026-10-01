use crate::piece::{Piece, PieceType};

pub const WIDTH: i16 = 10;
pub const HEIGHT: i16 = 20;

/// A single board cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Empty,
    Filled(PieceType),
}

/// The playfield grid plus collision and line-clearing logic.
#[derive(Debug, Clone)]
pub struct Board {
    pub grid: Vec<Cell>,
}

impl Board {
    pub fn new() -> Self {
        Board {
            grid: vec![Cell::Empty; (WIDTH * HEIGHT) as usize],
        }
    }

    fn index(x: i16, y: i16) -> Option<usize> {
        if !(0..WIDTH).contains(&x) || !(0..HEIGHT).contains(&y) {
            return None;
        }
        Some((y * WIDTH + x) as usize)
    }

    /// Returns the cell at (x, y). Cells above the top row read as empty so
    /// that pieces may start partially off-screen.
    pub fn get(&self, x: i16, y: i16) -> Cell {
        match Self::index(x, y) {
            Some(i) => self.grid[i],
            None => Cell::Empty,
        }
    }

    /// Whether `piece` fits at its current position without overlapping the
    /// walls, the floor, or any locked cell.
    pub fn is_valid_position(&self, piece: &Piece) -> bool {
        for (cx, cy) in piece.absolute_cells() {
            if !(0..WIDTH).contains(&cx) || cy >= HEIGHT {
                return false;
            }
            if cy >= 0 && matches!(self.get(cx, cy), Cell::Filled(_)) {
                return false;
            }
        }
        true
    }

    /// Writes the piece's cells into the grid.
    pub fn lock_piece(&mut self, piece: &Piece) {
        for (cx, cy) in piece.absolute_cells() {
            if let Some(i) = Self::index(cx, cy) {
                self.grid[i] = Cell::Filled(piece.piece_type);
            }
        }
    }

    /// Removes every full row, shifting the rows above down. Returns how many
    /// lines were cleared.
    pub fn clear_lines(&mut self) -> u16 {
        let mut cleared = 0;
        let mut y = HEIGHT - 1;
        while y >= 0 {
            let full = (0..WIDTH).all(|x| matches!(self.get(x, y), Cell::Filled(_)));
            if full {
                cleared += 1;
                for r in (1..=y).rev() {
                    for x in 0..WIDTH {
                        let src = Self::index(x, r - 1).unwrap();
                        let dst = Self::index(x, r).unwrap();
                        self.grid[dst] = self.grid[src];
                    }
                }
                for x in 0..WIDTH {
                    self.grid[Self::index(x, 0).unwrap()] = Cell::Empty;
                }
            } else {
                y -= 1;
            }
        }
        cleared
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::piece::Piece;

    #[test]
    fn line_clear_shifts_rows_down() {
        let mut board = Board::new();
        // Fill the bottom row completely.
        for x in 0..WIDTH {
            board.grid[Board::index(x, HEIGHT - 1).unwrap()] = Cell::Filled(PieceType::L);
        }
        // Put a marker in the row above.
        board.grid[Board::index(3, HEIGHT - 2).unwrap()] = Cell::Filled(PieceType::T);

        assert_eq!(board.clear_lines(), 1);
        // Marker should have dropped one row.
        assert!(matches!(board.get(3, HEIGHT - 1), Cell::Filled(PieceType::T)));
        assert!(matches!(board.get(3, HEIGHT - 2), Cell::Empty));
    }

    #[test]
    fn detects_collision_with_wall_and_piece() {
        let mut board = Board::new();
        let mut piece = Piece {
            piece_type: PieceType::O,
            x: 0,
            y: 0,
            rotation: 0,
        };
        assert!(board.is_valid_position(&piece));

        // Slide fully off the left wall -> invalid.
        piece.x = -4;
        assert!(!board.is_valid_position(&piece));

        // A filled cell occupying one of the piece's cells -> invalid.
        piece = Piece {
            piece_type: PieceType::O,
            x: 3,
            y: 5,
            rotation: 0,
        };
        assert!(board.is_valid_position(&piece));
        board.grid[Board::index(4, 6).unwrap()] = Cell::Filled(PieceType::I);
        assert!(!board.is_valid_position(&piece));
    }

    #[test]
    fn out_of_bounds_reads_as_empty() {
        let board = Board::new();
        assert!(matches!(board.get(5, -3), Cell::Empty));
        assert!(matches!(board.get(99, 5), Cell::Empty));
    }
}
