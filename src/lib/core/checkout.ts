import { dart, type Dart } from './dart';

// Preferred setup darts: trebles from the top, then outer bull, then singles.
const SETUP: Dart[] = [
  ...Array.from({ length: 20 }, (_, i) => dart(20 - i, 'triple')),
  dart(25, 'obull'),
  ...Array.from({ length: 20 }, (_, i) => dart(20 - i, 'single')),
  dart(25, 'bull'),
];

// Preferred finishing doubles: the ones that halve cleanly come first.
const DOUBLES: Dart[] = [20, 16, 18, 12, 10, 8, 14, 6, 4, 2, 19, 17, 15, 13, 11, 9, 7, 5, 3, 1]
  .map((n) => dart(n, 'double'))
  .concat(dart(25, 'bull'));

// Without double out, finishing on a single is as good as any.
const ANY_FINISH: Dart[] = [...SETUP, ...DOUBLES.filter((d) => d.ring === 'double')];

/** First route of exactly `used` darts that ends on `last`. */
function routeEndingOn(remaining: number, used: number, last: Dart): Dart[] | null {
  const rest = remaining - last.points;
  if (used === 1) return rest === 0 ? [last] : null;
  if (used === 2) {
    const a = SETUP.find((s) => s.points === rest);
    return a ? [a, last] : null;
  }
  for (const a of SETUP) {
    const b = SETUP.find((s) => s.points === rest - a.points);
    if (b) return [a, b, last];
  }
  return null;
}

/**
 * Checkout routes for `remaining` with `dartsLeft` darts, best first.
 * The fewest-darts routes come first; each route finishes on a different dart,
 * so the alternatives are real choices (e.g. D20 vs D16), not reorderings.
 * With double out the last dart must be a double or the bull.
 */
export function checkouts(remaining: number, dartsLeft: number, doubleOut = true, limit = 3): Dart[][] {
  if (remaining < 1 || dartsLeft < 1) return [];
  if (doubleOut && (remaining < 2 || remaining > 170)) return [];
  if (!doubleOut && remaining > 180) return [];
  const finish = doubleOut ? DOUBLES : ANY_FINISH;

  const routes: Dart[][] = [];
  const finishes = new Set<string>();
  for (let used = 1; used <= Math.min(3, dartsLeft); used++) {
    for (const last of finish) {
      const key = `${last.ring}${last.segment}`;
      if (finishes.has(key)) continue;
      const route = routeEndingOn(remaining, used, last);
      if (!route) continue;
      routes.push(route);
      finishes.add(key);
      if (routes.length >= limit) return routes;
    }
  }
  return routes;
}

/** The preferred checkout route, or `null`. */
export function checkout(remaining: number, dartsLeft: number, doubleOut = true): Dart[] | null {
  return checkouts(remaining, dartsLeft, doubleOut, 1)[0] ?? null;
}
