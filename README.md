# Oche

A keyboard-first darts scorer and practice tracker for the Linux desktop, made for
[Omarchy](https://omarchy.org) (Hyprland). Type your darts, confirm with Enter, and
Oche keeps the score, suggests checkouts and stores every dart for statistics.

*Polski: aplikacja ma pełne tłumaczenie na polski — przełączysz je w ustawieniach (`Ctrl+,`).*

## Features

- **Games:** X01 (301/501/701, double in/out, any number of players, first to N legs),
  Cricket (standard and cut-throat), Around the Clock (optionally skipping ahead on doubles and trebles).
- **Training:** X01 solo, Checkout (random finishes from a range), Scoring (visits at T20/T19/T18/bull), Bob's 27.
- **Checkout hints:** the main route plus alternatives finishing on other doubles, for the darts left in the turn.
- **History** of every game and session, and **statistics**: 3-dart average, first 9, checkout %,
  180s, best leg, average over time, a heat map of where your darts land, Cricket MPR,
  training results, and a side-by-side comparison of players — for all time or the last 30/90/365 days.
- **Storage:** everything is saved locally in SQLite after each confirmed turn. Optionally,
  connect an empty **PostgreSQL** or **MySQL/MariaDB** database and Oche creates its tables there
  and sends games in the background; a second computer can fetch them from the same server.
- **Omarchy integration:** colours follow the active Omarchy theme live, the font follows
  `omarchy font set`, and the window has no decorations (window class `oche`).
- English and Polish.

## Entering darts

Each dart is a number followed by the key for the area, so `2` and `20` never clash and no Enter is needed per dart.

| Typed | Area | Points |
|---|---|---|
| `20↑` | outer single (between treble and double) | 20 |
| `20↓` | inner single (between bull and treble) | 20 |
| `20⏎` | single, area not given | 20 |
| `20d` | double | 40 |
| `20t` | treble | 60 |
| `25` / `50` | outer bull / bull, no key needed | 25 / 50 |
| `0` or `m` | miss | 0 |

`⏎` on an empty line confirms the turn (also with fewer than three darts), `⌫` deletes a digit or undoes
the last dart — across turns and legs — and `Esc` clears the line. Clicking the board works too.
`?` shows the cheat sheet.

| View | Key |
|---|---|
| New game | `Ctrl+N` |
| Training | `Ctrl+T` |
| Back to the game | `Ctrl+G` |
| History | `Ctrl+H` |
| Statistics | `Ctrl+S` |
| Settings | `Ctrl+,` |

## Install

### Arch Linux / Omarchy

Build the package from the repository and install it:

```sh
packaging/arch/build-local.sh           # builds the current commit
sudo pacman -U packaging/arch/out/oche-*.pkg.tar.zst
```

Oche then shows up in Walker (`Super+Space`). `packaging/arch/PKGBUILD` builds from a tagged GitHub release
and is the one meant for the AUR.

Remembering the password of a remote database needs a Secret Service provider such as `gnome-keyring`.

### Other distributions

Tagged releases have `.deb`, `.rpm` and AppImage builds attached. Building from source works anywhere Tauri 2 does (below).

### Where things are

| What | Where |
|---|---|
| Games and players | `~/.local/share/oche/oche.db` |
| Settings | `~/.config/oche/config.toml` (remote database), language in the app's local storage |
| Remote database password | the keyring (Secret Service), never in a file |

## Development

Requirements: Rust (stable, 1.94+), Node 20+, `webkit2gtk-4.1`, `base-devel`.

```sh
npm install
npm run tauri dev               # run with hot reload
npm test                        # game logic (Vitest)
npm run check                   # svelte-check / TypeScript
cd src-tauri && cargo test      # database code (SQLite in memory)
```

The remote database tests need real servers, each with an empty database `oche` and a database `other` holding any table:

```sh
OCHE_TEST_PG=postgres://oche:secret@localhost:5432 \
OCHE_TEST_MYSQL=mysql://oche:secret@localhost:3306 \
  cargo test remote -- --ignored
```

`packaging/install-local.sh` builds a release binary and installs it into `~/.local` without a package.

### Layout

```
src/lib/core/        game rules as pure functions: input grammar, X01, Cricket, Around the Clock,
                     drills, checkout routes, stored records (all tested)
src/lib/views/       screens; src/lib/play/ holds the parts every game screen shares
src/lib/i18n/        translations (en.ts is the source of the keys)
src-tauri/src/store/ local SQLite, remote PostgreSQL/MySQL, background sync
src-tauri/migrations/{sqlite,postgres,mysql}/
packaging/           desktop file, icon, Arch package
```

A game is stored as its list of events (darts and confirmations) and its state is replayed from that,
so undo is "drop the last event". Every confirmed turn saves the whole game. Changes for the remote
database go through an outbox table, so the game never waits for the network.

Migrations that have been released must never be edited (sqlx checks their checksums); add a new one instead.

### Adding a language

Copy `src/lib/i18n/locales/en.ts` to `<code>.ts`, translate it and add it in `src/lib/i18n/index.svelte.ts`.
TypeScript checks that every key is there.

### Releasing

1. Bump the version in `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` and `packaging/arch/PKGBUILD`, and add it to `CHANGELOG.md`.
2. Tag `vX.Y.Z` and push the tag: the release workflow builds the Linux bundles into a draft release.
3. In `packaging/arch`, run `updpkgsums` and publish the PKGBUILD to the AUR.

## License

[MIT](LICENSE) © 2026 Marek Pilczuk
