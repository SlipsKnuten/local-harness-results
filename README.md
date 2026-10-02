# Local model x coding harness results (RTX 5090, Qwen3.8)

Same local model, five coding harnesses, one-shot tasks, and the effect of a 10-line
instruction file. Every generated program was built, play-tested in a real terminal
through Playwright, and judged by eye from screenshots, because "builds and passes the
automated checks" hid most of the bugs.

## Setup

- RTX 5090 32 GB, Ryzen 7 9800X3D, 64 GB RAM, Windows 11 + WSL2 (Ubuntu 24.04).
- **27b**: Qwen3.8-27B UD-Q4_K_XL (Unsloth), llama.cpp `2145525a4`, full 262k context, q8_0
  KV, speculative decoding `ngram-mod` + DFlash2 draft (n=7), Qwen thinking sampling
  (temp 1.0, top-p 0.95, top-k 20), `--reasoning-effort medium`. ~140–170 tok/s generation.
- **coder**: Qwen3.8-Flash-Next GSQ-RCO Coder IQ1_M (ISTA-DASLab, 256 of 512 experts) on
  the Strata engine. ~100–130 tok/s.
- Harnesses: opencode 1.17.14, Codex CLI 0.159.2, Qwen Code 0.24.7, Pi 0.73.1, Hermes
  Agent 0.21.5, each unattended in full-auto mode against the same server
  (`kit/run-harness.sh`, configs in `kit/configs/`, API key replaced by `<YOUR_KEY>`).

## Task 1: one-shot Tetris

Prompt, exactly: `create Tetris with rust and ratatui`, in an empty folder.
Scoring per run (`kit/bench-tetris.sh`):
1. `cargo build` / `cargo clippy` (`kit/score.py`).
2. Playwright play-test (`kit/gui-test.mjs`): the game runs in ttyd/xterm.js; 14 checks
   (draws, moves, both walls, rotation incl. at a wall, gravity, hard drop, 70-piece stress
   with game over, score, quit), screenshots at every step.
3. Verdict by eye from the screenshots (`results/tetris/verdicts.json`). The best game was
   also played by hand through tmux (`kit/play.sh`) to check collision.

| 27b, no instructions | runs | clean playable |
|---|:-:|---|
| Qwen Code | 2 | 1 (other: layout) |
| Codex | 2 | 1 (other: falling piece invisible) |
| Hermes | 2 | 1 (other: layout) |
| Pi | 2 | 0 (layout; cells drawn with gaps) |
| opencode | 3 | 0 (crash; no build*; layout) |
| **total** | 11 | **3 (27%)** |

"layout" = the board frame is drawn larger than the logical field, so pieces land in the
middle of the frame: 4 of 11 runs, every harness except Codex.
*opencode's unattended mode ends the whole session on one rejected permission
(`external_directory` defaults to ask); later runs allow it.

| coder (Strata) | result |
|---|---|
| opencode, Qwen Code, Pi | 0 of 3 compile: elementary API errors never fixed (`Duration::milliseconds`, `u16(..)`). Fast, but too weak at Rust APIs. |

### Lever: a 10-line instruction file

`kit/AGENTS.rules.md` (also as QWEN.md): build + clippy clean, unit tests, run the program
in tmux and read the screen like a user, check APIs against the locked crate sources,
don't stop until all pass.

| 27b + instruction file | runs | clean playable | time per run |
|---|:-:|---|---|
| opencode | 2 | 2 | 12–14 min |
| Qwen Code | 2 | 2 (most polished) | 12–13 min |
| Codex | 2 | 2 | 12–16 min |
| Hermes | 2 | 1 (other: layout) | 10–11 min |
| **total** | 8 | **7 (88%)** | (4–12 min without the file) |

## Task 2: edit an existing project (opencode + 27b)

`kit/edit-task/`: a small Rust CLI (clap + serde). Fix a crash, add `edit`, `rm` (ids never
reused), `list --overdue`, `stats`, keep old files loadable. 7 hidden tests run the binary.
One run per variant.

| variant | total time | hidden tests |
|---|---:|:-:|
| base | 106 s | 6/7 (reuses ids after rm) |
| + `/verify` (same instructions as a follow-up command) | 236 s | 7/7 |
| + `/verify` + LSP (rust-analyzer) | 374 s | 7/7 |
| + `/verify` + `reasoningEffort: high` | 244 s | 7/7 |

## Reference point: local-model-bench's suite

[tijs/local-model-bench](https://github.com/tijs/local-model-bench) (25 tasks: Hermes tool
use + Rust/Swift/TypeScript coding, hidden checks) run at `38e917b` against the 27b setup,
mirroring their Qwen3.8-27B Q5 config except quant, CUDA and the speed stack
(`results/local-model-bench/`, config + raw rows):

| setup | score | avg decode | total wall |
|---|:-:|---:|---:|
| Qwen3.8-27B UD-Q4_K_XL + DFlash2, RTX 5090 | **24/25** | 119.5 tok/s | 15.9 min |
| their #1 Ornith-1.5-35B-A3B (Mac, Mei 0.6.1) | 24/25 | 15.6 tok/s | 38.5 min |
| their #2 Qwen3.6-35B-A3B (Mac, Mei 0.6.1) | 24/25 | 16.5 tok/s | 57.4 min |

Same single failure as their leaders (`kiem_mini-testwrite`).

## Terminal-Bench 2.1 (official harness: Harbor)

`results/terminal-bench-2.1/`. 89 tasks, harness defaults (no instruction file), 27b setup
above, Harbor 0.23.0.

- **Parallel run — NOT representative** (`parallel-3slots-NOT-representative/`): 3 server slots
  x 64k context, 3 tasks at a time, so each agent ran at ~1/3 of single-user speed while the
  task time limits are wall-clock. opencode: **46/89 solved (51.7%)**; 20 wrong, 21 time-limit,
  2 not run (task image without Node.js). Codex: stopped early. Kept for reference only.
- **Sequential run — in progress** (`run-seq.sh`, `start-tb-server-seq.sh`): 1 slot, full 262k
  context, 1 task at a time, i.e. the real single-user setup. opencode, then Codex.

For scale: Qwen reports 73.0% for Qwen3.8-27B (Claude Code harness, xhigh thinking).

## Take-aways

- The harness and its instructions moved quality far more than anything else we tried on a
  fixed model: 27% -> 88% clean one-shot games from one short instruction file.
- Most bugs were visible the moment the program ran; a "run it and look at the screen" step
  fixes them. Build checks and even automated UI checks missed them.
- LSP diagnostics cost time without improving results here; extra reasoning effort made no
  measurable difference.

## Limitations

Small samples (1–3 runs per cell), two tasks, Rust only, one machine. Verdicts are by eye
(Claude, an AI assistant that ran the tests). Timings include occasional overlap with
other work on the same GPU. Everything needed to rerun is in `kit/`.
