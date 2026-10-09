import type { Dart } from './core/dart';
import { replayDrill, type DrillSettings } from './core/drills';
import { drillRecord, uuid7, x01Record, type GameRecord } from './core/record';
import { replay, type GameEvent, type Player, type X01Settings } from './core/x01';

/** A running X01 match: its event log and the state replayed from it. */
export class Match {
  readonly id = uuid7();
  readonly startedAt = new Date().toISOString();
  readonly settings: X01Settings;
  readonly players: Player[];
  events = $state<GameEvent[]>([]);
  state = $derived.by(() => replay(this.settings, this.players, this.events));
  record = $derived.by((): GameRecord => x01Record(this, this.state, this.events));

  constructor(settings: X01Settings, players: Player[]) {
    this.settings = settings;
    this.players = players;
  }

  /** Records a dart. Returns `false` when the turn is complete and must be confirmed first. */
  throw(d: Dart): boolean {
    if (this.state.winner !== null || this.state.complete) return false;
    this.events.push({ type: 'dart', dart: d });
    return true;
  }

  undo() {
    this.events.pop();
  }

  /** Ends the current turn (also with fewer than three darts). */
  confirm() {
    if (this.state.turn) this.events.push({ type: 'confirm', at: new Date().toISOString() });
  }
}

/** A running training drill for one player. */
export class Drill {
  readonly id = uuid7();
  readonly startedAt = new Date().toISOString();
  readonly settings: DrillSettings;
  readonly player: Player;
  events = $state<GameEvent[]>([]);
  state = $derived.by(() => replayDrill(this.settings, this.events));
  record = $derived.by((): GameRecord => drillRecord(this, this.player, this.state, this.events));

  constructor(settings: DrillSettings, player: Player) {
    this.settings = settings;
    this.player = player;
  }

  throw(d: Dart): boolean {
    if (this.state.done || this.state.complete) return false;
    this.events.push({ type: 'dart', dart: d });
    return true;
  }

  undo() {
    this.events.pop();
  }

  confirm() {
    if (this.state.turn) this.events.push({ type: 'confirm', at: new Date().toISOString() });
  }
}
