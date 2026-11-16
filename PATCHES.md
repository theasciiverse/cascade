# Changes from upstream

This is the Asciiverse packaging of [Cascade](https://github.com/glebmatz/cascade)
by Gleb Matsko (MIT OR Apache-2.0, `LICENSE-MIT`, `LICENSE-APACHE`), the
five-lane terminal rhythm game that builds its own beatmaps from any song.
Upstream is the `upstream` remote; this build is upstream's v0.10.0
(`c67c9b2`, 2026-10-02). The `asciiverse` branch carries the changes below,
one commit each, written so any of them could go upstream on its own.

## Code

- **Session logging** with `log` and `env_logger`:
  `RUST_LOG=cascade=debug cascade 2>cascade.log` (plain `RUST_LOG=debug`
  adds the audio decoders' own messages). Every terminal event is logged as
  it is read, before it is turned into an action, with the screen it
  arrived on; screen changes, each song started (title, beatmap,
  difficulty, mods, note count, practice, autoplay), pause and resume, and
  each song's end (score, accuracy, combo, judgements) at info; twice a
  second a snapshot at debug (in a song: position, score, combo, accuracy,
  health, judgements, held lanes; elsewhere: the song list's selection,
  difficulty and search). Without `RUST_LOG`, only errors.
  (`src/main.rs`, `src/screens/gameplay.rs`)
- **State folder.** Settings, songs, scores, achievements, play history
  and themes live in `$XDG_STATE_HOME/cascade` when it is set, else
  `~/.local/state/cascade` (Linux and the BSDs),
  `~/Library/Application Support/cascade` (macOS) or
  `%LOCALAPPDATA%\cascade` (Windows), instead of `~/.cascade`. A
  `~/.cascade` from upstream's build is copied over the first time, so a
  library and its scores carry on; the old folder is left as it was.
  (`src/config.rs`)
- **Playing without a sound device.** Upstream stopped with "Audio error"
  when no output device could be opened (a server, a container, a session
  over ssh). The song now plays silently on a clock of its own (wall time
  at the song's speed, paused, sought and stopped like the device), so the
  game plays the same. `CASCADE_NO_AUDIO=1` opens no device at all, for
  recordings and tests; the menu sounds and the song preview stay quiet
  too. (`src/audio/player.rs`, `src/audio/sfx.rs`, `src/audio/preview.rs`)
- **No network.** Downloading audio (`cascade add <URL>`, `cascade import`
  of a package that names a source URL) is behind a new cargo feature,
  `net`, which this build leaves off, so the game never goes online.
  Without it a URL is an error that says to fetch the file and add it, and
  an imported package reports the audio file to drop into its folder, as
  upstream does for packages without a URL. (`Cargo.toml`, `src/share.rs`)
- **Songs that come with the game.** A `songs` folder beside the program
  holds ready-made song folders (audio, metadata and the four beatmaps,
  exactly as `cascade add` leaves them). The first time the game sees one
  it copies it into the player's library, where it plays, scores, renames
  and deletes like any other song; `bundled` in the state folder lists the
  songs already offered, so a deleted one stays deleted and a song new in
  a later release still arrives. (`src/bundled.rs`, `src/main.rs`)
- **`--autoplay`** for `cascade play`: the game hits every note as it
  reaches the line, holds holds and slides slides, and shows the results;
  nothing is scored or saved. **`CASCADE_FPS`** draws fewer frames a second
  while the game still runs at 60. Both are for the catalog preview.
  (`src/screens/gameplay.rs`, `src/main.rs`, `src/cli.rs`)
- **Rust 1.93.1** in `rust-toolchain.toml` (upstream 1.95.0): it builds
  the same, and our release builds are pinned to it.

## The songs (`songs/`)

Nine songs from [Bellbeat](https://github.com/theasciiverse/bellbeat), the
Asciiverse's own rhythm game, whose music was all written for it: Hello,
World; Segfault March (2/4); Waltz for a Spinning Disk (3/4); Carriage
Return; Five Nines (5/4); Pipe Dreams; Escape Sequence; Kernel Panic;
SIGTERM. Each is rendered by Bellbeat's own synth (`bellbeat --render`,
added for this) to Ogg Vorbis, about 0.6 to 0.9 MB a song, 7.3 MB in all,
and imported with `cascade add`, so Cascade's own generator made the
beatmaps. MIT, as Bellbeat.

`asciiverse/bellbeat-songs.sh BELLBEAT_CHECKOUT` makes them again.
`asciiverse/check-beatmaps.py` measures the maps against the songs, which
are written on a grid of sixteenths, so the right answer is known: Cascade
finds every song's tempo exactly (Null Pointer Samba, left out, at half),
and 95 to 100% of the notes on Easy, Medium and Hard land within 25 ms of a
sixteenth (Hello, World: 82 to 94%), a median 1 to 8 ms late, inside the
35 ms Perfect window. Songs left out sat worse: the slow flute songs
(Heartbeat of the Fan, Lullaby for a Daemon) at 49 to 75%.

## Packaging (new files)

- `.github/workflows/asciiverse-release.yml`: av archives for Linux
  (glibc, x86-64 and ARM64, built on Ubuntu 22.04), macOS (Apple silicon
  on macos-15, Intel on macos-15-intel) and Windows (x86-64 and ARM64),
  each with `songs/` beside the binary, plus SHA256SUMS. Linux is not
  static musl: sound goes out through cpal and ALSA, linked dynamically, so
  the binary needs `libasound2`. Upstream's `release.yml` now skips
  `*-av.*` tags; its `ci.yml` runs on `main` only and is left as it is.
- `asciiverse/record-preview.sh`, `asciiverse/shrink-cast.py`: the
  catalog preview, Kernel Panic on Hard by `--autoplay`, recorded with
  hub's `castrec.py` and its colours rounded so it stays small.
- `asciiverse/cascade.toml`, `asciiverse/cascade.md`,
  `asciiverse/cascade.cast`: the catalog manifest, store page and preview,
  drafts for hub.
