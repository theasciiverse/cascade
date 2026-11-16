#!/usr/bin/env python3
"""check-beatmaps.py RENDER SONGS: how well Cascade's generated beatmaps fit
Bellbeat's songs, which are written on a grid of sixteenths, so the truth is
known. For each song and difficulty: notes a second, the share of notes
within 25 ms of a sixteenth (swing included), the median offset in ms
(positive is late), holds and slides; and the tempo Cascade found against
the song's own. RENDER holds `bellbeat --render` output, one folder a song;
SONGS holds Cascade's song folders (bellbeat-<song>)."""
import bisect, json, os, statistics, sys

render, songs = sys.argv[1], sys.argv[2]
for song in sorted(os.listdir(render)):
    lines = open(os.path.join(render, song, "song.txt")).read().splitlines()
    info = dict(l.split(" ", 1) for l in lines if not l.startswith("section"))
    bpm, swing, secs = float(info["bpm"]), float(info["swing"]), float(info["secs"])
    step = 60 / bpm / 4
    def at(s):
        return ((s // 4) + [0, swing / 2, swing, swing + (1 - swing) / 2][s % 4]) * 4 * step * 1000
    grid = [at(s) for s in range(int(secs / step) + 8)]
    folder = os.path.join(songs, "bellbeat-" + song)
    row = []
    for diff in ("easy", "medium", "hard", "expert"):
        bm = json.load(open(os.path.join(folder, diff + ".json")))
        notes = bm["notes"]
        offs = []
        for n in notes:
            i = bisect.bisect_left(grid, n["time_ms"])
            near = [grid[j] for j in (i - 1, i) if 0 <= j < len(grid)]
            offs.append(min((n["time_ms"] - g for g in near), key=abs))
        on = sum(abs(o) <= 25 for o in offs) / max(1, len(offs))
        holds = sum(n["duration_ms"] > 0 for n in notes)
        slides = sum("slide_to" in n for n in notes)
        med = statistics.median(offs) if offs else 0
        row.append(f"{diff[:2]} {len(notes) / secs:3.1f}/s {on * 100:3.0f}% {med:+3.0f}ms h{holds} s{slides}")
    print(f"{song[:24]:24} {bpm:5.1f} bpm, found {bm['song']['bpm']:5.1f} | " + " | ".join(row))
