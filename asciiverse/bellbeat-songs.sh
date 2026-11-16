#!/bin/sh
# Makes the songs that come with Cascade in the Asciiverse: songs from
# Bellbeat, the Asciiverse's own rhythm game, all written for it and
# rendered by its own synth (`bellbeat --render`), as Ogg Vorbis, then
# imported with `cascade add` so each arrives with its four beatmaps.
# The result replaces songs/ (one folder per song, copied into the
# player's library on first start, see src/bundled.rs).
#   asciiverse/bellbeat-songs.sh BELLBEAT_CHECKOUT
# Needs a Bellbeat checkout with --render (theasciiverse/bellbeat, ce45503
# or later), cargo and ffmpeg with libvorbis.
set -eu
SONGS="hello-world segfault-march waltz-for-a-spinning-disk carriage-return five-nines pipe-dreams escape-sequence kernel-panic sigterm"
bellbeat=$(cd "$1" && pwd)
here=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
(cd "$bellbeat" && cargo build --release -q)
(cd "$here" && cargo build --release -q)
mkdir -p "$work/ogg" "$work/state"
for s in $SONGS; do
  "$bellbeat/target/release/bellbeat" --render "$s" "$work/render/$s"
  title=$(sed -n 's/^title //p' "$work/render/$s/song.txt")
  ffmpeg -loglevel error -y -i "$work/render/$s/mix.wav" -c:a libvorbis -q:a 4 \
    -metadata title="$title" -metadata artist="Bellbeat" \
    -metadata album="Bellbeat songs, Asciiverse" "$work/ogg/$s.ogg"
done
XDG_STATE_HOME="$work/state" HOME="$work" "$here/target/release/cascade" add "$work/ogg"
rm -rf "$here/songs"
cp -r "$work/state/cascade/songs" "$here/songs"
python3 -I "$here/asciiverse/check-beatmaps.py" "$work/render" "$here/songs"
