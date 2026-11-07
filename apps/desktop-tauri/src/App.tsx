import { ACTIVITY_EVENTS, shouldAutoLock } from "./lib/auto-lock";
import { gateView, type GateRequest } from "./lib/first-launch";
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { call, type CmdError } from "./lib/ipc";
import { parseDensity, parseTheme } from "./lib/ui-preferences";
import { App as AppContext, readPref, writePref, type AppCtx, type Density, type Nav, type Theme, type WorkspaceStatus } from "./lib/app";
import { isRouteId, routes, type RouteId } from "./routes";
import { Header, Palette, Sidebar } from "./components/shell";
import { Mark } from "./components/brand";
import { ErrorState, ToastHost } from "./components/ui";
import { Access, Welcome } from "./pages/Gate";
import { Home } from "./pages/Home";
import { PatientDetail, Patients } from "./pages/Patients";
import { Documents, Evidence, Insights } from "./pages/Clinical";
import { Analytics, Browse, Data, Knowledge, Projects, ResearchOS } from "./pages/Research";
import { MedAgent, ModelFleet, Models } from "./pages/Intelligence";
import { Audio, Collaboration, Messages, Tasks, Workflows } from "./pages/Operations";
import { About, AuditTrail, Exports, Integrations, Privacy, Settings } from "./pages/Governance";

/** Optional start state from the URL hash (route, subject, theme, density); used for
 *  deep links and visual QA. Unknown values are ignored. */
const hash = new URLSearchParams(window.location.hash.slice(1));
const startSubject = hash.get("subject") ?? undefined;
// A deep-linked patient reference is needed only for initial in-memory navigation.
// Remove every subject parameter from the visible WebView URL/history entry
// before rendering. This is not secure erasure of already-written browser/OS bytes.
if (hash.has("subject")) {
  hash.delete("subject");
  const remaining = hash.toString();
  try {
    window.history.replaceState(
      window.history.state,
      "",
      window.location.pathname + window.location.search + (remaining ? `#${remaining}` : ""),
    );
  } catch { /* A restricted custom-protocol history must not block local startup. */ }
}
const hashRoute = hash.get("route");
const startNav: Nav = { route: hashRoute && isRouteId(hashRoute) ? hashRoute : "Home", subject: startSubject };

const initialTheme = (): Theme =>
  parseTheme(hash.get("theme")) ?? parseTheme(readPref("theme")) ?? (window.matchMedia?.("(prefers-color-scheme: light)").matches ? "light" : "dark");

const PAGES: Record<RouteId, () => ReactNode> = {
  Home: () => <Home />,
  Patients: () => <Patients />,
  Documents: () => <Documents />,
  Insights: () => <Insights />,
  Evidence: () => <Evidence />,
  Projects: () => <Projects />,
  Data: () => <Data />,
  Browse: () => <Browse />,
  Analytics: () => <Analytics />,
  Knowledge: () => <Knowledge />,
  "Research OS": () => <ResearchOS />,
  Models: () => <Models />,
  MedAgent: () => <MedAgent />,
  "Model Fleet": () => <ModelFleet />,
  Workflows: () => <Workflows />,
  Tasks: () => <Tasks />,
  Messages: () => <Messages />,
  Collaboration: () => <Collaboration />,
  Audio: () => <Audio />,
  Privacy: () => <Privacy />,
  "Audit Trail": () => <AuditTrail />,
  Exports: () => <Exports />,
  Integrations: () => <Integrations />,
  Settings: () => <Settings />,
  About: () => <About />,
};

export function App() {
  const [ws, setWs] = useState<WorkspaceStatus | null>(null);
  const [wsError, setWsError] = useState<string | null>(null);
  const [gate, setGate] = useState<GateRequest>(hash.get("gate") === "access" ? "access" : "auto");
  const [recoveryCodesShown, setRecoveryCodesShown] = useState(false);
  const [nav, setNav] = useState<Nav>(startNav);
  // Project selection belongs only to the currently unlocked workspace. Never persist its ID in WebView storage.
  const [project, setProjectState] = useState<string | null>(null);
  const [theme, setThemeState] = useState<Theme>(initialTheme);
  const [density, setDensityState] = useState<Density>(parseDensity(hash.get("density")) ?? parseDensity(readPref("density")) ?? "standard");
  const [palette, setPalette] = useState(false);
  const [lockError, setLockError] = useState<CmdError | null>(null);
  const [locking, setLocking] = useState(false);

  const refreshWorkspace = useCallback(async () => {
    try {
      const status = await call<WorkspaceStatus>("workspace_status");
      setWs(status);
      if (!status.open) setProjectState(null);
      setWsError(null);
    } catch (e) {
      setWsError((e as { message?: string }).message ?? "Local runtime disconnected");
    }
  }, []);

  useEffect(() => { void refreshWorkspace(); }, [refreshWorkspace]);
  // Remove the legacy identifier preference on startup. Storage engines may retain physical pages,
  // so this is logical minimization, not a secure erasure or full OS privacy claim.
  useEffect(() => { writePref("project", null); }, []);
  useEffect(() => { document.documentElement.dataset.theme = theme; }, [theme]);

  // Saved through Core: the release WebView runs InPrivate and forgets localStorage on exit.
  const savedPrefsApplied = useRef(false);
  useEffect(() => {
    if (!ws || savedPrefsApplied.current) return;
    savedPrefsApplied.current = true;
    const savedTheme = hash.get("theme") ? null : parseTheme(ws.ui_prefs?.theme ?? null);
    const savedDensity = hash.get("density") ? null : parseDensity(ws.ui_prefs?.density ?? null);
    if (savedTheme) setThemeState(savedTheme);
    if (savedDensity) setDensityState(savedDensity);
  }, [ws]);
  const savingPrefs = useRef<Promise<unknown>>(Promise.resolve());
  const savePrefs = useCallback((nextTheme: Theme, nextDensity: Density) => {
    // One at a time, so the last choice is the one that stays saved.
    savingPrefs.current = savingPrefs.current
      .then(() => call("ui_prefs_set", { theme: nextTheme, density: nextDensity }))
      .catch(() => { /* the choice still applies until the app closes */ });
  }, []);
  const setTheme = useCallback((t: Theme) => { setThemeState(t); writePref("theme", t); savePrefs(t, density); }, [density, savePrefs]);
  const setDensity = useCallback((d: Density) => { setDensityState(d); writePref("density", d); savePrefs(theme, d); }, [theme, savePrefs]);
  const setProject = useCallback((id: string | null) => { setProjectState(id); }, []);
  const navigate = useCallback((route: RouteId, subject?: string) => { setNav({ route, subject }); setPalette(false); }, []);
  const vaultClosed = useCallback(async () => {
    setProjectState(null);
    setGate("access");
    setNav({ route: "Home" });
    await refreshWorkspace();
  }, [refreshWorkspace]);
  const lock = useCallback(async () => {
    setLockError(null);
    setLocking(true);
    try {
      await call("workspace_lock");
    } catch (e) {
      // The vault is still open and unsealed: stay here and say so.
      setLockError(e as CmdError);
      await refreshWorkspace();
      return;
    } finally {
      setLocking(false);
    }
    await vaultClosed();
  }, [refreshWorkspace, vaultClosed]);

  // An open encrypted vault locks itself after a quiet spell, so a shared or
  // unattended computer doesn't leave records open. The sample workspace has no
  // secret, so it doesn't lock. Locking waits for any running command to finish.
  const encryptedOpen = !!ws?.open && ws?.workspace?.kind === "encrypted";
  useEffect(() => {
    if (!encryptedOpen || recoveryCodesShown) return;
    let last = Date.now();
    const seen = () => { last = Date.now(); };
    ACTIVITY_EVENTS.forEach((name) => window.addEventListener(name, seen, { passive: true }));
    const timer = window.setInterval(() => { if (shouldAutoLock(last, Date.now())) void lock(); }, 15_000);
    return () => {
      ACTIVITY_EVENTS.forEach((name) => window.removeEventListener(name, seen));
      window.clearInterval(timer);
    };
  }, [encryptedOpen, recoveryCodesShown, lock]);

  // A route change moves focus to the new page and names it, so keyboard and
  // screen-reader users are not left on a control in the previous view.
  const routeKey = ws?.open ? `${nav.route}:${nav.subject ?? ""}` : null;
  const lastRouteKey = useRef(routeKey);
  useEffect(() => {
    document.title = ws?.open ? `${nav.route} · MedScale` : "MedScale";
    if (lastRouteKey.current === routeKey) return;
    const entered = lastRouteKey.current !== null;
    lastRouteKey.current = routeKey;
    if (entered && routeKey) document.getElementById("main")?.focus();
  }, [routeKey, ws?.open, nav.route]);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (recoveryCodesShown) return;
      const mod = e.ctrlKey || e.metaKey;
      if (mod && e.key.toLowerCase() === "k") { e.preventDefault(); setPalette((p) => !p); }
      else if (mod && e.shiftKey && e.key.toLowerCase() === "l" && ws?.open) { e.preventDefault(); void lock(); }
      else if (mod && e.key === ",") { e.preventDefault(); setNav({ route: "Settings" }); }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [lock, ws?.open, recoveryCodesShown]);

  const ctx: AppCtx = useMemo(() => ({
    ws, refreshWorkspace, vaultClosed, nav, navigate, project, setProject, theme, setTheme, density, setDensity, lock,
    openPalette: () => setPalette(true),
  }), [ws, refreshWorkspace, vaultClosed, nav, navigate, project, setProject, theme, setTheme, density, setDensity, lock]);

  const route = routes.find((r) => r.id === nav.route) ?? routes[0];
  const crumbs: string[] = route.area === "Home" || route.area === "Utility" ? [route.label] : [route.area, route.label];
  if (nav.subject) crumbs.push(nav.subject);

  let body: ReactNode;
  const view = gateView({ runtimeError: !!wsError, status: ws, routeNeedsWorkspace: route.scope !== "none", request: gate, recoveryCodesShown });
  if (view === "runtime-unavailable") {
    body = <div className="splash" role="alert"><div className="stack"><Mark height={38} className="mark" title="MedScale" /><p className="t-sm i2">Local runtime disconnected. {wsError}</p><p className="t-sm i3">Your workspace stays closed until the runtime responds. If this keeps happening, close MedScale and open it again.</p><button type="button" className="btn btn-s" onClick={() => void refreshWorkspace()}>Try again</button></div></div>;
  } else if (view === "loading") {
    body = <div className="splash" aria-busy="true"><div className="stack"><Mark height={38} className="mark" title="MedScale" /><p className="t-sm i3">Opening the local runtime…</p></div></div>;
  } else if (view === "welcome") {
    body = <Welcome onContinue={() => setGate("access")} />;
  } else if (view === "unlock" || view === "create") {
    body = <Access onBack={() => setGate("welcome")} onRecoveryCodes={setRecoveryCodesShown} />;
  } else {
    body = (
      <div className="app">
        <button type="button" className="btn btn-s skip-link" onClick={() => document.getElementById("main")?.focus()}>Skip to main content</button>
        <Sidebar />
        <div className="main">
          <Header crumbs={crumbs} />
          {locking && <div className="t-sm i2" role="status" style={{ padding: "12px 24px 0" }}>Locking… A large vault can take a while. Keep MedScale open.</div>}
          {lockError && <div style={{ padding: "12px 24px 0" }}><ErrorState error={{ ...lockError, message: `The workspace is still open: it could not be locked. ${lockError.message}` }} onRetry={() => void lock()} /></div>}
          <main id="main" className="split" tabIndex={-1} aria-label={nav.route} style={{ outline: "none" }}>
            {(nav.route === "Patients" && nav.subject) ? <PatientDetail subject={nav.subject} /> : PAGES[nav.route]()}
          </main>
        </div>
      </div>
    );
  }

  return (
    <AppContext.Provider value={ctx}>
      <ToastHost>
        <div className="ms" data-theme={theme} data-density={density} style={{ height: "100%" }}>
          {body}
          {palette && ws?.open && <Palette onClose={() => setPalette(false)} />}
        </div>
      </ToastHost>
    </AppContext.Provider>
  );
}
