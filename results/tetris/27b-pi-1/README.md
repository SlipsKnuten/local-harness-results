# Tetris (Rust + ratatui)

A terminal Tetris built with [Rust](https://rust-lang.org) and
[ratatui](https://ratatui.rs), using [crossterm](https://github.com/crossterm-rs/crossterm)
for terminal input/output.

## Build & run

```sh
cargo run                 # debug
cargo run --release       # faster
```

A working terminal (TUI app) is required.

## Controls

| Key            | Action        |
| -------------- | ------------- |
| `←` / `a`      | move left     |
| `→` / `d`      | move right    |
| `↓` / `s`      | soft drop     |
| `↑` / `x` / `w`| rotate CW     |
| `z`            | rotate CCW    |
| `space`        | hard drop     |
| `p`            | pause / resume|
| `r`            | restart (after game over) |
| `q` / `Esc`    | quit          |

## Gameplay

- Standard 10×20 board, seven tetrominoes, "next piece" preview.
- Gravity speeds up as your level rises (every 10 lines).
- Scoring: soft drop +1/cell, hard drop +2/cell, cleared lines
  `0 / 100 / 300 / 500 / 800 × level`.
- Basic wall-kicks let you rotate near walls.

## Project layout

- `src/piece.rs` — tetromino shapes, rotation states, colors.
- `src/board.rs` — the grid, collision, locking, line clearing.
- `src/game.rs`  — game state, spawning, movement, gravity, scoring.
- `src/main.rs`  — terminal setup, input loop, ratatui rendering.

## Tests

```sh
cargo test
```

Covers rendering, line clearing, wall/rotation rules, scoring, top-out
(game over) and pause. Rotation states are verified for consistency across
all seven pieces.
