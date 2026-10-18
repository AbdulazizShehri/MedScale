import { createContext, useContext } from "react";
import type { RouteId } from "../routes";

export type WorkspaceInfo = {
  kind: "synthetic" | "encrypted";
  label: string;
  vault_id: string;
  subjects: number;
  sources: number;
};

export type WorkspaceStatus = {
  open: boolean;
  workspace: WorkspaceInfo | null;
  encrypted_vault_exists: boolean;
  version: string;
  local_only: boolean;
  real_phi_authorized: boolean;
  network_default_deny: boolean;
};

export type Theme = "dark" | "light";
export type Density = "standard" | "compact";

export type Nav = { route: RouteId; subject?: string };

export type AppCtx = {
  ws: WorkspaceStatus | null;
  refreshWorkspace: () => Promise<void>;
  nav: Nav;
  navigate: (route: RouteId, subject?: string) => void;
  project: string | null;
  setProject: (id: string | null) => void;
  theme: Theme;
  setTheme: (t: Theme) => void;
  density: Density;
  setDensity: (d: Density) => void;
  lock: () => Promise<void>;
  openPalette: () => void;
};

export const App = createContext<AppCtx | null>(null);

export function useApp(): AppCtx {
  const ctx = useContext(App);
  if (!ctx) throw new Error("MedScale app context missing");
  return ctx;
}

/** Per-viewer UI preferences only (theme, density, last project). Never data. */
export function readPref(key: string): string | null {
  try { return window.localStorage.getItem(`medscale.${key}`); } catch { return null; }
}
export function writePref(key: string, value: string | null) {
  try {
    if (value === null) window.localStorage.removeItem(`medscale.${key}`);
    else window.localStorage.setItem(`medscale.${key}`, value);
  } catch { /* storage unavailable: preference stays in memory */ }
}
