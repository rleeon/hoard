#!/usr/bin/env python3
"""
Convert the Ludusavi manifest YAML into a compact JSON catalog for
embedding in `hoard-manifest`.

The raw YAML is ~17 MB and slow to parse with serde-yaml at runtime. We
strip it to just what detection needs (display name, steam id, per-OS save
paths with optional store constraint + tags), and emit one minified JSON
object per game so the consumer can either parse the whole array or
stream entries.

Run:
    python3 crates/hoard-manifest/data/convert-ludusavi.py

Inputs : crates/hoard-manifest/data/ludusavi-manifest.yaml
Outputs: crates/hoard-manifest/data/ludusavi-catalog.json
"""

import json
import sys
import yaml
from pathlib import Path

HERE = Path(__file__).parent
SRC = HERE / "ludusavi-manifest.yaml"
DST = HERE / "ludusavi-catalog.json"

OS_ALIASES = {
    "windows": "windows",
    "win": "windows",
    "linux": "linux",
    "mac": "mac",
    "macos": "mac",
    "osx": "mac",
    "darwin": "mac",
}


def slugify(name: str) -> str:
    out = []
    last_dash = True
    for ch in name:
        c = ch.lower()
        if c.isascii() and c.isalnum():
            out.append(c)
            last_dash = False
        elif not last_dash:
            out.append("-")
            last_dash = True
    s = "".join(out).strip("-")
    if not s:
        s = "game"
    if len(s) > 96:
        s = s[:96].rstrip("-")
    if not (s[0].isascii() and s[0].isalnum()):
        s = "g" + s
    return s


def transform_files(files: dict) -> dict:
    """Convert a Ludusavi `files:` block into our PathsByOs shape."""
    windows: list = []
    linux: list = []
    mac: list = []

    for path_template, p in (files or {}).items():
        p = p or {}
        tags = p.get("tags") or []
        # Keep only save paths. Many entries have no tags at all -- we keep
        # those too (Ludusavi convention is "if no tags, it's a save").
        if tags and not any(t == "save" for t in tags):
            continue

        when = p.get("when") or []
        oses: list = []
        if not when:
            oses = ["windows", "linux", "mac"]
        else:
            for w in when:
                if not w:
                    continue
                os_name = (w or {}).get("os")
                if not os_name:
                    continue
                norm = OS_ALIASES.get(os_name.lower())
                if norm and norm not in oses:
                    oses.append(norm)
            if not oses:
                oses = ["windows", "linux", "mac"]

        if not when:
            constraints = [{}]
        else:
            constraints = []
            for w in when:
                w = w or {}
                store = w.get("store")
                if store:
                    constraints.append({"store": store})
                else:
                    constraints.append({})

        if not tags:
            tags_out = ["save"]
        else:
            tags_out = list(tags)

        entry = {"path": path_template, "constraints": constraints, "tags": tags_out}

        if "windows" in oses:
            windows.append(entry)
        if "linux" in oses:
            linux.append(entry)
        if "mac" in oses:
            mac.append(entry)

    return {"windows": windows, "linux": linux, "mac": mac}


def main():
    if not SRC.exists():
        print(f"ERROR: {SRC} not found", file=sys.stderr)
        sys.exit(1)

    print(f"Reading {SRC} ...")
    with SRC.open("r", encoding="utf-8") as f:
        data = yaml.safe_load(f)

    catalog = []
    skipped_alias = 0
    skipped_no_paths = 0
    seen_slugs: set = set()

    for display_name, entry in data.items():
        entry = entry or {}
        if entry.get("alias"):
            skipped_alias += 1
            continue
        paths = transform_files(entry.get("files") or {})
        if not (paths["windows"] or paths["linux"] or paths["mac"]):
            skipped_no_paths += 1
            continue

        slug = slugify(display_name)
        # Collisions on slug are extremely rare in Ludusavi, but disambiguate
        # to keep the catalog deterministic.
        if slug in seen_slugs:
            i = 2
            while f"{slug}-{i}" in seen_slugs:
                i += 1
            slug = f"{slug}-{i}"
        seen_slugs.add(slug)

        steam = entry.get("steam") or {}
        steam_id = steam.get("id")

        rec = {
            "slug": slug,
            "display_name": display_name,
            "paths": paths,
        }
        if steam_id is not None:
            rec["steam_app_id"] = int(steam_id)
        catalog.append(rec)

    # Stable order so binary diffs are review-friendly.
    catalog.sort(key=lambda r: r["slug"])

    print(
        f"Parsed {len(data)} entries: "
        f"{len(catalog)} kept, {skipped_alias} alias, {skipped_no_paths} no-paths"
    )
    print(f"Writing {DST} ...")
    with DST.open("w", encoding="utf-8") as f:
        # Compact (no spaces, no indent). Saves ~30% over indent=2.
        json.dump(catalog, f, separators=(",", ":"), ensure_ascii=False)
    print(f"Done. Output size: {DST.stat().st_size / 1024 / 1024:.2f} MB")


if __name__ == "__main__":
    main()
