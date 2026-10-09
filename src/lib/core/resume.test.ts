import { describe, expect, it } from 'vitest';
import { replayAtc } from './atc';
import { replayCricket } from './cricket';
import { dart, type Dart } from './dart';
import { replayDrill, type DrillSettings } from './drills';
import { atcRecord, cricketRecord, drillRecord, x01Record, type GameRecord } from './record';
import { eventsFromRecord } from './resume';
import { replay, type GameEvent } from './x01';

const ev = (...items: (Dart | 'ok')[]): GameEvent[] =>
  items.map((it) => (it === 'ok' ? { type: 'confirm' } : { type: 'dart', dart: it }));
const players = [{ id: 'a', name: 'A' }, { id: 'b', name: 'B' }];
const meta = { id: 'g', startedAt: '2026-10-09T11:00:00.000Z' };
const T20 = dart(20, 'triple');
const miss = dart(0, 'miss');

/** Saving, rebuilding the events and saving again gives the same record. */
function roundTrip(record: (events: GameEvent[]) => GameRecord, events: GameEvent[]) {
  const saved = record(events);
  expect(record(eventsFromRecord(saved))).toEqual(saved);
}

describe('resuming a stored game', () => {
  it('rebuilds X01 across legs, busts, early confirms and a turn in progress', () => {
    const S = { start: 101, doubleIn: false, doubleOut: true, legsToWin: 3 };
    const events = ev(
      T20, dart(1, 'outer'), dart(20, 'double'), 'ok', // A: 101 checked out
      T20, T20, 'ok', // B starts leg 2: bust, confirmed after two darts
      dart(25, 'obull'), 'ok', // A: one dart, confirmed early
      T20, dart(5, 'inner'), // B: in progress, not saved
    );
    roundTrip((e) => x01Record(meta, replay(S, players, e), e), events);
    const resumed = replay(S, players, eventsFromRecord(x01Record(meta, replay(S, players, events), events)));
    expect(resumed.legsWon).toEqual([1, 0]);
    expect(resumed.remaining).toEqual([76, 101]);
    expect(resumed.current).toBe(1);
    expect(resumed.turn).toBeNull();
  });

  it('rebuilds Cricket and Around the Clock', () => {
    const c = { legsToWin: 1, cutThroat: true };
    roundTrip((e) => cricketRecord(meta, replayCricket(c, players, e), e), ev(T20, T20, dart(19, 'double'), 'ok', miss, 'ok', dart(25, 'bull')));
    const a = { legsToWin: 1, skips: true };
    roundTrip((e) => atcRecord(meta, replayAtc(a, players, e), e), ev(dart(1, 'triple'), dart(4, 'outer'), 'ok', miss, miss, miss, 'ok'));
  });

  it('rebuilds drills, keeping checkout targets', () => {
    const settings: DrillSettings[] = [
      { kind: 'checkout', min: 41, max: 80, attempts: 3, turns: 2, seed: 7 },
      { kind: 'scoring', segment: 20, rounds: 3 },
      { kind: 'bobs27' },
    ];
    for (const s of settings) {
      roundTrip((e) => drillRecord(meta, players[0], replayDrill(s, e), e), ev(T20, miss, dart(1, 'double'), 'ok', dart(20, 'double'), 'ok', miss));
    }
  });
});
