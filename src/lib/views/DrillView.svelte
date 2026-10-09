<script lang="ts">
  import Board from '$lib/board/Board.svelte';
  import { checkouts } from '$lib/core/checkout';
  import { routeLabel } from '$lib/core/dart';
  import { drillStats } from '$lib/core/drills';
  import { preview } from '$lib/core/input';
  import { autosave } from '$lib/db.svelte';
  import { decimal, t } from '$lib/i18n/index.svelte';
  import type { MessageKey } from '$lib/i18n/types';
  import type { Drill } from '$lib/match.svelte';
  import PlayLayout from '$lib/play/PlayLayout.svelte';
  import Slots from '$lib/play/Slots.svelte';
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

  const hint = $derived(cmd.hint(thrown.length, s.complete, s.turn?.bust));
</script>

<svelte:window onkeydown={(e) => cmd.gameKey(e, drill.over, ondone)} />

<PlayLayout
  {cmd}
  {hint}
  attn={s.complete}
  label="{t(`training.${kind}` as MessageKey)} · {t(kind === 'checkout' ? 'drill.attempt' : 'drill.round', { n: s.round, total: s.rounds })}"
  banner={s.done ? banner : undefined}
>
  {#snippet left()}
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

    <div class="history">
      {#if kind === 'checkout'}
        {#each finished as a, k (k)}
          <span class="chip" class:hi={a.success} class:bust={!a.success}>{a.target}</span>
        {/each}
      {:else}
        {#each history.slice(0, 30) as turn, k (k)}
          <span class="chip" class:hi={turn.scored > 0} class:bust={turn.scored < 0}>{turn.target} {turn.scored > 0 && kind === 'bobs27' ? '+' : ''}{turn.scored}</span>
        {/each}
      {/if}
    </div>
  {/snippet}

  {#snippet board()}
    <Board preview={preview(cmd.input) ?? s.aimArea} darts={thrown} onpick={(d) => cmd.pick(d)} />
  {/snippet}

  {#snippet side()}
    {#if !s.done}
      <div class="aim">
        <span class="label">{t('drill.aim')}</span>
        <span class="target">{s.aim}</span>
      </div>
    {/if}
    <Slots {thrown} ghost={ghost?.darts} />
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
  {/snippet}
</PlayLayout>

{#snippet banner()}
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
{/snippet}

<style>
  .card { padding: 12px 14px; border-radius: var(--r); border: 1px solid color-mix(in oklab, var(--accent) 60%, transparent); background: var(--bg-light); display: grid; gap: 2px; }
  .head { display: flex; justify-content: space-between; }
  .name { font-weight: 700; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .rem { font-family: var(--font-score); font-weight: 800; font-size: clamp(48px, 7vh, 80px); line-height: 0.9; color: var(--accent); font-variant-numeric: tabular-nums; }
  .rem.neg { color: var(--red); }
  .stats { display: grid; grid-template-columns: 1fr 1fr; gap: 8px 12px; font-variant-numeric: tabular-nums; }
  .stats div { display: grid; }
  .stats b { font-size: 16px; font-weight: 500; }
  .history { display: flex; flex-wrap: wrap; gap: 4px; }
  .aim { display: flex; justify-content: space-between; align-items: baseline; }
  .target { font-family: var(--font-score); font-weight: 800; font-size: 36px; color: var(--accent); }
  .bust { color: var(--red); }
  .checkout { padding: 10px 12px; border-radius: var(--r); background: var(--bg-dark); display: grid; gap: 6px; }
  .route { display: flex; gap: 6px; flex-wrap: wrap; }
  .route span { padding: 2px 8px; border-radius: 4px; border: 1px solid color-mix(in oklab, var(--accent) 50%, transparent); color: var(--accent); font-weight: 700; }
  .none { color: var(--muted); }
</style>
