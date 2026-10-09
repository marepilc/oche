import type { Dart } from './dart';
import type { GameEvent, Player } from './x01';

/**
 * Cricket: close 20–15 and the bull with three marks each (double = 2, treble = 3,
 * outer bull = 1, bull = 2). Marks on a number you have closed score its value
 * while someone else still has it open. The leg goes to the first player with
 * everything closed and at least as many points as anyone else.
 *
 * In cut-throat the extra points go to every opponent who has the number open,
 * and the lowest score wins.
 */
export interface CricketSettings {
  legsToWin: number;
  cutThroat: boolean;
}

/** Board numbers in scoreboard order; 25 is the bull. */
export const CRICKET_TARGETS = [20, 19, 18, 17, 16, 15, 25] as const;

export interface CricketTurn {
  player: number;
  darts: Dart[];
  /** Marks each dart made on a cricket number (0 for anything else). */
  marks: number[];
  pointsBefore: number;
  /** Points scored by the player (standard) or handed to opponents (cut-throat). */
  scored: number;
  /** The turn won the leg. */
  checkout: boolean;
}

export interface CricketLeg {
  starter: number;
  turns: CricketTurn[];
  winner: number | null;
}

export interface CricketState {
  settings: CricketSettings;
  players: Player[];
  legs: CricketLeg[];
  legsWon: number[];
  /** Marks per player and target (index into CRICKET_TARGETS), not capped at 3. */
  marks: number[][];
  points: number[];
  current: number;
  turn: CricketTurn | null;
  complete: boolean;
  lastTurn: CricketTurn | null;
  winner: number | null;
}

/** Which cricket number a dart hit and with how many marks. */
export function cricketHit(d: Dart): { target: number; marks: number } | null {
  const target = CRICKET_TARGETS.indexOf(d.segment as (typeof CRICKET_TARGETS)[number]);
  if (target < 0) return null;
  const marks = d.ring === 'triple' ? 3 : d.ring === 'double' || d.ring === 'bull' ? 2 : d.ring === 'miss' ? 0 : 1;
  return marks ? { target, marks } : null;
}

const value = (t: number) => (CRICKET_TARGETS[t] === 25 ? 25 : CRICKET_TARGETS[t]);

export function replayCricket(settings: CricketSettings, players: Player[], events: GameEvent[]): CricketState {
  const n = players.length;
  const s: CricketState = {
    settings,
    players,
    legs: [],
    legsWon: players.map(() => 0),
    marks: [],
    points: [],
    current: 0,
    turn: null,
    complete: false,
    lastTurn: null,
    winner: null,
  };

  const startLeg = (starter: number) => {
    s.legs.push({ starter, turns: [], winner: null });
    s.marks = players.map(() => CRICKET_TARGETS.map(() => 0));
    s.points = players.map(() => 0);
    s.current = starter;
  };
  const closedAll = (p: number) => s.marks[p].every((m) => m >= 3);
  const wins = (p: number) => {
    if (!closedAll(p)) return false;
    const others = s.points.filter((_, q) => q !== p);
    return settings.cutThroat ? others.every((x) => s.points[p] <= x) : others.every((x) => s.points[p] >= x);
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
      s.turn = { player: p, darts: [], marks: [], pointsBefore: s.points[p], scored: 0, checkout: false };
      s.legs[s.legs.length - 1].turns.push(s.turn);
    }
    const turn = s.turn;
    turn.darts.push(ev.dart);
    const hit = cricketHit(ev.dart);
    turn.marks.push(hit?.marks ?? 0);
    if (hit) {
      const { target, marks } = hit;
      const extra = Math.max(0, marks - Math.max(0, 3 - s.marks[p][target]));
      s.marks[p][target] += marks;
      const open = players.map((_, q) => q).filter((q) => q !== p && s.marks[q][target] < 3);
      if (extra && open.length) {
        const pts = extra * value(target);
        if (settings.cutThroat) for (const q of open) s.points[q] += pts;
        else s.points[p] += pts;
        turn.scored += settings.cutThroat ? pts * open.length : pts;
      }
    }
    turn.checkout = wins(p);
    s.complete = turn.checkout || turn.darts.length === 3;
  }
  return s;
}

export interface CricketStats {
  darts: number;
  marks: number;
  /** Marks per round of three darts. */
  mpr: number;
}

export function cricketStats(s: CricketState, p: number): CricketStats {
  const turns = s.legs.flatMap((l) => l.turns.filter((t) => t.player === p));
  const darts = turns.reduce((a, t) => a + t.darts.length, 0);
  const marks = turns.reduce((a, t) => a + t.marks.reduce((x, m) => x + m, 0), 0);
  return { darts, marks, mpr: darts ? (marks / darts) * 3 : 0 };
}
