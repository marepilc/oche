<script lang="ts">
  import Board from '$lib/board/Board.svelte';
  import { checkouts } from '$lib/core/checkout';
  import { dartLabel, routeLabel, type Dart } from '$lib/core/dart';
  import { preview } from '$lib/core/input';
  import { playerStats } from '$lib/core/x01';
  import { autosave } from '$lib/db.svelte';
  import { decimal, t } from '$lib/i18n/index.svelte';
  import type { Match } from '$lib/match.svelte';
  import PlayLayout from '$lib/play/PlayLayout.svelte';
  import Slots from '$lib/play/Slots.svelte';
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
  const hint = $derived(cmd.hint(thrown.length, g.complete, g.turn?.bust));
</script>

<svelte:window onkeydown={(e) => cmd.gameKey(e, match.over, ondone)} />

<PlayLayout
  {cmd}
  {hint}
  attn={g.complete}
  label={t('game.summary', {
    start: match.settings.start,
    in: match.settings.doubleIn ? t('game.doubleIn') : '',
    out: t(match.settings.doubleOut ? 'game.doubleOut' : 'game.singleOut'),
    leg: g.legs.length,
    legs: match.settings.legsToWin,
  })}
  banner={pendingCheckout || g.winner !== null ? banner : undefined}
>
  {#snippet left()}
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
  {/snippet}

  {#snippet board()}
    <Board preview={preview(cmd.input)} darts={thrown} onpick={(d) => cmd.pick(d)} />
  {/snippet}

  {#snippet side()}
    <div class="turn-head">
      <span class="label">{t('game.turn', { name: match.players[g.current].name })}</span>
      <span class="sum" class:bust={g.turn?.bust}>{turnSum}</span>
    </div>
    <Slots {thrown} ghost={ghost?.darts} />
    {#if ghost}
      <div class="prev">
        {t('game.previous', { name: match.players[ghost.player].name })}
        {#if ghost.bust}<span class="bust">{t('game.bust')}</span>{:else if ghost.checkout}{t('game.checkedOut', { n: ghost.scored })}{:else}{ghost.scored}{/if}
      </div>
    {/if}
    {#if g.complete}
      <div class="confirm" class:bust={g.turn?.bust}>{hint}</div>
    {:else}
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
  {/snippet}
</PlayLayout>

{#snippet banner()}
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
{/snippet}

<style>
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
  .chip.bust { text-decoration: line-through; }
  .finish { font-size: 11px; color: var(--muted); }

  .turn-head { display: flex; justify-content: space-between; align-items: baseline; }
  .sum { font-family: var(--font-score); font-weight: 800; font-size: 36px; font-variant-numeric: tabular-nums; }
  .sum.bust { color: var(--red); text-decoration: line-through; }
  .bust { color: var(--red); }
  .checkout { padding: 10px 12px; border-radius: var(--r); background: var(--bg-dark); display: grid; gap: 6px; }
  .route { display: flex; gap: 6px; flex-wrap: wrap; }
  .route span { padding: 2px 8px; border-radius: 4px; border: 1px solid color-mix(in oklab, var(--accent) 50%, transparent); color: var(--accent); font-weight: 700; }
  .none { color: var(--muted); }
  .alts { color: var(--muted); font-size: 12px; }
  .stats { display: grid; grid-template-columns: 1fr 1fr; gap: 8px 12px; font-variant-numeric: tabular-nums; }
  .stats div { display: grid; }
  .stats b { font-size: 16px; font-weight: 500; }
</style>
