# Tetris (Rust + ratatui)

A classic Tetris playable in the terminal, built with [`ratatui`](https://ratatui.rs)
for rendering and [`crossterm`](https://github.com/crossterm-rs/crossterm) for input.

## Build & run

```sh
cargo run            # debug build + play
cargo run --release  # optimized binary
```

Or build a standalone binary:

```sh
cargo build --release
./target/release/tetris
```

> The game needs a real terminal (TTY). It takes the screen over with the
> alternate screen buffer and restores it on exit.

## Controls

| Key          | Action            |
| ------------ | ----------------- |
| `←` / `→`    | Move left / right |
| `↓`          | Soft drop (+1 pt/cell) |
| `↑` or `X`   | Rotate clockwise  |
| `Z`          | Rotate counter-clockwise |
| `Space`      | Hard drop (+2 pts/cell) |
| `P`          | Pause / resume    |
| `R`          | Restart (after game over) |
| `Q` or `Esc` | Quit              |

## Features

- 7 standard tetrominoes with SRS-style rotation and simple wall kicks.
- 7-bag randomizer for fair piece distribution.
- Ghost piece showing where the active piece will land.
- Score, in-session high score, level, and lines-cleared.
- Speed increases with level (10 lines per level).
- Next-piece preview, pause overlay, and game-over/restart screen.

### Scoring

| Event           | Points          |
| --------------- | --------------- |
| 1 line          | 100 × (level+1) |
| 2 lines         | 300 × (level+1) |
| 3 lines         | 500 × (level+1) |
| 4 lines (Tetris)| 800 × (level+1) |
| Soft drop cell  | +1              |
| Hard drop cell  | +2              |

## Layout

- `src/tetromino.rs` — the 7 pieces, their rotation states, and colors.
- `src/game.rs` — board, collision, 7-bag, line clearing, scoring, gravity (with unit tests).
- `src/main.rs` — event loop, rendering, overlays.

## Tests

```sh
cargo test
```
