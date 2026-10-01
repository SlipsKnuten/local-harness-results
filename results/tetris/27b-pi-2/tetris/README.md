# Tetris (Rust + ratatui)

A complete, terminal-based Tetris game written in Rust using
[`ratatui`](https://github.com/ratatui-org/ratatui) for the UI and
[`crossterm`](https://github.com/crossterm-rs/crossterm) for input.

![features](https://img.shields.io/badge/lines%20cleared-yes-brightgreen)

## Features

- Classic 10×20 play field with all 7 tetrominoes (I, O, T, S, Z, J, L)
- 4-state rotation with simple wall kicks
- **Ghost piece** showing where the active piece will land
- **Next-piece preview**
- Soft drop, hard drop, gravity that speeds up with level
- Line clearing (single, double, triple, tetris) with proper gravity compaction
- Scoring + level progression (level rises every 10 lines)
- Pause / resume, restart, game-over detection
- Color-coded pieces

## Requirements

- Rust toolchain (`rustc` / `cargo`)

## Build & Run

```sh
# debug
cargo run

# release (recommended, faster)
cargo run --release
```

> The game needs a real terminal (TTY). It runs in raw mode on the alternate
> screen and restores the terminal on exit, even if it panics.

## Controls

| Key            | Action                |
|----------------|-----------------------|
| `←` / `→`      | Move left / right     |
| `↓`            | Soft drop (+1 pt/row) |
| `↑`            | Rotate clockwise      |
| `z`            | Rotate counter-clockwise |
| `Space`        | Hard drop (+2 pt/row) |
| `p`            | Pause / resume        |
| `r`            | Restart               |
| `q` / `Esc`    | Quit                  |

## Scoring

- Soft drop: `1` point per row dropped
- Hard drop: `2` points per row dropped
- Line clears (multiplied by current level):

  | Lines | Base points |
  |-------|-------------|
  | 1     | 100         |
  | 2     | 300         |
  | 3     | 500         |
  | 4     | 800 (Tetris)|

## Project layout

```
src/
├── main.rs         # entry point, terminal setup, event/game loop
├── game.rs         # game state, rules, collision, line clearing, scoring
├── tetrominoes.rs  # the 7 pieces and their 4 rotation states
└── ui.rs           # ratatui rendering (board, next, stats, controls, overlays)
```

## Tests

The game logic is covered by unit tests:

```sh
cargo test
```

A headless self-test that scripts a few moves through the same methods the key
handler uses and prints the resulting state (handy for CI):

```sh
cargo run --release -- --selftest
# e.g. selftest OK score=358 lines=1 level=1 filled_cells=48 over=false next=J
```
