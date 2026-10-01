# Tetris (ratatui)

A terminal Tetris game written in Rust with [ratatui](https://ratatui.rs) and crossterm.

## Run

```sh
cargo run --release
```

Needs a terminal of at least 64x24.

## Controls

| Key         | Action          |
| ----------- | --------------- |
| ← / →       | Move            |
| ↑ or x      | Rotate          |
| z           | Rotate back     |
| ↓           | Soft drop (+1)  |
| space       | Hard drop (+2)  |
| p           | Pause           |
| r           | Restart         |
| q / Esc     | Quit            |

## Features

- 10x20 board with hidden spawn row, ghost piece, and next-piece preview
- 7-bag randomizer (every piece appears once per bag)
- Scoring: 100/300/500/800 for 1-4 line clears (x level), soft/hard drop bonuses
- Level up every 10 lines; gravity speeds up with level (800ms down to a 50ms cap)
- Wall kicks for rotations, pause, game-over and restart, panic-safe terminal restore

## Tests

```sh
cargo test
```
