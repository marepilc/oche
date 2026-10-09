import type { Dart } from './dart';
import type { DrillSettings, DrillState } from './drills';
import type { GameEvent, Player, X01Settings, X01State } from './x01';

/** A game as it is stored. Mirrors `GameRecord` in src-tauri/src/store/mod.rs. */
export interface GameRecord {
  id: string;
  mode: 'x01' | DrillSettings['kind'];
  settings: X01Settings | DrillSettings;
  startedAt: string;
  finishedAt: string | null;
  winnerId: string | null;
  players: string[];
  legs: LegRecord[];
}

export interface LegRecord {
  setNo: number;
  legNo: number;
  starterId: string | null;
  winnerId: string | null;
  turns: TurnRecord[];
}

export interface TurnRecord {
  playerId: string;
  turnNo: number;
  target: string | null;
  scoreBefore: number;
  scored: number;
  bust: boolean;
  checkout: boolean;
  darts: Dart[];
}

export interface RecordMeta {
  id: string;
  startedAt: string;
}

/** When the last turn was confirmed: the finish time of a game that is over. */
export function lastConfirmAt(events: GameEvent[]): string | null {
  for (let i = events.length - 1; i >= 0; i--) {
    const ev = events[i];
    if (ev.type === 'confirm') return ev.at ?? null;
  }
  return null;
}

const dartsOnly = (darts: Dart[]) => darts.map(({ segment, ring, points }) => ({ segment, ring, points }));

/** Confirmed turns only: the turn being thrown is saved once it is confirmed. */
export function x01Record(meta: RecordMeta, s: X01State, events: GameEvent[]): GameRecord {
  const id = (i: number | null) => (i === null ? null : s.players[i].id);
  const legs = s.legs
    .map((leg, k) => ({
      setNo: 1,
      legNo: k + 1,
      starterId: id(leg.starter),
      winnerId: id(leg.winner),
      turns: leg.turns
        .filter((t) => t !== s.turn)
        .map((t, n) => ({
          playerId: s.players[t.player].id,
          turnNo: n + 1,
          target: null,
          scoreBefore: t.scoreBefore,
          scored: t.scored,
          bust: t.bust,
          checkout: t.checkout,
          darts: dartsOnly(t.darts),
        })),
    }))
    .filter((leg) => leg.turns.length);
  return {
    id: meta.id,
    mode: 'x01',
    settings: s.settings,
    startedAt: meta.startedAt,
    finishedAt: s.winner === null ? null : lastConfirmAt(events),
    winnerId: id(s.winner),
    players: s.players.map((p) => p.id),
    legs,
  };
}

export function drillRecord(meta: RecordMeta, player: Player, s: DrillState, events: GameEvent[]): GameRecord {
  const legs = s.attempts
    .map((a, k) => ({
      setNo: 1,
      legNo: k + 1,
      starterId: player.id,
      winnerId: a.success ? player.id : null,
      turns: a.turns
        .filter((t) => t !== s.turn)
        .map((t, n) => ({
          playerId: player.id,
          turnNo: n + 1,
          target: t.target,
          scoreBefore: t.scoreBefore,
          scored: t.scored,
          bust: t.bust,
          checkout: t.checkout,
          darts: dartsOnly(t.darts),
        })),
    }))
    .filter((leg) => leg.turns.length);
  return {
    id: meta.id,
    mode: s.settings.kind,
    settings: s.settings,
    startedAt: meta.startedAt,
    finishedAt: s.done ? lastConfirmAt(events) : null,
    winnerId: null,
    players: [player.id],
    legs,
  };
}

/** UUID v7: time-ordered, like the ids the backend creates. */
export function uuid7(now = Date.now()): string {
  const b = crypto.getRandomValues(new Uint8Array(16));
  for (let i = 5, t = now; i >= 0; i--, t = Math.floor(t / 256)) b[i] = t % 256;
  b[6] = (b[6] & 0x0f) | 0x70;
  b[8] = (b[8] & 0x3f) | 0x80;
  const hex = Array.from(b, (x) => x.toString(16).padStart(2, '0')).join('');
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}
