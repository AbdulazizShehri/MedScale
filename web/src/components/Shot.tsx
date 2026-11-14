import { asset, shots, type ShotKey } from "@/lib/site";

/** A real desktop screenshot (synthetic workspace), shown unaltered with its provenance. */
export function Shot({ name, priority = false, caption = true }: { name: ShotKey; priority?: boolean; caption?: boolean }) {
  const s = shots[name];
  return (
    <figure style={{ margin: 0 }}>
      <div className="frame">
        <img src={asset(s.src)} alt={s.alt} width={s.w} height={s.h} loading={priority ? "eager" : "lazy"} decoding="async" />
      </div>
      {caption && <figcaption className="frame-caption"><span>Real screenshot · MedScale desktop for Windows</span><span>Synthetic workspace · no patient data</span></figcaption>}
    </figure>
  );
}
