import { describe, expect, it } from 'vitest';
import { checkout, checkouts } from './checkout';
import { dart, routeLabel, type Dart } from './dart';
import { emptyInput, press, type InputAction } from './input';
import { playerStats, replay, type GameEvent, type X01Settings } from './x01';

function type(keys: string[]): InputAction[] {
  let state = emptyInput;
  const actions: InputAction[] = [];
  for (const k of keys) {
    const r = press(state, k);
    state = r.state;
    if (r.action.type !== 'none') actions.push(r.action);
  }
  return actions;
}
const typed = (...keys: string[]) => {
  const a = type(keys);
  expect(a).toHaveLength(1);
  return a[0].type === 'dart' ? a[0].dart : a[0];
};

describe('input grammar', () => {
  it('reads singles with area', () => {
    expect(typed('2', '0', 'ArrowUp')).toEqual(dart(20, 'outer'));
    expect(typed('2', '0', 'ArrowDown')).toEqual(dart(20, 'inner'));
    expect(typed('7', 'Enter')).toEqual(dart(7, 'single'));
    expect(typed('2', 'ArrowUp')).toEqual(dart(2, 'outer'));
  });

  it('reads doubles and trebles with the letter last, in either case', () => {
    expect(typed('2', '0', 't')).toMatchObject({ ring: 'triple', points: 60 });
    expect(typed('2', '0', 'T')).toMatchObject({ ring: 'triple', points: 60 });
    expect(typed('1', '6', 'd')).toMatchObject({ ring: 'double', points: 32 });
    expect(typed('1', 'T')).toMatchObject({ ring: 'triple', points: 3 });
    expect(typed('1', '1', 't')).toMatchObject({ ring: 'triple', points: 33 });
    expect(typed('2', 'D')).toMatchObject({ ring: 'double', points: 4 });
  });

  it('commits bulls and misses without Enter', () => {
    expect(typed('2', '5')).toMatchObject({ ring: 'obull', points: 25 });
    expect(typed('5', '0')).toMatchObject({ ring: 'bull', points: 50 });
    expect(typed('0')).toMatchObject({ ring: 'miss', points: 0 });
    expect(typed('m')).toMatchObject({ ring: 'miss', points: 0 });
  });

  it('rejects input that is not on the board', () => {
    expect(typed('2', '1')).toMatchObject({ type: 'error', error: { code: 'notOnBoard', value: '21' } });
    expect(typed('d')).toMatchObject({ type: 'error', error: { code: 'numberFirst' } });
    expect(typed('ArrowUp')).toMatchObject({ type: 'error', error: { code: 'numberFirst' } });
    expect(typed('2', 'm')).toMatchObject({ type: 'error', error: { code: 'clearFirst' } });
  });

  it('keeps the buffer after an error', () => {
    expect(type(['2', '0', '7', 'ArrowUp']).map((a) => a.type)).toEqual(['error', 'dart']);
  });

  it('Enter on an empty line confirms the turn', () => {
    expect(type(['Enter'])).toEqual([{ type: 'confirm' }]);
  });

  it('backspace edits, then undoes', () => {
    expect(type(['2', '0', 'Backspace', 'ArrowUp'])).toEqual([{ type: 'dart', dart: dart(2, 'outer') }]);
    expect(type(['Backspace'])).toEqual([{ type: 'undo' }]);
  });
});

const S: X01Settings = { start: 501, doubleIn: false, doubleOut: true, legsToWin: 2 };
const players = [{ id: 'a', name: 'A' }, { id: 'b', name: 'B' }];
const T20 = dart(20, 'triple');
const OK: GameEvent = { type: 'confirm' };
const ev = (...ds: (Dart | 'ok')[]): GameEvent[] => ds.map((d) => (d === 'ok' ? OK : { type: 'dart', dart: d }));

describe('x01', () => {
  it('waits for confirmation after three darts', () => {
    const s = replay(S, players, ev(T20, T20, T20));
    expect(s.complete).toBe(true);
    expect(s.current).toBe(0);
    expect(s.remaining).toEqual([321, 501]);
  });

  it('ignores darts until the turn is confirmed', () => {
    const s = replay(S, players, ev(T20, T20, T20, T20));
    expect(s.turn?.darts).toHaveLength(3);
  });

  it('alternates turns after confirmation', () => {
    const s = replay(S, players, ev(T20, T20, T20, 'ok', dart(20, 'outer')));
    expect(s.remaining).toEqual([321, 481]);
    expect(s.current).toBe(1);
    expect(s.turn?.darts).toHaveLength(1);
    expect(s.lastTurn?.scored).toBe(180);
  });

  it('confirms a turn early', () => {
    const s = replay(S, players, ev(T20, 'ok'));
    expect(s.current).toBe(1);
    expect(s.turn).toBeNull();
  });

  it('busts below zero and on 1, keeping the score', () => {
    const s = replay({ ...S, start: 41 }, players, ev(dart(20, 'outer'), dart(20, 'outer')));
    expect(s.complete).toBe(true);
    expect(s.turn?.bust).toBe(true);
    expect(s.remaining).toEqual([41, 41]);
    expect(replay({ ...S, start: 41 }, players, ev(dart(20, 'outer'), dart(20, 'outer'), 'ok')).current).toBe(1);
  });

  it('busts when finishing on a single with double out', () => {
    const s = replay({ ...S, start: 20 }, players, ev(dart(20, 'outer')));
    expect(s.turn?.bust).toBe(true);
    expect(s.remaining[0]).toBe(20);
  });

  it('awards the leg only after the checkout is confirmed', () => {
    const pending = replay({ ...S, start: 40 }, players, ev(dart(20, 'double')));
    expect(pending.complete).toBe(true);
    expect(pending.turn?.checkout).toBe(true);
    expect(pending.legsWon).toEqual([0, 0]);

    const s = replay({ ...S, start: 40 }, players, ev(dart(20, 'double'), 'ok'));
    expect(s.legsWon).toEqual([1, 0]);
    expect(s.legs).toHaveLength(2);
    expect(s.current).toBe(1);
    expect(s.remaining).toEqual([40, 40]);
  });

  it('finishes the match', () => {
    const D20 = dart(20, 'double');
    const s = replay({ ...S, start: 40 }, players, ev(D20, 'ok', D20, 'ok', D20, 'ok'));
    // A, B, A each check out on their first dart: A wins 2–1.
    expect(s.legsWon).toEqual([2, 1]);
    expect(s.winner).toBe(0);
  });

  it('allows any finish without double out', () => {
    const s = replay({ ...S, start: 20, doubleOut: false, legsToWin: 1 }, players, ev(dart(20, 'outer'), 'ok'));
    expect(s.winner).toBe(0);
  });

  it('ignores points before the double with double in', () => {
    const s = replay({ ...S, doubleIn: true }, players, ev(T20, dart(10, 'double'), T20));
    expect(s.remaining[0]).toBe(501 - 20 - 60);
  });

  it('computes averages', () => {
    const one = dart(1, 'outer');
    const s = replay(S, players, ev(T20, T20, T20, 'ok', one, one, one));
    expect(playerStats(s, 0)).toMatchObject({ darts: 3, average: 180, best: 180 });
    expect(playerStats(s, 1).average).toBe(3);
  });
});

describe('checkout', () => {
  const route = (r: number, left = 3) => checkout(r, left)?.map(routeLabel).join(' ') ?? null;
  it('suggests standard finishes', () => {
    expect(route(170)).toBe('T20 T20 BULL');
    expect(route(100)).toBe('T20 D20');
    expect(route(40)).toBe('D20');
    expect(route(120)).toBe('T20 S20 D20');
    expect(route(59)).toBe('S19 D20');
  });
  it('offers alternatives that finish on different doubles', () => {
    const alts = (r: number, left = 3) => checkouts(r, left).map((rt) => rt.map(routeLabel).join(' '));
    expect(alts(100)).toEqual(['T20 D20', 'BULL BULL', 'T20 S8 D16']);
    expect(alts(40)).toEqual(['D20', 'S8 D16', 'S4 D18']);
    expect(alts(170)).toEqual(['T20 T20 BULL']);
    expect(alts(100, 1)).toEqual([]);
  });

  it('finishes on any dart without double out', () => {
    expect(checkout(20, 1, false)?.map(routeLabel)).toEqual(['S20']);
    expect(checkout(180, 3, false)?.map(routeLabel)).toEqual(['T20', 'T20', 'T20']);
  });

  it('knows impossible finishes', () => {
    expect(route(169)).toBeNull();
    expect(route(171)).toBeNull();
    expect(route(1)).toBeNull();
    expect(route(100, 1)).toBeNull();
  });
});
