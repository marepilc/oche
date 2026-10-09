<script lang="ts">
  import Board from '$lib/board/Board.svelte';
  import { dart, dartLabel, type Ring } from '$lib/core/dart';
  import type { DrillKind } from '$lib/core/drills';
  import type { Player } from '$lib/core/x01';
  import { hasBackend, playerStats, players as loadPlayers, type GameSummary, type PlayerStats } from '$lib/db.svelte';
  import { decimal, i18n, t } from '$lib/i18n/index.svelte';
  import type { MessageKey } from '$lib/i18n/types';
  import Bars from '$lib/stats/Bars.svelte';
  import Trend from '$lib/stats/Trend.svelte';

  const KEY = 'oche:stats-player';
  let players = $state<Player[]>([]);
  let current = $state<string | null>(null);
  let stats = $state<PlayerStats | null>(null);
  let error = $state('');

  loadPlayers()
    .then((p) => {
      players = p;
      let saved: string | null = null;
      try {
        saved = localStorage.getItem(KEY);
      } catch {
        /* no storage */
      }
      current = p.find((x) => x.id === saved)?.id ?? p[0]?.id ?? null;
    })
    .catch((e) => (error = String(e)));

  $effect(() => {
    const id = current;
    if (!id) return;
    try {
      localStorage.setItem(KEY, id);
    } catch {
      /* not critical */
    }
    playerStats(id)
      .then((s) => {
        if (current === id) stats = s;
      })
      .catch((e) => (error = String(e)));
  });

  function onkeydown(e: KeyboardEvent) {
    if (e.ctrlKey || e.altKey || e.metaKey || players.length < 2) return;
    const i = players.findIndex((p) => p.id === current);
    if (e.key === 'ArrowRight' || e.key === 'l') current = players[(i + 1) % players.length].id;
    else if (e.key === 'ArrowLeft' || e.key === 'h') current = players[(i - 1 + players.length) % players.length].id;
    else return;
    e.preventDefault();
  }

  const avg = (scored: number, darts: number) => (darts ? (scored / darts) * 3 : 0);
  const pct = (a: number, b: number) =>
    b ? new Intl.NumberFormat(i18n.locale, { style: 'percent', maximumFractionDigits: 0 }).format(a / b) : '—';
  const day = (iso: string) => new Intl.DateTimeFormat(i18n.locale, { dateStyle: 'medium' }).format(new Date(iso));

  const trend = $derived(
    (stats?.timeline ?? []).map((p) => ({
      value: p.average,
      label: t('stats.point', { date: day(p.startedAt), avg: decimal(p.average), f9: decimal(p.first9) }),
    })),
  );

  const heat = $derived.by(() => {
    const cells = stats?.heat ?? [];
    const total = cells.reduce((a, c) => a + c.count, 0);
    const counts = new Map<string, number>();
    const add = (k: string, n: number) => counts.set(k, (counts.get(k) ?? 0) + n);
    let misses = 0;
    for (const c of cells) {
      if (c.ring === 'miss') misses += c.count;
      // A single without its area is drawn half in each single bed.
      else if (c.ring === 'single') {
        add(`${c.segment}:inner`, c.count / 2);
        add(`${c.segment}:outer`, c.count / 2);
      } else add(`${c.segment}:${c.ring}`, c.count);
    }
    const max = Math.max(1, ...counts.values());
    const map = new Map<string, { value: number; label: string }>();
    for (const [k, n] of counts) {
      const [seg, ring] = k.split(':');
      const shown = Math.round(n * 10) / 10;
      map.set(k, {
        value: n / max,
        label: t('stats.heatLabel', { area: dartLabel(dart(Number(seg), ring as Ring)), n: shown, pct: pct(n, total) }),
      });
    }
    return { map, total, misses };
  });

  /** The number a drill session is judged by. */
  function drillValue(g: GameSummary): number {
    const p = g.players[0];
    if (g.mode === 'checkout') return g.legs ? (p.legsWon / g.legs) * 100 : 0;
    if (g.mode === 'scoring') return avg(p.scored, p.darts);
    return 27 + p.scored;
  }
  const drillText = (kind: string, v: number) =>
    kind === 'checkout' ? `${Math.round(v)}%` : kind === 'scoring' ? decimal(v) : String(v);

  const drills = $derived.by(() => {
    const out: { kind: DrillKind; sessions: GameSummary[]; values: number[] }[] = [];
    for (const kind of ['checkout', 'scoring', 'bobs27'] as DrillKind[]) {
      const sessions = (stats?.training ?? []).filter((g) => g.mode === kind && g.players[0]?.darts).reverse();
      if (sessions.length) out.push({ kind, sessions, values: sessions.map(drillValue) });
    }
    return out;
  });
</script>

<svelte:window {onkeydown} />

<div class="wrap">
  <h1>{t('stats.title')}</h1>

  {#if !hasBackend}
    <p class="hint">{t('history.noBackend')}</p>
  {:else if error}
    <p class="err">{error}</p>
  {:else if !players.length}
    <p class="hint">{t('stats.noPlayers')}</p>
  {:else}
    <div class="who" role="tablist">
      {#each players as p (p.id)}
        <button role="tab" aria-selected={p.id === current} class:on={p.id === current} onclick={() => (current = p.id)}>{p.name}</button>
      {/each}
      {#if players.length > 1}
        <span class="hint">
          {#each t('stats.keys').split(/(\{\w+\})/) as part, k (k)}
            {#if part === '{lr}'}<kbd>←</kbd> <kbd>→</kbd>{:else}{part}{/if}
          {/each}
        </span>
      {/if}
    </div>
  {/if}

  {#if stats}
    <section>
      <div class="label">{t('stats.x01')}</div>
      <div class="tiles">
        <div class="tile hero"><span class="label">{t('stats.average')}</span><b>{decimal(avg(stats.scored, stats.darts))}</b></div>
        <div class="tile"><span class="label">{t('stats.first9')}</span><b>{decimal(avg(stats.first9Scored, stats.first9Darts))}</b></div>
        <div class="tile">
          <span class="label">{t('stats.checkout')}</span>
          <b>{pct(stats.checkouts, stats.checkoutChances)}</b>
          <small>{stats.checkouts} / {stats.checkoutChances}</small>
        </div>
        <div class="tile"><span class="label">{t('stats.bestCheckout')}</span><b>{stats.bestCheckout || '—'}</b></div>
        <div class="tile">
          <span class="label">{t('stats.bestLeg')}</span>
          <b>{stats.bestLeg ? t('stats.darts', { count: stats.bestLeg }) : '—'}</b>
        </div>
        <div class="tile"><span class="label">{t('stats.legs')}</span><b>{stats.legsWon} / {stats.legsPlayed}</b></div>
        <div class="tile"><span class="label">180</span><b>{stats.n180}</b></div>
        <div class="tile"><span class="label">140+</span><b>{stats.n140}</b></div>
        <div class="tile"><span class="label">100+</span><b>{stats.n100}</b></div>
      </div>
    </section>

    {#if stats.cricket.games || stats.atc.games}
      <div class="split">
        {#if stats.cricket.games}
          <section>
            <div class="label">{t('newGame.cricket')} · {t('stats.games', { count: stats.cricket.games })}</div>
            <div class="tiles">
              <div class="tile"><span class="label">{t('stats.mpr')}</span><b>{decimal(stats.cricket.darts ? (stats.cricket.marks / stats.cricket.darts) * 3 : 0)}</b></div>
              <div class="tile"><span class="label">{t('stats.legs')}</span><b>{stats.cricket.legsWon} / {stats.cricket.legsPlayed}</b></div>
            </div>
          </section>
        {/if}
        {#if stats.atc.games}
          <section>
            <div class="label">{t('newGame.atc')} · {t('stats.games', { count: stats.atc.games })}</div>
            <div class="tiles">
              <div class="tile"><span class="label">{t('stats.bestLeg')}</span><b>{stats.atc.bestLeg ? t('stats.darts', { count: stats.atc.bestLeg }) : '—'}</b></div>
              <div class="tile"><span class="label">{t('stats.legs')}</span><b>{stats.atc.legsWon} / {stats.atc.legsPlayed}</b></div>
            </div>
          </section>
        {/if}
      </div>
    {/if}

    <section>
      <div class="label">{t('stats.trend')}</div>
      {#if trend.length}
        <Trend points={trend} ariaLabel={t('stats.trend')} />
      {:else}
        <p class="hint">{t('stats.trendEmpty')}</p>
      {/if}
    </section>

    <div class="split">
      <section>
        <div class="label">{t('stats.heat')}</div>
        {#if heat.total}
          <div class="heat"><Board heat={heat.map} /></div>
          <div class="hint">{t('stats.misses', { n: heat.misses, pct: pct(heat.misses, heat.total), total: heat.total })}</div>
        {:else}
          <p class="hint">{t('stats.heatEmpty')}</p>
        {/if}
      </section>

      {#if drills.length}
        <section>
          <div class="label">{t('stats.training')}</div>
          {#each drills as d (d.kind)}
            {@const best = Math.max(...d.values)}
            {@const mean = d.values.reduce((a, v) => a + v, 0) / d.values.length}
            <div class="drill">
              <div class="dhead">
                <b>{t(`training.${d.kind}` as MessageKey)}</b>
                <span class="hint">{t('stats.sessions', { count: d.sessions.length })}</span>
              </div>
              <div class="dnums">
                <span>{t('stats.last', { v: drillText(d.kind, d.values[d.values.length - 1]) })}</span>
                <span>{t('stats.best', { v: drillText(d.kind, best) })}</span>
                <span>{t('stats.mean', { v: drillText(d.kind, mean) })}</span>
              </div>
              <Bars
                values={d.sessions.slice(-24).map((g, i, recent) => {
                  const v = d.values[d.values.length - recent.length + i];
                  return { value: v, label: t('stats.session', { date: day(g.startedAt), v: drillText(d.kind, v) }) };
                })}
                max={d.kind === 'checkout' ? 100 : undefined}
                ariaLabel={t(`training.${d.kind}` as MessageKey)}
              />
            </div>
          {/each}
        </section>
      {/if}
    </div>
  {/if}
</div>

<style>
  .wrap { max-width: 900px; margin-inline: auto; width: 100%; padding-block: 24px; display: grid; gap: 22px; overflow: auto; height: 100%; align-content: start; }
  h1 { margin: 0; font-family: var(--font-score); font-weight: 800; font-size: 44px; text-transform: uppercase; letter-spacing: 0.02em; }
  section { display: grid; gap: 8px; align-content: start; min-width: 0; }
  .who { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
  .who button { background: none; border: 1px solid var(--line); border-radius: var(--r); padding: 4px 12px; cursor: pointer; color: var(--muted); }
  .who button.on { border-color: var(--accent); color: var(--fg); background: var(--bg-light); }
  .tiles { display: grid; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: 8px; }
  .tile { border: 1px solid var(--line); border-radius: var(--r); padding: 8px 12px; display: grid; gap: 2px; font-variant-numeric: tabular-nums; }
  .tile b { font-size: 20px; font-weight: 600; }
  .tile.hero b { font-family: var(--font-score); font-weight: 800; font-size: 36px; line-height: 1; color: var(--accent); }
  .tile small { color: var(--muted); font-size: 11px; }
  .split { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 24px; }
  .heat { width: min(100%, 360px); aspect-ratio: 1; }
  .drill { border: 1px solid var(--line); border-radius: var(--r); padding: 8px 12px; display: grid; gap: 6px; }
  .dhead { display: flex; justify-content: space-between; align-items: baseline; }
  .dnums { display: flex; flex-wrap: wrap; gap: 4px 14px; font-size: 12px; font-variant-numeric: tabular-nums; }
  .hint { color: var(--muted); font-size: 11px; }
  .err { color: var(--red); font-size: 12px; }
</style>
