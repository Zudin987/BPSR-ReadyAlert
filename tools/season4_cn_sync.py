#!/usr/bin/env python3
"""Build a fallback-only CN/global ID delta for upcoming Season 4 data.

The base ReadyAlert catalog stays authoritative.  This script downloads the
public resonance-logs-cn English tables, compares normalized IDs against the
embedded v1.30 supplemental catalog, and writes only previously-uncovered
player-facing names.  Chinese/localized leftovers are reported separately for
manual translation rather than exposed to users.
"""
from __future__ import annotations

import argparse
import io
import json
import re
import urllib.request
from collections import defaultdict
from pathlib import Path

CN_BASE = "https://raw.githubusercontent.com/fudiyangjin/resonance-logs-cn/main/src/lib/config"
SOURCES = {
    "scene_en": f"{CN_BASE}/en-US/SceneName.json",
    "scene_cn": f"{CN_BASE}/SceneName.json",
    "monster_en": f"{CN_BASE}/en-US/MonsterIdNameType.json",
    "monster_cn": f"{CN_BASE}/MonsterIdNameType.json",
    "monster_skill_en": f"{CN_BASE}/en-US/MonsterSkillName.json",
    "monster_skill_cn": f"{CN_BASE}/MonsterSkillName.json",
    "buff_en": f"{CN_BASE}/en-US/BuffName.json",
    "buff_cn": f"{CN_BASE}/BuffName.json",
    "recount_en": f"{CN_BASE}/en-US/RecountTable.json",
    "recount_cn": f"{CN_BASE}/RecountTable.json",
}

# Explicit instance IDs following the current S3 CN/global content.  These are
# used only for the audit summary; all source rows still have to be absent from
# the base ReadyAlert catalog before they are emitted.
SEASON4_SCENE_IDS = {
    5900,
    6561, 6562, 6563, 6564, 6565,
    6591, 6592, 6593, 6594,
    6610, 6611, 6612, 6613, 6614, 6615,
}

CJK_RE = re.compile(r"[\u3400-\u9fff\uf900-\ufaff]")
DEV_RE = re.compile(
    r"(?:^|[\s_\-])(test|testing|template|placeholder|debug|dev|gm)(?:$|[\s_\-])",
    re.IGNORECASE,
)


def fetch_json(url: str):
    req = urllib.request.Request(url, headers={"User-Agent": "BPSR-ReadyAlert-season4-audit"})
    with urllib.request.urlopen(req, timeout=45) as response:
        return json.load(io.TextIOWrapper(response, encoding="utf-8-sig"))


def signed_i32(value: int) -> int:
    value &= 0xFFFF_FFFF
    return value - 0x1_0000_0000 if value >= 0x8000_0000 else value


def norm_id(kind: str, raw_id: int | str) -> int:
    value = int(raw_id)
    return signed_i32(value) if kind == "S" else value


def load_base_catalog(root: Path) -> dict[str, set[int]]:
    try:
        import zstandard as zstd
    except ImportError as exc:
        raise SystemExit("zstandard is required: pip install zstandard") from exc

    parts = sorted((root / "rust" / "data").glob("game_names_v1300.tsv.zst.*"))
    if not parts:
        raise SystemExit("base v1.30 catalog parts were not found")
    compressed = b"".join(path.read_bytes() for path in parts)
    with zstd.ZstdDecompressor().stream_reader(io.BytesIO(compressed)) as reader:
        text = io.TextIOWrapper(reader, encoding="utf-8").read()

    ids: dict[str, set[int]] = defaultdict(set)
    for line in text.splitlines():
        if not line or line.startswith("#"):
            continue
        fields = line.split("\t", 2)
        if len(fields) != 3:
            continue
        kind, raw_id, _name = fields
        try:
            ids[kind].add(norm_id(kind, raw_id))
        except ValueError:
            continue
    return ids


def clean_name(name) -> str:
    if not isinstance(name, str):
        return ""
    return " ".join(name.replace("\u200b", "").split()).strip()


def player_facing_english(name: str) -> bool:
    return bool(name) and not CJK_RE.search(name) and not DEV_RE.search(name)


def monster_name(value) -> str:
    if isinstance(value, dict):
        return clean_name(value.get("Name") or value.get("NameDesign"))
    return clean_name(value)


def buff_rows(data):
    if isinstance(data, list):
        for value in data:
            if isinstance(value, dict) and "Id" in value:
                yield value["Id"], clean_name(value.get("NameDesign") or value.get("Name"))
    elif isinstance(data, dict):
        for key, value in data.items():
            if isinstance(value, dict):
                yield value.get("Id", key), clean_name(value.get("NameDesign") or value.get("Name"))
            else:
                yield key, clean_name(value)


def add_candidate(candidates, base_ids, kind: str, raw_id, name: str, source: str):
    name = clean_name(name)
    if not name:
        return
    try:
        normalized = norm_id(kind, raw_id)
    except (TypeError, ValueError):
        return
    if normalized in base_ids.get(kind, set()):
        return
    key = (kind, normalized)
    if key not in candidates:
        candidates[key] = (str(raw_id), name, source)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--output", type=Path, default=Path("rust/data/game_names_season4_cn.tsv"))
    parser.add_argument("--report", type=Path, default=Path("docs/season4-cn-id-audit.md"))
    args = parser.parse_args()
    root = args.root.resolve()
    output = args.output if args.output.is_absolute() else root / args.output
    report = args.report if args.report.is_absolute() else root / args.report

    base_ids = load_base_catalog(root)
    data = {key: fetch_json(url) for key, url in SOURCES.items()}
    candidates = {}
    untranslated = []

    # Scene/place IDs.
    for raw_id, value in data["scene_en"].items():
        en_name = clean_name(value)
        cn_name = clean_name(data["scene_cn"].get(str(raw_id), ""))
        if player_facing_english(en_name):
            add_candidate(candidates, base_ids, "E", raw_id, en_name, "resonance-logs-cn:SceneName")
        elif cn_name and norm_id("E", raw_id) not in base_ids.get("E", set()):
            untranslated.append(("E", str(raw_id), cn_name, en_name, "SceneName"))

    # Monster/entity IDs.
    for raw_id, value in data["monster_en"].items():
        en_name = monster_name(value)
        cn_name = monster_name(data["monster_cn"].get(str(raw_id), {}))
        if player_facing_english(en_name):
            add_candidate(candidates, base_ids, "M", raw_id, en_name, "resonance-logs-cn:MonsterIdNameType")
        elif cn_name and norm_id("M", raw_id) not in base_ids.get("M", set()):
            untranslated.append(("M", str(raw_id), cn_name, en_name, "MonsterIdNameType"))

    # Monster action/skill IDs.
    for raw_id, value in data["monster_skill_en"].items():
        en_name = clean_name(value)
        cn_name = clean_name(data["monster_skill_cn"].get(str(raw_id), ""))
        if player_facing_english(en_name):
            add_candidate(candidates, base_ids, "S", raw_id, en_name, "resonance-logs-cn:MonsterSkillName")
        elif cn_name and norm_id("S", raw_id) not in base_ids.get("S", set()):
            untranslated.append(("S", str(raw_id), cn_name, en_name, "MonsterSkillName"))

    # Buff/status IDs.
    cn_buff = {str(raw_id): name for raw_id, name in buff_rows(data["buff_cn"])}
    for raw_id, en_name in buff_rows(data["buff_en"]):
        cn_name = cn_buff.get(str(raw_id), "")
        if player_facing_english(en_name):
            add_candidate(candidates, base_ids, "B", raw_id, en_name, "resonance-logs-cn:BuffName")
        elif cn_name and norm_id("B", raw_id) not in base_ids.get("B", set()):
            untranslated.append(("B", str(raw_id), cn_name, en_name, "BuffName"))

    # Recount rows and their concrete damage IDs.  Recount labels are the most
    # useful public English fallback when the game reports an otherwise unnamed
    # player skill variant.
    cn_recount = data["recount_cn"] if isinstance(data["recount_cn"], dict) else {}
    for raw_id, value in data["recount_en"].items():
        if not isinstance(value, dict):
            continue
        en_name = clean_name(value.get("RecountName") or value.get("Name"))
        cn_value = cn_recount.get(str(raw_id), {})
        cn_name = clean_name(cn_value.get("RecountName") if isinstance(cn_value, dict) else "")
        if player_facing_english(en_name):
            add_candidate(candidates, base_ids, "R", raw_id, en_name, "resonance-logs-cn:RecountTable")
            for damage_id in value.get("DamageId", []) or []:
                add_candidate(candidates, base_ids, "S", damage_id, en_name, "resonance-logs-cn:RecountTable.DamageId")
        elif cn_name and norm_id("R", raw_id) not in base_ids.get("R", set()):
            untranslated.append(("R", str(raw_id), cn_name, en_name, "RecountTable"))

    kind_order = {"S": 0, "B": 1, "E": 2, "D": 3, "M": 4, "R": 5}
    english_rows = []
    for (kind, normalized), (raw_id, name, source) in sorted(
        candidates.items(), key=lambda item: (kind_order.get(item[0][0], 99), item[0][1])
    ):
        if player_facing_english(name):
            english_rows.append((kind, raw_id, name, source, normalized))

    output.parent.mkdir(parents=True, exist_ok=True)
    lines = [
        "# BPSR CN/global Season 4 supplemental fallback IDs.",
        "# Generated by tools/season4_cn_sync.py; existing v1.30 IDs stay authoritative.",
        "# Primary source: fudiyangjin/resonance-logs-cn (public en-US config).",
    ]
    lines += [f"{kind}\t{raw_id}\t{name}" for kind, raw_id, name, _source, _norm in english_rows]
    output.write_text("\n".join(lines) + "\n", encoding="utf-8")

    counts = defaultdict(int)
    for kind, *_rest in english_rows:
        counts[kind] += 1
    s4_scene_rows = [
        row for row in english_rows
        if row[0] == "E" and int(row[1]) in SEASON4_SCENE_IDS
    ]

    report.parent.mkdir(parents=True, exist_ok=True)
    report_lines = [
        "# Season 4 CN ID audit",
        "",
        "Generated from the latest public `fudiyangjin/resonance-logs-cn` English tables and compared against ReadyAlert's embedded v1.30 supplemental catalog. Existing ReadyAlert/ZDPS names remain authoritative; this delta contains only IDs that were not already covered.",
        "",
        "## Generated missing English mappings",
        "",
        f"- Skills/actions: {counts['S']}",
        f"- Buffs/statuses: {counts['B']}",
        f"- Scenes/places: {counts['E']}",
        f"- Monsters/entities: {counts['M']}",
        f"- Recount rows: {counts['R']}",
        f"- Total: {len(english_rows)}",
        "",
        "## Season 4 scene candidates missing from the base catalog",
        "",
    ]
    if s4_scene_rows:
        report_lines += [f"- `{raw_id}` — {name}" for _kind, raw_id, name, _src, _norm in s4_scene_rows]
    else:
        report_lines.append("- None: all audited Season 4 scene IDs were already present in the base catalog.")
    report_lines += [
        "",
        "## Chinese-only / untranslated missing rows",
        "",
        "These are intentionally **not** emitted into the runtime catalog. Review and translate only player-facing rows before adding them.",
        "",
    ]
    if untranslated:
        report_lines.append("| Kind | ID | Chinese | Existing en-US value | Source |")
        report_lines.append("|---|---:|---|---|---|")
        for kind, raw_id, cn_name, en_name, source in untranslated:
            safe = lambda text: text.replace("|", "\\|")
            report_lines.append(f"| {kind} | {raw_id} | {safe(cn_name)} | {safe(en_name)} | {source} |")
    else:
        report_lines.append("No untranslated missing rows were found in the audited tables.")
    report.write_text("\n".join(report_lines) + "\n", encoding="utf-8")

    print(f"wrote {output.relative_to(root)} with {len(english_rows)} missing mappings")
    print(f"wrote {report.relative_to(root)} with {len(untranslated)} untranslated rows")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
