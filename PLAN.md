# Oche — plan aplikacji

Desktopowa aplikacja do liczenia wyników w darta i śledzenia postępów graczy.
Projektowana pod Omarchy (Hyprland, Wayland, motywy Omarchy), obsługiwana głównie z klawiatury.

## 1. Stos technologiczny

| Warstwa | Wybór | Dlaczego |
|---|---|---|
| Powłoka | **Tauri 2** | mała binarka, natywny WebKitGTK, dobre wsparcie Wayland |
| Frontend | **Svelte 5 (runes) + Vite + TypeScript** | mało kodu, reaktywność bez boilerplate'u, szybkie SVG (tarcza) |
| Styl | czysty CSS na zmiennych (`--accent`, `--bg`…) | zmienne podmieniane na żywo kolorami z motywu Omarchy |
| Logika gry | czysty TypeScript w `src/lib/core` | reguły X01/Cricket jako czyste funkcje, testy w Vitest |
| Backend | Rust: `sqlx` (sqlite + mysql + postgres), `notify`, `keyring`, `toml`, `serde` | jeden kod zapytań dla trzech baz, obserwowanie zmian motywu, hasła w keyringu |
| Testy | Vitest (core), `cargo test` (repo + migracje), Playwright przez `tauri-driver` (e2e, później) | |

Frontend stoi na oficjalnym szablonie Tauri (SvelteKit w trybie SPA, `adapter-static`), ale widoki
przełącza prosty stan w `+page.svelte`, bez routingu po URL-ach.

## 2. Integracja z Omarchy

- **Motyw:** przy starcie czytamy `~/.local/state/omarchy/current/theme/colors.toml`
  (`background`, `foreground`, `accent`, `red`, `green`, `muted`, `lighter_background`…)
  i mapujemy na zmienne CSS. Crate `notify` obserwuje katalog `current/theme` —
  `omarchy theme set` przemalowuje aplikację bez restartu. Brak pliku → wbudowany motyw zapasowy.
  `mode = "light" | "dark"` decyduje o wariantach (np. kolor pól tarczy).
- **Czcionka:** monospace systemowy (`fc-match monospace`, w Omarchy JetBrainsMono Nerd Font)
  dla pola wpisywania i tabel; duże liczby wyników — dołączona czcionka wyświetlacza.
- **Okno:** `decorations: false`, klasa okna `oche` (dla reguł Hyprland), sensowny minimalny rozmiar,
  układ działający zarówno w kafelku 50% jak i na pełnym ekranie.
- **Uruchamianie:** plik `.desktop` + ikona → aplikacja widoczna w Walkerze (Super+Space).
- **Dystrybucja:** `PKGBUILD` budujący ze źródeł (AUR `oche`, z tagu na GitHubie) — Omarchy stoi na Archu.
- **WebKitGTK:** na NVIDIA ustawiamy domyślnie `WEBKIT_DISABLE_DMABUF_RENDERER=1`, jeśli wykryjemy problem z renderowaniem.
- Ścieżki XDG: config `~/.config/oche/config.toml`, dane `~/.local/share/oche/oche.db`.

## 3. Wprowadzanie rzutów — gramatyka klawiaturowa

Każdy rzut to **liczba + klawisz pola**. Klawisz po liczbie zatwierdza rzut, więc „2” i „20” się nie mylą
i nigdzie nie trzeba Entera. Litery `d`/`t` działają w obu wielkościach.

| Wpis | Obszar tarczy | Punkty |
|---|---|---|
| `20↑` | pojedyncze zewnętrzne (między potrójnym a podwójnym) | 20 |
| `20↓` | pojedyncze wewnętrzne (między bullem a potrójnym) | 20 |
| `20⏎` | pojedyncze, bez podania pola | 20 |
| `20d` / `20D` | podwójne | 40 |
| `20t` / `20T`, `1t`, `11t` | potrójne | 60, 3, 33 |
| `25` | zielony pierścień (outer bull), od razu | 25 |
| `50` | bull, od razu | 50 |
| `0` lub `m` | pudło, od razu | 0 |

**Tura kończy się dopiero po potwierdzeniu Enterem** (na pustej linii). Po trzeciej lotce, busta albo
zamknięciu aplikacja czeka na `⏎`, więc pomyłkę w ostatniej lotce da się poprawić `⌫`.
Enter po jednej czy dwóch lotkach kończy turę wcześniej. Leg jest przyznawany dopiero po potwierdzeniu zamknięcia.

Klawisze pomocnicze:

- `Backspace` — kasuje cyfrę, a przy pustej linii cofa ostatni rzut (cofanie wielopoziomowe, także przez tury i legi).
- `Esc` — czyści linię.
- Podczas pisania tarcza od razu podświetla wycinek.
  Wartości spoza tarczy (`21`) są odrzucane z krótkim komunikatem przy polu.
- Podpowiedź zamknięcia: główna trasa i do dwóch alternatyw kończących na innych podwójnych,
  liczona dla lotek pozostałych w turze; przy czekających graczach — trasa na ich następną turę.

Rozróżnienie wewnętrzne/zewnętrzne pole pojedyncze nie zmienia wyniku, ale daje dokładną mapę cieplną trafień.

## 4. Widoki i skróty

| Widok | Skrót | Zawartość |
|---|---|---|
| Gra | `Ctrl+G` | tablica wyników, tarcza z podświetleniem, pole wpisywania, podpowiedź checkoutu, statystyki bieżącego legu |
| Nowa gra | `Ctrl+N` | tryb (501/301/701, Cricket, Around the Clock), double in/out, sety/legi, wybór graczy strzałkami |
| Trening | `Ctrl+T` | X01 solo, Checkout (losowe zamknięcia z zakresu), Scoring (tury w T20/T19/T18/bull), Bob's 27 |
| Historia | `Ctrl+H` | rozegrane gry i treningi, wyniki graczy, `Del` ×2 usuwa |
| Gracze | `Ctrl+P` | lista, dodawanie (`a`), edycja (`e`), archiwizacja |
| Statystyki | `Ctrl+S` | średnia 3 lotek w czasie, first-9, % checkoutów, % podwójnych, 180/140+/100+, mapa cieplna tarczy, porównanie graczy |
| Ustawienia | `Ctrl+,` | język, baza danych, motyw (auto z Omarchy), dźwięki, domyślny tryb |
| Pomoc | `?` | ściągawka klawiszy |

Nawigacja w listach: `j/k` lub strzałki, `Enter` wybór, `Esc` wstecz. W widoku gry cyfry i strzałki
są zarezerwowane dla wpisywania rzutów, dlatego przełączanie widoków idzie przez `Ctrl`.

## 4a. Języki

Domyślnie angielski, polski do wyboru w ustawieniach. Teksty są w `src/lib/i18n/locales/<kod>.ts`;
`en.ts` jest źródłem kluczy, a TypeScript pilnuje, żeby każdy język miał komplet. Nowy język:
skopiuj `en.ts`, przetłumacz i dopisz jedną linię w `src/lib/i18n/index.svelte.ts`.
Liczba mnoga przez `Intl.PluralRules` (`one`/`few`/`many`/`other`), liczby przez `Intl.NumberFormat`.
Wybór języka jest na razie w localStorage; trafi do `config.toml` razem z resztą ustawień.

## 5. Model danych

Identyfikatory **UUID v7** (sortowalne, bezpieczne przy synchronizacji między bazami).

```
players      (id, name, created_at, archived_at)
games        (id, mode, settings_json, started_at, finished_at, winner_id)
game_players (game_id, player_id, seat)
legs         (id, game_id, set_no, leg_no, starter_id, winner_id, started_at, finished_at)
turns        (id, leg_id, player_id, turn_no, target, score_before, scored, bust, checkout)
darts        (id, turn_id, dart_no, segment, ring, multiplier, points)
sync_outbox  (id, entity, entity_id, op, created_at)   -- tylko SQLite
```

`ring ∈ {inner, outer, single, double, triple, obull, bull, miss}` (jak w `src/lib/core/dart.ts`).
`games.mode ∈ {x01, checkout, scoring, bobs27}`; w treningach leg to jedna próba (checkout) albo cała sesja,
a `turns.target` mówi, w co celowała tura (`121`, `T20`, `D7`).

Frontend zapisuje całą grę (`game_save`) po każdej zatwierdzonej turze; zapis zastępuje wszystko pod tym `id`.
Outbox nie trzyma kopii danych — wysyłający czyta bieżący stan z SQLite, więc kilka zapisów jednej gry
zlewa się w jedno wysłanie. Do outboxa trafiają zmiany tylko wtedy, gdy zdalna baza jest włączona;
przy podłączeniu nowej bazy kolejkowana jest cała lokalna historia.
Statystyki liczone zapytaniami SQL (widoki `v_turn_stats`, `v_dart_heatmap`) — działają tak samo na trzech silnikach.

## 6. Przechowywanie: lokalnie + opcjonalna zdalna baza

- **Źródłem prawdy jest zawsze lokalny SQLite.** Gra działa offline i nigdy nie czeka na sieć.
- W ustawieniach można dodać **PostgreSQL lub MySQL/MariaDB**: host, port, baza, użytkownik, SSL.
  Hasło trafia do keyringa (Secret Service / gnome-keyring), nie do `config.toml`.
- Przycisk „Testuj połączenie” → tworzy schemat (migracje `sqlx` osobno dla każdego dialektu) na **pustej** bazie;
  jeśli baza ma już schemat Oche, sprawdza wersję.
- Każdy zapis lokalny dopisuje rekord do `sync_outbox`. Zadanie w tle (Tokio) wypycha zmiany
  na zdalną bazę (`INSERT … ON CONFLICT` / `ON DUPLICATE KEY`), z ponawianiem i wskaźnikiem stanu w stopce.
- „Importuj ze zdalnej bazy” — pobiera graczy i historię (np. drugi komputer z tym samym serwerem).
- Rust: trait `Store` z implementacjami `SqliteStore` i `RemoteStore` (sqlx `Any` lub enum nad `PgPool`/`MySqlPool`).

## 7. Struktura repo

```
oche/
  src/                     # Svelte
    lib/core/              # reguły gier, parser wpisów, checkouty (czysty TS + testy)
    lib/board/             # komponent tarczy SVG
    lib/views/             # Game, NewGame, Players, Stats, Settings
    lib/theme.ts           # zmienne CSS z motywu Omarchy
  src-tauri/
    src/commands.rs        # komendy Tauri (save_turn, undo, stats, settings…)
    src/store/             # sqlite.rs, remote.rs, sync.rs
    src/theme.rs           # parsowanie colors.toml + watcher
    migrations/{sqlite,postgres,mysql}/
  packaging/               # oche.desktop, ikona, PKGBUILD
```

## 8. Kamienie milowe

1. ✅ **Szkielet** — Tauri 2 + Svelte 5, okno, motyw Omarchy na żywo, plik `.desktop`.
2. ✅ **Parser i tarcza** — gramatyka wpisów z testami, komponent tarczy z podświetlaniem.
3. ✅ **X01** — 301/501/701, double in/out, bust, cofanie, podpowiedź checkoutu, dowolna liczba graczy.
3a. ✅ **Języki** — angielski (domyślny) i polski, ustawienia.
4. ✅ **Zapis lokalny** — SQLite, migracje, gracze, historia gier.
5. ✅ **Statystyki** (`Ctrl+S`) — średnia i first 9, checkout % (tury zaczęte na zamknięciu do 170),
   180/140+/100+, najlepszy leg i zamknięcie, średnia w kolejnych grach, mapa cieplna, wyniki treningów.
   ✅ Porównanie graczy, zakresy dat (30/90/365 dni).
6. ✅ **Zdalna baza** — PostgreSQL: ustawienia, keyring, migracje, outbox i synchronizacja.
   Import ze zdalnej bazy (automatycznie przy podłączeniu istniejącej bazy Oche i przyciskiem). ✅ MySQL/MariaDB.
7. ✅ **Więcej trybów** — trening (X01 solo, Checkout, Scoring, Bob's 27), Cricket (także cut-throat), Around the Clock.
8. ✅ **Wydanie** — `packaging/arch/PKGBUILD` (+ `build-local.sh`), CI i workflow wydań na GitHubie (.deb/.rpm/AppImage).
   Do zrobienia przy pierwszym wydaniu: tag `v0.1.0`, `updpkgsums`, publikacja w AUR.

## 9. Wymagania do budowania

- Rust (`rust` z pacmana, `rustup` lub `mise`).
- `webkit2gtk-4.1`, `base-devel` (pacman).
- Node (jest przez mise) + npm/pnpm.
