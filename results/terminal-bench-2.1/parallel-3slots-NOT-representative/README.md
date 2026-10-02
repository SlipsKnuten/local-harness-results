# Terminal-Bench 2.1 — parallel run (NOT representative)

Run 2026-10-01 19:20 → 2026-10-02 18:20, with pauses. Qwen3.8-27B UD-Q4_K_XL + ngram-mod + DFlash2,
llama.cpp, **3 server slots x 64k context, 3 tasks at a time** (`start-tb-server.sh` here).

Kept for reference only: each agent got roughly a third of the model's single-user speed, and
Terminal-Bench time limits are wall-clock, so time-limit failures are inflated.

- opencode: 46/89 solved (51.7%); 20 wrong, 21 time-limit failures, 2 not run (no Node.js in
  the task image: qemu-startup, qemu-alpine-ssh).
- codex: stopped after ~1 hour (partial, discard).

Superseded by the sequential run (1 slot, 1 task at a time).
