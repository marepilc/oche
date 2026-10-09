import { isDouble, type Dart } from './dart';

export interface X01Settings {
  start: number;
  doubleIn: boolean;
  doubleOut: boolean;
  /** Legs needed to win the match (first to N). */
  legsToWin: number;
}

export interface Player {
  id: string;
  name: string;
}

/**
 * The game is stored as the list of things that happened; the state is derived by replaying it.
 * Undo is "drop the last event", and the same list is what gets persisted.
 *
 * A turn ends only when it is confirmed, so a mistyped last dart can still be corrected.
 * Confirming early ends the turn with fewer than three darts.
 * `at` (ISO time of the confirmation) is only kept for the history.
 */
export type GameEvent = { type: 'dart'; dart: Dart } | { type: 'confirm'; at?: string };

export interface Turn {
  player: number;
  darts: Dart[];
  scoreBefore: number;
  /** Points that counted (0 on a bust, and for darts thrown before a double-in). */
  scored: number;
  bust: boolean;
  checkout: boolean;
}

export interface Leg {
  starter: number;
  turns: Turn[];
  winner: number | null;
}

export interface X01State {
  settings: X01Settings;
  players: Player[];
  legs: Leg[];
  legsWon: number[];
  /** Remaining score in the current leg, including darts already thrown this turn. */
  remaining: number[];
  /** Whose turn it is. */
  current: number;
  /** The turn being thrown, or `null` between turns. */
  turn: Turn | null;
  /** The turn is over (three darts, bust or checkout) and waits for confirmation. */
  complete: boolean;
  /** The turn confirmed most recently, shown while the next player steps up. */
  lastTurn: Turn | null;
  /** Match winner. */
  winner: number | null;
}

export function replay(settings: X01Settings, players: Player[], events: GameEvent[]): X01State {
  const n = players.length;
  const s: X01State = {
    settings,
    players,
    legs: [],
    legsWon: players.map(() => 0),
    remaining: [],
    current: 0,
    turn: null,
    complete: false,
    lastTurn: null,
    winner: null,
  };
  let opened: boolean[] = [];

  const startLeg = (starter: number) => {
    s.legs.push({ starter, turns: [], winner: null });
    s.remaining = players.map(() => settings.start);
    opened = players.map(() => !settings.doubleIn);
    s.current = starter;
  };

  const confirm = (turn: Turn) => {
    const leg = s.legs[s.legs.length - 1];
    s.lastTurn = turn;
    s.turn = null;
    s.complete = false;
    s.current = (s.current + 1) % n;
    if (!turn.checkout) return;
    leg.winner = turn.player;
    s.legsWon[turn.player]++;
    if (s.legsWon[turn.player] >= settings.legsToWin) s.winner = turn.player;
    else startLeg((leg.starter + 1) % n);
  };

  startLeg(0);

  for (const ev of events) {
    if (s.winner !== null) break;

    if (ev.type === 'confirm') {
      if (s.turn) confirm(s.turn);
      continue;
    }
    if (s.complete) continue;

    const p = s.current;
    if (!s.turn) {
      s.turn = { player: p, darts: [], scoreBefore: s.remaining[p], scored: 0, bust: false, checkout: false };
      s.legs[s.legs.length - 1].turns.push(s.turn);
    }
    const turn = s.turn;
    const d = ev.dart;
    turn.darts.push(d);

    let counted = d.points;
    if (!opened[p]) {
      if (isDouble(d)) opened[p] = true;
      else counted = 0;
    }
    const left = turn.scoreBefore - turn.scored - counted;
    const bust =
      left < 0 ||
      (settings.doubleOut && left === 1) ||
      (left === 0 && settings.doubleOut && !isDouble(d));

    if (bust) {
      turn.bust = true;
      turn.scored = 0;
      s.remaining[p] = turn.scoreBefore;
      s.complete = true;
      continue;
    }

    turn.scored += counted;
    s.remaining[p] = left;
    if (left === 0) turn.checkout = true;
    s.complete = left === 0 || turn.darts.length === 3;
  }

  return s;
}

export interface PlayerStats {
  darts: number;
  /** Three-dart average over the whole match. */
  average: number;
  /** Average of the first three turns of the current leg. */
  first9: number;
  best: number;
  checkouts: number;
}

export function playerStats(s: X01State, p: number): PlayerStats {
  const all = s.legs.flatMap((l) => l.turns.filter((t) => t.player === p));
  const darts = all.reduce((a, t) => a + t.darts.length, 0);
  const scored = all.reduce((a, t) => a + t.scored, 0);
  const leg = s.legs[s.legs.length - 1];
  const first = leg.turns.filter((t) => t.player === p).slice(0, 3);
  const firstDarts = first.reduce((a, t) => a + t.darts.length, 0);
  return {
    darts,
    average: darts ? (scored / darts) * 3 : 0,
    first9: firstDarts ? (first.reduce((a, t) => a + t.scored, 0) / firstDarts) * 3 : 0,
    best: Math.max(0, ...all.map((t) => t.scored)),
    checkouts: all.filter((t) => t.checkout).length,
  };
}
