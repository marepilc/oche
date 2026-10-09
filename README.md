# Oche

Liczenie wyników w darta i śledzenie postępów, pisane pod [Omarchy](https://omarchy.org).
Rzuty wpisuje się z klawiatury: `20↑`, `20↓`, `20d`, `20t`, `25`, `50`, `m`, a turę zatwierdza `⏎`.
Pełna ściągawka jest pod `?` w aplikacji. Język: angielski (domyślnie) lub polski w ustawieniach (`Ctrl+,`).

Kolory pochodzą z aktywnego motywu Omarchy i zmieniają się razem z nim.

## Uruchamianie

Wymagania: Rust, Node, `webkit2gtk-4.1`, `base-devel`.

```sh
npm install
npm run tauri dev      # tryb deweloperski
npm test               # testy logiki gry
cd src-tauri && cargo test   # testy bazy (SQLite w pamięci)
# testy zdalnych baz: na każdym serwerze pusta baza `oche` i baza `other` z dowolną tabelą
OCHE_TEST_PG=postgres://oche:haslo@localhost:5432 \
OCHE_TEST_MYSQL=mysql://oche:haslo@localhost:3306 cargo test remote -- --ignored
packaging/install-local.sh   # build release + instalacja do ~/.local (widoczne w Walkerze)
```

Plan rozwoju: [PLAN.md](PLAN.md).
