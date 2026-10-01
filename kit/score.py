#!/usr/bin/env python3
"""Score a Rust/ratatui Tetris project produced by a coding agent.

usage: score.py <project dir>   -> prints one JSON object

build:  cargo build succeeds, number of errors / warnings
clippy: number of clippy warnings (only if it builds)
run:    started in a real pseudo-terminal (100x40): still alive after 2 s, drew
        something, survived arrow keys + space, quit on 'q' (or Esc) within 3 s,
        left the alternate screen (terminal restored)
size:   lines of Rust
"""
import fcntl, glob, json, os, pty, re, select, signal, struct, subprocess, sys, termios, time

root = sys.argv[1]
res = {"dir": root}

tomls = sorted(glob.glob(os.path.join(root, "**", "Cargo.toml"), recursive=True), key=len)
tomls = [t for t in tomls if "/target/" not in t]
if not tomls:
    res["build"] = "no Cargo.toml"
    print(json.dumps(res)); sys.exit()
proj = os.path.dirname(tomls[0])
res["project"] = os.path.relpath(proj, root)
res["rust_lines"] = sum(sum(1 for _ in open(f, errors="ignore"))
                        for f in glob.glob(os.path.join(proj, "src", "**", "*.rs"), recursive=True))

b = subprocess.run(["cargo", "build", "--message-format", "short"], cwd=proj, capture_output=True, text=True, timeout=900)
out = b.stderr
res["build_ok"] = b.returncode == 0
res["errors"] = len(re.findall(r"^\S+:\d+:\d+: error", out, re.M)) or (0 if b.returncode == 0 else 1)
res["warnings"] = len(re.findall(r"^\S+:\d+:\d+: warning", out, re.M))
if not res["build_ok"]:
    res["first_errors"] = [l for l in out.splitlines() if ": error" in l][:5]
    print(json.dumps(res)); sys.exit()

c = subprocess.run(["cargo", "clippy", "--message-format", "short"], cwd=proj, capture_output=True, text=True, timeout=900)
res["clippy_warnings"] = len(re.findall(r"^\S+:\d+:\d+: warning", c.stderr, re.M))

meta = json.loads(subprocess.run(["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=proj,
                                 capture_output=True, text=True).stdout)
bins = [t["name"] for p in meta["packages"] for t in p["targets"] if "bin" in t["kind"]]
exe = os.path.join(meta["target_directory"], "debug", bins[0]) if bins else None
if not exe or not os.path.exists(exe):
    res["run"] = "no binary"
    print(json.dumps(res)); sys.exit()

pid, fd = pty.fork()
if pid == 0:
    os.chdir(proj)
    os.environ["TERM"] = "xterm-256color"
    os.execv(exe, [exe])
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 100, 0, 0))
buf = bytearray()

def pump(seconds):
    end = time.time() + seconds
    while time.time() < end:
        r, _, _ = select.select([fd], [], [], 0.05)
        if r:
            try:
                buf.extend(os.read(fd, 65536))
            except OSError:
                return

state = {"exited": False, "status": None}

def alive():
    if state["exited"]:
        return False
    p, st = os.waitpid(pid, os.WNOHANG)
    if p == 0:
        return True
    state["exited"], state["status"] = True, st
    return False

pump(2.0)
res["alive_after_2s"] = alive()
res["drew_output"] = len(buf) > 500
if res["alive_after_2s"]:
    for k in [b"\x1b[D", b"\x1b[C", b"\x1b[A", b"\x1b[B", b" ", b"\x1b[D", b"\x1b[A", b" "]:
        os.write(fd, k); pump(0.25)
    res["alive_after_keys"] = alive()
    quit_ok = False
    for k in [b"q", b"\x1b", b"\x03"]:
        if not alive():
            break
        os.write(fd, k); pump(1.5)
        if not alive():
            res["quit_key"] = {b"q": "q", b"\x1b": "esc", b"\x03": "ctrl-c"}[k]
            quit_ok = True
    res["quits"] = quit_ok
    if alive():
        os.kill(pid, signal.SIGKILL)
    res["restores_terminal"] = b"\x1b[?1049l" in buf
if not state["exited"]:
    try:
        _, st = os.waitpid(pid, 0)
        state["status"] = st
    except ChildProcessError:
        pass
if state["status"] is not None and os.WIFSIGNALED(state["status"]) and os.WTERMSIG(state["status"]) != signal.SIGKILL:
    res["crashed_signal"] = os.WTERMSIG(state["status"])
elif state["status"] is not None and os.WIFEXITED(state["status"]):
    res["exit_code"] = os.WEXITSTATUS(state["status"])
res["screen_tail"] = re.sub(r"\x1b\[[0-9;?]*[A-Za-z]", "", buf[-3000:].decode("utf-8", "ignore"))[-400:]
print(json.dumps(res))
