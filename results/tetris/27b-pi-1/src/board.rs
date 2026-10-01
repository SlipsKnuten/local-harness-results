//! The playing field: a fixed grid of cells, plus collision / merge / line
//! clearing helpers.

use crate::piece::{Piece, PieceType};

pub const WIDTH: usize = 10;
pub const HEIGHT: usize = 20;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cell {
    Empty,
    Filled(PieceType),
}

#[derive(Clone, Debug)]
pub struct Board {
    pub grid: Vec<Vec<Cell>>,
}

impl Board {
    pub fn new() -> Self {
        Board {
            grid: vec![vec![Cell::Empty; WIDTH]; HEIGHT],
        }
    }

    /// Whether the given coordinate is a free (non-colliding) cell.
    ///
    /// Out of bounds to the left/right or below the floor is a wall.
    /// Anything above the visible top of the board counts as free so that
    /// pieces can spawn just off-screen.
    pub fn is_free(&self, x: i32, y: i32) -> bool {
        if x < 0 || x >= WIDTH as i32 || y >= HEIGHT as i32 {
            return false;
        }
        if y < 0 {
            return true;
        }
        self.grid[y as usize][x as usize] == Cell::Empty
    }

    /// True if any block of `piece` overlaps a wall or a filled cell.
    pub fn collides(&self, piece: &Piece) -> bool {
        piece
            .blocks()
            .iter()
            .any(|&(dx, dy)| !self.is_free(piece.x + dx, piece.y + dy))
    }

    /// Merge the piece into the grid (called when it locks in).
    pub fn lock(&mut self, piece: &Piece) {
        for &(dx, dy) in piece.blocks() {
            let x = piece.x + dx;
            let y = piece.y + dy;
            if (0..WIDTH as i32).contains(&x) && (0..HEIGHT as i32).contains(&y) {
                self.grid[y as usize][x as usize] = Cell::Filled(piece.shape);
            }
        }
    }

    /// Remove all full rows and return how many were cleared.
    pub fn clear_lines(&mut self) -> usize {
        let mut cleared = 0usize;
        let mut rows: Vec<Vec<Cell>> = Vec::with_capacity(HEIGHT);
        for row in self.grid.drain(..) {
            if row.iter().all(|c| matches!(c, Cell::Filled(_))) {
                cleared += 1;
            } else {
                rows.push(row);
            }
        }
        for _ in 0..cleared {
            rows.insert(0, vec![Cell::Empty; WIDTH]);
        }
        self.grid = rows;
        cleared
    }
}
