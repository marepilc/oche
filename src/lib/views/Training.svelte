<script lang="ts">
  import type { DrillSettings } from '$lib/core/drills';
  import { uuid7 } from '$lib/core/record';
  import type { Player, X01Settings } from '$lib/core/x01';
  import { playerNamed, players as storedPlayers } from '$lib/db.svelte';
  import { t } from '$lib/i18n/index.svelte';
  import type { MessageKey } from '$lib/i18n/types';

  interface Props {
    onx01: (settings: X01Settings, player: Player) => void;
    ondrill: (settings: DrillSettings, player: Player) => void;
  }
  let { onx01, ondrill }: Props = $props();

  type Kind = 'x01' | DrillSettings['kind'];
  interface Saved {
    name: string;
    kind: Kind;
    x01: { start: number; legs: number; doubleOut: boolean };
    checkout: { range: string; attempts: number; turns: number };
    scoring: { segment: number; rounds: number };
  }
  const KEY = 'oche:training';
  const defaults: Saved = {
    name: '',
    kind: 'checkout',
    x01: { start: 501, legs: 3, doubleOut: true },
    checkout: { range: '61-100', attempts: 10, turns: 3 },
    scoring: { segment: 20, rounds: 10 },
  };
  const saved = (() => {
    try {
      return { ...defaults, ...(JSON.parse(localStorage.getItem(KEY) ?? '{}') as Partial<Saved>) };
    } catch {
      return defaults;
    }
  })();
  let o = $state<Saved>(saved);
  let known = $state<Player[]>([]);
  storedPlayers().then((p) => (known = p)).catch(() => {});
  let nameInput: HTMLInputElement;

  const KINDS: Kind[] = ['x01', 'checkout', 'scoring', 'bobs27'];
  const RANGES = ['2-40', '41-80', '61-100', '81-120', '101-170', '2-170'];

  $effect(() => {
    nameInput.focus();
  });

  let starting = false;
  async function start() {
    const name = o.name.trim();
    if (!name) return nameInput.focus();
    if (starting) return;
    starting = true;
    let player: Player;
    try {
      player = await playerNamed(name);
    } catch {
      player = { id: uuid7(), name };
    } finally {
      starting = false;
    }
    try {
      localStorage.setItem(KEY, JSON.stringify(o));
    } catch {
      /* not critical */
    }
    if (o.kind === 'x01') {
      onx01({ start: o.x01.start, doubleIn: false, doubleOut: o.x01.doubleOut, legsToWin: o.x01.legs }, player);
    } else if (o.kind === 'checkout') {
      const [min, max] = o.checkout.range.split('-').map(Number);
      const seed = Math.floor(Math.random() * 2 ** 31);
      ondrill({ kind: 'checkout', min, max, attempts: o.checkout.attempts, turns: o.checkout.turns, seed }, player);
    } else if (o.kind === 'scoring') {
      ondrill({ kind: 'scoring', segment: o.scoring.segment, rounds: o.scoring.rounds }, player);
    } else {
      ondrill({ kind: 'bobs27' }, player);
    }
  }

  function onWindowKey(e: KeyboardEvent) {
    const outside = document.activeElement === document.body;
    if (e.key === 'Enter' && (e.ctrlKey || outside)) {
      e.preventDefault();
      start();
    }
  }
</script>

<svelte:window onkeydown={onWindowKey} />

<div class="wrap">
  <h1>{t('training.title')}</h1>

  <section>
    <div class="label">{t('training.player')}</div>
    <input
      bind:this={nameInput}
      bind:value={o.name}
      onkeydown={(e) => { if (e.key === 'Enter' && !e.ctrlKey) { e.preventDefault(); start(); } }}
      placeholder={t('training.playerHint')}
      maxlength="40"
      aria-label={t('training.player')}
      list="training-players"
    />
    <datalist id="training-players">
      {#each known as k (k.id)}<option value={k.name}></option>{/each}
    </datalist>
  </section>

  <section>
    <div class="label">{t('training.drill')}</div>
    <div class="drills" role="radiogroup" aria-label={t('training.drill')}>
      {#each KINDS as k (k)}
        <label>
          <input type="radio" name="drill" value={k} bind:group={o.kind} />
          <span class="dname">{t(`training.${k}` as MessageKey)}</span>
          <span class="desc">{t(`training.${k}Desc` as MessageKey)}</span>
        </label>
      {/each}
    </div>
  </section>

  {#if o.kind === 'x01'}
    <section>
      <div class="label">{t('training.start')}</div>
      <div class="seg">
        {#each [301, 501, 701] as v (v)}
          <label><input type="radio" name="x01start" value={v} bind:group={o.x01.start} /> {v}</label>
        {/each}
        <label><input type="checkbox" bind:checked={o.x01.doubleOut} /> {t('newGame.doubleOut')}</label>
      </div>
      <div class="label">{t('training.legs')}</div>
      <div class="seg">
        {#each [1, 3, 5, 10] as v (v)}
          <label><input type="radio" name="x01legs" value={v} bind:group={o.x01.legs} /> {v}</label>
        {/each}
      </div>
    </section>
  {:else if o.kind === 'checkout'}
    <section>
      <div class="label">{t('training.range')}</div>
      <div class="seg">
        {#each RANGES as v (v)}
          <label><input type="radio" name="range" value={v} bind:group={o.checkout.range} /> {v.replace('-', '–')}</label>
        {/each}
      </div>
      <div class="label">{t('training.attempts')}</div>
      <div class="seg">
        {#each [5, 10, 20] as v (v)}
          <label><input type="radio" name="attempts" value={v} bind:group={o.checkout.attempts} /> {v}</label>
        {/each}
      </div>
      <div class="label">{t('training.visits')}</div>
      <div class="seg">
        {#each [1, 2, 3] as v (v)}
          <label><input type="radio" name="visits" value={v} bind:group={o.checkout.turns} /> {v} × 3</label>
        {/each}
      </div>
    </section>
  {:else if o.kind === 'scoring'}
    <section>
      <div class="label">{t('training.segment')}</div>
      <div class="seg">
        {#each [20, 19, 18, 25] as v (v)}
          <label><input type="radio" name="segment" value={v} bind:group={o.scoring.segment} /> {v === 25 ? 'BULL' : `T${v}`}</label>
        {/each}
      </div>
      <div class="label">{t('training.rounds')}</div>
      <div class="seg">
        {#each [10, 20, 30] as v (v)}
          <label><input type="radio" name="rounds" value={v} bind:group={o.scoring.rounds} /> {v}</label>
        {/each}
      </div>
    </section>
  {/if}

  <div class="actions">
    <button class="btn" onclick={start} disabled={!o.name.trim()}>{t('training.go')}</button>
    <span class="hint">
      {#each t('training.keys').split(/(\{\w+\})/) as part, k (k)}
        {#if part === '{enter}'}<kbd>⏎</kbd>{:else if part === '{tab}'}<kbd>Tab</kbd>{:else}{part}{/if}
      {/each}
    </span>
  </div>
</div>

<style>
  .wrap { max-width: 640px; margin-inline: auto; width: 100%; padding-block: 24px; display: grid; gap: 22px; overflow: auto; height: 100%; align-content: start; }
  h1 { margin: 0; font-family: var(--font-score); font-weight: 800; font-size: 44px; text-transform: uppercase; letter-spacing: 0.02em; }
  section { display: grid; gap: 8px; }
  input:not([type]) { background: var(--bg-dark); border: 1px solid var(--line); border-radius: var(--r); padding: 9px 12px; outline: none; }
  input:not([type]):focus { border-color: var(--accent); }
  .drills { display: grid; gap: 6px; }
  .drills label { display: grid; grid-template-columns: auto 1fr; column-gap: 10px; align-items: baseline; border: 1px solid var(--line); border-radius: var(--r); padding: 8px 12px; cursor: pointer; }
  .drills input { accent-color: var(--accent); margin: 0; grid-row: span 2; align-self: center; }
  .dname { font-weight: 700; }
  .desc { color: var(--muted); font-size: 12px; }
  .drills label:has(input:checked) { border-color: var(--accent); background: var(--bg-light); }
  .drills label:has(input:focus-visible), .seg label:has(input:focus-visible) { outline: 2px solid var(--accent); outline-offset: 2px; }
  .seg { display: flex; flex-wrap: wrap; gap: 6px; }
  .seg label { border: 1px solid var(--line); border-radius: var(--r); padding: 6px 12px; cursor: pointer; display: flex; gap: 6px; align-items: center; font-variant-numeric: tabular-nums; }
  .seg input { accent-color: var(--accent); margin: 0; }
  .seg label:has(input:checked) { border-color: var(--accent); background: var(--bg-light); }
  .actions { display: flex; flex-wrap: wrap; align-items: center; gap: 14px; }
  .btn { background: var(--accent); color: var(--bg); border: 0; border-radius: var(--r); padding: 8px 20px; font-weight: 700; cursor: pointer; }
  .btn:disabled { opacity: 0.4; cursor: default; }
  .hint { color: var(--muted); font-size: 11px; }
</style>
