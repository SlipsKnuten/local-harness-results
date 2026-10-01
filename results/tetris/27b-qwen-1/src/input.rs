//! Mapping of key presses to game actions.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

/// A game action produced by a key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Move the active piece one column left (dx = -1) or right (dx = 1).
    Move(i32),
    /// Nudge the active piece one row down.
    SoftDrop,
    /// Drop the active piece straight to the bottom and lock it.
    HardDrop,
    /// Rotate clockwise.
    RotateCw,
    /// Rotate counter-clockwise.
    RotateCcw,
    /// Swap the active piece with the hold slot (once per piece).
    Hold,
    /// Toggle pause.
    Pause,
    /// Restart the game.
    Restart,
    /// Quit.
    Quit,
}

/// Convert a key event into an [`Action`], if any.
///
/// Only key presses are handled (not repeats being released), so held keys rely on
/// the terminal's own key auto-repeat for movement.
pub fn handle_event(event: KeyEvent) -> Option<Action> {
    if event.kind != KeyEventKind::Press {
        return None;
    }
    match event.code {
        KeyCode::Left => Some(Action::Move(-1)),
        KeyCode::Right => Some(Action::Move(1)),
        KeyCode::Down => Some(Action::SoftDrop),
        KeyCode::Up => Some(Action::RotateCw),
        KeyCode::Char(' ') => Some(Action::HardDrop),
        KeyCode::Char('x') | KeyCode::Char('X') => Some(Action::RotateCw),
        KeyCode::Char('z') | KeyCode::Char('Z') => Some(Action::RotateCcw),
        KeyCode::Char('c') | KeyCode::Char('C') => Some(Action::Hold),
        KeyCode::Char('p') | KeyCode::Char('P') => Some(Action::Pause),
        KeyCode::Char('r') | KeyCode::Char('R') => Some(Action::Restart),
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => Some(Action::Quit),
        _ => None,
    }
}
