# Russian Draughts

A desktop application: a human plays Russian draughts against the computer.
Written in Rust; runs on macOS (Apple Silicon and Intel) and Linux (Wayland and X11).

*Русская версия этого файла: [README.ru.md](README.ru.md).*

- The official rules of Russian draughts, including the Turkish strike and all five
  conditions of an automatic draw.
- Ten levels of computer strength; on the tenth it thinks up to 4.5 s per move.
- The history of a game is a tree of variations; games are saved and loaded in the PDN
  format (GameType 25).
- The interface is available in twelve languages and switches without a restart.
- Images and sounds are made by the program. The only resource files are three small
  font subsets for Chinese, Japanese and Korean.

## Building and running

A stable Rust toolchain, version 1.85 or newer, is required.

### macOS

```bash
xcode-select --install
```

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

```bash
cargo build --release
```

```bash
./target/release/rucheckers
```

The Command Line Tools provide the linker and `libclang`, which the sound library needs
while building (its bindings to CoreAudio are generated at build time).

### Debian 13

System packages are enough: Rust 1.85 from the Debian 13 repository is suitable.

```bash
sudo apt install cargo build-essential pkg-config libasound2-dev
```

```bash
cargo build --release
```

```bash
./target/release/rucheckers
```

To run (not to build) the program needs libraries that every desktop installation has:
`libwayland-client0` or `libx11-6`, `libxkbcommon0`, `libxkbcommon-x11-0`, `libgl1`,
`libegl1`, `libxcursor1`, `libxi6`, `libxrandr2`, `libasound2t64`.
The file dialog is opened through the XDG Desktop Portal (`xdg-desktop-portal` and its
implementation for your desktop: `-gnome`, `-kde` or `-gtk`); without a portal `zenity`
is used.

If Rust is installed with rustup, the `cargo` package in the command above is not needed.

### Opening a game from the command line

```bash
./target/release/rucheckers game.pdn
```

## Tests

Everything except drawing and sound playback is tested without a window or a sound
device.

```bash
cargo test
```

Long checks (a hundred thousand random games, the ladder of levels, games of the tenth
level, the time limit on all levels and positions) are run separately and take several
minutes:

```bash
cargo test -- --ignored
```

A single group of tests:

```bash
cargo test --test draws
```

| File | What it checks |
| --- | --- |
| `tests/perft.rs` | reference for the move generator: depths 1–9 from the starting position |
| `tests/rules.rs` | every rule of moving and capturing on concrete positions |
| `tests/draws.rs` | every draw condition: the exact move it fires on and the reset of the counters |
| `tests/random_games.rs` | 3000 random games with invariants checked after every move |
| `tests/ai.rs` | legal moves and the time limit on every level, tactics, draws, games between levels |
| `tests/tree_pdn.rs` | the tree of variations, notation, PDN, damaged files |
| `tests/ui_logic.rs` | conversion of mouse coordinates, choosing a move, the game session |

Unit tests are next to the code in `src/`. Among other things they check that every
interface text exists in every language, that templates keep their placeholders, that
the built-in fonts have a glyph for every character of every text, and that every
combination of settings survives saving and loading.

### Code quality

```bash
cargo clippy --all-targets
```

The manifest forbids `unsafe` code and enables `clippy::pedantic`; the few exceptions
(numeric casts in geometry, pixel and sound code) are listed in `Cargo.toml` with their
reasons. The command above reports no warnings.

## Controls

- A move: click a piece, then a square; or drag the piece.
- A multi-capture can be made one jump at a time (then, with several routes, the player
  chooses the next square) or by pointing at the final square at once (it is marked
  with a ring). Only a square whose capture is determined unambiguously can be pointed
  at directly; if captures with different results lead to one square, the route is
  given jump by jump.
- Right mouse button or `Esc`: clear the selection, cancel an unfinished capture.
- Left and Right arrows: step through the moves; Up and Down: neighbouring variations;
  `Home` and `End`: the start and the end of the game.
- `Space`: "Continue from here" — the computer makes a move from the position shown.
- `F9`: hide or show the move panel.
- `Ctrl+N`, `Ctrl+O`, `Ctrl+S` (`Cmd` on macOS): new game, open, save.

While the history is being browsed the computer does not move. The game resumes with a
move by the human (if it differs from the recorded one, a variation is created) or with
"Continue from here".

### Special effects

A capture of two or more pieces in one move comes with an electric discharge: a
greenish-yellow lightning trails the flying piece along its route and dies out behind
it, and each captured piece flashes and bursts into sparks the moment the capturing
piece flies over it. In the settings, under "Special effects", the effects are switched
off all at once or one by one: lightning, sparks, flashes and glow. With everything off,
a capture of several pieces is shown like an ordinary one: the captured pieces dim and
disappear at the end of the move.

### Languages

The language is chosen in the settings from a list of native names:
Bahasa Indonesia, Deutsch, English, Español, Français, Italiano, Português, Türkçe,
Русский, 简体中文, 日本語, 한국어.

At the first start the language follows the system locale (`LC_ALL`, `LC_MESSAGES`,
`LANG`); if the locale is not set, Russian is used, and for a language without a
translation, English.

Limits of the localization:

- The translations were written together with the program and have not been reviewed by
  native speakers.
- Languages whose scripts need text shaping or right-to-left layout (Arabic, Hindi,
  Bengali, Urdu and others) are not offered: the interface toolkit draws text glyph by
  glyph, left to right, and would show them incorrectly.
- The built-in fonts cover Latin, Cyrillic and exactly the Chinese, Japanese and Korean
  characters of the interface texts. Other characters, for example in the path of a
  file named in an error message, are shown as placeholder boxes.

### Settings

The settings are kept in `~/.config/rucheckers/settings.conf` (Linux, respecting
`XDG_CONFIG_HOME`) or in `~/Library/Application Support/rucheckers/settings.conf`
(macOS). The file is plain text of `key=value` lines: `language` (a two-letter code such
as `ru`, `en` or `ja`), `level` (1–10), `color` (`white` or `black`), and the switches
`sound`, `panel`, `effects` (the main switch of the special effects), `effect_lightning`,
`effect_sparks` and `effect_flashes` with the values `on` and `off`. Missing and damaged
lines are replaced with default values.

The file is written after every change — a new game, the level list, any switch in the
settings dialog — so nothing depends on how the program is closed. Writing is atomic
(a temporary file replaces the old one in one step). If the file cannot be written, the
program says so once and goes on working.

## Structure

The library `rucheckers` (`src/lib.rs`) does not depend on a window:

| Module | Purpose |
| --- | --- |
| `board` | squares, pieces, a position (bit masks), FEN |
| `moves` | moves, the move generator, making and taking back a move, perft |
| `rules` | the end of a game, counters and limits of the draws (named constants) |
| `notation` | Russian notation: `c3-d4`, `c3:e5:g7` |
| `tree` | the game tree with variations |
| `pdn` | writing and parsing PDN |
| `ai` | evaluation, search, levels, the background calculation thread |
| `ui::geometry` | placement of the board, mouse coordinates → square |
| `ui::input` | choosing a move with clicks and dragging |
| `ui::anim` | timing of animation frames |
| `ui::fx` | the electric discharge of a multi-capture: lightning behind the piece, flashes, sparks |
| `ui::sprite` | the picture of a piece: computed pixel by pixel with anti-aliasing |
| `ui::session` | a game against the computer: whose move it is, browsing the history, variations |
| `i18n` | interface texts: one module per language (`src/i18n/*.rs`) and the message templates |
| `settings`, `sound`, `rng` | settings, sound synthesis, the random number generator |

The executable (`src/main.rs`, `src/gui/`) contains only the window, drawing, fonts,
sound playback and system dialogs.

Dependencies: `eframe` (window and vector drawing), `winit` (the windowing library
`eframe` runs on; listed explicitly so that the window has a title bar under Wayland),
`rodio` (sound output), `rfd` (the system file dialog). The versions are pinned in
`Cargo.toml` and `Cargo.lock`.

### Changing the texts

Every language is a file in `src/i18n/` with one `match` over all text keys, so a
missing text does not compile. After changing the Chinese, Japanese or Korean texts,
rebuild the font subsets (the font test fails until they cover the texts):

```bash
tools/subset_cjk_fonts.py /usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc
```

The script needs `fonttools` (`pip install fonttools`) and Noto Sans CJK (the package
`fonts-noto-cjk` on Debian). The subsets in `assets/fonts/` are distributed under the
SIL Open Font License 1.1, see `assets/fonts/OFL.txt`.

## Assumptions

Rules:

1. The "balance of forces" is the number of men and kings of each side. It changes with
   every capture and every promotion; the counters of rules 2, 4 and 5 are then reset.
2. Rule 2 (three or more kings against one): the stronger side has at least three
   kings and may have men as well; the weaker side has a single king. The moves of the
   stronger side are counted; the draw is declared right after its 15th move.
3. Rule 5 (the main road): the count runs while the lone king stands on the main road
   after every move. If it leaves the road, the count stops, and it starts anew when
   the king returns.
4. A move after which the opponent has no moves wins even if the same move exhausts a
   draw limit.
5. If several rules fire at once, the reason is named after the first of them in the
   order 1, 2, 3, 4, 5.
6. A repetition is the same placement and side to move on the branch that leads to the
   current position; neighbouring variations are not counted.

The computer:

7. The limit of the tenth level is 4.5 s of calculation; the margin up to 5 s is left
   for overhead. Levels 1–7 are limited by depth and do not depend on the speed of the
   computer.
8. The weakness of the lower levels is a small depth, a random error in the evaluation,
   "careless" moves that do not check the capture in reply, and now and then a random
   move.
9. A pause of at least 0.6 s is kept before the computer's reply.
10. The search is single-threaded.

Files:

11. From a PDN file with several games the first one is read.
12. A record in which moves continue after the game has ended by the rules of this
    program is rejected with a message.
13. The capture separators `:` and `x` are accepted, and so is a shortened capture if
    it is unambiguous. Numeric square notation is not supported.
14. The date in the PDN header is in universal time.
15. The player's colour is restored from the tags `WhiteType` and `BlackType`; the
    level does not change on loading.

Interface:

16. The default language follows the locale; Russian is used when the locale is not
    set, English for a language without a translation.
17. A loaded game opens at the last move of the main line, in browsing mode.

## Manual checklist

The tests do not cover what is seen and heard. Before a release it is worth going
through this list on both systems.

Appearance

- [ ] The board is in warm wood tones, with a frame carrying the letters `a–h` and the
      digits `1–8` on all sides.
- [ ] The square `a1` is dark, at the bottom left when playing White.
- [ ] A piece is an even circle in the centre of its cell, with smooth edges.
- [ ] A king differs from a man at a glance (a crown and a golden ring).
- [ ] Text in the panel, the dialogs and the window title is shown without placeholder
      boxes in every language, Chinese, Japanese and Korean included.
- [ ] Highlights: the selected piece, reachable squares, pieces that must capture, the
      last move.

Scaling

- [ ] The window resizes freely; the board stays square and centred.
- [ ] Cell borders and pieces are sharp at any window size.
- [ ] On a high-density screen (Retina; 150–200 % scale on Linux) the picture is crisp.
- [ ] With the panel hidden (`F9`) the board takes the whole window.

Play

- [ ] The player's colour is at the bottom; when playing Black the board is turned and
      the computer moves first.
- [ ] Moves by clicks and by dragging; a piece dropped elsewhere returns to its place.
- [ ] A multi-capture goes step by step or with one click on the final square (marked
      with a ring); captured pieces dim and disappear at the end of the move.
- [ ] While the computer is thinking the interface responds and the indicator is
      visible (in the panel and in the corner of the board frame).
- [ ] A new game and a step through the history can be started without waiting for the
      computer's reply.
- [ ] At the end of a game the result and the reason are shown; for a draw the rule is
      named.

Animation and sound

- [ ] Moves and captures travel smoothly, without jerks.
- [ ] When two or more pieces are captured, a greenish-yellow lightning trails the
      flying piece along its route (together with the movement, not after it); captured
      pieces burst into sparks as the capturing piece flies over them; the game does
      not slow down.
- [ ] In the settings, under "Special effects", the main switch removes all effects and
      the separate boxes remove the lightning, the sparks, and the flashes and glow one
      by one.
- [ ] The sounds of a move, a capture, a discharge, a promotion and the end of a game
      are distinct and not harsh.
- [ ] The sound button switches the sound off and on; the program works without a sound
      device.

Files and settings

- [ ] "Save…" and "Open…" bring up the system dialog; the `.pdn` extension is added
      automatically.
- [ ] A saved game with variations opens without losses.
- [ ] A foreign or damaged file gives a clear message, and the game does not change.
- [ ] The language, level, colour, sound, panel visibility and the choice of special
      effects are restored after a restart.
- [ ] The language list shows all twelve languages; choosing one changes every text at
      once, including the window title.
