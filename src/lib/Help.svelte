<script lang="ts">
  import { t } from '$lib/i18n/index.svelte';
  import type { MessageKey } from '$lib/i18n/types';

  let { onclose }: { onclose: () => void } = $props();

  const rows: [string[], MessageKey][] = [
    [['20', '↑'], 'help.outer'],
    [['20', '↓'], 'help.inner'],
    [['20', '⏎'], 'help.single'],
    [['20', 'd'], 'help.double'],
    [['20', 't'], 'help.treble'],
    [['25'], 'help.obull'],
    [['50'], 'help.bull'],
    [['m'], 'help.miss'],
    [['⏎'], 'help.confirm'],
    [['⌫'], 'help.backspace'],
    [['Esc'], 'help.escape'],
    [['Ctrl', 'N'], 'help.newGame'],
    [['Ctrl', 'G'], 'help.game'],
    [['Ctrl', 'T'], 'help.training'],
    [['Ctrl', 'H'], 'help.history'],
    [['Ctrl', 'S'], 'help.stats'],
    [['Ctrl', ','], 'help.settings'],
  ];
  const [before, after] = $derived(t('help.click').split('{esc}'));
</script>

<svelte:window onkeydown={(e) => { if (e.key === 'Escape' || e.key === '?') { e.preventDefault(); e.stopImmediatePropagation(); onclose(); } }} />

<div class="help" role="presentation" onclick={onclose}>
  <div class="card" role="dialog" aria-label={t('help.title')}>
    <strong>{t('help.title')}</strong>
    <table>
      <tbody>
        {#each rows as [keys, what] (what)}
          <tr><td>{#each keys as k (k)}<kbd>{k}</kbd> {/each}</td><td>{t(what)}</td></tr>
        {/each}
      </tbody>
    </table>
    <span class="hint">{before}<kbd>Esc</kbd>{after}</span>
  </div>
</div>

<style>
  .help { position: fixed; inset: 0; display: grid; place-items: center; background: color-mix(in oklab, var(--bg) 75%, transparent); z-index: 10; padding: 16px; }
  .card { background: var(--bg-light); border: 1px solid var(--line); border-radius: 10px; padding: 20px 24px; max-width: 600px; width: 100%; display: grid; gap: 10px; }
  table { border-collapse: collapse; width: 100%; }
  td { padding: 5px 0; border-bottom: 1px solid var(--line); }
  td:first-child { white-space: nowrap; padding-right: 16px; }
  td:last-child { text-align: right; color: var(--muted); }
  .hint { font-size: 11px; color: var(--muted); }
</style>
