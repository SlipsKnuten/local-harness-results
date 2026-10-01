# How to work

- Finish only when the project builds with zero errors and zero warnings:
  run the build and the linter (`cargo build`, `cargo clippy` for Rust) and fix everything they report.
- Write unit tests for the core logic and run them (`cargo test`). Fix failures; don't delete tests.
- Run the program yourself and check that it really works, not just that it compiles.
  For terminal UIs, run it in tmux and read the screen:
  `tmux new -d -s check -x 100 -y 40 <program>; sleep 2; tmux capture-pane -p -t check`,
  send input with `tmux send-keys -t check <keys>`, then `tmux kill-session -t check`.
- Check the result like a user would: everything that should be visible is drawn, sizes and
  positions line up, input does what it says, and quitting restores the terminal.
- Use the APIs of the dependency versions in Cargo.lock. If unsure about an API, read its
  source under ~/.cargo/registry instead of guessing.
- Keep going until all of the above pass, then give a short summary.
