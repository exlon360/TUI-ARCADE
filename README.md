# TUI Arcade

Rust terminal arcade. Run it with:

```sh
games
```

The UI uses the alternate terminal screen and expands to the current terminal size. Put your terminal in fullscreen or resize it before or during play; menus and most game arenas will grow to use the extra space.

The app opens on a console-style home screen with three buttons:

- Play
- Settings
- Exit

## Play

Play opens the game library. Use W/S or the arrow keys to choose a game, then press Enter to launch it. High scores save automatically.

The library has 113 games, including the original arcade set plus Chess, Checkers, Tic Tac Toe, Connect Four, Mancala Rush, Sudoku Sweep, Word Vault, Hangman Vault, Reversi Flip, Blackjack Table, Battleship Radar, Tower Stack, Laser Maze, Domino Slider, Bowling Lane, Skee Ball, Mini Golf, Darts Board, Soccer Keeper, Pirate Plunder, Robot Factory, Moon Miner, Dragon Hoard, Wizard Duel, Mirror Maze, and many more.

The expanded library no longer routes its later entries through repeated one-size-fits-all templates. Chess validates real piece movement, prevents moving into check, promotes pawns, and detects checkmate/stalemate; Checkers enforces diagonal moves, mandatory captures, and king promotion. Each microgame also has a distinct dispatch mode and a distinct rule profile: Connect Four has column drops and CPU blocking, Mancala sows and captures stones, Sudoku uses a playable 4x4 number grid, Word Vault and Hangman use different word-guess budgets, Domino builds a matching chain, Blackjack Blitz is a target-21 push-your-luck draw, Battleship has a hidden fleet and radar misses, Tower Stack uses timing/overlap, Laser Maze uses Lights Out toggles, Curling reads ice weight, Archery compensates for wind, Basket Toss uses a moving hoop, Bomb Sweeper defuses adjacent bombs, Reactor Trace has a meltdown timer, and the later adventure/sports/fantasy entries use separate quest, lane, catch, aim, or sequence rules.

The Rust tests validate unique game names, valid library indices, distinct microgame dispatch modes, distinct visible-game mechanic signatures, and distinct shared-engine rule profiles so repeated entries are easier to catch before they ship.

Library tools:

- `[` and `]` cycle categories
- `/` edits the search filter
- `X` clears search
- Left/Right changes difficulty

## Settings

Settings includes:

- Difficulty
- Endless mode for very long runs
- Pong assist and Pong speed tuning
- Color theme preset
- Glyph set preset for borders, menu markers, scroll bars, and buttons
- Startup title text
- Controls rebinding tab
- Theme Lab and individual saved custom colors
- Sound on/off
- Sound test
- Click effects on/off
- Erase scores with a warning confirmation

## Controls

- Arrow keys or WASD move
- Enter activates buttons and launches games
- Space fires, drops, whacks, flips, or submits depending on the game
- P pauses/resumes games
- Q or Esc backs out
- T cycles color theme presets from Play
- G cycles glyph set presets from Play
- C opens Theme Lab from Play

Controls tab:

- Rebind move up/down/left/right
- Rebind action, pause, and quit
- Reset controls to defaults
- Arrow keys and Enter/Esc remain available as safety controls

Theme Lab:

- Up/Down chooses a UI role
- Left/Right changes the selected color
- 0 sets the background to terminal default
- P copies the current preset into Custom
- R randomizes the custom theme

Scores save to `~/.tui_arcade_scores.json`. Custom Rust theme colors save to `~/.tui_arcade_theme_rust.txt`, and the selected color theme saves to `~/.tui_arcade_theme_index.txt`. The selected glyph set saves to `~/.tui_arcade_glyphs.txt`. The startup title saves to `~/.tui_arcade_title.txt`. Rebound controls save to `~/.tui_arcade_controls.txt`.

Difficulty, Endless mode, sound/click toggles, and Pong tuning also persist between launches. Master difficulty is available for faster, lower-life runs.

Sound uses throttled, single-voice macOS `afplay` system sounds for clicks, hits, wall bounces, alerts, and scoring. The terminal bell is only used as a fallback if system sounds are unavailable.
