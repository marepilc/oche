<script lang="ts" module>
  import type { AtcSettings } from '$lib/core/atc';
  import type { CricketSettings } from '$lib/core/cricket';
  import type { X01Settings } from '$lib/core/x01';

  export type NewGameChoice =
    | { mode: 'x01'; settings: X01Settings }
    | { mode: 'cricket'; settings: CricketSettings }
    | { mode: 'atc'; settings: AtcSettings };
</script>

<script lang="ts">
  import type { Player } from '$lib/core/x01';
  import { playerNamed, players as storedPlayers } from '$lib/db.svelte';
  import { t } from '$lib/i18n/index.svelte';

  interface Props {
    onstart: (choice: NewGameChoice, players: Player[]) => void;
  }
  let { onstart }: Props = $props();

  // The last line-up is remembered locally; names are matched to stored players on start.
  const KEY = 'oche:new-game';
  const saved = (() => {
    try {
      return JSON.parse(localStorage.getItem(KEY) ?? 'null') as {
        mode?: NewGameChoice['mode'];
        settings: X01Settings;
        cricket?: { cutThroat: boolean };
        atc?: { skips: boolean };
        players: Player[];
      } | null;
    } catch {
      return null;
    }
  })();

  let settings = $state<X01Settings>(saved?.settings ?? { start: 501, doubleIn: false, doubleOut: true, legsToWin: 2 });
  let players = $state<Player[]>(saved?.players ?? []);
  let mode = $state<NewGameChoice['mode']>(saved?.mode ?? 'x01');
  let cricket = $state(saved?.cricket ?? { cutThroat: false });
  let atc = $state(saved?.atc ?? { skips: false });
  let name = $state('');
  let nameInput: HTMLInputElement;
  let known = $state<Player[]>([]);
  storedPlayers().then((p) => (known = p)).catch(() => {});

  $effect(() => {
    nameInput.focus();
    // The webview may get keyboard focus only after mount.
    const refocus = () => document.activeElement === document.body && nameInput.focus();
    window.addEventListener('focus', refocus);
    return () => window.removeEventListener('focus', refocus);
  });

  function onWindowKey(e: KeyboardEvent) {
    const outside = document.activeElement === document.body;
    if (e.key === 'Enter' && (e.ctrlKey || outside)) {
      e.preventDefault();
      start();
    } else if (outside && e.key.length === 1 && !e.ctrlKey && !e.altKey && !e.metaKey && e.key !== '?') {
      nameInput.focus();
    }
  }

  function add() {
    const n = name.trim();
    if (!n) return;
    players.push({ id: crypto.randomUUID(), name: n });
    name = '';
  }

  let starting = false;
  async function start() {
    if (!players.length) return nameInput.focus();
    if (starting) return;
    starting = true;
    try {
      players = await Promise.all(players.map((p) => playerNamed(p.name)));
    } catch {
      /* the game still works, it just won't be saved */
    } finally {
      starting = false;
    }
    try {
      localStorage.setItem(KEY, JSON.stringify({ mode, settings, cricket, atc, players }));
    } catch {
      /* not critical */
    }
    const legsToWin = settings.legsToWin;
    const choice: NewGameChoice =
      mode === 'cricket'
        ? { mode, settings: { legsToWin, cutThroat: cricket.cutThroat } }
        : mode === 'atc'
          ? { mode, settings: { legsToWin, skips: atc.skips } }
          : { mode, settings: $state.snapshot(settings) };
    onstart(choice, $state.snapshot(players));
  }

  function onNameKey(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.ctrlKey) {
      e.preventDefault();
      if (name.trim()) add();
      else start();
    } else if (e.key === 'Backspace' && !name && players.length) {
      e.preventDefault();
      players.pop();
    }
  }

  function move(i: number, by: number) {
    const j = i + by;
    if (j < 0 || j >= players.length) return;
    [players[i], players[j]] = [players[j], players[i]];
  }
</script>

<svelte:window onkeydown={onWindowKey} />

<div class="wrap">
  <h1>{t('newGame.title')}</h1>

  <section>
    <div class="label">{t('newGame.players')}</div>
    <ol class="players">
      {#each players as p, i (p.id)}
        <li>
          <span class="seat">{i + 1}</span>
          <span class="pname">{p.name}</span>
          <button tabindex="-1" onclick={() => move(i, -1)} aria-label={t('newGame.moveUp')}>↑</button>
          <button tabindex="-1" onclick={() => move(i, 1)} aria-label={t('newGame.moveDown')}>↓</button>
          <button tabindex="-1" onclick={() => players.splice(i, 1)} aria-label={t('newGame.remove')}>✕</button>
        </li>
      {/each}
    </ol>
    <input
      bind:this={nameInput}
      bind:value={name}
      onkeydown={onNameKey}
      placeholder={t(players.length ? 'newGame.addNext' : 'newGame.addFirst')}
      maxlength="40"
      aria-label={t('newGame.addPlayer')}
      list="known-players"
    />
    <datalist id="known-players">
      {#each known.filter((k) => !players.some((p) => p.name.toLowerCase() === k.name.toLowerCase())) as k (k.id)}
        <option value={k.name}></option>
      {/each}
    </datalist>
  </section>

  <section>
    <div class="label">{t('newGame.mode')}</div>
    <div class="seg" role="radiogroup" aria-label={t('newGame.mode')}>
      {#each ['x01', 'cricket', 'atc'] as const as m (m)}
        <label><input type="radio" name="mode" value={m} bind:group={mode} /> {t(`newGame.${m}`)}</label>
      {/each}
    </div>
    {#if mode === 'x01'}
      <div class="seg" role="radiogroup" aria-label={t('newGame.startScore')}>
        {#each [301, 501, 701] as v (v)}
          <label><input type="radio" name="start" value={v} bind:group={settings.start} /> {v}</label>
        {/each}
      </div>
      <div class="seg">
        <label><input type="checkbox" bind:checked={settings.doubleIn} /> {t('newGame.doubleIn')}</label>
        <label><input type="checkbox" bind:checked={settings.doubleOut} /> {t('newGame.doubleOut')}</label>
      </div>
    {:else if mode === 'cricket'}
      <div class="seg">
        <label><input type="checkbox" bind:checked={cricket.cutThroat} /> {t('newGame.cutThroat')}</label>
      </div>
    {:else}
      <div class="seg">
        <label><input type="checkbox" bind:checked={atc.skips} /> {t('newGame.skips')}</label>
      </div>
    {/if}
  </section>

  <section>
    <div class="label">{t('newGame.legsToWin')}</div>
    <div class="seg" role="radiogroup" aria-label={t('newGame.legsToWin')}>
      {#each [1, 2, 3, 5, 7] as v (v)}
        <label><input type="radio" name="legs" value={v} bind:group={settings.legsToWin} /> {v}</label>
      {/each}
    </div>
  </section>

  <div class="actions">
    <button class="btn" onclick={start} disabled={!players.length}>{t('newGame.play')}</button>
    <span class="hint">
      {#each t('newGame.keys').split(/(\{\w+\})/) as part, k (k)}
        {#if part === '{ctrlEnter}'}<kbd>Ctrl</kbd>+<kbd>⏎</kbd>{:else if part === '{tab}'}<kbd>Tab</kbd>{:else}{part}{/if}
      {/each}
    </span>
  </div>
</div>

<style>
  .wrap { max-width: 640px; margin-inline: auto; width: 100%; padding-block: 24px; display: grid; gap: 22px; overflow: auto; height: 100%; align-content: start; }
  h1 { margin: 0; font-family: var(--font-score); font-weight: 800; font-size: 44px; text-transform: uppercase; letter-spacing: 0.02em; }
  section { display: grid; gap: 8px; }
  .players { list-style: none; margin: 0; padding: 0; display: grid; gap: 4px; }
  .players li { display: flex; align-items: center; gap: 10px; padding: 6px 10px; border-radius: var(--r); background: var(--bg-light); }
  .seat { color: var(--accent); font-weight: 700; width: 1.5ch; }
  .pname { flex: 1; }
  .players button { background: none; border: 0; color: var(--muted); cursor: pointer; padding: 0 4px; }
  .players button:hover { color: var(--fg); }
  input:not([type]) { background: var(--bg-dark); border: 1px solid var(--line); border-radius: var(--r); padding: 9px 12px; outline: none; }
  input:not([type]):focus { border-color: var(--accent); }
  .seg { display: flex; flex-wrap: wrap; gap: 6px; }
  .seg label { border: 1px solid var(--line); border-radius: var(--r); padding: 6px 12px; cursor: pointer; display: flex; gap: 6px; align-items: center; font-variant-numeric: tabular-nums; }
  .seg input { accent-color: var(--accent); margin: 0; }
  .seg label:has(input:checked) { border-color: var(--accent); background: var(--bg-light); }
  .seg label:has(input:focus-visible) { outline: 2px solid var(--accent); outline-offset: 2px; }
  .actions { display: flex; flex-wrap: wrap; align-items: center; gap: 14px; }
  .btn { background: var(--accent); color: var(--bg); border: 0; border-radius: var(--r); padding: 8px 20px; font-weight: 700; cursor: pointer; }
  .btn:disabled { opacity: 0.4; cursor: default; }
  .hint { color: var(--muted); font-size: 11px; }
</style>
