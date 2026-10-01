# Tetris (Rust + ratatui)

A complete terminal Tetris game written in Rust with [ratatui](https://ratatui.rs) and crossterm.

## Features

- Full 10×20 board with all seven SRS tetrominoes
- SRS rotation with proper wall kicks
- 7-bag randomizer (fair piece distribution)
- Ghost piece (landing preview)
- Hold piece (once per piece)
- Next-piece queue (3 pieces)
- Soft drop (+1 pt/row), hard drop (+2 pts/row)
- Line clears with guideline scoring (100/300/500/800 × level)
- Levels: gravity speeds up every 10 lines
- Pause and restart
- Game-over detection (top-out)

## Build & Run

```sh
cargo run            # debug
cargo run --release  # faster
```

The terminal must be at least 46×23 characters.

## Controls

| Key              | Action           |
| ---------------- | ---------------- |
| ← / → or A / D   | Move             |
| ↓ / S            | Soft drop        |
| ↑ / W / X        | Rotate clockwise |
| Z                | Rotate ccw       |
| Space            | Hard drop        |
| C / H            | Hold piece       |
| P                | Pause / resume   |
| R                | Restart (game over screen) |
| Q / Esc          | Quit             |

## Tests

The game logic (`src/game.rs`) is pure and fully unit tested:

```sh
cargo test
```
