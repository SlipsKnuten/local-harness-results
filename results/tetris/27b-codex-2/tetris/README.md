# Tetris — Rust + ratatui

A classic Tetris clone built with [ratatui](https://github.com/ratatui/ratatui) and [crossterm](https://github.com/crossterm-rs/crossterm).

## Run

```sh
cargo run --release
```

## Controls

| Key              | Action      |
| ---------------- | ----------- |
| ←/a, →/d         | Move        |
| ↓/s              | Soft drop   |
| ↑/w, x           | Rotate CW   |
| z                | Rotate CCW  |
| space            | Hard drop   |
| p                | Pause       |
| r                | Restart     |
| q / Esc          | Quit        |

## Features

- 7-bag randomizer (fair piece distribution)
- Ghost piece showing the landing spot
- Wall kicks on rotation
- DAS/ARR auto-repeat for held movement keys
- Line-clear scoring (100/300/500/800 × level), soft/hard drop bonuses
- Level-up every 10 lines with guideline-style gravity acceleration
