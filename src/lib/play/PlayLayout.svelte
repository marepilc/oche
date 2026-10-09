<script lang="ts">
  import type { Snippet } from 'svelte';
  import { t } from '$lib/i18n/index.svelte';
  import type { ThrowInput } from '$lib/throwInput.svelte';

  /** The screen of every game mode: scoreboard, board, turn panel and the command line. */
  interface Props {
    cmd: ThrowInput;
    hint: string;
    /** The turn waits for confirmation: the hint is highlighted. */
    attn?: boolean;
    label: string;
    left: Snippet;
    board: Snippet;
    side: Snippet;
    banner?: Snippet;
  }
  let { cmd, hint, attn = false, label, left, board, side, banner }: Props = $props();
</script>

<div class="game">
  <section class="left" aria-label={label}>
    <div class="label">{label}</div>
    {@render left()}
  </section>

  <section class="board-wrap">{@render board()}</section>

  <aside class="side">{@render side()}</aside>

  <footer class="cmd">
    <div class="prompt"><span class="p">›</span>{cmd.input.buf}<span class="caret"></span></div>
    {#if cmd.error}
      <div class="err">{t(cmd.error.key, cmd.error.params)}</div>
    {:else}
      <div class="hint" class:attn>{hint}</div>
    {/if}
    <div class="keys">
      <span><kbd>⏎</kbd> {t('game.keyConfirm')}</span>
      <span><kbd>⌫</kbd> {t('game.keyUndo')}</span>
      <span><kbd>m</kbd> {t('game.keyMiss')}</span>
      <span><kbd>?</kbd> {t('game.keyHelp')}</span>
    </div>
  </footer>
</div>

{#if banner}
  <div class="banner">
    <div class="card">{@render banner()}</div>
  </div>
{/if}

<style>
  .game {
    height: 100%;
    display: grid;
    grid-template-columns: minmax(220px, 1fr) minmax(0, 1.7fr) minmax(220px, 1fr);
    grid-template-rows: minmax(0, 1fr) auto;
    grid-template-areas: 'left board side' 'cmd cmd cmd';
    column-gap: 24px;
  }
  @media (max-width: 1000px) {
    .game {
      grid-template-columns: minmax(220px, 1fr) minmax(0, 1.4fr);
      grid-template-rows: auto minmax(0, 1fr) auto;
      grid-template-areas: 'left board' 'side board' 'cmd cmd';
    }
  }
  .left { grid-area: left; display: flex; flex-direction: column; gap: 10px; padding-block: 16px; overflow: auto; }
  .board-wrap { grid-area: board; min-height: 0; display: grid; place-items: center; padding-block: 16px; }
  .side { grid-area: side; display: flex; flex-direction: column; gap: 14px; padding-block: 16px; overflow: auto; }

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
  .card { background: var(--bg-light); border: 1px solid var(--accent); border-radius: 10px; padding: 24px 32px; text-align: center; display: grid; gap: 10px; justify-items: center; }
  .card :global(.t) { font-family: var(--font-score); font-weight: 800; font-size: 44px; color: var(--accent); max-width: 20ch; text-wrap: balance; }
  .card :global(.keys) { color: var(--muted); font-size: 11px; }
</style>
