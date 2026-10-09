import { checkouts } from './checkout';
import { isDouble, type Dart } from './dart';
import type { Preview } from './input';
import type { GameEvent } from './x01';

/**
 * Solo training drills. Like X01 they are replayed from the event log,
 * and a turn ends only when it is confirmed.
 *
 *   checkout  random finishes between `min` and `max`, `turns` visits per attempt, double out
 *   scoring   `rounds` visits aimed at one segment, every point counts
 *   bobs27    D1…D20 and the bull, start on 27; hits add, a visit with no hit costs the double
 */
export type DrillSettings =
  | { kind: 'checkout'; min: number; max: number; attempts: number; turns: number; seed: number }
  | { kind: 'scoring'; segment: number; rounds: number }
  | { kind: 'bobs27' };

export type DrillKind = DrillSettings['kind'];

export interface DrillTurn {
  /** What the turn aimed at: `121`, `T20`, `D7`, `BULL`. */
  target: string;
  darts: Dart[];
  scoreBefore: number;
  /** Points that counted: towards the finish, the total or Bob's score (negative on a miss). */
  scored: number;
  bust: boolean;
  checkout: boolean;
}

/** One checkout attempt, or the whole session for the other drills. */
export interface Attempt {
  target: number;
  turns: DrillTurn[];
  success: boolean | null;
}

export interface DrillState {
  settings: DrillSettings;
  attempts: Attempt[];
  turn: DrillTurn | null;
  /** The turn is over and waits for confirmation. */
  complete: boolean;
  lastTurn: DrillTurn | null;
  done: boolean;
  /** The big number: what is left to check out, the total, or Bob's score. */
  score: number;
  /** What to aim at next, as a label and as board areas. */
  aim: string;
  aimArea: Preview | null;
  /** Round number within the drill (1-based) and how many there are. */
  round: number;
  rounds: number;
}

/** Small seeded PRNG, so a checkout drill replays to the same targets. */
function mulberry32(seed: number) {
  return () => {
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Finishes for a checkout drill: only scores that can be checked out in three darts. */
export function checkoutTargets(min: number, max: number, count: number, seed: number): number[] {
  const pool: number[] = [];
  for (let n = Math.max(2, min); n <= Math.min(170, max); n++) if (checkouts(n, 3, true, 1).length) pool.push(n);
  if (!pool.length) return [];
  const rnd = mulberry32(seed);
  const out: number[] = [];
  for (let i = 0; i < count; i++) {
    let n = pool[Math.floor(rnd() * pool.length)];
    // Avoid the same finish twice in a row when there is a choice.
    if (pool.length > 1 && n === out[out.length - 1]) n = pool[(pool.indexOf(n) + 1) % pool.length];
    out.push(n);
  }
  return out;
}

/** Bob's 27 targets: doubles 1–20, then the bull (25). */
export const BOBS_TARGETS = [...Array.from({ length: 20 }, (_, i) => i + 1), 25];

const doubleLabel = (n: number) => (n === 25 ? 'BULL' : `D${n}`);
const doubleArea = (n: number): Preview => (n === 25 ? { segment: 25, rings: ['bull'] } : { segment: n, rings: ['double'] });
const scoringAim = (n: number) => (n === 25 ? 'BULL' : `T${n}`);
const scoringArea = (n: number): Preview =>
  n === 25 ? { segment: 25, rings: ['obull', 'bull'] } : { segment: n, rings: ['triple'] };

/** Bob's 27: the value of a hit on target `n`, and whether a dart hits it. */
const bobsValue = (n: number) => (n === 25 ? 50 : n * 2);
const bobsHit = (n: number, d: Dart) => (n === 25 ? d.ring === 'bull' : d.ring === 'double' && d.segment === n);

export function replayDrill(settings: DrillSettings, events: GameEvent[]): DrillState {
  const s: DrillState = {
    settings,
    attempts: [],
    turn: null,
    complete: false,
    lastTurn: null,
    done: false,
    score: 0,
    aim: '',
    aimArea: null,
    round: 1,
    rounds: 1,
  };

  if (settings.kind === 'checkout') {
    const targets = checkoutTargets(settings.min, settings.max, settings.attempts, settings.seed);
    s.rounds = targets.length;
    let remaining = 0;
    const next = () => {
      const target = targets[s.attempts.length];
      if (target === undefined) return void (s.done = true);
      s.attempts.push({ target, turns: [], success: null });
      remaining = target;
    };
    next();
    for (const ev of events) {
      if (s.done) break;
      const attempt = s.attempts[s.attempts.length - 1];
      if (ev.type === 'confirm') {
        if (!s.turn) continue;
        s.lastTurn = s.turn;
        s.turn = null;
        s.complete = false;
        if (s.lastTurn.checkout) attempt.success = true;
        else if (attempt.turns.length >= settings.turns) attempt.success = false;
        if (attempt.success !== null) next();
        continue;
      }
      if (s.complete) continue;
      if (!s.turn) {
        s.turn = { target: String(attempt.target), darts: [], scoreBefore: remaining, scored: 0, bust: false, checkout: false };
        attempt.turns.push(s.turn);
      }
      const turn = s.turn;
      const d = ev.dart;
      turn.darts.push(d);
      const left = turn.scoreBefore - turn.scored - d.points;
      if (left < 0 || left === 1 || (left === 0 && !isDouble(d))) {
        turn.bust = true;
        turn.scored = 0;
        remaining = turn.scoreBefore;
        s.complete = true;
        continue;
      }
      turn.scored += d.points;
      remaining = left;
      turn.checkout = left === 0;
      s.complete = left === 0 || turn.darts.length === 3;
    }
    const attempt = s.attempts[s.attempts.length - 1];
    s.score = s.done ? 0 : remaining;
    s.round = s.attempts.length;
    s.aim = attempt ? String(remaining) : '';
    s.aimArea = null;
    return s;
  }

  // Scoring and Bob's 27 run as one attempt of fixed rounds.
  const bobs = settings.kind === 'bobs27';
  const targets = bobs ? BOBS_TARGETS : Array.from({ length: settings.rounds }, () => settings.segment);
  s.rounds = targets.length;
  const attempt: Attempt = { target: bobs ? 27 : 0, turns: [], success: null };
  s.attempts.push(attempt);
  let score = attempt.target;

  for (const ev of events) {
    if (s.done) break;
    const n = targets[attempt.turns.length - (s.turn ? 1 : 0)];
    if (ev.type === 'confirm') {
      if (!s.turn) continue;
      const turn = s.turn;
      if (bobs && !turn.darts.some((d) => bobsHit(n, d))) turn.scored = -bobsValue(n);
      score = turn.scoreBefore + turn.scored;
      s.lastTurn = turn;
      s.turn = null;
      s.complete = false;
      if (bobs && score <= 0) {
        s.done = true;
        attempt.success = false;
      } else if (attempt.turns.length >= targets.length) {
        s.done = true;
        attempt.success = bobs ? true : null;
      }
      continue;
    }
    if (s.complete) continue;
    if (!s.turn) {
      s.turn = { target: bobs ? doubleLabel(n) : scoringAim(n), darts: [], scoreBefore: score, scored: 0, bust: false, checkout: false };
      attempt.turns.push(s.turn);
    }
    const turn = s.turn;
    turn.darts.push(ev.dart);
    if (bobs) {
      const hits = turn.darts.filter((d) => bobsHit(n, d)).length;
      // A visit without a hit is charged on confirmation, so a correction can still save it.
      turn.scored = hits * bobsValue(n) - (hits === 0 && turn.darts.length === 3 ? bobsValue(n) : 0);
    } else {
      turn.scored += ev.dart.points;
    }
    s.complete = turn.darts.length === 3;
  }

  const index = Math.min(attempt.turns.length - (s.turn ? 1 : 0), targets.length - 1);
  const n = targets[index];
  s.round = index + 1;
  s.score = s.turn ? s.turn.scoreBefore + s.turn.scored : score;
  s.aim = s.done ? '' : bobs ? doubleLabel(n) : scoringAim(n);
  s.aimArea = s.done ? null : bobs ? doubleArea(n) : scoringArea(n);
  return s;
}

export interface DrillStats {
  darts: number;
  /** Three-dart average of everything thrown (scoring) or of the points that counted. */
  average: number;
  /** Checkout: attempts made and finished. */
  attempts: number;
  successes: number;
  /** Scoring: darts in the aimed-at segment, and in its treble (or the bull). */
  hits: number;
  bigHits: number;
  /** Bob's 27: doubles hit. */
  doubles: number;
}

export function drillStats(s: DrillState): DrillStats {
  const turns = s.attempts.flatMap((a) => a.turns);
  const darts = turns.flatMap((t) => t.darts);
  const points = darts.reduce((a, d) => a + d.points, 0);
  const st: DrillStats = {
    darts: darts.length,
    average: darts.length ? (points / darts.length) * 3 : 0,
    attempts: s.attempts.filter((a) => a.success !== null).length,
    successes: s.attempts.filter((a) => a.success).length,
    hits: 0,
    bigHits: 0,
    doubles: 0,
  };
  if (s.settings.kind === 'scoring') {
    const seg = s.settings.segment;
    st.hits = darts.filter((d) => d.segment === seg).length;
    st.bigHits = darts.filter((d) => d.segment === seg && (seg === 25 ? d.ring === 'bull' : d.ring === 'triple')).length;
  }
  if (s.settings.kind === 'bobs27') {
    st.doubles = turns.reduce((a, t, i) => a + t.darts.filter((d) => bobsHit(BOBS_TARGETS[i], d)).length, 0);
  }
  return st;
}
