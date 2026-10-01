# Tetris (Rust + ratatui)

A classic Tetris game for the terminal, built with [Rust](https://rust-lang.org),
[ratatui](https://ratatui.rs) and [crossterm](https://github.com/crossterm-rs/crossterm).

```
                T E T R I S   LEVEL 1
┌──────────┐ ┌────────────────┐ ┌────────┐
│ SCORE    │ │                │ │  NEXT  │
│   1250   │ │        ▓▓      │ │  ▓▓    │
└──────────┘ │      ▓▓▓▓      │ │        │
┌──────────┐ │              ░░│ │  ▓▓▓▓  │
│  HIGH    │ │        ▓▓▓▓░░  │ │        │
│   8400   │ │    ▓▓▓▓        │ │  ▓▓    │
└──────────┘ │    ▓▓  ▓▓      │ │        │
┌──────────┐ │  ▓▓▓▓▓▓▓▓▓▓    │ │  ▓▓▓▓  │
│  LEVEL   │ │  ▓▓▓▓▓▓▓▓▓▓    │ └────────┘
│     2    │ └────────────────┘
└──────────┘
┌──────────┐
│  LINES   │
│     14   │
└──────────┘
┌──────────┐
│  HOLD    │
│  ▓▓▓▓    │
└──────────┘

 ←/→ move   ↓ soft drop   SPACE hard drop   ↑/X rotate   C hold   P pause   Q quit
```

## Features

- Standard **10 × 20** playfield and all **7 tetrominoes** (I, O, T, S, Z, J, L)
- **SRS-style rotation** with wall kicks, in both directions
- **7-bag randomizer** for fair, predictable piece distribution
- **Ghost piece** showing where the active piece will land
- **Hold** slot (once per piece)
- **Next-piece preview** (the next 3)
- Classic **scoring** (100 / 300 / 500 / 800 × level) plus soft/hard drop points
- **Level progression** — gravity speeds up every 10 lines
- **Persistent high score** stored in `~/.tetris-rs-highscore`
- Pause, restart, and a coloured mosaic UI with side panels

## Controls

| Key           | Action                  |
|---------------|-------------------------|
| `←` / `→`     | Move left / right       |
| `↓`           | Soft drop               |
| `Space`       | Hard drop               |
| `↑` / `X`     | Rotate clockwise        |
| `Z`           | Rotate counter-clockwise|
| `C`           | Hold                    |
| `P`           | Pause / resume          |
| `R`           | Restart (after game over) |
| `Q` / `Esc`   | Quit                    |

## Build & run

```sh
cargo run --release
```

## Tests

The game logic and UI rendering are covered by unit tests:

```sh
cargo test
```

## Project layout

| File                 | Purpose                                          |
|----------------------|--------------------------------------------------|
| `src/main.rs`        | Terminal lifecycle and the main event loop        |
| `src/game.rs`        | Game state, movement, rotation, scoring, levels   |
| `src/tetromino.rs`   | The 7 pieces, rotations, and colours              |
| `src/board.rs`       | The 10×20 field and line clearing                 |
| `src/input.rs`       | Key → action mapping                              |
| `src/ui.rs`          | ratatui rendering (board, panels, overlays)       |
