/**
 * Where a dart landed.
 * `inner` / `outer` are the two single areas (bull–treble and treble–double);
 * `single` means a single whose area was not specified.
 */
export type Ring = 'inner' | 'outer' | 'single' | 'double' | 'triple' | 'obull' | 'bull' | 'miss';

export interface Dart {
  /** 1–20, 25 for both bulls, 0 for a miss. */
  segment: number;
  ring: Ring;
  points: number;
}

/** Board order clockwise from the top. */
export const BOARD_ORDER = [20, 1, 18, 4, 13, 6, 10, 15, 2, 17, 3, 19, 7, 16, 8, 11, 14, 9, 12, 5] as const;

export function dart(segment: number, ring: Ring): Dart {
  if (ring === 'miss') return { segment: 0, ring, points: 0 };
  if (ring === 'obull') return { segment: 25, ring, points: 25 };
  if (ring === 'bull') return { segment: 25, ring, points: 50 };
  if (!Number.isInteger(segment) || segment < 1 || segment > 20) {
    throw new RangeError(`No segment ${segment} on a dartboard`);
  }
  const mult = ring === 'double' ? 2 : ring === 'triple' ? 3 : 1;
  return { segment, ring, points: segment * mult };
}

/** Bull counts as a double for checkouts and double-in. */
export const isDouble = (d: Dart) => d.ring === 'double' || d.ring === 'bull';

/** Label as it is typed: `20↑`, `20↓`, `D20`, `T20`, `25`, `BULL`. */
export function dartLabel(d: Dart): string {
  switch (d.ring) {
    case 'double': return `D${d.segment}`;
    case 'triple': return `T${d.segment}`;
    case 'outer': return `${d.segment}↑`;
    case 'inner': return `${d.segment}↓`;
    case 'single': return `${d.segment}`;
    case 'obull': return '25';
    case 'bull': return 'BULL';
    case 'miss': return '—';
  }
}

/** Label for checkout routes, where singles are written `S19`. */
export function routeLabel(d: Dart): string {
  return d.ring === 'single' ? `S${d.segment}` : dartLabel(d);
}
