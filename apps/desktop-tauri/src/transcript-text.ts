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
  engine?: { is_fixture?: boolean } | null,
): string {
  // Fixture output is not speech recognition; a pasted copy must say so.
  const fixture = engine?.is_fixture ? " · fixture engine output, not speech recognition" : "";
  const header = `MedScale draft transcript · revision ${revision.revision_no} · ${revision.state}${fixture} · not clinically verified`;
  const lines = [...revision.segments]
    .sort((a, b) => a.seq - b.seq)
    .map((s) => {
      const speaker = s.speaker?.kind === "anonymous" && s.speaker.index !== undefined ? ` speaker ${s.speaker.index}` : "";
      const body = s.text?.trim() ? s.text.trim() : "(untranscribed)";
      return `[${clock(s.start_ms)}–${clock(s.end_ms)}${speaker}] ${body}`;
    });
  return [header, "", ...lines].join("\n") + "\n";
}
