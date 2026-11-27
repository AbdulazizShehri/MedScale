import { Fragment, useEffect, type ReactNode } from "react";
import { useApp } from "../lib/app";
import { useCommand } from "../lib/ipc";
import { Empty, ErrorState, PageHead, Skeleton } from "./ui";

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

  const allRows = state.status === "ready" ? state.data : [];
  const rows = allRows.filter((p) => p.status !== "archived");
  const archivedCount = allRows.length - rows.length;
  const current = rows.find((p) => p.id === project) ?? null;

  useEffect(() => {
    if (state.status === "ready" && !current && rows.length) setProject(rows[0]!.id);
  }, [state.status, current, rows, setProject]);

  return (
    <div className="pane">
      <PageHead title={title} sub={sub}>
        {rows.length > 0 && (
          <label className="picker">
            <span className="t-label">Project</span>
            <select value={current?.id ?? ""} onChange={(e) => setProject(e.target.value)} aria-label="Project">
              {rows.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
            </select>
          </label>
        )}
        {current && actions?.(current.id)}
      </PageHead>
      {state.status === "loading" && <Skeleton rows={4} />}
      {state.status === "error" && <ErrorState error={state.error} onRetry={reload} />}
      {state.status === "ready" && !rows.length && (
        <Empty
          icon="projects"
          title={archivedCount > 0 ? "No active project" : "No project yet"}
          action={<button type="button" className="btn btn-p" onClick={() => navigate("Projects")}>Create a project</button>}
        >
          {archivedCount > 0
            ? `All ${archivedCount} existing ${archivedCount === 1 ? "project is" : "projects are"} archived. Create a new project to use ${title}.`
            : `${title} organizes its sources, runs, and outputs inside a project so everything stays traceable. Create one to get started.`}
        </Empty>
      )}
      {/* Keep all project-local form, transcript, draft and pending-action state
          scoped to a single project. A keyed boundary remounts its children
          synchronously on a selection change, before effects can run. */}
      {current && <Fragment key={current.id}>{children(current.id, current)}</Fragment>}
    </div>
  );
}
