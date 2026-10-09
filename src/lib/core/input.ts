import { dart, type Dart } from './dart';

/**
 * Keyboard grammar for entering darts.
 *
 *   20↑  outer single    20↓  inner single    20⏎  single
 *   20d  double          20t  treble          (letters in either case)
 *   25   outer bull      50   bull            0 / m  miss
 *
 * The key after the number commits the dart, so "2" and "20" never clash.
 * 25, 50 and 0 cannot be anything else, so they commit as soon as they are typed.
 * Enter on an empty line confirms the turn.
 */
export interface InputState {
  buf: string;
}

/** Errors are codes; the UI translates them. */
export type InputError =
  | { code: 'notOnBoard'; value: string }
  | { code: 'numberFirst' }
  | { code: 'clearFirst' };

export type InputAction =
  | { type: 'none' }
  | { type: 'dart'; dart: Dart }
  | { type: 'error'; error: InputError }
  | { type: 'undo' }
  | { type: 'confirm' };

export const emptyInput: InputState = { buf: '' };

const isBoardNumber = (n: number) => (n >= 0 && n <= 20) || n === 25 || n === 50;

const none = (state: InputState) => ({ state, action: { type: 'none' } as InputAction });
const error = (state: InputState, error: InputError) => ({ state, action: { type: 'error', error } as InputAction });
const commit = (d: Dart) => ({ state: emptyInput, action: { type: 'dart', dart: d } as InputAction });

function digit(state: InputState, key: string) {
  const buf = state.buf + key;
  const n = Number(buf);
  if (buf.length > 2 || !isBoardNumber(n)) return error(state, { code: 'notOnBoard', value: buf });
  if (buf === '0') return commit(dart(0, 'miss'));
  if (n === 25) return commit(dart(25, 'obull'));
  if (n === 50) return commit(dart(25, 'bull'));
  return none({ buf });
}

function withArea(state: InputState, ring: 'inner' | 'outer' | 'single' | 'double' | 'triple') {
  if (!state.buf) return error(state, { code: 'numberFirst' });
  return commit(dart(Number(state.buf), ring));
}

/** Feeds one `KeyboardEvent.key` into the input. */
export function press(state: InputState, key: string): { state: InputState; action: InputAction } {
  if (/^[0-9]$/.test(key)) return digit(state, key);

  switch (key.length === 1 ? key.toLowerCase() : key) {
    case 'd':
      return withArea(state, 'double');
    case 't':
      return withArea(state, 'triple');
    case 'ArrowUp':
      return withArea(state, 'outer');
    case 'ArrowDown':
      return withArea(state, 'inner');
    case 'Enter':
      if (!state.buf) return { state, action: { type: 'confirm' } };
      return withArea(state, 'single');
    case 'm':
      if (state.buf) return error(state, { code: 'clearFirst' });
      return commit(dart(0, 'miss'));
    case 'Backspace':
      if (state.buf) return none({ buf: state.buf.slice(0, -1) });
      return { state, action: { type: 'undo' } };
    case 'Escape':
      return none(emptyInput);
    default:
      return none(state);
  }
}

/** Board areas to highlight while typing. */
export interface Preview {
  segment: number;
  rings: Array<'inner' | 'outer' | 'double' | 'triple' | 'obull' | 'bull'>;
}

export function preview(state: InputState): Preview | null {
  const n = Number(state.buf);
  if (!state.buf || n < 1 || n > 20) return null;
  return { segment: n, rings: ['inner', 'triple', 'outer', 'double'] };
}
