# SPDX-License-Identifier: Apache-2.0
"""Minimal stdlib YAML-subset loader for offline vendored clinical data files."""

from __future__ import annotations

import json
import pathlib
from typing import Any


def _parse_scalar(raw: str) -> Any:
    s = raw.strip()
    if not s or s in {"null", "Null", "NULL", "~"}:
        return None
    if s in {"true", "True", "TRUE"}:
        return True
    if s in {"false", "False", "FALSE"}:
        return False
    if (s.startswith('"') and s.endswith('"')) or (s.startswith("'") and s.endswith("'")):
        return s[1:-1]
    if s.startswith("[") and s.endswith("]"):
        try:
            return json.loads(s)
        except Exception:
            inner = s[1:-1].strip()
            return [_parse_scalar(x) for x in inner.split(",")] if inner else []
    try:
        return int(s)
    except ValueError:
        pass
    try:
        return float(s)
    except ValueError:
        return s


def safe_load(stream: Any) -> Any:
    text = stream.read() if hasattr(stream, "read") else str(stream)
    stripped = text.lstrip()
    if stripped.startswith("{") or stripped.startswith("["):
        return json.loads(text)
    lines = []
    for raw in text.splitlines():
        if not raw.strip() or raw.lstrip().startswith("#"):
            continue
        indent = len(raw) - len(raw.lstrip(" "))
        lines.append((indent, raw.strip()))
    if not lines:
        return None

    def parse_block(idx: int, min_indent: int) -> tuple[Any, int]:
        is_list = lines[idx][1].startswith("- ") or lines[idx][1] == "-"
        if is_list:
            out: list[Any] = []
            cur_indent = lines[idx][0]
            while idx < len(lines) and lines[idx][0] == cur_indent and (
                lines[idx][1].startswith("- ") or lines[idx][1] == "-"
            ):
                item_str = lines[idx][1][2:].strip() if lines[idx][1].startswith("- ") else ""
                idx += 1
                if not item_str:
                    val, idx = parse_block(idx, cur_indent + 1)
                    out.append(val)
                elif ":" in item_str and not (item_str.startswith('"') or item_str.startswith("'")):
                    k, v = item_str.split(":", 1)
                    d = {k.strip(): _parse_scalar(v) if v.strip() else None}
                    if not v.strip() and idx < len(lines) and lines[idx][0] > cur_indent + 1:
                        d[k.strip()], idx = parse_block(idx, cur_indent + 2)
                    while idx < len(lines) and lines[idx][0] >= cur_indent + 2 and not lines[idx][1].startswith("- "):
                        sub_k, sub_v = lines[idx][1].split(":", 1)
                        sub_indent = lines[idx][0]
                        idx += 1
                        if sub_v.strip():
                            d[sub_k.strip()] = _parse_scalar(sub_v)
                        elif idx < len(lines) and lines[idx][0] > sub_indent:
                            d[sub_k.strip()], idx = parse_block(idx, sub_indent + 1)
                        else:
                            d[sub_k.strip()] = {}
                    out.append(d)
                else:
                    out.append(_parse_scalar(item_str))
            return out, idx
        mapping: dict[str, Any] = {}
        cur_indent = lines[idx][0]
        while idx < len(lines) and lines[idx][0] == cur_indent and not lines[idx][1].startswith("- "):
            k, v = lines[idx][1].split(":", 1)
            key = k.strip().strip('"').strip("'")
            idx += 1
            if v.strip():
                mapping[key] = _parse_scalar(v)
            elif idx < len(lines) and lines[idx][0] > cur_indent:
                mapping[key], idx = parse_block(idx, cur_indent + 1)
            else:
                mapping[key] = {}
        return mapping, idx

    result, _ = parse_block(0, 0)
    return result
