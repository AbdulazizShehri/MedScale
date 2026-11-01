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

function clock(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

export function transcriptPlainText(
  revision: { revision_no: number; state: string; segments: readonly TranscriptTextSegment[] },
): string {
  const header = `MedScale draft transcript · revision ${revision.revision_no} · ${revision.state} · not clinically verified`;
  const lines = [...revision.segments]
    .sort((a, b) => a.seq - b.seq)
    .map((s) => {
      const speaker = s.speaker?.kind === "anonymous" && s.speaker.index !== undefined ? ` speaker ${s.speaker.index}` : "";
      const body = s.text?.trim() ? s.text.trim() : "(untranscribed)";
      // Keep ASR/correction line breaks but indent continuations so untrusted
      // speech text cannot visually impersonate a new timed segment or header.
      const boundedLines = body.replace(/\r\n|\r|\n|\u2028|\u2029/g, "\n    ");
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
