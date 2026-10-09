<script lang="ts">
  /** One series over time, with a crosshair tooltip. Points are evenly spaced (one per game). */
  interface Props {
    points: { value: number; label: string }[];
    height?: number;
    ariaLabel: string;
  }
  let { points, height = 180, ariaLabel }: Props = $props();

  let width = $state(600);
  let hover = $state<number | null>(null);

  const M = { top: 10, right: 12, bottom: 10, left: 34 };
  const domain = $derived.by(() => {
    const vs = points.map((p) => p.value);
    let lo = Math.floor(Math.min(...vs) / 10) * 10;
    let hi = Math.ceil(Math.max(...vs) / 10) * 10;
    if (hi - lo < 20) {
      lo = Math.max(0, lo - 10);
      hi = lo + 20;
    }
    return [lo, hi] as const;
  });
  const ticks = $derived.by(() => {
    const [lo, hi] = domain;
    const step = (hi - lo) / 4 >= 10 ? Math.ceil((hi - lo) / 4 / 10) * 10 : 5;
    const out: number[] = [];
    for (let v = lo; v <= hi; v += step) out.push(v);
    return out;
  });
  const x = (i: number) => M.left + (points.length < 2 ? (width - M.left - M.right) / 2 : (i / (points.length - 1)) * (width - M.left - M.right));
  const y = (v: number) => M.top + (1 - (v - domain[0]) / (domain[1] - domain[0])) * (height - M.top - M.bottom);
  const path = $derived(points.map((p, i) => `${i ? 'L' : 'M'}${x(i)} ${y(p.value)}`).join(''));

  function move(e: PointerEvent) {
    const rect = (e.currentTarget as SVGElement).getBoundingClientRect();
    const px = e.clientX - rect.left;
    if (points.length < 2) return void (hover = 0);
    const i = Math.round(((px - M.left) / (width - M.left - M.right)) * (points.length - 1));
    hover = Math.max(0, Math.min(points.length - 1, i));
  }
</script>

<div class="trend" bind:clientWidth={width}>
  <svg {width} {height} role="img" aria-label={ariaLabel} onpointermove={move} onpointerleave={() => (hover = null)}>
    {#each ticks as v (v)}
      <line class="grid" x1={M.left} x2={width - M.right} y1={y(v)} y2={y(v)} />
      <text class="tick" x={M.left - 6} y={y(v)}>{v}</text>
    {/each}
    {#if hover !== null}
      <line class="cross" x1={x(hover)} x2={x(hover)} y1={M.top} y2={height - M.bottom} />
    {/if}
    <path class="line" d={path} />
    {#if points.length <= 60}
      {#each points as p, i (i)}
        <circle class="dot" cx={x(i)} cy={y(p.value)} r="3" />
      {/each}
    {/if}
    {#if hover !== null}
      <circle class="dot on" cx={x(hover)} cy={y(points[hover].value)} r="5" />
    {/if}
    <rect class="hit" x={M.left} y="0" width={Math.max(0, width - M.left - M.right)} {height} />
  </svg>
  {#if hover !== null}
    <div class="tip" style:left="{Math.min(Math.max(x(hover), 90), width - 90)}px">{points[hover].label}</div>
  {/if}
</div>

<style>
  .trend { position: relative; width: 100%; }
  svg { display: block; overflow: visible; }
  .grid { stroke: var(--line); stroke-width: 1; }
  .tick { fill: var(--muted); font: 10px var(--font-ui); text-anchor: end; dominant-baseline: central; font-variant-numeric: tabular-nums; }
  .line { fill: none; stroke: var(--accent); stroke-width: 2; stroke-linejoin: round; stroke-linecap: round; }
  .dot { fill: var(--accent); stroke: var(--bg); stroke-width: 2; pointer-events: none; }
  .dot.on { stroke-width: 2; }
  .cross { stroke: var(--muted); stroke-width: 1; stroke-dasharray: 2 3; }
  .hit { fill: transparent; }
  .tip {
    position: absolute; top: -6px; transform: translate(-50%, -100%); pointer-events: none; white-space: nowrap;
    background: var(--bg-light); border: 1px solid var(--line); border-radius: var(--r); padding: 3px 8px; font-size: 11px; color: var(--fg);
  }
</style>
