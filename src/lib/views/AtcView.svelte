<script lang="ts">
  import Board from '$lib/board/Board.svelte';
  import { ATC_TARGETS, atcArea, atcLabel, atcStats } from '$lib/core/atc';
  import { preview } from '$lib/core/input';
  import { autosave } from '$lib/db.svelte';
  import { i18n, t } from '$lib/i18n/index.svelte';
  import type { AtcMatch } from '$lib/match.svelte';
  import PlayLayout from '$lib/play/PlayLayout.svelte';
  import Slots from '$lib/play/Slots.svelte';
  import { ThrowInput } from '$lib/throwInput.svelte';

  interface Props {
    match: AtcMatch;
    ondone: () => void;
  }
  let { match, ondone }: Props = $props();

  const cmd = new ThrowInput(() => match);
  autosave(() => match.record, (e) => cmd.showError('game.saveError', { error: String(e) }));

  const g = $derived(match.state);
  const thrown = $derived(g.turn?.darts ?? []);
  const ghost = $derived(g.turn ? null : g.lastTurn);
  const at = $derived(g.progress[g.current]);
  const pendingWin = $derived(g.complete && g.turn?.checkout ? g.turn : null);
  const hint = $derived(cmd.hint(thrown.length, g.complete));
  const pct = (v: number) => new Intl.NumberFormat(i18n.locale, { style: 'percent', maximumFractionDigits: 0 }).format(v);
</script>

<svelte:window onkeydown={(e) => cmd.gameKey(e, match.over, ondone)} />

<PlayLayout
  {cmd}
  {hint}
  attn={g.complete}
  label={t('atc.summary', { sk: match.settings.skips ? t('atc.sk') : '', leg: g.legs.length, legs: match.settings.legsToWin })}
  banner={pendingWin || g.winner !== null ? banner : undefined}
>
  {#snippet left()}
    {#each match.players as player, i (player.id)}
      {@const st = atcStats(g, i)}
      <div class="player" class:active={i === g.current && g.winner === null}>
        <div class="head">
          <span class="name">{player.name}</span>
          <span class="legs">{t('game.legs', { n: g.legsWon[i] })}</span>
        </div>
        <div class="rem">{atcLabel(g.progress[i])}</div>
        <div class="track" aria-hidden="true">
          {#each ATC_TARGETS as _, k (k)}<span class:done={k < g.progress[i]}></span>{/each}
        </div>
        <div class="meta">
          <span>{t('atc.rate')} <b>{st.darts ? pct(st.rate) : '—'}</b></span>
          <span>{t('atc.darts')} <b>{st.darts}</b></span>
        </div>
      </div>
    {/each}
  {/snippet}

  {#snippet board()}
    <Board preview={preview(cmd.input) ?? atcArea(at)} darts={thrown} onpick={(d) => cmd.pick(d)} />
  {/snippet}

  {#snippet side()}
    <div class="aim">
      <span class="label">{t('game.turn', { name: match.players[g.current].name })} · {t('atc.target')}</span>
      <span class="target">{atcLabel(at)}</span>
    </div>
    <Slots {thrown} ghost={ghost?.darts} detail={(_, i) => ((g.turn ?? ghost)?.hit[i] ? t('atc.hit') : t('atc.noHit'))} />
    {#if ghost}
      <div class="prev">
        {t('game.previous', { name: match.players[ghost.player].name })}
        {atcLabel(ghost.before)} → {atcLabel(ghost.before + ghost.advanced)}
      </div>
    {/if}
    {#if g.complete}<div class="confirm">{hint}</div>{/if}
  {/snippet}
</PlayLayout>

{#snippet banner()}
  {#if g.winner !== null}
    <div class="t">{match.players.length > 1 ? t('game.winsMatch', { name: match.players[g.winner].name }) : t('game.soloDone')}</div>
    <div>
      {match.players.length > 1 ? t('game.legsScore', { score: g.legsWon.join(' : ') }) : `${t('atc.darts')}: ${atcStats(g, 0).darts} · ${t('atc.rate')}: ${pct(atcStats(g, 0).rate)}`}
    </div>
    <div class="keys">{t('game.newGameHint')}</div>
  {:else if pendingWin}
    <div class="t">{t('atc.finishes', { name: match.players[pendingWin.player].name })}</div>
    <div class="keys">{t('game.confirmCheckoutHint')}</div>
  {/if}
{/snippet}

<style>
  .player { padding: 12px 14px; border-radius: var(--r); border: 1px solid var(--line); display: grid; gap: 6px; }
  .player.active { background: var(--bg-light); border-color: color-mix(in oklab, var(--accent) 60%, transparent); }
  .head { display: flex; justify-content: space-between; align-items: baseline; gap: 8px; }
  .name { font-weight: 700; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .legs { color: var(--muted); font-size: 11px; }
  .rem { font-family: var(--font-score); font-weight: 800; font-size: clamp(48px, 7vh, 80px); line-height: 0.9; }
  .player.active .rem { color: var(--accent); }
  .track { display: grid; grid-template-columns: repeat(21, 1fr); gap: 2px; }
  .track span { height: 6px; border-radius: 2px; background: var(--bg-dark); }
  .track span.done { background: var(--accent); }
  .meta { display: flex; gap: 14px; color: var(--muted); font-size: 11px; font-variant-numeric: tabular-nums; }
  .meta b { color: var(--fg); font-weight: 500; }
  .aim { display: flex; justify-content: space-between; align-items: baseline; gap: 8px; }
  .target { font-family: var(--font-score); font-weight: 800; font-size: 44px; color: var(--accent); }
</style>
