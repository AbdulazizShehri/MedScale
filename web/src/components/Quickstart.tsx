"use client";
import { useState } from "react";

const lines = [
  "git clone https://github.com/AbdulazizShehri/MedScale.git",
  "cd MedScale",
  "cargo run -p medscale-cli --locked -- --version",
  "cargo run -p medscale-cli --locked -- doctor",
];

export function Quickstart() {
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(lines.join("\n"));
      setCopied(true);
      setTimeout(() => setCopied(false), 1600);
    } catch { /* clipboard unavailable: the commands stay selectable */ }
  };
  return (
    <div className="terminal">
      <div className="terminal-bar">
        <span aria-hidden="true"><i /><i /><i /></span>
        <span>Rust 1.97 · Windows, macOS, Linux</span>
        <button type="button" className="copy-btn" onClick={copy} aria-live="polite">{copied ? "Copied" : "Copy"}</button>
      </div>
      <pre aria-label="Quickstart commands" tabIndex={0}><code>
        <span className="comment"># Build and check the CLI from source</span>{"\n"}
        {lines.map((l) => <span key={l}><span className="prompt">$ </span>{l}{"\n"}</span>)}
      </code></pre>
    </div>
  );
}
