<script lang="ts">
  import { deleteGame, games as loadGames, hasBackend, type GameSummary } from '$lib/db.svelte';
  import { decimal, i18n, t } from '$lib/i18n/index.svelte';
  import type { MessageKey } from '$lib/i18n/types';

  let list = $state<GameSummary[]>([]);
  let loaded = $state(false);
  let error = $state('');
  let sel = $state(0);
  let armed = $state<string | null>(null);

  function reload() {
    loadGames(200)
      .then((g) => {
        list = g;
        sel = Math.min(sel, Math.max(0, g.length - 1));
      })
      .catch((e) => (error = String(e)))
      .finally(() => (loaded = true));
  }
  reload();

  const when = (iso: string) =>
    new Intl.DateTimeFormat(i18n.locale, { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(iso));

  function title(g: GameSummary): string {
    const s = g.settings as Record<string, unknown>;
    switch (g.mode) {
      case 'x01':
        return g.players.length === 1 ? `${s.start} solo` : `${s.start}${s.doubleIn ? ' · DI' : ''}${s.doubleOut ? ' · DO' : ''}`;
      case 'checkout':
        return `${t('training.checkout')} ${s.min}–${s.max}`;
      case 'scoring':
        return `${t('training.scoring')} ${s.segment === 25 ? 'BULL' : `T${s.segment}`}`;
      default:
        return t(`training.${g.mode}` as MessageKey);
    }
  }

  function result(g: GameSummary, p: GameSummary['players'][number]): string {
    const avg = p.darts ? (p.scored / p.darts) * 3 : 0;
    switch (g.mode) {
      case 'x01':
        return `${t('history.legs', { won: p.legsWon })} · ${t('history.avg', { avg: decimal(avg) })}`;
      case 'checkout':
        return t('history.finishes', { won: p.legsWon, legs: g.legs });
      case 'scoring':
        return `${t('history.total', { total: p.scored })} · ${t('history.avg', { avg: decimal(avg) })}`;
      default:
        return t('history.bobs', { score: 27 + p.scored });
    }
  }

  async function remove(g: GameSummary) {
    if (armed !== g.id) {
      armed = g.id;
      return;
    }
    armed = null;
    await deleteGame(g.id).catch((e) => (error = String(e)));
    reload();
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.ctrlKey || e.altKey || e.metaKey || !list.length) return;
    if (e.key === 'j' || e.key === 'ArrowDown') sel = Math.min(list.length - 1, sel + 1);
    else if (e.key === 'k' || e.key === 'ArrowUp') sel = Math.max(0, sel - 1);
    else if (e.key === 'Delete') return void remove(list[sel]);
    else if (e.key === 'Escape') armed = null;
    else return;
    e.preventDefault();
    if (e.key !== 'Escape') armed = null;
    document.getElementById(`game-${sel}`)?.scrollIntoView({ block: 'nearest' });
  }
</script>

<svelte:window {onkeydown} />

<div class="wrap">
  <h1>{t('history.title')}</h1>

  {#if !hasBackend}
    <p class="hint">{t('history.noBackend')}</p>
  {:else if error}
    <p class="err">{error}</p>
  {:else if loaded && !list.length}
    <p class="hint">{t('history.empty')}</p>
  {/if}

  <ol class="games">
    {#each list as g, i (g.id)}
      <li id="game-{i}" class:sel={i === sel}>
        <button class="row" tabindex="-1" onclick={() => (sel = i)}>
          <span class="when">{when(g.startedAt)}</span>
          <span class="mode">{title(g)}</span>
          {#if !g.finishedAt}<span class="tag">{t('history.unfinished')}</span>{/if}
        </button>
        <div class="players">
          {#each g.players as p (p.id)}
            <div class="p" class:win={g.winnerId === p.id && g.players.length > 1}>
              <span class="pname">{p.name}</span>
              <span class="res">{result(g, p)}</span>
            </div>
          {/each}
        </div>
        {#if armed === g.id}
          <div class="err">{t('history.confirmDelete')}</div>
        {/if}
      </li>
    {/each}
  </ol>

  {#if list.length}
    <span class="hint">
      {#each t('history.keys').split(/(\{\w+\})/) as part, k (k)}
        {#if part === '{jk}'}<kbd>j</kbd> <kbd>k</kbd>{:else if part === '{del}'}<kbd>Del</kbd>{:else}{part}{/if}
      {/each}
    </span>
  {/if}
</div>

<style>
  .wrap { max-width: 760px; margin-inline: auto; width: 100%; padding-block: 24px; display: grid; gap: 16px; overflow: auto; height: 100%; align-content: start; }
  h1 { margin: 0; font-family: var(--font-score); font-weight: 800; font-size: 44px; text-transform: uppercase; letter-spacing: 0.02em; }
  .games { list-style: none; margin: 0; padding: 0; display: grid; gap: 6px; }
  li { border: 1px solid var(--line); border-radius: var(--r); padding: 8px 12px; display: grid; gap: 4px; }
  li.sel { border-color: var(--accent); background: var(--bg-light); }
  .row { display: flex; gap: 12px; align-items: baseline; background: none; border: 0; padding: 0; text-align: left; cursor: pointer; }
  .when { color: var(--muted); font-size: 12px; font-variant-numeric: tabular-nums; }
  .mode { font-weight: 700; }
  .tag { font-size: 10px; text-transform: uppercase; letter-spacing: 0.1em; color: var(--warn); }
  .players { display: grid; gap: 2px; }
  .p { display: flex; justify-content: space-between; gap: 12px; font-variant-numeric: tabular-nums; }
  .p.win .pname { color: var(--accent); font-weight: 700; }
  .res { color: var(--muted); }
  .hint { color: var(--muted); font-size: 11px; }
  .err { color: var(--red); font-size: 12px; }
</style>
