#!/bin/bash
# One Tetris run: harness <h> against whatever profile is loaded, in a fresh empty
# folder, then score it. Keeps the project in results/<id>/ and appends a JSON line
# to results/results.jsonl.
# usage: bench-tetris.sh <harness> <model-label> <run-number>
set -uo pipefail
h=$1; model=$2; n=$3
B=/home/gud/bench-harnesses
id="$model-$h-$n"
dir=$B/results/$id
rm -rf "$dir"; mkdir -p "$dir"
# optional instruction file (lever test): RULES=/path/AGENTS.md -> project AGENTS.md
# (Codex, opencode, Hermes, Pi read AGENTS.md; Qwen Code reads QWEN.md)
if [ -n "${RULES:-}" ]; then cp "$RULES" "$dir/AGENTS.md"; cp "$RULES" "$dir/QWEN.md"; fi
llama_from=$(wc -l < /tmp/llama-server.log 2>/dev/null || echo 0)
strata_from=$(wc -l < /home/gud/Strata/strata-coder-iq1_m.log 2>/dev/null || echo 0)

start=$(date +%s)
timeout 2400 "$B/run-harness.sh" "$h" "$dir" "$B/tetris-prompt.txt" > "$B/results/$id.log" 2>&1
rc=$?
secs=$(( $(date +%s) - start ))

python3 - "$id" "$h" "$model" "$n" "$secs" "$rc" "$llama_from" "$strata_from" "$dir" <<'PY'
import json, re, subprocess, sys
id_, h, model, n, secs, rc, lfrom, sfrom, d = sys.argv[1:]
P = G = pm = gm = req = 0
try:
    for l in open("/tmp/llama-server.log").readlines()[int(lfrom):]:
        m = re.search(r'prompt eval time =\s+([\d.]+) ms /\s+(\d+) tokens', l)
        if m: pm += float(m[1]); P += int(m[2]); req += 1
        m = re.search(r'\s eval time =\s+([\d.]+) ms /\s+(\d+) tokens', l)
        if m and 'prompt' not in l: gm += float(m[1]); G += int(m[2])
except OSError: pass
try:
    for l in open("/home/gud/Strata/strata-coder-iq1_m.log").readlines()[int(sfrom):]:
        m = re.search(r'prompt \d+ tokens = \d+ reused \+ (\d+) read in (\d+) ms.*?(\d+) generated in (\d+) ms', l)
        if m: req += 1; P += int(m[1]); pm += int(m[2]); G += int(m[3]); gm += int(m[4])
except OSError: pass
score = json.loads(subprocess.run(["python3", "/home/gud/bench-harnesses/score.py", d],
                                  capture_output=True, text=True).stdout or "{}")
row = {"id": id_, "harness": h, "model": model, "run": int(n), "seconds": int(secs), "exit": int(rc),
       "requests": req, "prompt_tokens": P, "gen_tokens": G,
       "prompt_s": round(pm / 1000), "gen_s": round(gm / 1000), **{k: v for k, v in score.items() if k != "dir"}}
open("/home/gud/bench-harnesses/results/results.jsonl", "a").write(json.dumps(row) + "\n")
print(json.dumps(row))
PY
