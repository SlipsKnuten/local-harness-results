#!/bin/bash
# Play-test every result that has a binary and no gui/gui.json yet (handles projects in subfolders).
B=/home/gud/bench-harnesses
python3 - <<'PY' | while IFS='|' read -r id proj; do
import json
for l in open("/home/gud/bench-harnesses/results/results.jsonl"):
    r = json.loads(l)
    if r.get("build_ok"): print(f'{r["id"]}|{r.get("project",".")}')
PY
  d=$B/results/$id; [ -f "$d/gui/gui.json" ] && continue
  pd=$d/$proj
  exe=$(cd "$pd" && cargo metadata --no-deps --format-version 1 2>/dev/null | python3 -c 'import json,sys
m=json.load(sys.stdin); b=[t["name"] for p in m["packages"] for t in p["targets"] if "bin" in t["kind"]]
print(m["target_directory"]+"/debug/"+b[0] if b else "")')
  [ -x "$exe" ] || { echo "$id: no binary"; continue; }
  (cd $B/pw && timeout 300 node gui-test.mjs "$exe" "$pd" "$d/gui" 7691 > "$d/gui.out" 2>&1)
  echo "$id: $(python3 -c 'import json,sys; r=json.load(open(sys.argv[1])); c=r["checks"]; print(r["passed"],"/",r["total"], " fails:", ",".join(k for k,v in c.items() if not v) or "-")' $d/gui/gui.json 2>/dev/null || echo 'gui test failed')"
done
