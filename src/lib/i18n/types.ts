import type en from './locales/en';

/** A message that depends on a number; keys are `Intl.PluralRules` categories. */
export type Plural = Partial<Record<Intl.LDMLPluralRule, string>> & { other: string };

type Widen<T> = T extends string ? string : T extends Plural ? Plural : { [K in keyof T]: Widen<T[K]> };

/** The shape every locale must have: the English messages with any strings allowed. */
export type Messages = Widen<typeof en>;

/** Dotted paths to every message, e.g. `game.turn`. */
export type MessageKey = Leaves<typeof en>;
type Leaves<T, P extends string = ''> = {
  [K in keyof T & string]: T[K] extends string | Plural ? `${P}${K}` : Leaves<T[K], `${P}${K}.`>;
}[keyof T & string];
