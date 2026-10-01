//! Tetromino definitions and rotation logic.
//!
//! Each piece is stored as a set of four rotation states. A rotation state is a
//! list of relative `(x, y)` block offsets (already normalised so the minimum
//! x and y are both 0). Rotation is done by the standard 90° clockwise
//! transform `(x, y) -> (-y, x)` followed by re-normalisation, which keeps the
//! math simple and independent of the piece's bounding box size.

use ratatui::style::Color;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
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

    pub fn color(self) -> Color {
        match self {
            PieceType::I => Color::Cyan,
            PieceType::O => Color::Yellow,
            PieceType::T => Color::Magenta,
            PieceType::S => Color::Green,
            PieceType::Z => Color::Red,
            PieceType::J => Color::Blue,
            PieceType::L => Color::Rgb(255, 165, 0),
        }
    }
}

/// The base (unrotated) block offsets for every tetromino.
fn base(shape: PieceType) -> [(i32, i32); 4] {
    match shape {
        PieceType::I => [(0, 1), (1, 1), (2, 1), (3, 1)],
        PieceType::O => [(0, 0), (1, 0), (0, 1), (1, 1)],
        PieceType::T => [(1, 0), (0, 1), (1, 1), (2, 1)],
        PieceType::S => [(1, 0), (2, 0), (0, 1), (1, 1)],
        PieceType::Z => [(0, 0), (1, 0), (1, 1), (2, 1)],
        PieceType::J => [(0, 0), (0, 1), (1, 1), (2, 1)],
        PieceType::L => [(2, 0), (0, 1), (1, 1), (2, 1)],
    }
}

fn normalize(pts: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let min_x = pts.iter().map(|p| p.0).min().unwrap();
    let min_y = pts.iter().map(|p| p.1).min().unwrap();
    let mut out: Vec<(i32, i32)> = pts
        .iter()
        .map(|&(x, y)| (x - min_x, y - min_y))
        .collect();
    // Sort so a given shape's rotation states are in a canonical order (this
    // keeps symmetric pieces like O producing identical vectors).
    out.sort();
    out
}

/// Rotate a set of points 90° clockwise (screen coordinates, y points down)
/// and re-normalise to the origin.
fn rotate90(pts: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let rotated = pts.iter().map(|&(x, y)| (-y, x)).collect::<Vec<_>>();
    normalize(&rotated)
}

/// `shape -> [4 rotation states]`.
type RotationTable = HashMap<PieceType, Vec<Vec<(i32, i32)>>>;

static ROTATIONS: OnceLock<RotationTable> = OnceLock::new();

fn rotations() -> &'static RotationTable {
    ROTATIONS.get_or_init(|| {
        let mut map = HashMap::new();
        for shape in PieceType::ALL {
            let mut states = Vec::new();
            let mut cur = normalize(&base(shape));
            for _ in 0..4 {
                states.push(cur.clone());
                cur = rotate90(&cur);
            }
            map.insert(shape, states);
        }
        map
    })
}

#[derive(Clone, Copy, Debug)]
pub struct Piece {
    pub shape: PieceType,
    pub rot: u8,
    pub x: i32,
    pub y: i32,
}

impl Piece {
    pub fn new(shape: PieceType) -> Self {
        Piece {
            shape,
            rot: 0,
            x: 0,
            y: 0,
        }
    }

    /// The absolute-relative block offsets for the current rotation.
    pub fn blocks(&self) -> &[(i32, i32)] {
        let states = &rotations()[&self.shape];
        &states[self.rot as usize]
    }

    /// A clone of this piece translated by `(dx, dy)`.
    pub fn moved(&self, dx: i32, dy: i32) -> Self {
        let mut p = *self;
        p.x += dx;
        p.y += dy;
        p
    }
}
