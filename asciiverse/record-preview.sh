#!/bin/sh
# Records the 100x30 catalog preview, asciiverse/cascade.cast
# (hub/RECORDING.md), with hub's castrec.py: Kernel Panic, one of the
# Bellbeat songs that come with the game, on Hard, played by --autoplay
# with no sound device opened (CASCADE_NO_AUDIO), into its fast middle;
# then pause, back to the song list with the bundled songs, and out.
# The game draws 30 frames a second for it (CASCADE_FPS) and
# shrink-cast.py keeps 15 with colours rounded to steps of 16, so the file stays small.
# Needs Python 3 and a hub checkout beside games/.
#   asciiverse/record-preview.sh
set -eu
cd "$(dirname "$0")/.."
here=$(pwd)
castrec=$(cd ../../hub/tools && pwd)/castrec.py
cargo build --release -q
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cp target/release/cascade "$tmp/" && cp -r songs "$tmp/songs"
(cd "$tmp" && PATH="$tmp:$PATH" HOME="$tmp" XDG_STATE_HOME="$tmp/state" \
  CASCADE_NO_AUDIO=1 CASCADE_FPS=30 \
  python3 "$castrec" "$tmp/raw.cast" . "w24;kesc;w1.2;kq;w1.5;kdown;w0.6;kdown;w0.8;kesc;w1;kq;w0.5" \
  -- cascade play bellbeat-kernel-panic --hard --autoplay)
python3 -I asciiverse/shrink-cast.py "$tmp/raw.cast" asciiverse/cascade.cast 15 16
ls -l asciiverse/cascade.cast
