# Tetris (Rust + Ratatui)

A classic Tetris game for the terminal, built with [ratatui](https://ratatui.rs) and
[crossterm](https://github.com/crossterm-rs/crossterm).

## Build and run

```sh
cargo run --release
```

## Controls

| Key          | Action          |
| ------------ | --------------- |
| Left / Right | Move            |
| Up / `x`     | Rotate          |
| `z`          | Rotate back     |
| Down         | Soft drop (+1)  |
| Space        | Hard drop (+2/row) |
| `p`          | Pause           |
| `r`          | Restart (after game over) |
| `q` / Esc    | Quit            |

## Gameplay

- 10x20 field, 7-bag piece randomizer, ghost piece, next-piece preview.
- Scoring: 1/2/3/4 lines = 100/300/500/800 x level; level up every 10 lines,
  gravity gets faster each level.
- Pieces lock on the next gravity step after reaching the floor; rotations use
  simplified wall kicks.

## Tests

```sh
cargo test
```

Covers spawning, walls, rotation + wall kicks, gravity/locking, line clearing,
scoring, level progression, the 7-bag, game over, and the ghost piece.
