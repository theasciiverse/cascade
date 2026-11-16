#!/usr/bin/env python3
"""shrink-cast.py IN.cast OUT.cast [FPS] [STEP]: makes Cascade's preview small enough to
send. Cascade repaints most of the screen every frame because its
background (starfield, glow, vignette) shifts by one or two steps of colour
in the dark; those steps cannot be seen but cost bytes. This replays the
recording on a model of the screen, rounds every colour to a step of 8
(anything darker than 12 to black), keeps at most FPS frames a second
(15) and writes each frame again as only the cells that visibly changed. Frames are the synchronized updates the
game draws (`ESC [?2026h` to `ESC [?2026l`). Understands what Cascade
prints: cursor moves, SGR colours and attributes, plain characters; other
sequences pass through unchanged."""
import json, re, sys

src, dst = sys.argv[1], sys.argv[2]
FPS = float(sys.argv[3]) if len(sys.argv) > 3 else 15.0
STEP = int(sys.argv[4]) if len(sys.argv) > 4 else 8
lines = open(src, encoding="utf-8").read().splitlines()
header = json.loads(lines[0])
cols, rows = header["term"]["cols"], header["term"]["rows"]
events = [json.loads(l) for l in lines[1:]]

def q(v):
    return 0 if v < 12 else min(255, (v + STEP // 2) // STEP * STEP)

BLANK = (" ", None, None, ())
screen = [[BLANK] * cols for _ in range(rows)]
shown = [[BLANK] * cols for _ in range(rows)]
cur = [0, 0]
pen = [None, None, ()]  # fg, bg, attributes (tuple of SGR numbers)

def sgr(params):
    ps = [int(p) if p else 0 for p in params.split(";")] if params else [0]
    i = 0
    while i < len(ps):
        p = ps[i]
        if p == 0:
            pen[0], pen[1], pen[2] = None, None, ()
        elif p in (38, 48) and i + 1 < len(ps):
            if ps[i + 1] == 2 and i + 4 < len(ps):
                c = ("rgb", q(ps[i + 2]), q(ps[i + 3]), q(ps[i + 4]))
                i += 4
            elif ps[i + 1] == 5 and i + 2 < len(ps):
                c = ("idx", ps[i + 2])
                i += 2
            else:
                c = None
            pen[0 if p == 38 else 1] = c
        elif p == 39:
            pen[0] = None
        elif p == 49:
            pen[1] = None
        elif 30 <= p <= 37 or 90 <= p <= 97:
            pen[0] = ("idx", p - 30 if p < 90 else p - 82)
        elif 40 <= p <= 47 or 100 <= p <= 107:
            pen[1] = ("idx", p - 40 if p < 100 else p - 92)
        elif p in (22, 23, 24, 25, 27, 28, 29):
            off = {22: (1, 2), 23: (3,), 24: (4,), 25: (5,), 27: (7,), 28: (8,), 29: (9,)}[p]
            pen[2] = tuple(a for a in pen[2] if a not in off)
        else:
            pen[2] = tuple(sorted(set(pen[2]) | {p}))
        i += 1

def colour(c, fg):
    if c is None:
        return "39" if fg else "49"
    if c[0] == "rgb":
        return f"{38 if fg else 48};2;{c[1]};{c[2]};{c[3]}"
    n = c[1]
    if n < 8:
        return str((30 if fg else 40) + n)
    if n < 16:
        return str((90 if fg else 100) + n - 8)
    return f"{38 if fg else 48};5;{n}"

def diff():
    out, at, style = [], None, None
    for y in range(rows):
        for x in range(cols):
            cell = screen[y][x]
            if cell == shown[y][x]:
                continue
            if at != (y, x):
                out.append(f"\x1b[{y + 1};{x + 1}H")
            st = cell[1:]
            if st != style:
                out.append("\x1b[" + ";".join(["0", *map(str, st[2]), colour(st[0], True), colour(st[1], False)]) + "m")
                style = st
            out.append(cell[0])
            shown[y][x] = cell
            at = (y, x + 1)
    return "".join(out)

TOKEN = re.compile(r"\x1b\[([0-9;?<>=]*)([ -/]*)([@-~])|\x1b[^\[]|[^\x1b]")
out_events, t, last_out, pending = [], 0.0, 0.0, []
in_frame = False

def emit(text):
    global last_out
    if text:
        out_events.append([round(t - last_out, 6), "o", text])
        last_out = t

COMPLETE = re.compile(r"\x1b(\[[0-9;?<>=]*[ -/]*[@-~]|[^\[])")
carry = ""
for dt, kind, data in events:
    t += dt
    if kind != "o":
        out_events.append([round(t - last_out, 6), kind, data])
        last_out = t
        continue
    # an escape sequence can be split between two chunks of output
    data, carry = carry + data, ""
    esc = data.rfind("\x1b")
    if esc >= 0 and not COMPLETE.match(data, esc):
        data, carry = data[:esc], data[esc:]
    for m in TOKEN.finditer(data):
        tok = m.group(0)
        if m.group(3) is not None:
            params, final = m.group(1), m.group(3)
            if final == "m" and not m.group(2):
                sgr(params)
            elif final == "H" and not params.startswith("?"):
                p = (params.split(";") + ["", ""])[:2]
                cur[0] = min(rows - 1, max(0, (int(p[0]) if p[0] else 1) - 1))
                cur[1] = min(cols - 1, max(0, (int(p[1]) if p[1] else 1) - 1))
            elif params == "?2026" and final == "h":
                in_frame = True
            elif params == "?2026" and final == "l":
                in_frame = False
                if t - last_out >= 1 / FPS - 1e-6:
                    emit("".join(pending) + diff())
                    pending = []
            elif final == "J" and params in ("2", "3"):
                for y in range(rows):
                    screen[y] = [BLANK] * cols
                    shown[y] = [BLANK] * cols
                pending.append(tok)
            else:
                pending.append(tok)
        elif tok.startswith("\x1b"):
            pending.append(tok)
        elif tok in "\r\n":
            if tok == "\r":
                cur[1] = 0
            else:
                cur[0] = min(rows - 1, cur[0] + 1)
        else:
            y, x = cur
            if x < cols:
                screen[y][x] = (tok, pen[0], pen[1], pen[2])
            cur[1] = x + 1
    if not in_frame and pending and t - last_out >= 1 / FPS - 1e-6:
        emit("".join(pending) + diff())
        pending = []

t += 0.001
emit("".join(pending) + diff())
with open(dst, "w", encoding="utf-8") as f:
    f.write(json.dumps(header) + "\n")
    for e in out_events:
        f.write(json.dumps(e, ensure_ascii=False) + "\n")
