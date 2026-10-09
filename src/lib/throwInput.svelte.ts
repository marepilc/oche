import type { Dart } from './core/dart';
import { emptyInput, press, type InputError } from './core/input';
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

  /** A dart picked by clicking the board. */
  pick(d: Dart) {
    this.input = emptyInput;
    if (!this.#target().throw(d)) this.showError('input.turnComplete');
  }
}
