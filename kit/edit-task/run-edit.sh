#!/bin/bash
# One run of the "edit an existing project" benchmark with opencode on whatever model is loaded.
# usage: run-edit.sh <base|verify|verify+lsp|verify+xhigh> <run-number>
# Appends a JSON line to results.jsonl in this folder.
set -uo pipefail
E=/home/gud/bench-harnesses/edit-task; C=/home/gud/bench-harnesses/config
variant=$1; n=$2; id="$variant-$n"; dir=$E/runs/$id
rm -rf "$dir"; mkdir -p "$E/runs"; cp -r "$E/project" "$dir"; rm -rf "$dir/target"

case $variant in
  base|verify) cfg=$C/opencode-bench.json ;;
  verify+lsp)   cfg=$C/opencode-lsp.json ;;
  verify+xhigh) cfg=$C/opencode-xhigh.json ;;
  *) echo "unknown variant" >&2; exit 2 ;;
esac
from=$(wc -l < /tmp/llama-server.log)
start=$(date +%s)
cd "$dir"
OPENCODE_CONFIG=$cfg timeout 2400 opencode run -m llama-cpp/local --dir "$dir" "$(cat $E/prompt.txt)" < /dev/null > "$E/runs/$id.log" 2>&1
task_s=$(( $(date +%s) - start ))
if [ "$variant" != base ]; then
  # /verify in the same session, as the user would type it after the task
  OPENCODE_CONFIG=$cfg timeout 2400 opencode run -c -m llama-cpp/local --dir "$dir" --command verify < /dev/null >> "$E/runs/$id.log" 2>&1
fi
total_s=$(( $(date +%s) - start ))
# remove this run's opencode sessions from the user's history
(opencode session list < /dev/null 2>/dev/null | grep -o 'ses_[A-Za-z0-9]*' | while read -r s; do opencode session delete "$s" < /dev/null > /dev/null 2>&1; done) || true

build=$(cargo build --message-format short 2>&1); build_ok=$?
warnings=$(echo "$build" | grep -c ": warning")
clippy=$(cargo clippy --message-format short 2>&1 | grep -c ": warning")
tests=$(cargo test -q 2>&1 | grep -E "^test result" | tail -1)
acc='{}'; [ $build_ok = 0 ] && acc=$(python3 $E/acceptance.py target/debug/tally)

python3 - "$id" "$variant" "$n" "$task_s" "$total_s" "$build_ok" "$warnings" "$clippy" "$tests" "$acc" "$from" <<'PY'
import json, re, sys
id_, variant, n, task_s, total_s, build_ok, warnings, clippy, tests, acc, lfrom = sys.argv[1:]
P = G = pm = gm = req = 0
for l in open("/tmp/llama-server.log").readlines()[int(lfrom):]:
    m = re.search(r'prompt eval time =\s+([\d.]+) ms /\s+(\d+) tokens', l)
    if m: pm += float(m[1]); P += int(m[2]); req += 1
    m = re.search(r'\s eval time =\s+([\d.]+) ms /\s+(\d+) tokens', l)
    if m and 'prompt' not in l: gm += float(m[1]); G += int(m[2])
a = json.loads(acc)
row = {"id": id_, "variant": variant, "run": int(n), "task_s": int(task_s), "total_s": int(total_s),
       "requests": req, "gen_tokens": G, "build_ok": build_ok == "0", "warnings": int(warnings),
       "clippy": int(clippy), "own_tests": tests, "acceptance": f'{a.get("passed","-")}/{a.get("total",7)}',
       "acceptance_detail": a.get("results")}
open("/home/gud/bench-harnesses/edit-task/results.jsonl", "a").write(json.dumps(row) + "\n")
print(json.dumps({k: v for k, v in row.items() if k != "acceptance_detail"}))
PY
