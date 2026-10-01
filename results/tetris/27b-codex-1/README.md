# Tetris — Rust + Ratatui

A complete, from-scratch Tetris built with [Rust](https://rust-lang.org) and the
[ratatui](https://ratatui.rs) terminal UI library. No game frameworks, no
assets — just a clean, testable core plus a TUI on top.

## Features
- Classic 10×20 playfield with a hidden spawn buffer
- All seven tetrominoes with SRS-style rotation and wall kicks
- Ghost piece (shows the landing spot)
- Next-piece preview (3 pieces) and a **hold** slot
- 7-bag randomizer for fair piece distribution
- Scoring: line clears (100/300/500/800), soft drop (+1/cell), hard drop (+2/cell)
- Level-up every 10 lines with increasing gravity
- Pause and game-over overlays

## Build
```sh
cargo build --release
```

## Run
```sh
cargo run --release
```

## Controls
| Key                | Action                    |
| ------------------ | ------------------------- |
| `←` / `→`          | Move left / right         |
| `↓`                | Soft drop (+1 pt/cell)    |
| `↑` / `X`          | Rotate clockwise          |
| `Z` / `Backspace`  | Rotate counter-clockwise  |
| `Space`            | Hard drop (+2 pt/cell)    |
| `C`                | Hold / swap piece         |
| `P`                | Pause / resume            |
| `R`                | Restart                   |
| `Q` / `Esc`        | Quit                      |

## Tests
The game rules live in `src/game.rs` (no I/O) and are unit-tested:
```sh
cargo test
```

## Layout
- `src/main.rs` — terminal setup, game loop, timing, key handling
- `src/game.rs` — board, movement, rotation, gravity, line clears, scoring, hold, 7-bag
- `src/piece.rs` — the seven tetrominoes and their spawn shapes
- `src/render.rs` — ratatui drawing (playfield, ghost, sidebar, overlays)
