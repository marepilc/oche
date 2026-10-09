import type { Dart } from './dart';
import type { Preview } from './input';
import type { GameEvent, Player } from './x01';

/**
 * Around the Clock: hit 1, 2, … 20 in order, then the bull (either ring).
 * Any ring of the target counts; with `skips` a double moves two numbers on
 * and a treble three (never past 20, the bull still has to be hit).
 */
export interface AtcSettings {
  legsToWin: number;
  skips: boolean;
}

/** The targets in order; 25 is the bull. */
export const ATC_TARGETS = [...Array.from({ length: 20 }, (_, i) => i + 1), 25];

export interface AtcTurn {
  player: number;
  darts: Dart[];
  /** Whether each dart hit the target it was thrown at. */
  hit: boolean[];
  /** Index into ATC_TARGETS at the start of the turn. */
  before: number;
  /** Targets moved on during the turn. */
  advanced: number;
  hits: number;
  checkout: boolean;
}

export interface AtcLeg {
  starter: number;
  turns: AtcTurn[];
  winner: number | null;
}

export interface AtcState {
  settings: AtcSettings;
  players: Player[];
  legs: AtcLeg[];
  legsWon: number[];
  /** Index into ATC_TARGETS each player is on; ATC_TARGETS.length when finished. */
  progress: number[];
  current: number;
  turn: AtcTurn | null;
  complete: boolean;
  lastTurn: AtcTurn | null;
  winner: number | null;
}

export const atcLabel = (i: number) => (i >= ATC_TARGETS.length ? '✓' : ATC_TARGETS[i] === 25 ? 'BULL' : String(ATC_TARGETS[i]));

export function atcArea(i: number): Preview | null {
  if (i >= ATC_TARGETS.length) return null;
  const n = ATC_TARGETS[i];
  return n === 25 ? { segment: 25, rings: ['obull', 'bull'] } : { segment: n, rings: ['inner', 'triple', 'outer', 'double'] };
}

/** How far a dart moves a player on from target index `at`. */
function advance(settings: AtcSettings, at: number, d: Dart): number {
  const target = ATC_TARGETS[at];
  if (target === undefined || d.segment !== target || d.ring === 'miss') return 0;
  if (target === 25 || !settings.skips) return 1;
  const step = d.ring === 'triple' ? 3 : d.ring === 'double' ? 2 : 1;
  // Skipping never jumps the bull: the furthest it goes is onto it.
  return Math.min(step, ATC_TARGETS.length - 1 - at);
}

export function replayAtc(settings: AtcSettings, players: Player[], events: GameEvent[]): AtcState {
  const n = players.length;
  const s: AtcState = {
    settings,
    players,
    legs: [],
    legsWon: players.map(() => 0),
    progress: [],
    current: 0,
    turn: null,
    complete: false,
    lastTurn: null,
    winner: null,
  };
  const startLeg = (starter: number) => {
    s.legs.push({ starter, turns: [], winner: null });
    s.progress = players.map(() => 0);
    s.current = starter;
  };

  startLeg(0);
  for (const ev of events) {
    if (s.winner !== null) break;
    if (ev.type === 'confirm') {
      const turn = s.turn;
      if (!turn) continue;
      const leg = s.legs[s.legs.length - 1];
      s.lastTurn = turn;
      s.turn = null;
      s.complete = false;
      s.current = (s.current + 1) % n;
      if (turn.checkout) {
        leg.winner = turn.player;
        s.legsWon[turn.player]++;
        if (s.legsWon[turn.player] >= settings.legsToWin) s.winner = turn.player;
        else startLeg((leg.starter + 1) % n);
      }
      continue;
    }
    if (s.complete) continue;

    const p = s.current;
    if (!s.turn) {
      s.turn = { player: p, darts: [], hit: [], before: s.progress[p], advanced: 0, hits: 0, checkout: false };
      s.legs[s.legs.length - 1].turns.push(s.turn);
    }
    const turn = s.turn;
    turn.darts.push(ev.dart);
    const step = advance(settings, s.progress[p], ev.dart);
    turn.hit.push(step > 0);
    if (step) {
      turn.hits++;
      turn.advanced += step;
      s.progress[p] += step;
    }
    turn.checkout = s.progress[p] >= ATC_TARGETS.length;
    s.complete = turn.checkout || turn.darts.length === 3;
  }
  return s;
}

export function atcStats(s: AtcState, p: number) {
  const turns = s.legs.flatMap((l) => l.turns.filter((t) => t.player === p));
  const darts = turns.reduce((a, t) => a + t.darts.length, 0);
  const hits = turns.reduce((a, t) => a + t.hits, 0);
  return { darts, hits, rate: darts ? hits / darts : 0 };
}
