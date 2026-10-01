#!/bin/bash
# Run one coding harness non-interactively against the local server on :8080.
# usage: run-harness.sh <opencode|claude|codex|qwen|pi|hermes> <workdir> <prompt-file>
# All harness configs are isolated in ~/bench-harnesses/config (the user's own
# opencode/Claude Code/Codex setups are not touched). stdin is closed: some
# harnesses wait for EOF on an open pipe.
set -uo pipefail
h=$1; dir=$2; prompt=$(cat "$3")
B=/home/gud/bench-harnesses; C=$B/config
K=$(cat /home/gud/.llama-api-key)
cd "$dir" || exit 1
case $h in
  opencode)
    # benchmark-only overlay: allow reading ~/.cargo and ~/.rustup sources (default
    # "ask" is auto-rejected in non-interactive runs, which blocked API lookups)
    OPENCODE_CONFIG=$C/opencode-bench.json opencode run -m llama-cpp/local --dir "$dir" "$prompt" ;;
  claude)
    CLAUDE_CONFIG_DIR=$C/claude ANTHROPIC_BASE_URL=http://127.0.0.1:8080 ANTHROPIC_AUTH_TOKEN=$K \
    ANTHROPIC_API_KEY= ANTHROPIC_MODEL=claude-sonnet-4-5 ANTHROPIC_SMALL_FAST_MODEL=claude-sonnet-4-5 \
    CLAUDE_CODE_ATTRIBUTION_HEADER=0 CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1 \
      claude -p "$prompt" --dangerously-skip-permissions --output-format text ;;
  codex)
    CODEX_HOME=$C/codex LLAMA_API_KEY=$K \
      $B/bin/codex exec --skip-git-repo-check --dangerously-bypass-approvals-and-sandbox -C "$dir" "$prompt" ;;
  qwen)
    HOME=$C/qwen-home CARGO_HOME=/home/gud/.cargo RUSTUP_HOME=/home/gud/.rustup \
    OPENAI_API_KEY=$K OPENAI_BASE_URL=http://127.0.0.1:8080/v1 OPENAI_MODEL=local \
      $B/bin/qwen -p "$prompt" --approval-mode yolo ;;
  pi)
    PI_CODING_AGENT_DIR=$C/pi PI_OFFLINE=1 PI_SKIP_VERSION_CHECK=1 \
      $B/bin/pi -p --no-session --provider local --model local:medium "$prompt" ;;
  hermes)
    HERMES_HOME=$C/hermes $B/hermes-venv/bin/hermes chat --oneshot -Q --yolo --source tool -q "$prompt" ;;
  *) echo "unknown harness $h" >&2; exit 2 ;;
esac < /dev/null
