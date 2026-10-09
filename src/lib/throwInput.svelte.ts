import type { Dart } from './core/dart';
import { emptyInput, press, type InputError } from './core/input';
import { t } from './i18n/index.svelte';
import type { MessageKey } from './i18n/types';

/** Anything darts can be thrown into: a match or a training drill. */
export interface Throwable {
  /** Returns `false` when the turn is complete and must be confirmed first. */
  throw(d: Dart): boolean;
  undo(): void;
  confirm(): void;
}

const ERRORS: Record<InputError['code'], MessageKey> = {
  notOnBoard: 'input.notOnBoard',
  numberFirst: 'input.numberFirst',
  clearFirst: 'input.clearFirst',
};

/** The command line under the board: typed keys become darts, undo and confirm. */
export class ThrowInput {
  input = $state(emptyInput);
  error = $state<{ key: MessageKey; params?: Record<string, string> } | null>(null);
  #timer: ReturnType<typeof setTimeout> | undefined;
  #target: () => Throwable;

  constructor(target: () => Throwable) {
    this.#target = target;
  }

  showError(key: MessageKey, params?: Record<string, string>) {
    this.error = { key, params };
    clearTimeout(this.#timer);
    this.#timer = setTimeout(() => (this.error = null), 1800);
  }

  /** Handles a window keydown; ignores modified keys and keys typed into form fields. */
  key(e: KeyboardEvent) {
    if (e.ctrlKey || e.altKey || e.metaKey) return;
    if ((e.target as HTMLElement | null)?.closest?.('input, select, textarea')) return;
    const target = this.#target();
    const { state, action } = press(this.input, e.key);
    if (action.type !== 'none' || state !== this.input) e.preventDefault();
    this.input = state;
    switch (action.type) {
      case 'dart':
        this.error = null;
        if (!target.throw(action.dart)) this.showError('input.turnComplete');
        break;
      case 'undo':
        target.undo();
        break;
      case 'confirm':
        target.confirm();
        break;
      case 'error':
        this.showError(ERRORS[action.error.code], 'value' in action.error ? { value: action.error.value } : undefined);
        break;
    }
  }

  /** The hint next to the prompt: what can be typed now. */
  hint(thrown: number, complete: boolean, bust = false): string {
    if (complete) return t(bust ? 'game.confirmBust' : 'game.confirmTurn');
    const buf = this.input.buf;
    if (!buf) return t('input.dartOf', { n: thrown + 1 }) + (thrown ? `   ·   ${t('input.confirmHint')}` : '');
    if (buf === '1') return `${t('input.single')}   ·   ${t('input.grow1')}`;
    if (buf === '2') return `${t('input.single')}   ·   ${t('input.grow2')}`;
    return t('input.single');
  }

  /** Window keydown for a game screen: once the game is over, ⏎ leaves and ⌫ undoes. */
  gameKey(e: KeyboardEvent, over: boolean, ondone: () => void) {
    if (over && !e.ctrlKey && !e.altKey && !e.metaKey) {
      if (e.key === 'Enter') {
        e.preventDefault();
        ondone();
      } else if (e.key === 'Backspace') {
        e.preventDefault();
        this.#target().undo();
      }
      return;
    }
    this.key(e);
  }

  /** A dart picked by clicking the board. */
  pick(d: Dart) {
    this.input = emptyInput;
    if (!this.#target().throw(d)) this.showError('input.turnComplete');
  }
}
