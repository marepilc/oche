<script lang="ts">
  import { dartLabel, type Dart } from '$lib/core/dart';
  import { t } from '$lib/i18n/index.svelte';

  /** The three darts of the turn; between turns the previous turn is shown faded. */
  interface Props {
    thrown: Dart[];
    ghost?: Dart[] | null;
    /** Text under each dart; points by default. */
    detail?: (d: Dart, i: number) => string;
  }
  let { thrown, ghost = null, detail = (d) => t('game.points', { n: d.points }) }: Props = $props();
</script>

<div class="slots">
  {#each [0, 1, 2] as i (i)}
    {@const d = thrown[i] ?? ghost?.[i]}
    <div class="slot" class:filled={!!thrown[i]} class:ghost={!thrown[i] && !!d}>
      {#if d}
        <span class="s-label">{dartLabel(d)}</span>
        <span class="s-pts">{detail(d, i)}</span>
      {:else}
        <span class="s-pts">{t('game.dartN', { n: i + 1 })}</span>
      {/if}
    </div>
  {/each}
</div>

<style>
  .slots { display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px; }
  .slot { border: 1px dashed var(--line); border-radius: var(--r); padding: 8px 6px; text-align: center; min-height: 58px; display: grid; align-content: center; }
  .slot.filled { border-style: solid; background: var(--bg-light); }
  .slot.ghost { opacity: 0.45; }
  .s-label { font-weight: 700; font-size: 15px; }
  .s-pts { color: var(--muted); font-size: 11px; }
</style>
