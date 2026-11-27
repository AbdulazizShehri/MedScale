import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { areas, routes, searchRoutes, type Route, type RouteId } from "../routes";
import { useApp } from "../lib/app";
import { DEFAULT_CLOSED, navGroupOpen } from "../lib/nav-groups";
import { Logo, Mark } from "./brand";
import { Glyph, Icon, type IconName } from "./ui";

const isMac = typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform);
export const MOD = isMac ? "⌘" : "Ctrl";

const AREA_ENTRY: Record<string, RouteId> = {
  Home: "Home",
  Clinical: "Patients",
  Intelligence: "Models",
  Research: "Projects",
  Operations: "Workflows",
  Governance: "Privacy",
  Utility: "Settings",
};

export function Sidebar() {
  const { ws, nav, navigate, openPalette, lock, sidebarCollapsed, toggleSidebar } = useApp();
  const open = !!ws?.open;
  const counts: Partial<Record<RouteId, number>> = open && ws?.workspace ? { Patients: ws.workspace.subjects, Documents: ws.workspace.sources } : {};
  const main = routes.filter((r) => r.area !== "Utility");
  const foot = routes.filter((r) => r.area === "Utility");
  const activeArea = routes.find((r) => r.id === nav.route)?.area;
  const [closed, setClosed] = useState<ReadonlySet<string>>(DEFAULT_CLOSED);

  function toggleGroup(area: string, isOpen: boolean) {
    setClosed((current) => {
      const next = new Set(current);
      if (isOpen) next.add(area); else next.delete(area);
      return next;
    });
  }

  return (
    <aside className={`sb ${sidebarCollapsed ? "sb-collapsed" : ""}`} aria-label="Primary navigation">
      <div className="sb-brand">
        {sidebarCollapsed ? <Mark height={18} className="sb-word" title="MedScale" /> : <Logo height={18} className="sb-word" />}
        {!sidebarCollapsed && (
          <button type="button" className="ibtn" aria-label="Collapse sidebar" title={`Collapse sidebar (${MOD} B)`} onClick={toggleSidebar}>
            <Icon name="panelLeft" size={15} />
          </button>
        )}
      </div>

      <button
        type="button"
        className="sb-ws sb-ws-btn"
        aria-label={ws?.workspace ? `${ws.workspace.label}, open workspace settings` : "No workspace open, open settings"}
        title={ws?.workspace ? `${ws.workspace.label} (${ws.workspace.vault_id})` : "Workspace settings"}
        onClick={() => navigate("Settings")}
      >
        <span className={`mg ${ws?.workspace?.kind === "synthetic" ? "syn" : ""}`} style={{ width: 18, height: 18, borderStyle: open ? "solid" : "dashed" }}>
          <span><Icon name="workspace" size={12} /></span>
        </span>
        <span className="sb-ws-meta">
          <span className="sb-ws-name trunc">{ws?.workspace?.label ?? "No workspace open"}</span>
          <span className="mono trunc sb-ws-sub">
            {ws?.workspace ? (ws.workspace.kind === "encrypted" ? "encrypted vault open" : "sample fixtures open") : "unlock a vault to begin"}
          </span>
        </span>
      </button>

      <button type="button" className="sb-search sb-search-btn" aria-label="Search or run" title={`Search or run (${MOD} K)`} onClick={openPalette}>
        <Icon name="search" size={16} />
        <span className="sb-search-label">Search or run</span>
        <span className="kbd">{MOD} K</span>
      </button>

      <nav className="sb-nav" aria-label="Workspace sections">
        {areas.filter((a) => a !== "Utility").map((area) => {
          // The group holding the current page always stays open, so the page stays findable.
          const isOpen = sidebarCollapsed || area === "Home" || area === activeArea || navGroupOpen(area, closed);
          return (
            <div key={area}>
              {area !== "Home" && (
                <button
                  type="button"
                  className="sb-group"
                  aria-expanded={isOpen}
                  aria-disabled={area === activeArea}
                  title={area === activeArea ? "Stays open while you are on a page in this group" : undefined}
                  onClick={() => { if (area !== activeArea) toggleGroup(area, isOpen); }}
                >
                  <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true"><path d={isOpen ? "M1 3h8L5 8Z" : "M3 1v8l5-4Z"} fill="currentColor" /></svg>
                  {area}
                </button>
              )}
              {isOpen && main.filter((r) => r.area === area).map((r) => (
                <NavItem key={r.id} route={r} active={nav.route === r.id} count={counts[r.id]} collapsed={sidebarCollapsed} onClick={() => navigate(r.id)} />
              ))}
            </div>
          );
        })}
      </nav>

      <div className="sb-foot">
        <button type="button" className="rt sb-rt-btn" aria-label={`Local runtime, ${open ? (ws?.network_default_deny ? "no default egress" : "egress open") : "idle"}, version ${ws?.version ?? ""}`} title="Runtime and version details" onClick={() => navigate("About")}>
          <Glyph kind={open ? "available" : "unavailable"} />
          <span className="sb-rt-meta">
            <span className="sb-rt-title">Local runtime<span className="sr-only">. </span></span>
            <span className="sb-rt-sub">
              {open ? `${ws?.network_default_deny ? "No default egress" : "Egress open"} · v${ws?.version}` : "Idle · no vault loaded"}
            </span>
          </span>
        </button>
        {foot.map((r) => <NavItem key={r.id} route={r} active={nav.route === r.id} collapsed={sidebarCollapsed} onClick={() => navigate(r.id)} />)}
        {open && (
          <button type="button" className="nav" aria-label="Lock workspace" onClick={() => void lock()} title={`Lock workspace (${MOD} Shift L)`}>
            <Icon name="privacy" />
            <span>Lock workspace</span>
          </button>
        )}
      </div>
    </aside>
  );
}

function NavItem({ route, active, count, collapsed, onClick }: { route: Route; active: boolean; count?: number; collapsed: boolean; onClick: () => void }) {
  const ariaLabel = count !== undefined ? `${route.label}, ${count} ${count === 1 ? "item" : "items"}` : collapsed ? route.label : undefined;
  return (
    <button type="button" className={`nav ${active ? "on" : ""}`} aria-current={active ? "page" : undefined} aria-label={ariaLabel} title={collapsed ? route.label : undefined} onClick={onClick}>
      <Icon name={route.icon as IconName} />
      <span>{route.label}</span>
      {count !== undefined && <span className="nav-n">{count}</span>}
    </button>
  );
}

export function Header({ crumbs }: { crumbs: string[] }) {
  const { ws, nav, navigate, sidebarCollapsed, toggleSidebar, hasInspector, inspectorOpen, setInspectorOpen } = useApp();

  function onCrumbClick(label: string, index: number) {
    if (index === crumbs.length - 1) return;
    if (nav.subject && label === "Patients") { navigate("Patients"); return; }
    const target = AREA_ENTRY[label];
    if (target) navigate(target);
  }

  return (
    <header className="hd">
      {sidebarCollapsed && (
        <button type="button" className="ibtn" aria-label="Expand sidebar" title={`Expand sidebar (${MOD} B)`} onClick={toggleSidebar}>
          <Icon name="panelLeft" size={16} />
        </button>
      )}
      <nav className="crumb" aria-label="Breadcrumb">
        {crumbs.map((c, i) => {
          const isLast = i === crumbs.length - 1;
          const clickable = !isLast && (Boolean(AREA_ENTRY[c]) || (Boolean(nav.subject) && c === "Patients"));
          return (
            <span key={`${c}-${i}`} style={{ display: "contents" }}>
              {i > 0 && <span className="sep" aria-hidden="true">/</span>}
              {isLast ? <b>{c}</b> : clickable ? <button type="button" className="crumb-btn" onClick={() => onCrumbClick(c, i)}>{c}</button> : <span>{c}</span>}
            </span>
          );
        })}
      </nav>
      <span className="hd-spacer" />
      {hasInspector && (
        <button
          type="button"
          className="btn btn-s hd-ins-btn"
          aria-expanded={inspectorOpen}
          aria-label={inspectorOpen ? "Close inspector panel" : "Open inspector panel"}
          onClick={() => setInspectorOpen(!inspectorOpen)}
        >
          <Icon name="panelRight" size={14} />
          <span>Inspector</span>
        </button>
      )}
      {ws?.workspace?.kind === "synthetic" && <span className="synth"><span className="g g-synthetic" />Synthetic workspace</span>}
      {ws?.workspace?.kind === "encrypted" && <span className="synth vault"><Glyph kind="available" />Encrypted vault</span>}
    </header>
  );
}

type Cmd = { id: string; group: "Routes" | "Actions"; label: string; area?: string; hint?: string; icon: IconName; run: () => void };

export function Palette({ onClose }: { onClose: () => void }) {
  const { navigate, theme, setTheme, density, setDensity, lock, ws, sidebarCollapsed, toggleSidebar } = useApp();
  const [query, setQuery] = useState("");
  const [sel, setSel] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const previouslyFocused = document.activeElement;
    input.current?.focus();
    return () => {
      if (previouslyFocused instanceof HTMLElement && previouslyFocused.isConnected) {
        previouslyFocused.focus();
      }
    };
  }, []);

  const commands = useMemo<Cmd[]>(() => {
    const open = !!ws?.open;
    const go = (r: Route): Cmd => ({ id: `go-${r.id}`, group: "Routes", label: r.label, area: r.area, hint: r.description, icon: r.icon as IconName, run: () => navigate(r.id) });
    const actions: Cmd[] = [
      { id: "theme", group: "Actions", label: `Switch to ${theme === "dark" ? "light" : "dark"} theme`, hint: "Appearance", icon: "settings", run: () => setTheme(theme === "dark" ? "light" : "dark") },
      { id: "density", group: "Actions", label: `Use ${density === "standard" ? "compact" : "standard"} density`, hint: "Appearance", icon: "settings", run: () => setDensity(density === "standard" ? "compact" : "standard") },
      { id: "sidebar", group: "Actions", label: sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar", hint: `${MOD} B`, icon: "panelLeft", run: toggleSidebar },
    ];
    if (open) actions.push({ id: "lock", group: "Actions", label: "Lock workspace", hint: `${MOD} Shift L`, icon: "privacy", run: () => void lock() });
    const q = query.trim().toLowerCase();
    const availableRoutes = open ? routes : routes.filter((r) => r.scope === "none");
    const routeHits = (q ? searchRoutes(query).filter((r) => open || r.scope === "none") : availableRoutes).map(go);
    const actionHits = actions.filter((a) => !q || a.label.toLowerCase().includes(q) || (a.hint ?? "").toLowerCase().includes(q));
    return [...routeHits, ...actionHits].slice(0, 40);
  }, [query, navigate, theme, setTheme, density, setDensity, ws, lock, sidebarCollapsed, toggleSidebar]);

  const choose = (c: Cmd | undefined) => { if (!c) return; c.run(); onClose(); };

  function onKey(e: KeyboardEvent<HTMLDivElement>) {
    if (e.key === "Escape") { e.preventDefault(); onClose(); }
    else if (e.key === "ArrowDown") { e.preventDefault(); setSel((i) => Math.min(i + 1, Math.max(0, commands.length - 1))); }
    else if (e.key === "ArrowUp") { e.preventDefault(); setSel((i) => Math.max(i - 1, 0)); }
    else if (e.key === "Home") { e.preventDefault(); setSel(0); }
    else if (e.key === "End") { e.preventDefault(); setSel(Math.max(0, commands.length - 1)); }
    else if (e.key === "Enter") { e.preventDefault(); choose(commands[sel]); }
    else if (e.key === "Tab") { e.preventDefault(); input.current?.focus(); }
  }

  useEffect(() => { box.current?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: "nearest" }); }, [sel]);

  const routeItems = commands.filter((c) => c.group === "Routes");
  const actionItems = commands.filter((c) => c.group === "Actions");

  const renderItem = (c: Cmd, idx: number) => (
    <button
      key={c.id}
      id={`pal-${c.id}`}
      type="button"
      role="option"
      aria-selected={idx === sel}
      className={`menu-i ${idx === sel ? "on" : ""}`}
      onMouseEnter={() => setSel(idx)}
      onClick={() => choose(c)}
    >
      <Icon name={c.icon} size={15} />
      <span style={{ color: "var(--ink-1)" }}>{c.label}</span>
      {c.area && c.area !== "Home" && <span className="palette-area">{c.area}</span>}
      {c.hint && <small className="trunc">{c.hint}</small>}
    </button>
  );

  return (
    <div className="scrim" onMouseDown={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div ref={box} className="overlay palette no-anim" role="dialog" aria-modal="true" aria-label="Search or run" onKeyDown={onKey}>
        <div className="palette-in">
          <Icon name="search" size={16} />
          <input
            ref={input}
            type="text"
            value={query}
            maxLength={128}
            placeholder="Go to a route or run a command"
            aria-label="Search or run"
            role="combobox"
            aria-expanded={true}
            aria-autocomplete="list"
            aria-controls="palette-list"
            aria-activedescendant={commands[sel] ? `pal-${commands[sel].id}` : undefined}
            onChange={(e) => { setQuery(e.target.value); setSel(0); }}
          />
          <span className="kbd">Esc</span>
        </div>
        <div id="palette-list" className="palette-list" role="listbox">
          {commands.length === 0 && <p className="t-sm i3" style={{ padding: "12px 10px" }}>No match. Search covers desktop routes and app commands only.</p>}
          {routeItems.length > 0 && (
            <>
              <div className="palette-group"><span>Routes</span><span className="mono tnum">{routeItems.length}</span></div>
              {routeItems.map((c, i) => renderItem(c, i))}
            </>
          )}
          {actionItems.length > 0 && (
            <>
              <div className="palette-group"><span>Actions</span><span className="mono tnum">{actionItems.length}</span></div>
              {actionItems.map((c, i) => renderItem(c, routeItems.length + i))}
            </>
          )}
        </div>
        <div className="palette-foot">
          <span>↑↓ move</span>
          <span>Enter open</span>
          <span aria-live="polite" className="sr-only">{commands.length} {commands.length === 1 ? "result" : "results"}</span>
          <span style={{ marginLeft: "auto" }}>Local only · {commands.length}</span>
        </div>
      </div>
    </div>
  );
}
