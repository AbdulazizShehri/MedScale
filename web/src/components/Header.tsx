"use client";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useRef, useState } from "react";
import { asset, github, nav } from "@/lib/site";
import { Close, GitHub, Menu } from "./Icons";

export function Header() {
  const pathname = usePathname() || "/";
  const [open, setOpen] = useState(false);
  const [scrolled, setScrolled] = useState(false);
  const toggle = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 8);
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  useEffect(() => {
    if (!open) return;
    document.body.style.overflow = "hidden";
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") { setOpen(false); toggle.current?.focus(); }
    };
    window.addEventListener("keydown", onKey);
    return () => { document.body.style.overflow = ""; window.removeEventListener("keydown", onKey); };
  }, [open]);

  useEffect(() => setOpen(false), [pathname]);

  const current = (href: string) => (pathname.replace(/\/?$/, "/").endsWith(href) ? "page" : undefined);

  return (
    <header className="site-header" data-scrolled={scrolled || open}>
      <div className="shell header-inner">
        <Link href="/" className="brand-link" aria-label="MedScale home">
          <img src={asset("/brand/medscale-horizontal-white.svg")} alt="" width={103} height={26} fetchPriority="high" />
        </Link>
        <nav className="main-nav" aria-label="Main">
          {nav.map((item) => <Link key={item.href} href={item.href} aria-current={current(item.href)}>{item.label}</Link>)}
        </nav>
        <div className="header-actions">
          <a className="btn btn-secondary btn-sm gh-link" href={github}><GitHub size={15} /> GitHub</a>
          <Link className="btn btn-primary btn-sm" href="/download/">Get MedScale</Link>
          <button ref={toggle} type="button" className="menu-toggle" aria-expanded={open} aria-controls="mobile-menu" aria-label={open ? "Close menu" : "Open menu"} onClick={() => setOpen(!open)}>
            {open ? <Close /> : <Menu />}
          </button>
        </div>
      </div>
      <div id="mobile-menu" className="mobile-menu" data-open={open} hidden={!open}>
        <nav aria-label="Main">
          <Link href="/" onClick={() => setOpen(false)}>Home</Link>
          {nav.map((item) => <Link key={item.href} href={item.href} aria-current={current(item.href)} onClick={() => setOpen(false)}>{item.label}</Link>)}
        </nav>
        <div className="actions">
          <a className="btn btn-secondary" href={github}><GitHub size={15} /> GitHub</a>
          <Link className="btn btn-primary" href="/download/">Get MedScale</Link>
        </div>
      </div>
    </header>
  );
}
