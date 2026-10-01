# Tetris

A terminal Tetris built with Rust, [ratatui](https://crates.io/crates/ratatui), and crossterm.

## Run

```sh
cargo run
```

## Controls

| Key       | Action        |
|-----------|---------------|
| Left/Right| Move          |
| Down      | Soft drop     |
| Up / X    | Rotate CW     |
| Z         | Rotate CCW    |
| Space     | Hard drop     |
| P         | Pause         |
| R         | Restart       |
| Q         | Quit          |

## Features

- SRS rotation with wall kicks, 7-bag randomizer, ghost piece
- Classic scoring (100/300/500/800 x level), level-up every 10 lines
- Next-piece preview, pause and game-over overlays

## Tests

The game logic in `src/game.rs` is UI-free and unit-tested:

```sh
cargo test
```
