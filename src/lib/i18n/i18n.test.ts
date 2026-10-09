import { describe, expect, it } from 'vitest';
import { decimal, setLocale, t } from './index.svelte';

describe('i18n', () => {
  it('defaults to English and fills placeholders', () => {
    expect(t('game.turn', { name: 'Ann' })).toBe('Turn · Ann');
    expect(t('game.dartsLeft', { count: 1 })).toBe('1 dart');
    expect(t('game.dartsLeft', { count: 2 })).toBe('2 darts');
  });

  it('uses Polish plural forms', () => {
    setLocale('pl');
    expect(t('game.dartsLeft', { count: 1 })).toBe('1 lotka');
    expect(t('game.dartsLeft', { count: 3 })).toBe('3 lotki');
    expect(t('game.dartsLeft', { count: 5 })).toBe('5 lotek');
    expect(decimal(51.8)).toBe('51,8');
    setLocale('en');
    expect(decimal(51.8)).toBe('51.8');
  });

  it('leaves unknown placeholders alone', () => {
    expect(t('newGame.keys')).toContain('{ctrlEnter}');
  });
});
