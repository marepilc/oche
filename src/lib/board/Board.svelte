<script lang="ts">
  import { BOARD_ORDER, dart, type Dart } from '$lib/core/dart';
  import type { Preview } from '$lib/core/input';
  import { t } from '$lib/i18n/index.svelte';

  type Area = 'inner' | 'triple' | 'outer' | 'double';

  interface Props {
    preview?: Preview | null;
    /** Darts of the current turn, drawn as numbered markers. */
    darts?: Dart[];
    onpick?: (d: Dart) => void;
    /** Heat map: share of darts (0–1, relative to the busiest area) keyed `segment:area`, e.g. `20:triple`, `25:bull`. */
    heat?: Map<string, { value: number; label: string }> | null;
  }
  let { preview = null, darts = [], onpick, heat = null }: Props = $props();

  // Regulation radii in mm, scaled so the double ring ends at 200.
  const K = 200 / 170;
  const R = { bull: 6.35 * K, obull: 15.9 * K, tIn: 99 * K, tOut: 107 * K, dIn: 162 * K, dOut: 200 };
  const AREAS: [Area, number, number][] = [
    ['inner', R.obull, R.tIn],
    ['triple', R.tIn, R.tOut],
    ['outer', R.tOut, R.dIn],
    ['double', R.dIn, R.dOut],
  ];

  const pt = (r: number, deg: number): [number, number] => {
    const a = (deg * Math.PI) / 180;
    return [r * Math.cos(a), r * Math.sin(a)];
  };
  const sector = (r1: number, r2: number, a0: number, a1: number) => {
    const [x1, y1] = pt(r1, a0), [x2, y2] = pt(r2, a0), [x3, y3] = pt(r2, a1), [x4, y4] = pt(r1, a1);
    return `M${x1} ${y1}L${x2} ${y2}A${r2} ${r2} 0 0 1 ${x3} ${y3}L${x4} ${y4}A${r1} ${r1} 0 0 0 ${x1} ${y1}Z`;
  };
  const centre = (i: number) => -90 + i * 18;

  const regions = BOARD_ORDER.flatMap((n, i) =>
    AREAS.map(([area, r1, r2]) => {
      const dark = i % 2 === 0;
      const fill =
        area === 'inner' || area === 'outer'
          ? dark ? 'var(--board-a)' : 'var(--board-b)'
          : dark ? 'var(--red)' : 'var(--green)';
      return { n, area, d: sector(r1, r2, centre(i) - 9, centre(i) + 9), fill };
    }),
  );
  const numbers = BOARD_ORDER.map((n, i) => ({ n, xy: pt(219, centre(i)) }));

  const lit = (n: number, area: string) =>
    !!preview && preview.segment === n && (preview.rings as string[]).includes(area);

  /** Where a dart is drawn: the middle of its area; the outer bull at 12 o'clock, the bull in the centre. */
  function spot(d: Dart): [number, number] | null {
    if (d.ring === 'miss') return null;
    if (d.ring === 'bull') return [0, 0];
    if (d.ring === 'obull') return pt((R.bull + R.obull) / 2, -90);
    const i = BOARD_ORDER.indexOf(d.segment as (typeof BOARD_ORDER)[number]);
    // A single without its area is drawn in the outer single, the larger of the two.
    const area = AREAS.find(([a]) => a === (d.ring === 'single' ? 'outer' : d.ring))!;
    return pt((area[1] + area[2]) / 2, centre(i));
  }

  /** One marker per area hit; darts in the same area share it ("1·3"). */
  const markers = $derived.by(() => {
    const out: { key: string; xy: [number, number]; label: string }[] = [];
    darts.forEach((d, k) => {
      const xy = spot(d);
      if (!xy) return;
      const key = `${d.segment}:${d.ring === 'single' ? 'outer' : d.ring}`;
      const m = out.find((o) => o.key === key);
      if (m) m.label += `·${k + 1}`;
      else out.push({ key, xy, label: String(k + 1) });
    });
    return out;
  });
</script>

<svg class="board" class:dim={!!heat} viewBox="-240 -240 480 480" role="img" aria-label={t('game.board')}>
  <circle r="236" fill="var(--bg-darker)" />
  {#each regions as r (r.n + r.area)}
    <path
      class="reg"
      class:lit={lit(r.n, r.area)}
      d={r.d}
      fill={r.fill}
      role="presentation"
      onclick={() => onpick?.(dart(r.n, r.area))}
    />
  {/each}
  <circle class="reg" class:lit={lit(25, 'obull')} r={R.obull} fill="var(--green)" role="presentation" onclick={() => onpick?.(dart(25, 'obull'))} />
  <circle class="reg" class:lit={lit(25, 'bull')} r={R.bull} fill="var(--red)" role="presentation" onclick={() => onpick?.(dart(25, 'bull'))} />
  {#if heat}
    {#each regions as r (r.n + r.area)}
      {@const h = heat.get(`${r.n}:${r.area}`)}
      <path class="heat" d={r.d} fill-opacity={h ? 0.15 + h.value * 0.8 : 0}><title>{h?.label ?? ''}</title></path>
    {/each}
    {@const ob = heat.get('25:obull')}
    {@const b = heat.get('25:bull')}
    <circle class="heat" r={R.obull} fill-opacity={ob ? 0.15 + ob.value * 0.8 : 0}><title>{ob?.label ?? ''}</title></circle>
    <circle class="heat" r={R.bull} fill-opacity={b ? 0.15 + b.value * 0.8 : 0}><title>{b?.label ?? ''}</title></circle>
  {/if}
  {#each [R.tIn, R.tOut, R.dIn, R.dOut] as r (r)}
    <circle class="wire" {r} />
  {/each}
  {#each numbers as { n, xy } (n)}
    <text class="num" class:on={preview?.segment === n} x={xy[0]} y={xy[1]}>{n}</text>
  {/each}
  {#each markers as m (m.key)}
    {@const w = 14 + (m.label.length - 1) * 4.5}
    <rect class="marker" x={m.xy[0] - w / 2} y={m.xy[1] - 7} width={w} height="14" rx="7" />
    <text class="marker-n" x={m.xy[0]} y={m.xy[1]}>{m.label}</text>
  {/each}
</svg>

<style>
  .board { width: 100%; height: 100%; max-height: 100%; display: block; }
  .reg { stroke: var(--bg-darker); stroke-width: 0.8; cursor: pointer; transition: fill-opacity 0.12s; }
  .reg:hover { fill-opacity: 0.8; }
  .reg.lit { fill: var(--accent); fill-opacity: 0.6; }
  .board.dim .reg { opacity: 0.25; }
  .heat { fill: var(--accent); stroke: var(--bg-darker); stroke-width: 0.8; }
  .wire { fill: none; stroke: color-mix(in oklab, var(--fg) 30%, transparent); stroke-width: 0.6; pointer-events: none; }
  .num { fill: var(--muted); font: 600 15px var(--font-ui); text-anchor: middle; dominant-baseline: central; }
  .num.on { fill: var(--accent); }
  .marker { fill: var(--bg-darker); stroke: var(--fg); stroke-width: 2; pointer-events: none; }
  .marker-n { fill: var(--fg); font: 700 9px var(--font-ui); text-anchor: middle; dominant-baseline: central; pointer-events: none; }
</style>
