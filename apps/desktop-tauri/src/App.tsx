import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { call } from "./lib/ipc";
import { App as AppContext, readPref, writePref, type AppCtx, type Density, type Nav, type Theme, type WorkspaceStatus } from "./lib/app";
import { isRouteId, routes, type RouteId } from "./routes";
import { Header, Palette, Sidebar } from "./components/shell";
import { Mark, ToastHost } from "./components/ui";
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
const hashRoute = hash.get("route");
const startNav: Nav = { route: hashRoute && isRouteId(hashRoute) ? hashRoute : "Home", subject: hash.get("subject") ?? undefined };

const initialTheme = (): Theme =>
  (hash.get("theme") === "light" || hash.get("theme") === "dark" ? hash.get("theme") as Theme : null) ?? (readPref("theme") as Theme | null) ?? (window.matchMedia?.("(prefers-color-scheme: light)").matches ? "light" : "dark");

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
  const [gate, setGate] = useState<"welcome" | "access">(hash.get("gate") === "access" ? "access" : "welcome");
  const [nav, setNav] = useState<Nav>(startNav);
  const [project, setProjectState] = useState<string | null>(readPref("project"));
  const [theme, setThemeState] = useState<Theme>(initialTheme);
  const [density, setDensityState] = useState<Density>((hash.get("density") === "compact" ? "compact" : null) ?? (readPref("density") as Density | null) ?? "standard");
  const [palette, setPalette] = useState(false);

  const refreshWorkspace = useCallback(async () => {
    try {
      setWs(await call<WorkspaceStatus>("workspace_status"));
      setWsError(null);
    } catch (e) {
      setWsError((e as { message?: string }).message ?? "Local runtime disconnected");
    }
  }, []);

  useEffect(() => { void refreshWorkspace(); }, [refreshWorkspace]);
  useEffect(() => { document.documentElement.dataset.theme = theme; }, [theme]);

  const setTheme = useCallback((t: Theme) => { setThemeState(t); writePref("theme", t); }, []);
  const setDensity = useCallback((d: Density) => { setDensityState(d); writePref("density", d); }, []);
  const setProject = useCallback((id: string | null) => { setProjectState(id); writePref("project", id); }, []);
  const navigate = useCallback((route: RouteId, subject?: string) => { setNav({ route, subject }); setPalette(false); }, []);
  const lock = useCallback(async () => {
    try { await call("workspace_lock"); } finally {
      setGate("access");
      setNav({ route: "Home" });
      await refreshWorkspace();
    }
  }, [refreshWorkspace]);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      const mod = e.ctrlKey || e.metaKey;
      if (mod && e.key.toLowerCase() === "k") { e.preventDefault(); setPalette((p) => !p); }
      else if (mod && e.shiftKey && e.key.toLowerCase() === "l" && ws?.open) { e.preventDefault(); void lock(); }
      else if (mod && e.key === ",") { e.preventDefault(); setNav({ route: "Settings" }); }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [lock, ws?.open]);

  const ctx: AppCtx = useMemo(() => ({
    ws, refreshWorkspace, nav, navigate, project, setProject, theme, setTheme, density, setDensity, lock,
    openPalette: () => setPalette(true),
  }), [ws, refreshWorkspace, nav, navigate, project, setProject, theme, setTheme, density, setDensity, lock]);

  const route = routes.find((r) => r.id === nav.route) ?? routes[0];
  const crumbs: string[] = route.area === "Home" || route.area === "Utility" ? [route.label] : [route.area, route.label];
  if (nav.subject) crumbs.push(nav.subject);

  let body: ReactNode;
  if (!ws) {
    body = wsError
      ? <div className="splash" role="alert"><div className="stack"><Mark width={64} className="mark" /><span className="brand-rule" aria-hidden="true" /><p className="t-sm i2">Local runtime disconnected. {wsError}</p></div></div>
      : <div className="splash" aria-busy="true"><div className="stack"><Mark width={64} className="mark" /><span className="brand-rule" aria-hidden="true" /><p className="t-sm i3">Opening the local runtime…</p></div></div>;
  } else if (!ws.open && route.scope !== "none") {
    body = gate === "welcome" ? <Welcome onContinue={() => setGate("access")} /> : <Access onBack={() => setGate("welcome")} />;
  } else {
    body = (
      <div className="app">
        <Sidebar />
        <div className="main">
          <Header crumbs={crumbs} />
          <main id="main" className="split" tabIndex={-1} style={{ outline: "none" }}>
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
