<script lang="ts">
  /** Small bar sparkline, oldest left; each bar has a hover label. */
  interface Props {
    values: { value: number; label: string }[];
    /** Value at the top of the chart; defaults to the largest value. */
    max?: number;
    height?: number;
    ariaLabel: string;
  }
  let { values, max, height = 40, ariaLabel }: Props = $props();
  let hover = $state<number | null>(null);
  const top = $derived(max ?? Math.max(1, ...values.map((v) => v.value)));
  const W = 10;
  const GAP = 2;
</script>

<div class="bars">
  <svg width={values.length * (W + GAP)} {height} role="img" aria-label={ariaLabel}>
    {#each values as v, i (i)}
      {@const h = Math.max(2, (Math.max(0, v.value) / top) * height)}
      <rect class="bar" class:on={hover === i} x={i * (W + GAP)} y={height - h} width={W} height={h} rx="2" />
      <rect
        class="hit"
        role="presentation"
        x={i * (W + GAP)}
        y="0"
        width={W + GAP}
        {height}
        onpointerenter={() => (hover = i)}
        onpointerleave={() => (hover = null)}
      />
    {/each}
  </svg>
  <span class="tip">{hover !== null ? values[hover].label : ''}</span>
</div>

<style>
  .bars { display: flex; align-items: flex-end; gap: 10px; min-height: 40px; }
  svg { display: block; flex: none; }
  .bar { fill: var(--accent); opacity: 0.75; }
  .bar.on { opacity: 1; }
  .hit { fill: transparent; }
  .tip { font-size: 11px; color: var(--muted); white-space: nowrap; }
</style>
