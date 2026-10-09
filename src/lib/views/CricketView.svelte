<script lang="ts">
  import Board from '$lib/board/Board.svelte';
  import { CRICKET_TARGETS, cricketHit, cricketStats } from '$lib/core/cricket';
  import { preview } from '$lib/core/input';
  import { autosave } from '$lib/db.svelte';
  import { decimal, t } from '$lib/i18n/index.svelte';
  import type { CricketMatch } from '$lib/match.svelte';
  import PlayLayout from '$lib/play/PlayLayout.svelte';
  import Slots from '$lib/play/Slots.svelte';
  import { ThrowInput } from '$lib/throwInput.svelte';

  interface Props {
    match: CricketMatch;
    ondone: () => void;
  }
  let { match, ondone }: Props = $props();

  const cmd = new ThrowInput(() => match);
  autosave(() => match.record, (e) => cmd.showError('game.saveError', { error: String(e) }));

  const g = $derived(match.state);
  const thrown = $derived(g.turn?.darts ?? []);
  const ghost = $derived(g.turn ? null : g.lastTurn);
  const turnMarks = $derived(g.turn?.marks.reduce((a, m) => a + m, 0) ?? 0);
  const current = $derived(cricketStats(g, g.current));
  const pendingWin = $derived(g.complete && g.turn?.checkout ? g.turn : null);
  const hint = $derived(cmd.hint(thrown.length, g.complete));
  const closedByAll = (k: number) => g.marks.every((m) => m[k] >= 3);
  const markGlyph = (m: number) => (m <= 0 ? '' : m === 1 ? '/' : m === 2 ? '✕' : 'Ⓧ');
  const targetLabel = (n: number) => (n === 25 ? 'B' : String(n));
  const detail = (d: (typeof thrown)[number]) => {
    const hit = cricketHit(d);
    return hit ? t('cricket.turnMarks', { count: hit.marks }) : t('cricket.noMark');
  };
</script>

<svelte:window onkeydown={(e) => cmd.gameKey(e, match.over, ondone)} />

<PlayLayout
  {cmd}
  {hint}
  attn={g.complete}
  label={t('cricket.summary', {
    ct: match.settings.cutThroat ? t('cricket.ct') : '',
    leg: g.legs.length,
    legs: match.settings.legsToWin,
  })}
  banner={pendingWin || g.winner !== null ? banner : undefined}
>
  {#snippet left()}
    <table class="board-table" style:--cols={match.players.length}>
      <thead>
        <tr>
          <th></th>
          {#each match.players as p, i (p.id)}
            <th class:active={i === g.current && g.winner === null}>
              <span class="name">{p.name}</span>
              <span class="legs">{t('game.legs', { n: g.legsWon[i] })}</span>
            </th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each CRICKET_TARGETS as n, k (n)}
          <tr class:dead={closedByAll(k)}>
            <th class="num">{targetLabel(n)}</th>
            {#each match.players as p, i (p.id)}
              <td class:active={i === g.current && g.winner === null} class:closed={g.marks[i][k] >= 3}>{markGlyph(g.marks[i][k])}</td>
            {/each}
          </tr>
        {/each}
        <tr class="pts">
          <th>{t('cricket.pts')}</th>
          {#each match.players as p, i (p.id)}
            <td class:active={i === g.current && g.winner === null}>{g.points[i]}</td>
          {/each}
        </tr>
        <tr class="mpr">
          <th>{t('cricket.mpr')}</th>
          {#each match.players as p, i (p.id)}
            <td>{decimal(cricketStats(g, i).mpr)}</td>
          {/each}
        </tr>
      </tbody>
    </table>
  {/snippet}

  {#snippet board()}
    <Board preview={preview(cmd.input)} darts={thrown} onpick={(d) => cmd.pick(d)} />
  {/snippet}

  {#snippet side()}
    <div class="turn-head">
      <span class="label">{t('game.turn', { name: match.players[g.current].name })}</span>
      <span class="sum">{t('cricket.turnMarks', { count: turnMarks })}</span>
    </div>
    <Slots {thrown} ghost={ghost?.darts} {detail} />
    {#if ghost}
      <div class="prev">
        {t('game.previous', { name: match.players[ghost.player].name })}
        {t('cricket.turnMarks', { count: ghost.marks.reduce((a, m) => a + m, 0) })}{ghost.scored ? ` · +${ghost.scored}` : ''}
      </div>
    {/if}
    {#if g.complete}<div class="confirm">{hint}</div>{/if}
    <div class="stats">
      <div><span class="label">{t('cricket.mpr')}</span><b>{decimal(current.mpr)}</b></div>
      <div><span class="label">{t('cricket.marks')}</span><b>{current.marks}</b></div>
      <div><span class="label">{t('cricket.darts')}</span><b>{current.darts}</b></div>
      <div><span class="label">{t('cricket.pts')}</span><b>{g.points[g.current]}</b></div>
    </div>
  {/snippet}
</PlayLayout>

{#snippet banner()}
  {#if g.winner !== null}
    <div class="t">{t('game.winsMatch', { name: match.players[g.winner].name })}</div>
    <div>{t('game.legsScore', { score: g.legsWon.join(' : ') })}</div>
    <div class="keys">{t('game.newGameHint')}</div>
  {:else if pendingWin}
    <div class="t">{t('cricket.closes', { name: match.players[pendingWin.player].name })}</div>
    <div class="keys">{t('game.confirmCheckoutHint')}</div>
  {/if}
{/snippet}

<style>
  .board-table { border-collapse: collapse; width: 100%; font-variant-numeric: tabular-nums; table-layout: fixed; }
  .board-table th, .board-table td { padding: 4px 6px; text-align: center; border-bottom: 1px solid var(--line); }
  thead th { vertical-align: bottom; }
  thead th:first-child, tbody th { width: 3.2em; }
  .name { display: block; font-weight: 700; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .legs { display: block; color: var(--muted); font-size: 10px; font-weight: 400; }
  .num { font-family: var(--font-score); font-weight: 800; font-size: 22px; color: var(--fg); }
  td { font-size: 22px; font-weight: 700; color: var(--muted); }
  td.closed { color: var(--accent); }
  .active { background: var(--bg-light); }
  thead th.active .name { color: var(--accent); }
  tr.dead .num, tr.dead td { opacity: 0.35; }
  .pts td { font-family: var(--font-score); font-size: 30px; color: var(--fg); }
  .pts th, .mpr th { font-size: 10px; text-transform: uppercase; letter-spacing: 0.1em; color: var(--muted); font-weight: 400; }
  .mpr td { font-size: 12px; font-weight: 400; }

  .turn-head { display: flex; justify-content: space-between; align-items: baseline; }
  .sum { font-family: var(--font-score); font-weight: 800; font-size: 28px; }
  .stats { display: grid; grid-template-columns: 1fr 1fr; gap: 8px 12px; font-variant-numeric: tabular-nums; }
  .stats div { display: grid; }
  .stats b { font-size: 16px; font-weight: 500; }
</style>
