import { describe, expect, it } from 'vitest';
import { atcStats, replayAtc } from './atc';
import { cricketStats, replayCricket } from './cricket';
import { dart, type Dart } from './dart';
import { atcRecord, cricketRecord } from './record';
import type { GameEvent } from './x01';

const ev = (...items: (Dart | 'ok')[]): GameEvent[] =>
  items.map((it) => (it === 'ok' ? { type: 'confirm', at: '2026-10-09T12:00:00.000Z' } : { type: 'dart', dart: it }));
const players = [{ id: 'a', name: 'A' }, { id: 'b', name: 'B' }];
const miss = dart(0, 'miss');
const T = (n: number) => (n === 25 ? dart(25, 'bull') : dart(n, 'triple'));
const meta = { id: 'g', startedAt: '2026-10-09T11:00:00.000Z' };

describe('cricket', () => {
  const S = { legsToWin: 1, cutThroat: false };

  it('counts marks and scores only on numbers others have open', () => {
    const s = replayCricket(S, players, ev(T(20), dart(20, 'double'), dart(19, 'outer')));
    expect(s.marks[0][0]).toBe(5);
    expect(s.marks[0][1]).toBe(1);
    expect(s.points).toEqual([40, 0]);
    expect(s.complete).toBe(true);
  });

  it('does not score on a number everyone has closed', () => {
    const s = replayCricket(S, players, ev(T(20), 'ok', T(20), 'ok', T(20)));
    expect(s.points).toEqual([0, 0]);
  });

  it('gives points to open opponents in cut-throat', () => {
    const s = replayCricket({ ...S, cutThroat: true }, players, ev(T(20), T(20)));
    expect(s.points).toEqual([0, 60]);
    expect(s.turn?.scored).toBe(60);
  });

  it('bulls count two marks, the outer bull one', () => {
    const s = replayCricket(S, players, ev(dart(25, 'bull'), dart(25, 'obull')));
    expect(s.marks[0][6]).toBe(3);
  });

  it('wins with everything closed and enough points, after confirming', () => {
    // A closes 20–18, B misses, A closes 17–15, B misses, A closes the bull with a bull and an outer bull.
    const closeAll: (Dart | 'ok')[] = [T(20), T(19), T(18), 'ok', miss, 'ok', T(17), T(16), T(15), 'ok', miss, 'ok', T(25), dart(25, 'obull')];
    const s = replayCricket(S, players, ev(...closeAll));
    expect(s.turn?.checkout).toBe(true);
    expect(s.winner).toBeNull();
    const done = replayCricket(S, players, ev(...closeAll, 'ok'));
    expect(done.winner).toBe(0);
    expect(cricketStats(done, 0)).toMatchObject({ marks: 21, darts: 8 });
    const r = cricketRecord(meta, done, ev(...closeAll, 'ok'));
    expect(r).toMatchObject({ mode: 'cricket', winnerId: 'a' });
    expect(r.legs[0].turns.at(-1)).toMatchObject({ checkout: true, playerId: 'a' });
  });

  it('does not win while behind on points', () => {
    // B scores 60 on 20 before A closes the rest.
    const s = replayCricket(S, players, ev(miss, 'ok', T(20), T(20), 'ok', T(20), T(19), T(18), 'ok', miss, 'ok', T(17), T(16), T(15), 'ok', miss, 'ok', T(25)));
    expect(s.points).toEqual([0, 60]);
    expect(s.turn?.checkout).toBe(false);
  });
});

describe('around the clock', () => {
  const S = { legsToWin: 1, skips: false };

  it('moves on one number per hit', () => {
    const s = replayAtc(S, players, ev(dart(1, 'outer'), dart(2, 'triple'), dart(4, 'outer')));
    expect(s.progress).toEqual([2, 0]);
    expect(s.turn).toMatchObject({ hits: 2, advanced: 2 });
  });

  it('skips with doubles and trebles but never past the bull', () => {
    const s = replayAtc({ ...S, skips: true }, players, ev(dart(1, 'triple'), dart(4, 'double')));
    expect(s.progress[0]).toBe(5);
    const trebles = [1, 4, 7, 10, 13, 16, 19].map((n) => dart(n, 'triple'));
    const end = replayAtc({ ...S, skips: true }, [players[0]], ev(...trebles.flatMap((d, i) => (i % 3 === 2 ? [d, 'ok' as const] : [d]))));
    expect(end.progress[0]).toBe(20);
  });

  it('finishes on the bull', () => {
    const darts = [...Array.from({ length: 20 }, (_, i) => dart(i + 1, 'outer')), dart(25, 'obull')];
    const events = ev(...darts.flatMap((d, i) => (i % 3 === 2 ? [d, 'ok' as const] : [d])), 'ok');
    const s = replayAtc(S, [players[0]], events);
    expect(s.winner).toBe(0);
    expect(atcStats(s, 0)).toMatchObject({ darts: 21, hits: 21, rate: 1 });
    expect(atcRecord(meta, s, events).legs[0].turns[0]).toMatchObject({ target: '1', scoreBefore: 0, scored: 3 });
  });
});
