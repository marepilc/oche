<script lang="ts">
  import Help from '$lib/Help.svelte';
  import { i18n, t } from '$lib/i18n/index.svelte';
  import { followSync, sync } from '$lib/db.svelte';
  import { Drill, Match } from '$lib/match.svelte';
  import { followOmarchyTheme, theme } from '$lib/theme.svelte';
  import DrillView from '$lib/views/DrillView.svelte';
  import Game from '$lib/views/Game.svelte';
  import History from '$lib/views/History.svelte';
  import NewGame from '$lib/views/NewGame.svelte';
  import Settings from '$lib/views/Settings.svelte';
  import Training from '$lib/views/Training.svelte';

  type View = 'new' | 'training' | 'game' | 'history' | 'settings';

  let view = $state<View>('new');
  /** The match or drill being played, and the view it was started from. */
  let session = $state<{ play: Match | Drill; from: View } | null>(null);
  let help = $state(false);

  function play(p: Match | Drill, from: View) {
    session = { play: p, from };
    view = 'game';
  }
  function done() {
    view = session?.from ?? 'new';
    session = null;
  }

  $effect(() => {
    followOmarchyTheme();
  });
  $effect(() => followSync());
  $effect(() => {
    document.documentElement.lang = i18n.locale;
  });

  function onkeydown(e: KeyboardEvent) {
    if (help) return;
    if (e.ctrlKey && !e.altKey && !e.metaKey) {
      const key = e.key.toLowerCase();
      if (key === 'n') { e.preventDefault(); view = 'new'; }
      else if (key === 't') { e.preventDefault(); view = 'training'; }
      else if (key === 'g' && session) { e.preventDefault(); view = 'game'; }
      else if (key === 'h') { e.preventDefault(); view = 'history'; }
      else if (key === ',') { e.preventDefault(); view = 'settings'; }
      return;
    }
    const typing = (e.target as HTMLElement | null)?.closest?.('input, select, textarea');
    if (e.key === '?' && !typing) { e.preventDefault(); help = true; }
  }
</script>

<svelte:window {onkeydown} />

<div class="app">
  <header class="top" data-tauri-drag-region>
    <div class="mark" data-tauri-drag-region>oc<span>h</span>e</div>
    <nav class="tabs">
      <button class="tab" aria-current={view === 'new'} onclick={() => (view = 'new')}>{t('nav.newGame')}<kbd>^N</kbd></button>
      <button class="tab" aria-current={view === 'training'} onclick={() => (view = 'training')}>{t('nav.training')}<kbd>^T</kbd></button>
      <button class="tab" aria-current={view === 'game'} disabled={!session} onclick={() => (view = 'game')}>{t('nav.game')}<kbd>^G</kbd></button>
      <button class="tab" aria-current={view === 'history'} onclick={() => (view = 'history')}>{t('nav.history')}<kbd>^H</kbd></button>
      <button class="tab" aria-current={view === 'settings'} onclick={() => (view = 'settings')}>{t('nav.settings')}<kbd>^,</kbd></button>
    </nav>
    <div class="spacer" data-tauri-drag-region></div>
    {#if sync.status.state === 'synced'}<div class="status">{t('nav.synced')}</div>
    {:else if sync.status.state === 'pushing'}<div class="status">{t('nav.pushing', { pending: sync.status.pending })}</div>
    {:else if sync.status.state === 'error'}<div class="status err" title={sync.status.message}>{t('nav.syncError')}</div>{/if}
    {#if theme.name}<div class="status">{t('nav.theme', { name: theme.name })}</div>{/if}
  </header>

  <main>
    {#if view === 'settings'}
      <Settings />
    {:else if view === 'history'}
      <History />
    {:else if view === 'training'}
      <Training onx01={(settings, player) => play(new Match(settings, [player]), 'training')} ondrill={(settings, player) => play(new Drill(settings, player), 'training')} />
    {:else if view === 'game' && session}
      {#key session.play}
        {#if session.play instanceof Match}
          <Game match={session.play} ondone={done} />
        {:else}
          <DrillView drill={session.play} ondone={done} />
        {/if}
      {/key}
    {:else}
      <NewGame onstart={(settings, players) => play(new Match(settings, players), 'new')} />
    {/if}
  </main>
</div>

{#if help}<Help onclose={() => (help = false)} />{/if}

<style>
  .app { height: 100%; display: grid; grid-template-rows: auto minmax(0, 1fr); padding-inline: 16px; }
  .top { display: flex; align-items: center; gap: 20px; padding-block: 8px; border-bottom: 1px solid var(--line); }
  .mark { font-family: var(--font-score); font-weight: 800; font-size: 22px; letter-spacing: 0.04em; text-transform: uppercase; }
  .mark span { color: var(--accent); }
  .tabs { display: flex; gap: 2px; }
  .tab { background: none; border: 0; padding: 3px 10px; border-radius: var(--r); color: var(--muted); cursor: pointer; }
  .tab:disabled { opacity: 0.4; cursor: default; }
  .tab kbd { font-size: 10px; margin-left: 6px; opacity: 0.7; }
  .tab[aria-current='true'] { color: var(--fg); background: var(--bg-light); }
  .spacer { flex: 1; align-self: stretch; }
  .status { font-size: 11px; color: var(--muted); }
  .status.err { color: var(--red); }
  main { min-height: 0; }
</style>
