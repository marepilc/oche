import { describe, expect, it } from 'vitest';
import { dart, type Dart } from './dart';
import { checkoutTargets, drillStats, replayDrill, type DrillSettings } from './drills';
import { drillRecord, uuid7, x01Record } from './record';
import { replay, type GameEvent } from './x01';

const ev = (...items: (Dart | 'ok')[]): GameEvent[] =>
  items.map((it) => (it === 'ok' ? { type: 'confirm', at: '2026-10-09T12:00:00.000Z' } : { type: 'dart', dart: it }));
const miss = dart(0, 'miss');

describe('checkout drill', () => {
  const S: DrillSettings = { kind: 'checkout', min: 40, max: 40, attempts: 2, turns: 2, seed: 1 };

  it('picks only finishes that exist, the same for the same seed', () => {
    const a = checkoutTargets(150, 170, 50, 7);
    expect(a).toEqual(checkoutTargets(150, 170, 50, 7));
    expect(a.every((n) => ![159, 162, 163, 165, 166, 168, 169].includes(n))).toBe(true);
    expect(new Set(a).size).toBeGreaterThan(1);
  });

  it('counts a finish as a success and moves to the next attempt', () => {
    const s = replayDrill(S, ev(dart(20, 'double'), 'ok'));
    expect(s.attempts[0].success).toBe(true);
    expect(s.attempts).toHaveLength(2);
    expect(s.score).toBe(40);
  });

  it('fails the attempt after its visits, busts keep the score', () => {
    const s = replayDrill(S, ev(dart(20, 'triple'), 'ok', miss, miss, miss, 'ok'));
    expect(s.attempts[0].turns[0].bust).toBe(true);
    expect(s.attempts[0].success).toBe(false);
    const done = replayDrill(S, ev(miss, 'ok', miss, 'ok', dart(20, 'double'), 'ok'));
    expect(done.done).toBe(true);
    expect(drillStats(done)).toMatchObject({ attempts: 2, successes: 1 });
  });
});

describe('scoring drill', () => {
  const S: DrillSettings = { kind: 'scoring', segment: 20, rounds: 2 };
  it('adds every point and counts hits', () => {
    const T20 = dart(20, 'triple');
    const s = replayDrill(S, ev(T20, T20, dart(1, 'outer'), 'ok', dart(20, 'outer'), miss, miss, 'ok'));
    expect(s.done).toBe(true);
    expect(s.score).toBe(141);
    expect(drillStats(s)).toMatchObject({ darts: 6, hits: 3, bigHits: 2, average: 70.5 });
  });
});

describe("Bob's 27", () => {
  const S: DrillSettings = { kind: 'bobs27' };
  it('adds hits and subtracts the double on a blank visit', () => {
    const s = replayDrill(S, ev(dart(1, 'double'), dart(1, 'double'), miss, 'ok', miss, miss, miss, 'ok'));
    expect(s.score).toBe(27 + 4 - 4);
    expect(s.aim).toBe('D3');
    expect(s.aimArea).toEqual({ segment: 3, rings: ['double'] });
  });

  it('charges a blank visit confirmed early', () => {
    expect(replayDrill(S, ev(miss, 'ok')).score).toBe(25);
  });

  it('ends when the score drops to zero', () => {
    const blank = (): (Dart | 'ok')[] => [miss, miss, miss, 'ok'];
    // 27 − 2 − 4 − 6 − 8 = 7, then −10 goes below zero.
    const s = replayDrill(S, ev(...blank(), ...blank(), ...blank(), ...blank(), ...blank()));
    expect(s.done).toBe(true);
    expect(s.score).toBe(-3);
    expect(s.attempts[0].success).toBe(false);
  });
});

describe('records', () => {
  const players = [{ id: 'a', name: 'A' }, { id: 'b', name: 'B' }];
  const meta = { id: 'g', startedAt: '2026-10-09T11:00:00.000Z' };

  it('keep confirmed turns only and finish with the match', () => {
    const settings = { start: 40, doubleIn: false, doubleOut: true, legsToWin: 1 };
    const events = ev(dart(20, 'outer'), 'ok', dart(1, 'outer'));
    let r = x01Record(meta, replay(settings, players, events), events);
    expect(r.legs[0].turns).toHaveLength(1);
    expect(r.finishedAt).toBeNull();

    const won = ev(dart(20, 'double'), 'ok');
    r = x01Record(meta, replay(settings, players, won), won);
    expect(r).toMatchObject({ winnerId: 'a', finishedAt: '2026-10-09T12:00:00.000Z' });
    expect(r.legs).toHaveLength(1);
    expect(r.legs[0]).toMatchObject({ winnerId: 'a', turns: [{ checkout: true, darts: [{ segment: 20, ring: 'double', points: 40 }] }] });
  });

  it('store drill targets', () => {
    const events = ev(dart(1, 'double'), 'ok');
    const r = drillRecord(meta, players[0], replayDrill({ kind: 'bobs27' }, events), events);
    expect(r.mode).toBe('bobs27');
    expect(r.legs[0].turns[0]).toMatchObject({ target: 'D1', scoreBefore: 27, scored: 2 });
  });
});

describe('uuid7', () => {
  it('is a version 7 UUID that starts with the time', () => {
    const id = uuid7(0x0190_1234_5678);
    expect(id).toMatch(/^01901234-5678-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  });
});
