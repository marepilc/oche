<script lang="ts">
  import Board from '$lib/board/Board.svelte';
  import { checkouts } from '$lib/core/checkout';
  import { dartLabel, routeLabel } from '$lib/core/dart';
  import { drillStats } from '$lib/core/drills';
  import { preview } from '$lib/core/input';
  import { autosave } from '$lib/db.svelte';
  import { decimal, t } from '$lib/i18n/index.svelte';
  import type { MessageKey } from '$lib/i18n/types';
  import type { Drill } from '$lib/match.svelte';
  import { ThrowInput } from '$lib/throwInput.svelte';

  interface Props {
    drill: Drill;
    /** Called when the finished drill is acknowledged. */
    ondone: () => void;
  }
  let { drill, ondone }: Props = $props();

  const cmd = new ThrowInput(() => drill);
  autosave(() => drill.record, (e) => cmd.showError('game.saveError', { error: String(e) }));

  const s = $derived(drill.state);
  const kind = $derived(drill.settings.kind);
  const st = $derived(drillStats(s));
  const thrown = $derived(s.turn?.darts ?? []);
  const ghost = $derived(s.turn ? null : s.lastTurn);
  const dartsLeft = $derived(3 - thrown.length);
  const routes = $derived(kind === 'checkout' && !s.complete && !s.done ? checkouts(s.score, dartsLeft) : []);
  /** Confirmed turns, newest first. */
  const history = $derived(
    s.attempts
      .flatMap((a) => a.turns)
      .filter((turn) => turn !== s.turn)
      .reverse(),
  );
  /** Finished checkout attempts, newest first. */
  const finished = $derived(s.attempts.filter((a) => a.success !== null).reverse());
  const pct = (a: number, b: number) => (b ? `${Math.round((a / b) * 100)}%` : '—');

  const scoreLabel: Record<string, MessageKey> = { checkout: 'drill.left', scoring: 'drill.total', bobs27: 'drill.score' };

  function onkeydown(e: KeyboardEvent) {
    if (s.done && !e.ctrlKey && !e.altKey && !e.metaKey) {
      if (e.key === 'Enter') {
        e.preventDefault();
        ondone();
      } else if (e.key === 'Backspace') {
        e.preventDefault();
        drill.undo();
      }
      return;
    }
    cmd.key(e);
  }

  const hint = $derived.by(() => {
    if (s.complete) return t(s.turn?.bust ? 'game.confirmBust' : 'game.confirmTurn');
    const n = Number(cmd.input.buf);
    if (!cmd.input.buf) return t('input.dartOf', { n: thrown.length + 1 }) + (thrown.length ? `   ·   ${t('input.confirmHint')}` : '');
    if (n === 1) return `${t('input.single')}   ·   ${t('input.grow1')}`;
    if (n === 2) return `${t('input.single')}   ·   ${t('input.grow2')}`;
    return t('input.single');
  });
</script>

<svelte:window {onkeydown} />

<div class="game">
  <section class="info" aria-label={t(`training.${kind}` as MessageKey)}>
    <div class="label">
      {t(`training.${kind}` as MessageKey)} ·
      {t(kind === 'checkout' ? 'drill.attempt' : 'drill.round', { n: s.round, total: s.rounds })}
    </div>
    <div class="card">
      <div class="head"><span class="name">{drill.player.name}</span></div>
      <div class="label">{t(scoreLabel[kind])}</div>
      <div class="rem" class:neg={s.score <= 0 && kind === 'bobs27'}>{s.score}</div>
    </div>

    <div class="stats">
      {#if kind === 'checkout'}
        <div><span class="label">{t('drill.finished')}</span><b>{st.successes} / {st.attempts}</b></div>
        <div><span class="label">{t('drill.rate')}</span><b>{pct(st.successes, st.attempts)}</b></div>
      {:else if kind === 'scoring'}
        <div><span class="label">{t('drill.average')}</span><b>{decimal(st.average)}</b></div>
        <div><span class="label">{t('drill.hits')}</span><b>{pct(st.hits, st.darts)}</b></div>
        <div>
          <span class="label">{t(drill.settings.kind === 'scoring' && drill.settings.segment === 25 ? 'drill.bulls' : 'drill.trebles')}</span>
          <b>{st.bigHits} · {pct(st.bigHits, st.darts)}</b>
        </div>
      {:else}
        <div><span class="label">{t('drill.doubles')}</span><b>{st.doubles} · {pct(st.doubles, st.darts)}</b></div>
      {/if}
      <div><span class="label">{t('drill.darts')}</span><b>{st.darts}</b></div>
    </div>

    {#if kind === 'checkout'}
      <div class="history">
        {#each finished as a, k (k)}
          <span class="chip" class:ok={a.success} class:bust={!a.success}>{a.target}</span>
        {/each}
      </div>
    {:else}
      <div class="history">
        {#each history.slice(0, 30) as turn, k (k)}
          <span class="chip" class:ok={turn.scored > 0} class:bust={turn.scored < 0}>{turn.target} {turn.scored > 0 && kind === 'bobs27' ? '+' : ''}{turn.scored}</span>
        {/each}
      </div>
    {/if}
  </section>

  <section class="board-wrap">
    <Board preview={preview(cmd.input) ?? s.aimArea} darts={thrown} onpick={(d) => cmd.pick(d)} />
  </section>

  <aside class="side">
    {#if !s.done}
      <div class="aim">
        <span class="label">{t('drill.aim')}</span>
        <span class="target">{s.aim}</span>
      </div>
    {/if}
    <div class="slots">
      {#each [0, 1, 2] as i (i)}
        {@const d = thrown[i] ?? ghost?.darts[i]}
        <div class="slot" class:filled={!!thrown[i]} class:ghost={!thrown[i] && !!d}>
          {#if d}
            <span class="s-label">{dartLabel(d)}</span>
            <span class="s-pts">{t('game.points', { n: d.points })}</span>
          {:else}
            <span class="s-pts">{t('game.dartN', { n: i + 1 })}</span>
          {/if}
        </div>
      {/each}
    </div>
    {#if ghost}
      <div class="prev">
        {t('drill.visit', { target: ghost.target })}:
        {#if ghost.bust}<span class="bust">{t('game.bust')}</span>
        {:else if ghost.checkout}{t('game.checkedOut', { n: ghost.scoreBefore })}
        {:else}{ghost.scored}{/if}
      </div>
    {/if}
    {#if s.complete}
      <div class="confirm" class:bust={s.turn?.bust}>{hint}</div>
    {/if}
    {#if kind === 'checkout' && !s.complete && !s.done}
      <div class="checkout">
        <span class="label">{t('game.checkout', { score: s.score, darts: t('game.dartsLeft', { count: dartsLeft }) })}</span>
        {#if routes.length}
          <div class="route">{#each routes[0] as d, k (k)}<span>{routeLabel(d)}</span>{/each}</div>
        {:else}
          <div class="none">{t('game.noFinish')}</div>
        {/if}
      </div>
    {/if}
  </aside>

  <footer class="cmd">
    <div class="prompt"><span class="p">›</span>{cmd.input.buf}<span class="caret"></span></div>
    {#if cmd.error}
      <div class="err">{t(cmd.error.key, cmd.error.params)}</div>
    {:else}
      <div class="hint" class:attn={s.complete}>{hint}</div>
    {/if}
    <div class="keys">
      <span><kbd>⏎</kbd> {t('game.keyConfirm')}</span>
      <span><kbd>⌫</kbd> {t('game.keyUndo')}</span>
      <span><kbd>m</kbd> {t('game.keyMiss')}</span>
      <span><kbd>?</kbd> {t('game.keyHelp')}</span>
    </div>
  </footer>
</div>

{#if s.done}
  <div class="banner">
    <div class="card-done">
      <div class="t">
        {#if kind === 'bobs27' && s.attempts[0].success === false}
          {t('drill.failed', { score: s.score, target: s.lastTurn?.target ?? '' })}
        {:else}
          {t('drill.done')}
        {/if}
      </div>
      <div>
        {#if kind === 'checkout'}
          {t('drill.finished')}: {st.successes} / {st.attempts} · {pct(st.successes, st.attempts)}
        {:else if kind === 'scoring'}
          {t('drill.total')}: {s.score} · {t('drill.average')}: {decimal(st.average)}
        {:else}
          {t('drill.score')}: {s.score} · {t('drill.doubles')}: {st.doubles}
        {/if}
      </div>
      <div class="keys">{t('drill.doneHint')}</div>
    </div>
  </div>
{/if}

<style>
  .game {
    height: 100%;
    display: grid;
    grid-template-columns: minmax(220px, 1fr) minmax(0, 1.7fr) minmax(220px, 1fr);
    grid-template-rows: minmax(0, 1fr) auto;
    grid-template-areas: 'info board side' 'cmd cmd cmd';
    column-gap: 24px;
  }
  @media (max-width: 1000px) {
    .game {
      grid-template-columns: minmax(220px, 1fr) minmax(0, 1.4fr);
      grid-template-rows: auto minmax(0, 1fr) auto;
      grid-template-areas: 'info board' 'side board' 'cmd cmd';
    }
  }

  .info { grid-area: info; display: flex; flex-direction: column; gap: 12px; padding-block: 16px; overflow: auto; }
  .card { padding: 12px 14px; border-radius: var(--r); border: 1px solid color-mix(in oklab, var(--accent) 60%, transparent); background: var(--bg-light); display: grid; gap: 2px; }
  .head { display: flex; justify-content: space-between; }
  .name { font-weight: 700; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .rem { font-family: var(--font-score); font-weight: 800; font-size: clamp(48px, 7vh, 80px); line-height: 0.9; color: var(--accent); font-variant-numeric: tabular-nums; }
  .rem.neg { color: var(--red); }
  .stats { display: grid; grid-template-columns: 1fr 1fr; gap: 8px 12px; font-variant-numeric: tabular-nums; }
  .stats div { display: grid; }
  .stats b { font-size: 16px; font-weight: 500; }
  .history { display: flex; flex-wrap: wrap; gap: 4px; }
  .chip { font-size: 11px; padding: 1px 6px; border-radius: 4px; background: var(--bg-dark); color: var(--muted); font-variant-numeric: tabular-nums; }
  .chip.ok { color: var(--fg); }
  .chip.bust { color: var(--red); }

  .board-wrap { grid-area: board; min-height: 0; display: grid; place-items: center; padding-block: 16px; }

  .side { grid-area: side; display: flex; flex-direction: column; gap: 14px; padding-block: 16px; overflow: auto; }
  .aim { display: flex; justify-content: space-between; align-items: baseline; }
  .target { font-family: var(--font-score); font-weight: 800; font-size: 36px; color: var(--accent); }
  .slots { display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px; }
  .slot { border: 1px dashed var(--line); border-radius: var(--r); padding: 8px 6px; text-align: center; min-height: 58px; display: grid; align-content: center; }
  .slot.filled { border-style: solid; background: var(--bg-light); }
  .slot.ghost { opacity: 0.45; }
  .s-label { font-weight: 700; font-size: 15px; }
  .s-pts { color: var(--muted); font-size: 11px; }
  .prev { font-size: 11px; color: var(--muted); }
  .bust { color: var(--red); }
  .confirm { padding: 8px 12px; border-radius: var(--r); border: 1px solid var(--accent); color: var(--accent); font-size: 12px; }
  .confirm.bust { border-color: var(--red); color: var(--red); }
  .checkout { padding: 10px 12px; border-radius: var(--r); background: var(--bg-dark); display: grid; gap: 6px; }
  .route { display: flex; gap: 6px; flex-wrap: wrap; }
  .route span { padding: 2px 8px; border-radius: 4px; border: 1px solid color-mix(in oklab, var(--accent) 50%, transparent); color: var(--accent); font-weight: 700; }
  .none { color: var(--muted); }

  .cmd { grid-area: cmd; background: var(--bg-dark); border-top: 1px solid var(--line); margin-inline: -16px; padding: 10px 16px; display: flex; flex-wrap: wrap; align-items: center; gap: 6px 20px; }
  .prompt { font-size: 22px; font-weight: 700; min-width: 8ch; display: flex; align-items: center; gap: 8px; }
  .p { color: var(--accent); }
  .caret { display: inline-block; width: 0.55em; height: 1.1em; background: var(--accent); animation: blink 1s steps(1) infinite; }
  @keyframes blink { 50% { opacity: 0; } }
  .hint { color: var(--muted); white-space: pre; flex: 1; }
  .hint.attn { color: var(--accent); }
  .err { color: var(--red); flex: 1; }
  .keys { display: flex; flex-wrap: wrap; gap: 4px 14px; color: var(--muted); font-size: 11px; }

  .banner { position: fixed; inset: 0; display: grid; place-items: center; background: color-mix(in oklab, var(--bg) 70%, transparent); backdrop-filter: blur(3px); z-index: 5; }
  .card-done { background: var(--bg-light); border: 1px solid var(--accent); border-radius: 10px; padding: 24px 32px; text-align: center; display: grid; gap: 10px; justify-items: center; }
  .card-done .t { font-family: var(--font-score); font-weight: 800; font-size: 44px; color: var(--accent); max-width: 20ch; text-wrap: balance; }
</style>
