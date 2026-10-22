import { asset, shots, type ShotKey } from "@/lib/site";

/** A real desktop screenshot (synthetic workspace) in a window frame. */
export function Shot({ name, title, priority = false, caption = true, fade = false }: { name: ShotKey; title: string; priority?: boolean; caption?: boolean; fade?: boolean }) {
  const s = shots[name];
  return (
    <figure style={{ margin: 0 }}>
      <div className={"frame" + (fade ? " frame-fade" : "")}>
        <div className="frame-inner">
          <div className="frame-bar" aria-hidden="true"><i /><i /><i /><span>MedScale — {title}</span></div>
          <img src={asset(s.src)} alt={s.alt} width={s.w} height={s.h} loading={priority ? "eager" : "lazy"} fetchPriority={priority ? "high" : "auto"} decoding="async" />
        </div>
      </div>
      {caption && <figcaption className="frame-caption"><span>Real screenshot · MedScale desktop for Windows</span><span>Synthetic workspace · no patient data</span></figcaption>}
    </figure>
  );
}
