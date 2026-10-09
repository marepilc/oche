import en from './locales/en';
import pl from './locales/pl';
import type { MessageKey, Messages, Plural } from './types';

/**
 * Available languages. To add one: copy locales/en.ts to locales/<code>.ts,
 * translate it (typed as `Messages`), and add it here.
 */
export const locales: Record<string, Messages> = { en, pl };
export type Locale = keyof typeof locales;

const KEY = 'oche:locale';
const DEFAULT: Locale = 'en';

function initial(): Locale {
  try {
    const saved = localStorage.getItem(KEY);
    if (saved && saved in locales) return saved;
  } catch {
    /* no storage */
  }
  return DEFAULT;
}

export const i18n = $state({ locale: initial() });

export function setLocale(locale: Locale) {
  i18n.locale = locale;
  try {
    localStorage.setItem(KEY, locale);
  } catch {
    /* not critical */
  }
}

function lookup(messages: Messages, key: string): string | Plural | undefined {
  let node: unknown = messages;
  for (const part of key.split('.')) node = (node as Record<string, unknown> | undefined)?.[part];
  return node as string | Plural | undefined;
}

/**
 * Translates `key` into the current language, falling back to English.
 * `{name}` placeholders come from `params`; plurals are picked by `params.count`.
 */
export function t(key: MessageKey, params: Record<string, string | number> = {}): string {
  const locale = i18n.locale;
  let msg = lookup(locales[locale], key) ?? lookup(en, key) ?? key;
  if (typeof msg !== 'string') {
    const category = new Intl.PluralRules(locale).select(Number(params.count ?? 0));
    msg = msg[category] ?? msg.other;
  }
  return msg.replace(/\{(\w+)\}/g, (m, name) => (name in params ? String(params[name]) : m));
}

/** Formats a number with one decimal place in the current language (51.8 / 51,8). */
export function decimal(value: number): string {
  return new Intl.NumberFormat(i18n.locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 }).format(value);
}
