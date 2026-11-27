import { useState } from "react";
import { useApp } from "../lib/app";
import { call, useCommand, type CmdError } from "../lib/ipc";
import { CopyButton, Notice, PageHead, Section, useToast } from "../components/ui";
import { SUPPORT_DATA_RULE, supportDiagnostics } from "../support";
import { RELEASE_NOTES } from "../release-notes";

type About = {
  product: string;
  version: string;
  shell: string;
  core: string;
  local_only: boolean;
  network_default_deny: boolean;
  build_profile: string;
  target: string;
  release_ready: boolean;
};
type UpdateResult = { current: string; latest: string | null; newer: boolean | null };

/** Opens one of the fixed Help pages. The app passes a name; Rust builds the address. */
async function open(target: string): Promise<CmdError | null> {
  try { await call("help_open", { target }); return null; } catch (e) { return e as CmdError; }
}

function HelpButton({ target, label, primary }: { target: string; label: string; primary?: boolean }) {
  const toast = useToast();
  return (
    <button type="button" className={primary ? "btn btn-p btn-inline" : "btn btn-s"} onClick={async () => {
      const error = await open(target);
      toast(error
        ? { kind: "blocked", title: "Couldn't open your browser", detail: "Go to github.com/AbdulazizShehri/MedScale/issues to report it." }
        : { kind: "completed", title: "Opened in your browser", detail: "Nothing from your workspace was sent." });
    }}>{label}</button>
  );
}

export function Help() {
  const { navigate } = useApp();
  const about = useCommand<About>("about_info");
  const [update, setUpdate] = useState<{ state: "idle" | "checking" } | { state: "done"; result: UpdateResult } | { state: "failed"; message: string }>({ state: "idle" });
  const version = about.state.status === "ready" ? about.state.data.version : "";
  const diagnostics = about.state.status === "ready" ? supportDiagnostics(about.state.data) : "";
  const notes = RELEASE_NOTES[version] ?? [];

  async function checkForUpdates() {
    setUpdate({ state: "checking" });
    try {
      setUpdate({ state: "done", result: await call<UpdateResult>("update_check") });
    } catch (e) {
      setUpdate({ state: "failed", message: (e as CmdError).message });
    }
  }

  return (
    <div className="pane">
      <PageHead title="Help & Feedback" sub="Tell us what broke or what you need. Every report goes to MedScale's public GitHub, where fixes are tracked in the open.">
        <div className="row-gap" style={{ gap: 8 }}>
          {diagnostics && <CopyButton text={diagnostics} label="Copy diagnostics" />}
          <button type="button" className="btn btn-g" onClick={() => navigate("About")}>About MedScale</button>
        </div>
      </PageHead>
      <div className="stack" style={{ gap: 24, maxWidth: 780 }}>
        <Section title="Report or request">
          <p className="t-sm i2" style={{ margin: "0 0 12px" }}>Opens a fixed GitHub template in your default browser with your MedScale version and operating system filled in. Nothing from your workspace is sent. Prefer copy-only reporting? Copy diagnostics above or in About.</p>
          <div className="row-gap" style={{ gap: 8, flexWrap: "wrap", marginBottom: 12 }}>
            <HelpButton target="bug" label="Report a bug" primary />
            <HelpButton target="feature" label="Request a feature" />
            <HelpButton target="install" label="Installation or update problem" />
            <HelpButton target="accessibility" label="Accessibility problem" />
          </div>
          <Notice kind="blocked" title="Keep reports free of personal information.">{SUPPORT_DATA_RULE}</Notice>
        </Section>

        <Section title="Security">
          <p className="t-sm i2" style={{ margin: "0 0 12px" }}>Found a vulnerability? Report it privately, so it can be fixed before anyone else knows about it. Please don&apos;t open a public issue for it.</p>
          <HelpButton target="security" label="Report a vulnerability privately" />
        </Section>

        <Section title="This version">
          <p className="t-sm i2" style={{ margin: "0 0 12px" }}>MedScale {version || "…"} Beta</p>
          {notes.length > 0 && <ul className="t-sm i2" style={{ margin: "0 0 12px", paddingLeft: 18, display: "grid", gap: 6 }}>{notes.map((n) => <li key={n}>{n}</li>)}</ul>}
          <div className="row-gap" style={{ gap: 8, flexWrap: "wrap" }}>
            <HelpButton target="changelog" label="Release notes" />
            <HelpButton target="known" label="Known issues" />
          </div>
        </Section>

        <Section title="Updates">
          <p className="t-sm i2" style={{ margin: "0 0 12px" }}>Checks MedScale&apos;s releases on GitHub once, when you click. Nothing is downloaded or installed for you.</p>
          <div className="row-gap" style={{ gap: 8, flexWrap: "wrap", alignItems: "center" }}>
            <button type="button" className="btn btn-s" onClick={() => void checkForUpdates()} disabled={update.state === "checking"}>{update.state === "checking" ? "Checking…" : "Check for updates"}</button>
            {update.state === "done" && update.result.newer === true && <HelpButton target="download" label={`Get ${update.result.latest}`} primary />}
          </div>
          <p className="t-sm i2" role="status" aria-live="polite" style={{ margin: "10px 0 0" }}>
            {update.state === "done" && (update.result.newer === true
              ? `${update.result.latest} is available. Download it from medscale.si, then run the installer. Your data stays where it is.`
              : update.result.newer === false
                ? "You have the latest version."
                : update.result.latest
                  ? `The latest release is ${update.result.latest}. Compare it with the version above.`
                  : "No releases are published yet.")}
            {update.state === "failed" && update.message}
          </p>
        </Section>
      </div>
    </div>
  );
}
