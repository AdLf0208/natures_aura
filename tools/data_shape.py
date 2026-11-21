#!/usr/bin/env python3
"""Check that the data naturesaura carries is in 26.3's dialect, not 1.21.1's.

A carried recipe is only as good as its JSON: 26.3 rejects the 1.21.1 ingredient
object form at load (`recipes: invalid ingredient (unresolved tag or malformed
input)`), so the recipe silently returns nothing — while `check`, `fits` and the
unit tests all stay green.  The two carried recipes are the only way a player
reaches the altar's gold bricks from vanilla, so a dead one is a dead progression.

26.3 takes a bare string in a shaped recipe's `key` and a shapeless recipe's
`ingredients`: `"minecraft:stone_bricks"` for an item, `"#c:nuggets/gold"` for
a tag.  The object form `{"item": …}` / `{"tag": …}` is the 1.21.1 dialect.

A loot-table pool's singular `"condition"` object is the 26.3 shape too; the
1.21.1 plural `"conditions"` array drops nothing, so it is refused here as well.

    python3 tools/data_shape.py <package.hmod> [more.hmod]
    python3 tools/data_shape.py --data <dir> [more dirs/files]

Exit 0 when every carried recipe and loot table is in the 26.3 dialect, 1
otherwise, printing each offender with the value it found.
"""
from __future__ import annotations

import json
import sys
import zipfile
from pathlib import Path

CRAFTING = ("minecraft:crafting_shaped", "minecraft:crafting_shapeless")


def recipe_offenders(recipe: dict) -> list[str]:
    """Every ingredient of one recipe that is not a bare item id / #tag string."""
    kind = recipe.get("type", "")
    if kind not in CRAFTING:
        return []
    if kind == "minecraft:crafting_shaped":
        values = list(recipe.get("key", {}).values())
    else:
        values = list(recipe.get("ingredients", []))
    return [json.dumps(v) for v in values if not isinstance(v, str)]


def loot_offenders(table: dict) -> list[str]:
    """Every pool of a loot table still using the 1.21.1 `conditions` array."""
    return [json.dumps(pool["conditions"])
            for pool in table.get("pools", [])
            if "conditions" in pool]


def check_recipe(name: str, parsed, bad: list[str]) -> None:
    for recipe in parsed if isinstance(parsed, list) else [parsed]:
        wrong = recipe_offenders(recipe)
        if wrong:
            bad.append(f"{name}: {recipe.get('type', '?')} ingredient(s) in the 1.21.1 "
                       "object form: " + ", ".join(wrong))
        elif recipe.get("type", "") in CRAFTING:
            print(f"  ok   {name}: {recipe['type'].split(':')[1]}, bare item/tag strings")


def check_loot(name: str, parsed, bad: list[str]) -> None:
    wrong = loot_offenders(parsed)
    if wrong:
        bad.append(f"{name}: loot pool condition in the 1.21.1 array form "
                   f"({wrong[0]}); 26.3 uses \"condition\": {{...}}")
    else:
        print(f"  ok   {name}: loot pool uses the 26.3 condition form")


def read(name: str) -> list[tuple[str, str]]:
    """(name, text) for every recipe and loot table under a package or a data dir."""
    found: list[tuple[str, str]] = []
    if name.endswith(".hmod"):
        with zipfile.ZipFile(name) as archive:
            for entry in sorted(archive.namelist()):
                if entry.startswith("data/") and entry.endswith(".json") \
                        and ("/recipe/" in entry or "/loot_table/" in entry):
                    found.append((f"{Path(name).name}:{entry}",
                                  archive.read(entry).decode()))
        return found
    root = Path(name)
    files = sorted(root.rglob("*.json")) if root.is_dir() else [root]
    for path in files:
        if "/recipe" in str(path) or "/loot_table" in str(path):
            found.append((str(path), path.read_text()))
    return found


def main(argv: list[str]) -> int:
    if not argv:
        print(__doc__.splitlines()[-1], file=sys.stderr)
        return 64
    targets = argv[1:] if argv[0] == "--data" else argv
    bad: list[str] = []
    checked = 0
    for target in targets:
        for name, text in read(target):
            try:
                parsed = json.loads(text)
            except json.JSONDecodeError as error:
                bad.append(f"{name}: not JSON ({error})")
                continue
            checked += 1
            if "/loot_table/" in name or "loot_table" in name:
                check_loot(name, parsed, bad)
            else:
                check_recipe(name, parsed, bad)
    if bad:
        for line in bad:
            print(f"  FAIL {line}", file=sys.stderr)
        return 1
    print(f"data shape: {checked} recipe/loot file(s), every one in the 26.3 dialect")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
