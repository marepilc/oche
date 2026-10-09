import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { uuid7, type GameRecord } from './core/record';
import type { DrillSettings } from './core/drills';
import type { Player, X01Settings } from './core/x01';

/** Outside Tauri (plain `vite dev` in a browser) nothing is stored. */
export const hasBackend = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export interface GameSummary {
  id: string;
  mode: GameRecord['mode'];
  settings: X01Settings | DrillSettings;
  startedAt: string;
  finishedAt: string | null;
  winnerId: string | null;
  legs: number;
  players: { id: string; name: string; legsWon: number; scored: number; darts: number; turns: number }[];
}

export interface RemoteConfig {
  enabled: boolean;
  host: string;
  port: number;
  database: string;
  user: string;
  ssl: 'disable' | 'prefer' | 'require';
}

export type RemoteContents =
  | { kind: 'empty' }
  | { kind: 'oche'; version: number; latest: number }
  | { kind: 'foreign'; tables: string[] };

export type SyncStatus =
  | { state: 'off' }
  | { state: 'synced' }
  | { state: 'pushing'; pending: number }
  | { state: 'error'; pending: number; message: string };

export const players = () => (hasBackend ? invoke<Player[]>('players_list') : Promise.resolve([]));

/** The stored player with this name, created when there is none. */
export const playerNamed = (name: string): Promise<Player> =>
  hasBackend ? invoke<Player>('player_named', { name }) : Promise.resolve({ id: uuid7(), name });

export const saveGame = (game: GameRecord) => (hasBackend ? invoke<void>('game_save', { game }) : Promise.resolve());
export const deleteGame = (id: string) => (hasBackend ? invoke<void>('game_delete', { id }) : Promise.resolve());
export const games = (limit = 100) =>
  hasBackend ? invoke<GameSummary[]>('games_list', { limit }) : Promise.resolve([] as GameSummary[]);

export const remoteGet = () => invoke<{ config: RemoteConfig; hasPassword: boolean }>('remote_get');
export const remoteTest = (config: RemoteConfig, password: string) =>
  invoke<RemoteContents>('remote_test', { config, password });
export const remoteConnect = (config: RemoteConfig, password: string) =>
  invoke<RemoteContents>('remote_connect', { config, password });
export const remoteDisconnect = () => invoke<void>('remote_disconnect');
export const syncNow = () => (hasBackend ? invoke<void>('sync_now') : Promise.resolve());

export const sync = $state<{ status: SyncStatus }>({ status: { state: 'off' } });

/** Keeps `sync.status` current. Returns a cleanup function. */
export function followSync(): () => void {
  if (!hasBackend) return () => {};
  invoke<SyncStatus>('sync_status').then((s) => {
    sync.status = s;
  });
  const off = listen<SyncStatus>('sync-status', (e) => {
    sync.status = e.payload;
  });
  return () => void off.then((f) => f());
}

/**
 * Saves `record()` whenever it changes. Call from a component; the effect lives as long as it does.
 * Games without a confirmed turn are not stored.
 */
export function autosave(record: () => GameRecord, onerror: (e: unknown) => void) {
  let last = '';
  // Saves run one after another, so an undo right after a confirm cannot be overtaken.
  let queue = Promise.resolve();
  $effect(() => {
    const r = record();
    const json = r.legs.length ? JSON.stringify(r) : '';
    if (json === last) return;
    // Undoing every turn takes the game out of the history again.
    const save = json ? () => saveGame(r) : () => deleteGame(r.id);
    last = json;
    queue = queue.then(save).catch(onerror);
  });
}
