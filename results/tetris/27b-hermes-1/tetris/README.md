# Tetris

A complete, terminal-based Tetris built with **Rust** and **ratatui** (crossterm backend).

## Build

```sh
cargo build            # debug
cargo build --release  # optimized
```

## Run

```sh
cargo run
# or
./target/release/tetris
```

## Controls

| Key         | Action             |
|-------------|--------------------|
| ← / →       | Move left / right  |
| ↑ / x       | Rotate clockwise   |
| z           | Rotate counter-clockwise |
| ↓           | Soft drop          |
| Space       | Hard drop          |
| p           | Pause / resume     |
| r           | Restart (on game over) |
| q / Esc     | Quit               |

## Features

- 10×20 board with the standard 7 tetrominoes (I, O, T, S, Z, J, L), each in its own color
- **7-bag randomizer** for fair piece distribution
- **Next-piece preview** and **ghost piece** (drop-shadow)
- Wall kicks on rotation so pieces fit in corners
- Scoring: line clears (100/300/500/800 × level), soft/hard-drop points
- Level-up every 10 lines, with gravity that speeds up as level rises
- Pause, restart, best-score tracking, game-over overlay
