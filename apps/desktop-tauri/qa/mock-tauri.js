// Visual-QA replay of real Core responses recorded by the Rust feature test
// (MEDSCALE_UI_FIXTURES). Loaded only by qa/serve.mjs; never part of the app.
(function () {
  const ID_KEYS = ["subjectRef", "runId", "fleetId", "roomId", "threadId", "sourceId", "snapshotId"];
  const keyOf = (cmd, args) => cmd + "|" + ID_KEYS.filter((k) => args && args[k] != null).map((k) => k + "=" + args[k]).join("&");
  const ready = fetch("/ui-fixtures.json").then((r) => r.json()).then((log) => {
    const exact = new Map(), byCmd = new Map(), firstByCmd = new Map();
    for (const e of log) if ("ok" in e) { exact.set(keyOf(e.cmd, e.args), e.ok); byCmd.set(e.cmd, e.ok); if (!firstByCmd.has(e.cmd)) firstByCmd.set(e.cmd, e.ok); }
    return { exact, byCmd, firstByCmd };
  });
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main", windowLabel: "main" } },
    transformCallback: () => 0,
    invoke: async (cmd, args) => {
      const { exact, byCmd, firstByCmd } = await ready;
      // #locked=1 replays the first (pre-open) workspace status so the Welcome/Access gate can be captured.
      if (cmd === "workspace_status" && /(^|[#&])locked=1/.test(window.location.hash)) return structuredClone(firstByCmd.get(cmd));
      const k = keyOf(cmd, args || {});
      if (exact.has(k)) return structuredClone(exact.get(k));
      if (byCmd.has(cmd)) return structuredClone(byCmd.get(cmd));
      throw { kind: "unavailable", message: "QA replay: no recorded response for " + cmd };
    },
  };
})();
