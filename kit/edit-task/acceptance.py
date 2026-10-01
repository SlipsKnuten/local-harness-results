#!/usr/bin/env python3
"""Hidden acceptance tests for the tally edit task. usage: acceptance.py <binary>"""
import json, os, subprocess, sys, tempfile
BIN = sys.argv[1]
results = {}

def run(args, f, today=None):
    env = dict(os.environ, TALLY_FILE=f)
    if today: env["TALLY_TODAY"] = today
    p = subprocess.run([BIN] + args, capture_output=True, text=True, env=env, timeout=20)
    return p.returncode, p.stdout.strip(), p.stderr.strip()

def check(name, fn):
    try:
        fn(); results[name] = True
    except Exception as e:
        results[name] = f"FAIL: {e}"[:200]

def fresh():
    return os.path.join(tempfile.mkdtemp(), "t.json")

def t_existing_formats():
    f = fresh()
    assert run(["add", "buy milk", "--due", "2026-01-05", "--tag", "home"], f)[1] == "added #1"
    run(["add", "write report"], f)
    assert run(["done", "2"], f)[1] == "done #2"
    out = run(["list"], f)[1]
    assert out == "#1 [ ] buy milk (due 2026-01-05) +home", out
    out = run(["list", "--all"], f)[1].splitlines()
    assert out == ["#1 [ ] buy milk (due 2026-01-05) +home", "#2 [x] write report"], out

def t_done_unknown():
    f = fresh(); run(["add", "a"], f)
    rc, out, err = run(["done", "9"], f)
    assert rc == 1 and "no task #9" in err, (rc, out, err)

def t_edit():
    f = fresh(); run(["add", "old", "--due", "2026-01-01"], f)
    assert run(["edit", "1", "--title", "new"], f)[1] == "edited #1"
    assert run(["edit", "1", "--due", "2026-02-02"], f)[1] == "edited #1"
    out = run(["list"], f)[1]
    assert out == "#1 [ ] new (due 2026-02-02)", out
    rc, _, err = run(["edit", "7", "--title", "x"], f)
    assert rc == 1 and "no task #7" in err, (rc, err)

def t_rm_no_reuse():
    f = fresh()
    for t in ["a", "b", "c"]: run(["add", t], f)
    assert run(["rm", "3"], f)[1] == "removed #3"
    assert run(["add", "d"], f)[1] == "added #4", "id reused after rm"
    assert run(["rm", "1"], f)[1] == "removed #1"
    assert run(["add", "e"], f)[1] == "added #5"
    ids = [l.split()[0] for l in run(["list", "--all"], f)[1].splitlines()]
    assert ids == ["#2", "#4", "#5"], ids
    rc, _, err = run(["rm", "1"], f)
    assert rc == 1 and "no task #1" in err

def t_overdue():
    f = fresh()
    run(["add", "late", "--due", "2026-02-01"], f)
    run(["add", "future", "--due", "2026-04-01"], f)
    run(["add", "late-done", "--due", "2026-01-01"], f); run(["done", "3"], f)
    run(["add", "no-due"], f)
    run(["add", "due-today", "--due", "2026-03-01"], f)
    out = run(["list", "--overdue"], f, today="2026-03-01")[1].splitlines()
    assert out == ["#1 [ ] late (due 2026-02-01)"], out

def t_stats():
    f = fresh()
    run(["add", "late", "--due", "2026-02-01"], f)
    run(["add", "ok", "--due", "2026-05-01"], f)
    run(["add", "x"], f); run(["done", "3"], f)
    out = run(["stats"], f, today="2026-03-01")[1]
    assert out == "open: 2, done: 1, overdue: 1", out

def t_old_file():
    f = fresh()
    json.dump({"tasks": [{"id": 1, "title": "old", "done": False, "due": None, "tags": []},
                         {"id": 3, "title": "older", "done": True, "due": None, "tags": ["w"]}]}, open(f, "w"))
    out = run(["list", "--all"], f)[1].splitlines()
    assert out == ["#1 [ ] old", "#3 [x] older +w"], out
    assert run(["add", "new"], f)[1] == "added #4", "new id must follow the highest existing id"

for name, fn in [("existing_formats", t_existing_formats), ("done_unknown", t_done_unknown), ("edit", t_edit),
                 ("rm_no_reuse", t_rm_no_reuse), ("overdue", t_overdue), ("stats", t_stats), ("old_file", t_old_file)]:
    check(name, fn)
passed = sum(1 for v in results.values() if v is True)
print(json.dumps({"passed": passed, "total": len(results), "results": results}))
