/** Plain-text rendering of a draft transcript revision for the clipboard.
 * Untranscribed segments are kept as explicit gaps rather than dropped, so a
 * pasted copy never looks more complete than the recorded revision. */
export type TranscriptTextSegment = {
  seq: number;
  start_ms: number;
  end_ms: number;
  text: string | null;
  speaker?: { kind: string; index?: number };
};

/** Clipboard-only escaping: make control and bidirectional formatting visible
 * rather than letting untrusted ASR text spoof timestamps or provenance.
 * This does not modify the stored transcript or establish OS clipboard privacy. */
function neutralizeClipboardText(value: string): string {
  return value
    .replace(/\r\n|\r|\n|\u000B|\u000C|\u0085|\u2028|\u2029/g, "\n")
    .replace(/[\u0000-\u0008\u000E-\u001F\u007F-\u0084\u0086-\u009F\u061C\u200E\u200F\u202A-\u202E\u2066-\u206F]/g, (control) =>
      `[U+${control.charCodeAt(0).toString(16).toUpperCase().padStart(4, "0")}]`,
    );
}

function clock(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

export function transcriptPlainText(
  revision: { revision_no: number; state: string; segments: readonly TranscriptTextSegment[] },
): string {
  const header = `MedScale draft transcript · revision ${revision.revision_no} · ${neutralizeClipboardText(revision.state).replace(/\n/g, " ")} · not clinically verified`;
  const lines = [...revision.segments]
    .sort((a, b) => a.seq - b.seq)
    .map((s) => {
      const speaker = s.speaker?.kind === "anonymous" && s.speaker.index !== undefined ? ` speaker ${s.speaker.index}` : "";
      const body = s.text?.trim() ? s.text.trim() : "(untranscribed)";
      // Keep ASR/correction line breaks but indent continuations so untrusted
      // speech text cannot visually impersonate a new timed segment or header.
      const boundedLines = neutralizeClipboardText(body).replace(/\n/g, "\n    ");
      return `[${clock(s.start_ms)}–${clock(s.end_ms)}${speaker}] ${boundedLines}`;
    });
  return [header, "", ...lines].join("\n") + "\n";
}

/** One explicit, write-only clipboard effect. The caller owns the user gesture.
 * No read, retry, silent fallback, or OS clipboard-clearance claim is made. */
export type TranscriptClipboardWriter = { writeText(text: string): Promise<void> };

export async function copyTranscriptDraft(
  revision: Parameters<typeof transcriptPlainText>[0],
  writer: TranscriptClipboardWriter | undefined,
): Promise<void> {
  if (!writer || typeof writer.writeText !== "function") {
    throw new Error("Clipboard write unavailable");
  }
  await writer.writeText(transcriptPlainText(revision));
}
