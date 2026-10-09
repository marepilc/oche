import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

interface OmarchyTheme {
  name: string | null;
  colors: Record<string, string>;
  mono_font: string | null;
}

/** colors.toml key → CSS custom property. */
const VARS: Record<string, string> = {
  background: '--bg',
  dark_background: '--bg-dark',
  darker_background: '--bg-darker',
  lighter_background: '--bg-light',
  foreground: '--fg',
  muted: '--muted',
  accent: '--accent',
  red: '--red',
  green: '--green',
  yellow: '--warn',
};

export const theme = $state({ name: null as string | null });

function apply(t: OmarchyTheme) {
  const root = document.documentElement;
  for (const [key, cssVar] of Object.entries(VARS)) {
    const value = t.colors[key];
    if (value) root.style.setProperty(cssVar, value);
    else root.style.removeProperty(cssVar);
  }
  root.dataset.mode = t.colors.mode === 'light' ? 'light' : 'dark';
  if (t.mono_font) root.style.setProperty('--font-ui', `"${t.mono_font}", ui-monospace, monospace`);
  theme.name = t.name;
}

/** Applies the active Omarchy theme and follows `omarchy theme set` / `omarchy font set`. */
export async function followOmarchyTheme() {
  if (!isTauri()) return;
  apply(await invoke<OmarchyTheme>('get_theme'));
  await listen<OmarchyTheme>('theme-changed', (e) => apply(e.payload));
}
