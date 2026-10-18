import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { areas, routes, searchRoutes, type Route, type RouteId } from "../routes";
import { useApp } from "../lib/app";
import { Glyph, Icon, Mark, type IconName } from "./ui";

const isMac = typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform);
export const MOD = isMac ? "⌘" : "Ctrl";

export function Sidebar() {
  const { ws, nav, navigate, openPalette, lock } = useApp();
  const open = !!ws?.open;
  const counts: Partial<Record<RouteId, number>> = open && ws?.workspace ? { Patients: ws.workspace.subjects, Documents: ws.workspace.sources } : {};
  const main = routes.filter((r) => r.area !== "Utility");
  const foot = routes.filter((r) => r.area === "Utility");

  return (
    <aside className="sb" aria-label="Primary navigation">
      <div className="sb-brand">
        <Mark width={28} />
        <span className="sb-word">MedScale</span>
      </div>

      <button type="button" className="sb-ws" style={{ width: "calc(100% - 16px)", textAlign: "left" }} onClick={() => navigate("Settings")} title="Workspace settings">
        <span className={`mg ${ws?.workspace?.kind === "synthetic" ? "syn" : ""}`} style={{ width: 22, height: 22, borderStyle: open ? "solid" : "dashed" }}>
          <span><Icon name="workspace" size={12} /></span>
        </span>
        <span style={{ display: "flex", flexDirection: "column", minWidth: 0, flex: 1 }}>
          <span style={{ fontSize: 12.5, fontWeight: 600, lineHeight: "16px" }}>{ws?.workspace?.label ?? "No workspace open"}</span>
          <span className="mono trunc" style={{ fontSize: 10.5, lineHeight: "14px", color: "var(--ink-3)" }}>
            {ws?.workspace ? `vault open · ${ws.workspace.vault_id}` : "unlock a vault to begin"}
          </span>
        </span>
      </button>

      <button type="button" className="sb-search" style={{ width: "calc(100% - 16px)" }} onClick={openPalette}>
        <Icon name="search" size={16} />
        <span style={{ flex: 1, textAlign: "left" }}>Search or run</span>
        <span className="kbd">{MOD} K</span>
      </button>

      <nav className="sb-nav">
        {areas.filter((a) => a !== "Utility").map((area) => (
          <div key={area}>
            {area !== "Home" && <div className="sb-group">{area}</div>}
            {main.filter((r) => r.area === area).map((r) => (
              <NavItem key={r.id} route={r} active={nav.route === r.id} count={counts[r.id]} onClick={() => navigate(r.id)} />
            ))}
          </div>
        ))}
      </nav>

      <div className="sb-foot">
        <button type="button" className="rt" style={{ width: "100%", textAlign: "left" }} onClick={() => navigate("About")}>
          <Glyph kind={open ? "available" : "unavailable"} />
          <span style={{ display: "flex", flexDirection: "column", minWidth: 0 }}>
            <span style={{ fontSize: 12.5, fontWeight: 600, lineHeight: "18px" }}>Local runtime</span>
            <span style={{ fontSize: 11.5, lineHeight: "16px", color: "var(--ink-3)" }}>
              {open ? `${ws?.network_default_deny ? "No default egress" : "Egress open"} · v${ws?.version}` : "Idle · no vault loaded"}
            </span>
          </span>
        </button>
        {foot.map((r) => <NavItem key={r.id} route={r} active={nav.route === r.id} onClick={() => navigate(r.id)} />)}
        {open && (
          <button type="button" className="nav" onClick={() => void lock()} title={`Lock workspace (${MOD} Shift L)`}>
            <Icon name="privacy" />
            <span>Lock workspace</span>
          </button>
        )}
      </div>
    </aside>
  );
}

function NavItem({ route, active, count, onClick }: { route: Route; active: boolean; count?: number; onClick: () => void }) {
  return (
    <button type="button" className={`nav ${active ? "on" : ""}`} aria-current={active ? "page" : undefined} onClick={onClick}>
      <Icon name={route.icon as IconName} />
      <span>{route.label}</span>
      {count !== undefined && <span className="nav-n">{count}</span>}
    </button>
  );
}

export function Header({ crumbs }: { crumbs: string[] }) {
  const { ws } = useApp();
  return (
    <header className="hd">
      <div className="crumb">
        {crumbs.map((c, i) => (
          <span key={`${c}-${i}`} style={{ display: "contents" }}>
            {i > 0 && <span className="sep">/</span>}
            {i === crumbs.length - 1 ? <b>{c}</b> : <span>{c}</span>}
          </span>
        ))}
      </div>
      <span className="hd-spacer" />
      {ws?.workspace?.kind === "synthetic" && <span className="synth"><span className="g g-synthetic" />Synthetic workspace</span>}
      {ws?.workspace?.kind === "encrypted" && <span className="synth"><Glyph kind="available" />Encrypted vault</span>}
    </header>
  );
}

type Cmd = { id: string; label: string; hint?: string; icon: IconName; run: () => void };

export function Palette({ onClose }: { onClose: () => void }) {
  const { navigate, theme, setTheme, density, setDensity, lock, ws } = useApp();
  const [query, setQuery] = useState("");
  const [sel, setSel] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => { input.current?.focus(); }, []);

  const commands = useMemo<Cmd[]>(() => {
    const go = (r: Route): Cmd => ({ id: `go-${r.id}`, label: r.label, hint: r.description, icon: r.icon as IconName, run: () => navigate(r.id) });
    const actions: Cmd[] = [
      { id: "theme", label: `Switch to ${theme === "dark" ? "light" : "dark"} theme`, icon: "settings", run: () => setTheme(theme === "dark" ? "light" : "dark") },
      { id: "density", label: `Use ${density === "standard" ? "compact" : "standard"} density`, icon: "settings", run: () => setDensity(density === "standard" ? "compact" : "standard") },
    ];
    if (ws?.open) actions.push({ id: "lock", label: "Lock workspace", hint: `${MOD} Shift L`, icon: "privacy", run: () => void lock() });
    const q = query.trim().toLowerCase();
    const routeHits = (q ? searchRoutes(query) : routes).map(go);
    const actionHits = actions.filter((a) => !q || a.label.toLowerCase().includes(q));
    return [...routeHits, ...actionHits].slice(0, 40);
  }, [query, navigate, theme, setTheme, density, setDensity, ws, lock]);

  const choose = (c: Cmd | undefined) => { if (!c) return; c.run(); onClose(); };

  function onKey(e: KeyboardEvent<HTMLDivElement>) {
    if (e.key === "Escape") { e.preventDefault(); onClose(); }
    else if (e.key === "ArrowDown") { e.preventDefault(); setSel((i) => Math.min(i + 1, commands.length - 1)); }
    else if (e.key === "ArrowUp") { e.preventDefault(); setSel((i) => Math.max(i - 1, 0)); }
    else if (e.key === "Enter") { e.preventDefault(); choose(commands[sel]); }
    else if (e.key === "Tab") { e.preventDefault(); input.current?.focus(); }
  }

  useEffect(() => { box.current?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: "nearest" }); }, [sel]);

  return (
    <div className="scrim" onMouseDown={(e) => { if (e.target === e.currentTarget) onClose(); }}>
      <div ref={box} className="overlay palette anim" role="dialog" aria-modal="true" aria-label="Search or run" onKeyDown={onKey}>
        <div className="palette-in">
          <Icon name="search" size={16} />
          <input ref={input} type="text" value={query} maxLength={128} placeholder="Go to a route or run a command" aria-label="Search or run" aria-controls="palette-list" aria-activedescendant={commands[sel] ? `pal-${commands[sel].id}` : undefined} onChange={(e) => { setQuery(e.target.value); setSel(0); }} />
          <span className="kbd">Esc</span>
        </div>
        <div id="palette-list" className="palette-list" role="listbox">
          {commands.length === 0 && <p className="t-sm i3" style={{ padding: "10px" }}>No match. Search covers routes and app commands only.</p>}
          {commands.map((c, i) => (
            <button key={c.id} id={`pal-${c.id}`} type="button" role="option" aria-selected={i === sel} className={`menu-i ${i === sel ? "on" : ""}`} onMouseEnter={() => setSel(i)} onClick={() => choose(c)}>
              <Icon name={c.icon} size={16} />
              <span style={{ color: "var(--ink-1)" }}>{c.label}</span>
              {c.hint && <small>{c.hint}</small>}
            </button>
          ))}
        </div>
        <div className="palette-foot"><span>↑↓ move</span><span>Enter open</span><span style={{ marginLeft: "auto" }}>Local only</span></div>
      </div>
    </div>
  );
}
