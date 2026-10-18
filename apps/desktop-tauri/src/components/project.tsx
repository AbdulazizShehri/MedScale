import { useEffect, type ReactNode } from "react";
import { useApp } from "../lib/app";
import { useCommand } from "../lib/ipc";
import { Empty, ErrorState, Skeleton } from "./ui";

export type ProjectRow = { id: string; name: string; status: string; revision: number; experiments: number; refs: number; edges: number };

/** Loads projects, keeps the app-level selection valid, and renders children
 *  only when a project is selected. Project-scoped Core reads need a project. */
export function ProjectScope({ title, sub, actions, children }: {
  title: string;
  sub?: ReactNode;
  actions?: (projectId: string) => ReactNode;
  children: (projectId: string, project: ProjectRow) => ReactNode;
}) {
  const { project, setProject, navigate } = useApp();
  const { state, reload } = useCommand<ProjectRow[]>("projects_list");

  const rows = state.status === "ready" ? state.data.filter((p) => p.status !== "archived") : [];
  const current = rows.find((p) => p.id === project) ?? null;

  useEffect(() => {
    if (state.status === "ready" && !current && rows.length) setProject(rows[0]!.id);
  }, [state.status, current, rows, setProject]);

  return (
    <div className="pane">
      <div className="page-head">
        <div className="grow">
          <h1 className="t-h1">{title}</h1>
          {sub && <p className="ph-sub">{sub}</p>}
        </div>
        {rows.length > 0 && (
          <label className="picker">
            <span className="t-label">Project</span>
            <select value={current?.id ?? ""} onChange={(e) => setProject(e.target.value)} aria-label="Project">
              {rows.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
            </select>
          </label>
        )}
        {current && actions?.(current.id)}
      </div>
      {state.status === "loading" && <Skeleton rows={3} />}
      {state.status === "error" && <ErrorState error={state.error} onRetry={reload} />}
      {state.status === "ready" && !rows.length && (
        <Empty icon="projects" title="No project yet" action={<button type="button" className="btn btn-p" onClick={() => navigate("Projects")}>Create a project</button>}>
          {title} works inside a project, so Core can pin what it reads and writes. Create one first.
        </Empty>
      )}
      {current && children(current.id, current)}
    </div>
  );
}
