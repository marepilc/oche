<script lang="ts">
  import {
    hasBackend,
    remoteConnect,
    remoteDisconnect,
    remoteGet,
    remoteImport,
    remoteTest,
    sync,
    type RemoteConfig,
    type RemoteContents,
  } from '$lib/db.svelte';
  import { i18n, locales, setLocale, t } from '$lib/i18n/index.svelte';
  import { theme } from '$lib/theme.svelte';

  let cfg = $state<RemoteConfig>({ enabled: false, host: 'localhost', port: 5432, database: 'oche', user: 'oche', ssl: 'prefer' });
  let password = $state('');
  let hasPassword = $state(false);
  let busy = $state(false);
  let message = $state<{ text: string; bad: boolean } | null>(null);

  function load() {
    remoteGet()
      .then((r) => {
        cfg = r.config;
        hasPassword = r.hasPassword;
      })
      .catch(() => {});
  }
  if (hasBackend) load();

  function describe(c: RemoteContents): { text: string; bad: boolean } {
    if (c.kind === 'empty') return { text: t('settings.empty'), bad: false };
    if (c.kind === 'foreign') return { text: t('settings.foreign', { tables: c.tables.slice(0, 5).join(', ') }), bad: true };
    if (c.version > c.latest) return { text: t('settings.newer', { version: c.version, latest: c.latest }), bad: true };
    return { text: t('settings.oche', { version: c.version }), bad: false };
  }

  async function run(action: () => Promise<{ text: string; bad: boolean }>) {
    busy = true;
    message = { text: t('settings.testing'), bad: false };
    try {
      message = await action();
    } catch (e) {
      message = { text: String(e), bad: true };
    } finally {
      busy = false;
    }
  }

  const test = () => run(async () => describe(await remoteTest($state.snapshot(cfg), password)));
  const connect = () =>
    run(async () => {
      await remoteConnect($state.snapshot(cfg), password);
      password = '';
      load();
      return { text: t('settings.connected'), bad: false };
    });
  const pull = () =>
    run(async () => {
      const r = await remoteImport();
      return { text: t('settings.imported', { games: t('settings.nGames', { count: r.games }), players: t('settings.nPlayers', { count: r.players }) }), bad: false };
    });
  const disconnect = () =>
    run(async () => {
      await remoteDisconnect();
      load();
      return { text: t('settings.disconnected'), bad: false };
    });

  const status = $derived.by(() => {
    const st = sync.status;
    if (st.state === 'synced') return t('settings.statusSynced');
    if (st.state === 'pushing') return t('settings.statusPushing', { pending: st.pending });
    if (st.state === 'error') return t('settings.statusError', { message: st.message });
    return t('settings.statusOff');
  });
</script>

<div class="wrap">
  <h1>{t('settings.title')}</h1>

  <section>
    <div class="label">{t('settings.language')}</div>
    <div class="seg" role="radiogroup" aria-label={t('settings.language')}>
      {#each Object.entries(locales) as [code, messages] (code)}
        <label>
          <input type="radio" name="locale" value={code} checked={i18n.locale === code} onchange={() => setLocale(code)} />
          {messages.language}
        </label>
      {/each}
    </div>
    <div class="hint">{t('settings.languageHint')}</div>
  </section>

  <section>
    <div class="label">{t('settings.appearance')}</div>
    <div>{theme.name ? t('settings.themeActive', { name: theme.name }) : t('settings.noOmarchy')}</div>
    <div class="hint">{t('settings.themeHint')}</div>
  </section>

  <section>
    <div class="label">{t('settings.remote')}</div>
    <div class="hint">{t('settings.remoteHint')}</div>
    {#if !hasBackend}
      <div class="hint">{t('settings.desktopOnly')}</div>
    {:else}
      {#if cfg.enabled}
        <div>{t('settings.active', { user: cfg.user, host: cfg.host, database: cfg.database })}</div>
      {/if}
      <div class="form">
        <label class="wide">{t('settings.host')}<input bind:value={cfg.host} spellcheck="false" /></label>
        <label>{t('settings.port')}<input type="number" min="1" max="65535" bind:value={cfg.port} /></label>
        <label>{t('settings.database')}<input bind:value={cfg.database} spellcheck="false" /></label>
        <label>{t('settings.user')}<input bind:value={cfg.user} spellcheck="false" /></label>
        <label class="wide">
          {t('settings.password')}
          <input type="password" bind:value={password} placeholder={hasPassword ? t('settings.passwordStored') : ''} />
        </label>
        <label>
          {t('settings.ssl')}
          <select bind:value={cfg.ssl}>
            {#each ['disable', 'prefer', 'require'] as m (m)}<option value={m}>{m}</option>{/each}
          </select>
        </label>
      </div>
      <div class="buttons">
        <button class="btn ghost" disabled={busy} onclick={test}>{t('settings.test')}</button>
        <button class="btn" disabled={busy} onclick={connect}>{t(cfg.enabled ? 'settings.reconnect' : 'settings.connect')}</button>
        {#if cfg.enabled}
          <button class="btn ghost" disabled={busy} onclick={pull}>{t('settings.import')}</button>
          <button class="btn ghost" disabled={busy} onclick={disconnect}>{t('settings.disconnect')}</button>
        {/if}
      </div>
      {#if message}<div class:err={message.bad} class="msg">{message.text}</div>{/if}
      {#if cfg.enabled}<div class="hint">{t('settings.status', { status })}</div>{/if}
    {/if}
  </section>
</div>

<style>
  .wrap { max-width: 640px; margin-inline: auto; width: 100%; padding-block: 24px; display: grid; gap: 22px; align-content: start; height: 100%; overflow: auto; }
  h1 { margin: 0; font-family: var(--font-score); font-weight: 800; font-size: 44px; text-transform: uppercase; letter-spacing: 0.02em; }
  section { display: grid; gap: 8px; }
  .seg { display: flex; flex-wrap: wrap; gap: 6px; }
  .seg label { border: 1px solid var(--line); border-radius: var(--r); padding: 6px 12px; cursor: pointer; display: flex; gap: 6px; align-items: center; }
  .seg input { accent-color: var(--accent); margin: 0; }
  .seg label:has(input:checked) { border-color: var(--accent); background: var(--bg-light); }
  .seg label:has(input:focus-visible) { outline: 2px solid var(--accent); outline-offset: 2px; }
  .hint { color: var(--muted); font-size: 11px; }
  .form { display: grid; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: 8px 10px; }
  .form label { display: grid; gap: 3px; font-size: 11px; color: var(--muted); }
  .form .wide { grid-column: span 2; }
  .form input, .form select { background: var(--bg-dark); border: 1px solid var(--line); border-radius: var(--r); padding: 7px 10px; outline: none; color: var(--fg); font-size: 14px; min-width: 0; }
  .form input:focus, .form select:focus { border-color: var(--accent); }
  .buttons { display: flex; flex-wrap: wrap; gap: 8px; }
  .btn { background: var(--accent); color: var(--bg); border: 0; border-radius: var(--r); padding: 7px 16px; font-weight: 700; cursor: pointer; }
  .btn.ghost { background: none; color: var(--fg); border: 1px solid var(--line); }
  .btn:disabled { opacity: 0.4; cursor: default; }
  .msg { font-size: 12px; }
  .msg.err { color: var(--red); }
</style>
