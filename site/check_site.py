#!/usr/bin/env python3
"""Validate the dependency-free public MedScale landing site before deployment."""
from __future__ import annotations

from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urlparse
import sys

SITE = Path(__file__).resolve().parent


class SiteParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.ids: list[str] = []
        self.links: list[str] = []
        self.images: list[dict[str, str | None]] = []
        self.h1_count = 0
        self.title_count = 0
        self.lang = None
        self.scripts = 0

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        attributes = dict(attrs)
        if tag == "html":
            self.lang = attributes.get("lang")
        if attributes.get("id"):
            self.ids.append(attributes["id"])
        if tag == "a" or tag == "link":
            if attributes.get("href"):
                self.links.append(attributes["href"])
        if tag == "img":
            self.images.append(attributes)
        if tag == "h1":
            self.h1_count += 1
        if tag == "title":
            self.title_count += 1
        if tag == "script":
            self.scripts += 1


def main() -> int:
    html = (SITE / "index.html").read_text(encoding="utf-8")
    css = (SITE / "styles.css").read_text(encoding="utf-8")
    parser = SiteParser()
    parser.feed(html)
    errors: list[str] = []

    def require(condition: bool, message: str) -> None:
        if not condition:
            errors.append(message)

    require(html.lstrip().lower().startswith("<!doctype html>"), "Missing HTML5 doctype")
    require(parser.lang == "en", "Document language must be en")
    require(parser.title_count == 1, "Exactly one title is required")
    require(parser.h1_count == 1, "Exactly one h1 is required")
    require(not parser.scripts, "No JavaScript or tracking scripts are permitted")
    require(len(set(parser.ids)) == len(parser.ids), "Duplicate DOM IDs")
    require("Synthetic data only" in html and "Not for clinical use" in html, "Preview warning is missing")
    require("NOT AN APPLICATION SCREENSHOT OR CLINICAL RECORD" in html, "Mockup disclosure is missing")
    require("not clinically validated" in html and "real patient" in html, "Qualification status is incomplete")
    require("MedScale" in html and "Evidence first." in html, "Canonical brand copy is missing")
    require("url(" not in css.lower(), "External or implicit CSS asset URLs are not allowed")
    require("var(--blue)" in css, "Canonical brand accent is missing")
    for img in parser.images:
        require("alt" in img, "Image is missing alt text")
        if img.get("src"):
            require((SITE / img["src"]).is_file(), f"Image does not exist: {img['src']}")
    for href in parser.links:
        if href.startswith("#"):
            require(href[1:] in parser.ids, f"Unknown fragment: {href}")
        elif href.startswith(("http:", "https:")):
            parsed = urlparse(href)
            require(parsed.scheme == "https", f"Non-HTTPS outbound link: {href}")
        else:
            parsed = urlparse(href)
            require(not parsed.query and not parsed.fragment, f"Unexpected local URL suffix: {href}")
            require((SITE / parsed.path).is_file(), f"Local resource missing: {href}")
    if errors:
        print("\n".join(f"FAIL: {error}" for error in errors))
        return 1
    print("PASS: MedScale landing page links, assets, disclosure, semantic checks and dependency boundary")
    return 0


if __name__ == "__main__":
    sys.exit(main())
