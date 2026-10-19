// MedScale Logo 12 (founder-approved 2026-10-19). Generated from the brand
// masters in assets/brand; the path data must stay identical to them
// (crates/medscale-core/tests/brand_foundation_095.rs enforces this).
// The wordmark is first-party monoline geometry, not a font.

const FRAMES = [
  "M13.5 390.5L13.5 187Q13.5 136 59.93 114.91L270.07 19.47Q316.5 -1.62 316.5 49.38L316.5 449.56Q316.5 500.56 270.46 522.51L59.54 623.05Q13.5 645 13.5 594Z",
  "M209.5 725L209.5 293.5Q209.5 242.5 255.93 221.41L468.57 124.83Q515 103.74 515 154.74L515 415Q515 466 471.46 492.56L154.54 685.94Q111 712.5 111 661.5L111 237.5Q111 186.5 157.43 165.41L364.07 71.56Q410.5 50.47 410.5 101.47L410.5 529.76"
];
const WORDMARK = [
  "M11.5 180L11.5 25.85Q11.5 14.5 22.85 14.5L22.85 14.5Q34.2 14.5 38.27 25.1L84.18 144.73Q93.5 169 102.82 144.73L148.73 25.1Q152.8 14.5 164.15 14.5L164.15 14.5Q175.5 14.5 175.5 25.85L175.5 180",
  "M229.5 113L311.5 113Q329.5 113 329.5 95L329.5 76.5Q329.5 58.5 311.5 58.5L247.5 58.5Q229.5 58.5 229.5 76.5L229.5 150.5Q229.5 168.5 247.5 168.5L341 168.5",
  "M478.5 58.5L397.5 58.5Q379.5 58.5 379.5 76.5L379.5 150.5Q379.5 168.5 397.5 168.5L478.5 168.5",
  "M478.5 0L478.5 180",
  "M656 14.5L551.5 14.5Q533.5 14.5 533.5 32.5L533.5 72Q533.5 90 551.5 90L626.5 90Q644.5 90 644.5 108L644.5 150.5Q644.5 168.5 626.5 168.5L522 168.5",
  "M792 58.5L714.5 58.5Q696.5 58.5 696.5 76.5L696.5 150.5Q696.5 168.5 714.5 168.5L792 168.5",
  "M815 58.5L904.5 58.5Q922.5 58.5 922.5 76.5L922.5 180",
  "M922.5 111L844.5 111Q826.5 111 826.5 129L826.5 150.5Q826.5 168.5 844.5 168.5L922.5 168.5",
  "M976.5 0L976.5 180",
  "M1030.5 113L1111.5 113Q1129.5 113 1129.5 95L1129.5 76.5Q1129.5 58.5 1111.5 58.5L1048.5 58.5Q1030.5 58.5 1030.5 76.5L1030.5 150.5Q1030.5 168.5 1048.5 168.5L1141 168.5"
];
const SYMBOL_BOX = [528, 725] as const;
const WORDMARK_BOX = [1141, 180] as const;

/** The three-frame symbol. Below 48 px it uses the small-size optical stroke. */
export function Mark({ height = 32, className, title }: { height?: number; className?: string; title?: string }) {
  const [w, h] = SYMBOL_BOX;
  return (
    <svg height={height} width={Math.round((height * w) / h)} viewBox={`0 0 ${w} ${h}`} className={className}
      role={title ? "img" : undefined} aria-label={title} aria-hidden={title ? undefined : true}>
      <g fill="none" stroke="currentColor" strokeWidth={height < 48 ? 42.4 : 26.5} strokeLinejoin="round" strokeLinecap="butt">
        {FRAMES.map((d) => <path key={d} d={d} />)}
      </g>
    </svg>
  );
}

/** The MedScale wordmark at a given cap-to-baseline height, one uniform stroke. */
export function Wordmark({ height = 16, className, title = "MedScale" }: { height?: number; className?: string; title?: string }) {
  const [w, h] = WORDMARK_BOX;
  return (
    <svg height={height} width={Math.round((height * w) / h)} viewBox={`0 0 ${w} ${h}`} className={className} role="img" aria-label={title}>
      <g fill="none" stroke="currentColor" strokeWidth={23} strokeLinejoin="round" strokeLinecap="butt">
        {WORDMARK.map((d) => <path key={d} d={d} />)}
      </g>
    </svg>
  );
}

/** Horizontal lockup: symbol beside the wordmark. */
export function Logo({ height = 32, className }: { height?: number; className?: string }) {
  return (
    <span className={className} style={{ display: "inline-flex", alignItems: "center", gap: Math.round(height * 0.3) }}>
      <Mark height={height} />
      <Wordmark height={Math.round(height * 0.42)} />
    </span>
  );
}
