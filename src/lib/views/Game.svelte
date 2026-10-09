<script lang="ts">
  import Board from '$lib/board/Board.svelte';
  import { checkouts } from '$lib/core/checkout';
  import { dartLabel, routeLabel, type Dart } from '$lib/core/dart';
  import { preview } from '$lib/core/input';
  import { playerStats } from '$lib/core/x01';
  import { autosave } from '$lib/db.svelte';
  import { decimal, t } from '$lib/i18n/index.svelte';
  import type { Match } from '$lib/match.svelte';
  import { ThrowInput } from '$lib/throwInput.svelte';

  interface Props {
    match: Match;
    /** Called when the finished match is acknowledged. */
    ondone: () => void;
  }
  let { match, ondone }: Props = $props();

  const cmd = new ThrowInput(() => match);
  autosave(() => match.record, (e) => cmd.showError('game.saveError', { error: String(e) }));

  const g = $derived(match.state);
  const solo = $derived(match.players.length === 1);
  const leg = $derived(g.legs[g.legs.length - 1]);
  const thrown = $derived(g.turn?.darts ?? []);
  const dartsLeft = $derived(3 - thrown.length);
  const routes = $derived(g.complete ? [] : checkouts(g.remaining[g.current], dartsLeft, match.settings.doubleOut));
  const turnSum = $derived(thrown.reduce((a, d) => a + d.points, 0));
  const ghost = $derived(g.turn ? null : g.lastTurn);
  /** Confirmed turns of the current leg. */
  const closedTurns = $derived(leg.turns.filter((turn) => turn !== g.turn));
  const current = $derived(playerStats(g, g.current));
  /** Finish for a player waiting for their turn (a full three darts). */
  const nextFinish = (i: number) => checkouts(g.remaining[i], 3, match.settings.doubleOut, 1)[0];
  const routeText = (r: Dart[]) => r.map(routeLabel).join(' ');
  const pendingCheckout = $derived(g.complete && g.turn?.checkout ? g.turn : null);

  function onkeydown(e: KeyboardEvent) {
    if (g.winner !== null && !e.ctrlKey && !e.altKey && !e.metaKey) {
      if (e.key === 'Enter') {
        e.preventDefault();
        ondone();
      } else if (e.key === 'Backspace') {
        e.preventDefault();
        match.undo();
      }
      return;
    }
    cmd.key(e);
  }

  const hint = $derived.by(() => {
    if (g.complete) return t(g.turn?.bust ? 'game.confirmBust' : 'game.confirmTurn');
    const n = Number(cmd.input.buf);
    if (!cmd.input.buf) return t('input.dartOf', { n: thrown.length + 1 }) + (thrown.length ? `   ·   ${t('input.confirmHint')}` : '');
    if (n === 1) return `${t('input.single')}   ·   ${t('input.grow1')}`;
    if (n === 2) return `${t('input.single')}   ·   ${t('input.grow2')}`;
    return t('input.single');
  });
</script>

<svelte:window {onkeydown} />

<div class="game">
  <section class="players" aria-label={t('game.players')}>
    <div class="label">
      {t('game.summary', {
        start: match.settings.start,
        in: match.settings.doubleIn ? t('game.doubleIn') : '',
        out: t(match.settings.doubleOut ? 'game.doubleOut' : 'game.singleOut'),
        leg: g.legs.length,
        legs: match.settings.legsToWin,
      })}
    </div>
    {#each match.players as player, i (player.id)}
      {@const st = playerStats(g, i)}
      <div class="player" class:active={i === g.current && g.winner === null}>
        <div class="head">
          <span class="name">{player.name}</span>
          <span class="legs">{t('game.legs', { n: g.legsWon[i] })}</span>
        </div>
        <div class="rem">{g.remaining[i]}</div>
        {#if i !== g.current && nextFinish(i)}
          <div class="finish">{t('game.finishes', { route: routeText(nextFinish(i)) })}</div>
        {/if}
        <div class="meta">
          <span>{t('game.avg')} <b>{decimal(st.average)}</b></span>
          <span>{t('game.darts')} <b>{st.darts}</b></span>
        </div>
        <div class="history">
          {#each closedTurns.filter((turn) => turn.player === i).slice(-6) as turn, k (k)}
            <span class="chip" class:hi={turn.scored >= 100} class:bust={turn.bust}>{turn.darts.reduce((a, d) => a + d.points, 0)}</span>
          {/each}
        </div>
      </div>
    {/each}
  </section>

  <section class="board-wrap">
    <Board preview={preview(cmd.input)} darts={thrown} onpick={(d) => cmd.pick(d)} />
  </section>

  <aside class="side">
    <div class="turn-head">
      <span class="label">{t('game.turn', { name: match.players[g.current].name })}</span>
      <span class="sum" class:bust={g.turn?.bust}>{turnSum}</span>
    </div>
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
        {t('game.previous', { name: match.players[ghost.player].name })}
        {#if ghost.bust}<span class="bust">{t('game.bust')}</span>{:else if ghost.checkout}{t('game.checkedOut', { n: ghost.scored })}{:else}{ghost.scored}{/if}
      </div>
    {/if}
    {#if g.complete}
      <div class="confirm" class:bust={g.turn?.bust}>{hint}</div>
    {/if}

    {#if !g.complete}
      <div class="checkout">
        <span class="label">{t('game.checkout', { score: g.remaining[g.current], darts: t('game.dartsLeft', { count: dartsLeft }) })}</span>
        {#if routes.length}
          <div class="route">{#each routes[0] as d, k (k)}<span>{routeLabel(d)}</span>{/each}</div>
          {#if routes.length > 1}
            <div class="alts">{t('game.or', { routes: routes.slice(1).map(routeText).join('  ·  ') })}</div>
          {/if}
        {:else}
          <div class="none">
            {g.remaining[g.current] > 170 ? t('game.notYet', { score: g.remaining[g.current] }) : t('game.noFinish')}
          </div>
        {/if}
      </div>
    {/if}

    <div class="stats">
      <div><span class="label">{t('game.first9')}</span><b>{decimal(current.first9)}</b></div>
      <div><span class="label">{t('game.bestTurn')}</span><b>{current.best}</b></div>
      <div><span class="label">{t('game.checkouts')}</span><b>{current.checkouts}</b></div>
      <div><span class="label">{t('game.average')}</span><b>{decimal(current.average)}</b></div>
    </div>
  </aside>

  <footer class="cmd">
    <div class="prompt"><span class="p">›</span>{cmd.input.buf}<span class="caret"></span></div>
    {#if cmd.error}
      <div class="err">{t(cmd.error.key, cmd.error.params)}</div>
    {:else}
      <div class="hint" class:attn={g.complete}>{hint}</div>
    {/if}
    <div class="keys">
      <span><kbd>⏎</kbd> {t('game.keyConfirm')}</span>
      <span><kbd>⌫</kbd> {t('game.keyUndo')}</span>
      <span><kbd>m</kbd> {t('game.keyMiss')}</span>
      <span><kbd>?</kbd> {t('game.keyHelp')}</span>
    </div>
  </footer>
</div>

{#if pendingCheckout || g.winner !== null}
  <div class="banner">
    <div class="card">
      {#if g.winner !== null && solo}
        <div class="t">{t('game.soloDone')}</div>
        <div>{t('game.soloSummary', { legs: t('game.legs', { n: g.legsWon[0] }), avg: decimal(current.average), darts: current.darts })}</div>
        <div class="keys">{t('game.newGameHint')}</div>
      {:else if g.winner !== null}
        <div class="t">{t('game.winsMatch', { name: match.players[g.winner].name })}</div>
        <div>{t('game.legsScore', { score: g.legsWon.join(' : ') })}</div>
        <div class="keys">{t('game.newGameHint')}</div>
      {:else if pendingCheckout}
        <div class="t">
          {t('game.confirmCheckout', {
            name: match.players[pendingCheckout.player].name,
            score: pendingCheckout.scoreBefore,
            dart: dartLabel(pendingCheckout.darts[pendingCheckout.darts.length - 1]),
          })}
        </div>
        <div class="keys">{t('game.confirmCheckoutHint')}</div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .game {
    height: 100%;
    display: grid;
    grid-template-columns: minmax(220px, 1fr) minmax(0, 1.7fr) minmax(220px, 1fr);
    grid-template-rows: minmax(0, 1fr) auto;
    grid-template-areas: 'players board side' 'cmd cmd cmd';
    column-gap: 24px;
  }
  @media (max-width: 1000px) {
    .game {
      grid-template-columns: minmax(220px, 1fr) minmax(0, 1.4fr);
      grid-template-rows: auto minmax(0, 1fr) auto;
      grid-template-areas: 'players board' 'side board' 'cmd cmd';
    }
  }

  .players { grid-area: players; display: flex; flex-direction: column; gap: 10px; padding-block: 16px; overflow: auto; }
  .player { padding: 12px 14px; border-radius: var(--r); border: 1px solid var(--line); display: grid; gap: 4px; transition: background 0.2s; }
  .player.active { background: var(--bg-light); border-color: color-mix(in oklab, var(--accent) 60%, transparent); }
  .head { display: flex; justify-content: space-between; align-items: baseline; gap: 8px; }
  .name { font-weight: 700; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .legs { color: var(--muted); font-size: 11px; }
  .rem { font-family: var(--font-score); font-weight: 800; font-size: clamp(48px, 7vh, 80px); line-height: 0.9; font-variant-numeric: tabular-nums; }
  .player.active .rem { color: var(--accent); }
  .meta { display: flex; gap: 14px; color: var(--muted); font-size: 11px; font-variant-numeric: tabular-nums; }
  .meta b { color: var(--fg); font-weight: 500; }
  .history { display: flex; flex-wrap: wrap; gap: 4px; min-height: 20px; }
  .chip { font-size: 11px; padding: 1px 6px; border-radius: 4px; background: var(--bg-dark); color: var(--muted); font-variant-numeric: tabular-nums; }
  .chip.hi { color: var(--fg); }
  .chip.bust { color: var(--red); text-decoration: line-through; }

  .board-wrap { grid-area: board; min-height: 0; display: grid; place-items: center; padding-block: 16px; }

  .side { grid-area: side; display: flex; flex-direction: column; gap: 14px; padding-block: 16px; overflow: auto; }
  .turn-head { display: flex; justify-content: space-between; align-items: baseline; }
  .sum { font-family: var(--font-score); font-weight: 800; font-size: 36px; font-variant-numeric: tabular-nums; }
  .slots { display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px; }
  .slot { border: 1px dashed var(--line); border-radius: var(--r); padding: 8px 6px; text-align: center; min-height: 58px; display: grid; align-content: center; }
  .slot.filled { border-style: solid; background: var(--bg-light); }
  .slot.ghost { opacity: 0.45; }
  .s-label { font-weight: 700; font-size: 15px; }
  .s-pts { color: var(--muted); font-size: 11px; }
  .prev { font-size: 11px; color: var(--muted); }
  .bust { color: var(--red); }
  .checkout { padding: 10px 12px; border-radius: var(--r); background: var(--bg-dark); display: grid; gap: 6px; }
  .route { display: flex; gap: 6px; flex-wrap: wrap; }
  .route span { padding: 2px 8px; border-radius: 4px; border: 1px solid color-mix(in oklab, var(--accent) 50%, transparent); color: var(--accent); font-weight: 700; }
  .none { color: var(--muted); }
  .alts { color: var(--muted); font-size: 12px; }
  .finish { font-size: 11px; color: var(--muted); }
  .stats { display: grid; grid-template-columns: 1fr 1fr; gap: 8px 12px; font-variant-numeric: tabular-nums; }
  .stats div { display: grid; }
  .stats b { font-size: 16px; font-weight: 500; }

  .cmd { grid-area: cmd; background: var(--bg-dark); border-top: 1px solid var(--line); margin-inline: -16px; padding: 10px 16px; display: flex; flex-wrap: wrap; align-items: center; gap: 6px 20px; }
  .prompt { font-size: 22px; font-weight: 700; min-width: 8ch; display: flex; align-items: center; gap: 8px; }
  .p { color: var(--accent); }
  .caret { display: inline-block; width: 0.55em; height: 1.1em; background: var(--accent); animation: blink 1s steps(1) infinite; }
  @keyframes blink { 50% { opacity: 0; } }
  .hint { color: var(--muted); white-space: pre; flex: 1; }
  .err { color: var(--red); flex: 1; }
  .hint.attn { color: var(--accent); }
  .confirm { padding: 8px 12px; border-radius: var(--r); border: 1px solid var(--accent); color: var(--accent); font-size: 12px; }
  .confirm.bust { border-color: var(--red); color: var(--red); }
  .sum.bust { color: var(--red); text-decoration: line-through; }
  .keys { display: flex; flex-wrap: wrap; gap: 4px 14px; color: var(--muted); font-size: 11px; }

  .banner { position: fixed; inset: 0; display: grid; place-items: center; background: color-mix(in oklab, var(--bg) 70%, transparent); backdrop-filter: blur(3px); z-index: 5; }
  .card { background: var(--bg-light); border: 1px solid var(--accent); border-radius: 10px; padding: 24px 32px; text-align: center; display: grid; gap: 10px; justify-items: center; }
  .card .t { font-family: var(--font-score); font-weight: 800; font-size: 44px; color: var(--accent); max-width: 20ch; text-wrap: balance; }
</style>
