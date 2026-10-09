import { replayAtc, type AtcSettings, type AtcState } from './core/atc';
import { replayCricket, type CricketSettings, type CricketState } from './core/cricket';
import type { Dart } from './core/dart';
import { replayDrill, type DrillSettings, type DrillState } from './core/drills';
import { atcRecord, cricketRecord, drillRecord, uuid7, x01Record, type GameRecord, type RecordMeta } from './core/record';
import { replay, type GameEvent, type Player, type X01Settings, type X01State } from './core/x01';

/** How a mode turns its event log into a state, and what that state says about the turn. */
interface Rules<S> {
  replay(events: GameEvent[]): S;
  record(meta: RecordMeta, s: S, events: GameEvent[]): GameRecord;
  /** The game is over: no more darts. */
  over(s: S): boolean;
  /** The turn is complete and waits for confirmation. */
  complete(s: S): boolean;
  /** A turn is being thrown. */
  inTurn(s: S): boolean;
}

/**
 * A game being played: its event log and the state replayed from it.
 * Undo is "drop the last event", and the same log is what gets saved.
 */
export class Session<S> {
  readonly id = uuid7();
  readonly startedAt = new Date().toISOString();
  events = $state<GameEvent[]>([]);
  state: S = $derived.by(() => this.rules.replay(this.events));
  record = $derived.by((): GameRecord => this.rules.record(this, this.state, this.events));
  over = $derived.by(() => this.rules.over(this.state));
  private rules: Rules<S>;

  constructor(rules: Rules<S>) {
    this.rules = rules;
  }

  /** Records a dart. Returns `false` when the turn is complete and must be confirmed first. */
  throw(d: Dart): boolean {
    if (this.over || this.rules.complete(this.state)) return false;
    this.events.push({ type: 'dart', dart: d });
    return true;
  }

  undo() {
    this.events.pop();
  }

  /** Ends the current turn (also with fewer than three darts). */
  confirm() {
    if (this.rules.inTurn(this.state)) this.events.push({ type: 'confirm', at: new Date().toISOString() });
  }
}

type MatchState = { winner: number | null; complete: boolean; turn: unknown };
const matchRules = <S extends MatchState>(replay: (e: GameEvent[]) => S, record: Rules<S>['record']): Rules<S> => ({
  replay,
  record,
  over: (s) => s.winner !== null,
  complete: (s) => s.complete,
  inTurn: (s) => !!s.turn,
});

export class Match extends Session<X01State> {
  readonly kind = 'x01';
  readonly settings: X01Settings;
  readonly players: Player[];
  constructor(settings: X01Settings, players: Player[]) {
    super(matchRules((e) => replay(settings, players, e), x01Record));
    this.settings = settings;
    this.players = players;
  }
}

export class CricketMatch extends Session<CricketState> {
  readonly kind = 'cricket';
  readonly settings: CricketSettings;
  readonly players: Player[];
  constructor(settings: CricketSettings, players: Player[]) {
    super(matchRules((e) => replayCricket(settings, players, e), cricketRecord));
    this.settings = settings;
    this.players = players;
  }
}

export class AtcMatch extends Session<AtcState> {
  readonly kind = 'atc';
  readonly settings: AtcSettings;
  readonly players: Player[];
  constructor(settings: AtcSettings, players: Player[]) {
    super(matchRules((e) => replayAtc(settings, players, e), atcRecord));
    this.settings = settings;
    this.players = players;
  }
}

/** A training drill for one player. */
export class Drill extends Session<DrillState> {
  readonly kind = 'drill';
  readonly settings: DrillSettings;
  readonly player: Player;
  constructor(settings: DrillSettings, player: Player) {
    super({
      replay: (e) => replayDrill(settings, e),
      record: (meta, s, e) => drillRecord(meta, player, s, e),
      over: (s) => s.done,
      complete: (s) => s.complete,
      inTurn: (s) => !!s.turn,
    });
    this.settings = settings;
    this.player = player;
  }
}

export type AnySession = Match | CricketMatch | AtcMatch | Drill;
