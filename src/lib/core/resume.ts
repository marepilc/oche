import { dart, type Ring } from './dart';
import type { GameRecord } from './record';
import type { GameEvent } from './x01';

/**
 * The event log of a stored game: each saved turn's darts followed by its confirmation.
 * Replaying it gives back the state the game was saved in, so it can go on from there.
 * A turn that was being thrown when the game was left is not saved, so it starts again.
 */
export function eventsFromRecord(r: GameRecord): GameEvent[] {
  return r.legs.flatMap((leg) =>
    leg.turns.flatMap((turn): GameEvent[] => [
      ...turn.darts.map((d): GameEvent => ({ type: 'dart', dart: dart(d.segment, d.ring as Ring) })),
      { type: 'confirm' },
    ]),
  );
}
