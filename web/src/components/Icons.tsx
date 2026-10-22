type P = { size?: number; className?: string };
const base = (size: number) => ({ width: size, height: size, viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: 1.6, strokeLinecap: "round" as const, strokeLinejoin: "round" as const, "aria-hidden": true as const });

export const ArrowRight = ({ size = 16, className }: P) => <svg {...base(size)} className={"arrow " + (className ?? "")}><path d="M5 12h14M13 6l6 6-6 6" /></svg>;
export const ArrowUpRight = ({ size = 14, className }: P) => <svg {...base(size)} className={"arrow " + (className ?? "")}><path d="M7 17 17 7M8 7h9v9" /></svg>;
export const Check = ({ size = 16 }: P) => <svg {...base(size)}><path d="m5 12.5 4.2 4.2L19 7" /></svg>;
export const Plus = ({ size = 18 }: P) => <svg {...base(size)}><path d="M12 5v14M5 12h14" /></svg>;
export const Menu = ({ size = 18 }: P) => <svg {...base(size)}><path d="M4 8h16M4 16h16" /></svg>;
export const Close = ({ size = 18 }: P) => <svg {...base(size)}><path d="M6 6l12 12M18 6 6 18" /></svg>;
export const Lock = ({ size = 16 }: P) => <svg {...base(size)}><rect x="5" y="11" width="14" height="9" rx="2" /><path d="M8 11V8a4 4 0 0 1 8 0v3" /></svg>;
export const Terminal = ({ size = 16 }: P) => <svg {...base(size)}><rect x="3" y="4.5" width="18" height="15" rx="2.5" /><path d="m7.5 10 3 2.5-3 2.5M13 15h4" /></svg>;
export const Monitor = ({ size = 16 }: P) => <svg {...base(size)}><rect x="3" y="4" width="18" height="12" rx="2" /><path d="M9 20h6M12 16v4" /></svg>;
export const Doc = ({ size = 16 }: P) => <svg {...base(size)}><path d="M7 3h7l4 4v14H7z" /><path d="M14 3v4h4M10 12h5M10 16h5" /></svg>;

export function GitHub({ size = 16 }: P) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M12 1.8a10.2 10.2 0 0 0-3.23 19.88c.51.1.7-.22.7-.49v-1.72c-2.84.62-3.44-1.37-3.44-1.37-.46-1.18-1.13-1.5-1.13-1.5-.93-.63.07-.62.07-.62 1.02.07 1.56 1.05 1.56 1.05.91 1.56 2.39 1.11 2.97.85.09-.66.36-1.11.65-1.37-2.27-.26-4.65-1.13-4.65-5.04 0-1.11.4-2.02 1.05-2.73-.1-.26-.46-1.3.1-2.7 0 0 .86-.28 2.8 1.04a9.7 9.7 0 0 1 5.1 0c1.94-1.32 2.8-1.04 2.8-1.04.56 1.4.2 2.44.1 2.7.65.71 1.05 1.62 1.05 2.73 0 3.92-2.39 4.78-4.66 5.03.37.32.7.94.7 1.9v2.81c0 .27.18.6.7.49A10.2 10.2 0 0 0 12 1.8Z" /></svg>;
}

export type PlatformName = "windows" | "macos" | "linux";
export function PlatformIcon({ name, size = 26 }: { name: PlatformName; size?: number }) {
  const common = { width: size, height: size, viewBox: "0 0 24 24", fill: "currentColor", "aria-hidden": true as const };
  if (name === "windows") return <svg {...common}><path d="M3 5.1 10.4 4v7.3H3V5.1Zm8.4-1.25L21 2.5v8.8h-9.6V3.85ZM3 12.7h7.4V20L3 18.9v-6.2Zm8.4 0H21v8.8l-9.6-1.35V12.7Z" /></svg>;
  if (name === "macos") return <svg {...common}><path d="M16.6 12.8c0-2.4 2-3.6 2.1-3.6-1.1-1.7-2.9-1.9-3.6-1.9-1.5-.2-3 .9-3.7.9-.8 0-2-.9-3.2-.9-1.7 0-3.2 1-4.1 2.5-1.7 3-.4 7.4 1.2 9.8.8 1.2 1.8 2.5 3 2.4 1.2 0 1.7-.8 3.1-.8 1.5 0 1.9.8 3.2.8 1.3 0 2.2-1.2 3-2.4.9-1.4 1.3-2.7 1.3-2.8 0 0-2.4-1-2.3-4Zm-2.4-7.1c.7-.8 1.1-1.9 1-3-1 0-2.1.7-2.8 1.5-.6.7-1.2 1.8-1 2.9 1 .1 2.1-.6 2.8-1.4Z" /></svg>;
  return <svg {...common}><path d="M12 2c-2.3 0-3.7 1.9-3.7 5 0 1.1.2 2.3.5 3.2-.4.5-.8 1.2-1.1 2-.7 1.7-1 3.4-.9 4.2L4.6 18.8c-.4.4-.1 1.2.5 1.2h2.6l1.6-1.7c.8.4 1.7.6 2.7.6s1.9-.2 2.7-.6l1.6 1.7h2.6c.6 0 .9-.8.5-1.2l-2.2-2.4c.1-.8-.2-2.5-.9-4.2-.3-.8-.7-1.5-1.1-2 .3-.9.5-2.1.5-3.2C15.7 3.9 14.3 2 12 2Zm-1.5 4.7a.8.8 0 1 1 0 1.6.8.8 0 0 1 0-1.6Zm3 0a.8.8 0 1 1 0 1.6.8.8 0 0 1 0-1.6ZM9.3 12.8c.8.6 1.7.9 2.7.9s1.9-.3 2.7-.9c-.2 1.4-1.2 3-2.7 3s-2.5-1.6-2.7-3Z" /></svg>;
}
