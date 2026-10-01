#!/bin/bash
# Play a terminal game by hand (for Claude): runs it in a detached tmux session,
# sends keys, and prints the screen with colored cells made visible.
#   play.sh start <binary> [dir]     start the game (100x40 terminal)
#   play.sh keys <key>... [--wait S] send tmux keys (Left Right Up Down Space p q ...), then show
#   play.sh show                     print the screen: '#' = block, '+' = ghost/shade
#   play.sh move <key>...           while paused: resume, keys, capture, pause again
#   play.sh stop                     end the session
S=tetris-play
case $1 in
  start)
    tmux kill-session -t $S 2>/dev/null
    tmux new-session -d -s $S -x 100 -y 40 "cd '${3:-.}' && TERM=xterm-256color '$2'; echo EXITED with \$?; sleep 3600"
    sleep 1.5; exec "$0" show ;;
  keys)
    shift; wait=0.3
    for k in "$@"; do
      [ "$k" = --wait ] && { wait=next; continue; }
      [ "$wait" = next ] && { wait=$k; continue; }
      tmux send-keys -t $S "$k"; sleep 0.04
    done
    sleep "$wait"; exec "$0" show ;;
  move)
    # one Tetris move while the game is paused: resume, send keys, capture the
    # running screen, pause again (P toggles pause in most of these games)
    shift; tmux send-keys -t $S p; sleep 0.05
    for k in "$@"; do tmux send-keys -t $S "$k"; sleep 0.03; done
    sleep 0.12; "$0" show > /tmp/tetris-play-screen.txt; tmux send-keys -t $S p
    cat /tmp/tetris-play-screen.txt ;;
  show)
    tmux capture-pane -t $S -p -e | python3 -c '
import re, sys
from collections import Counter
rows = []
for line in sys.stdin.read().split("\n"):
    bg = None; cells = []; i = 0
    while i < len(line):
        m = re.match(r"\x1b\[([0-9;]*)m", line[i:])
        if m:
            codes = m.group(1).split(";") if m.group(1) else ["0"]
            j = 0
            while j < len(codes):
                c = codes[j]
                if c in ("0", "49"): bg = None
                elif c in ("38", "58"):   # foreground/underline colour: skip its parameters
                    j += 4 if codes[j+1:j+2] == ["2"] else 2
                elif c == "48":
                    n = 5 if codes[j+1:j+2] == ["2"] else 3
                    bg = ";".join(codes[j:j+n]); j += n - 1
                elif c.isdigit() and (40 <= int(c) <= 47 or 100 <= int(c) <= 107): bg = c
                j += 1
            i += m.end(); continue
        cells.append((line[i], bg)); i += 1
    rows.append(cells)
# a background counts as a block only if it is bright; dark ones are panels/board
def bright(b):
    if b is None: return False
    p = b.split(";")
    if p[:2] == ["48", "2"]: return max(int(v) for v in p[2:5]) > 90
    if p[:2] == ["48", "5"]: n = int(p[2]); return n not in (0, 16, 232, 233, 234, 235, 236, 237, 238)
    return b not in ("40", "100")
panel = set()
glyph = {"█": "#", "■": "#", "▇": "#", "▓": "@", "▒": "%", "░": ".", "⬛": "#"}
out = []
for r in rows:
    s = "".join(glyph.get(ch, "#" if (ch == " " and bright(b)) else ch) for ch, b in r)
    out.append(s.rstrip())
while out and not out[-1]: out.pop()
print("\n".join(out))' ;;
  board)
    # just the playfield as cells: board <first col> <first row> <cols> <rows> [chars per cell]
    # (reads the last screen captured by move/show if called right after, else a fresh one)
    x=$2; y=$3; w=$4; h=$5; cw=${6:-2}
    "$0" show | python3 -c '
import sys
x,y,w,h,cw = map(int, sys.argv[1:6])
lines = sys.stdin.read().split("\n")
print("   " + "".join(str(c % 10) for c in range(w)))
for r in range(h):
    line = lines[y + r] if y + r < len(lines) else ""
    cells = ""
    for c in range(w):
        seg = line[x + c*cw : x + c*cw + cw]
        cells += "#" if ("@" in seg or "#" in seg) else ("." if "." in seg or "%" in seg else " ")
    print("%2d|%s|" % (r, cells))' $x $y $w $h $cw ;;
  stop)
    tmux kill-session -t $S 2>/dev/null; echo stopped ;;
esac
