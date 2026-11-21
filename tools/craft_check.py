#!/usr/bin/env python3
"""Craft both recipes the naturesaura package carries, in a real crafting table.

Called by `tools/e2e.sh` with the open game's control socket. The harness's
crafting table exposes the 3x3 grid as slots 100-108 (row-major) and the result
as 200 (`ctl screen`). For each carried recipe this:

1. replaces the table (a fresh block entity, so a stale grid cannot leak in)
   and clears the inventory, in survival mode so ingredients are consumed;
2. gives exactly the recipe's ingredients;
3. places them in the shape the recipe declares (one right-click per slot);
4. asserts the result's **id and count**;
5. shift-clicks the result into the inventory and asserts it arrived;
6. asserts the grid is empty and the recipe's distinctive ingredient was
   consumed.

Two deliberately wrong patterns must leave the result empty, so the check cannot
pass on a table that crafts whatever it is handed. Exits 1 on the first
mismatch, printing the observed result.

The pair matters: `gold_fiber` is the shaped recipe (leaves, gold nuggets from
`#c:nuggets/gold`, short grass) and `gold_brick` is the shapeless one that
consumes the fibre — the only route from vanilla blocks to the gold bricks the
altar's multiblock needs.

    python3 tools/craft_check.py SOCK CTL X Y Z
"""
import json
import subprocess
import sys

# The crafting table's grid, row-major, and its result.
GRID = {
    "tl": 100, "tm": 101, "tr": 102,
    "ml": 103, "mm": 104, "mr": 105,
    "bl": 106, "bm": 107, "br": 108,
}
RESULT = 200

GOLD_FIBER = "naturesaura:gold_fiber"
GOLD_BRICK = "naturesaura:gold_brick"


def main() -> int:
    sock, ctl, x, y, z = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5]

    def run(*args):
        out = subprocess.run([ctl, "--socket", sock, *args], text=True, capture_output=True)
        try:
            return json.loads(out.stdout)
        except json.JSONDecodeError:
            raise SystemExit(f"ctl {' '.join(args)}: {out.stdout}{out.stderr}")

    def wait(ticks=4):
        run("wait", str(ticks))

    def slots():
        screen = run("screen")
        if "slots" not in screen:
            raise SystemExit(f"no menu is open: {screen}")
        return screen["slots"]

    def find(item):
        for slot in slots():
            if slot.get("item") == item:
                return slot["slot"]
        raise SystemExit(f"missing ingredient {item} in the inventory")

    def result():
        stack = [s for s in slots() if s["slot"] == RESULT]
        if not stack or not stack[0].get("item"):
            return None
        return stack[0]

    def grid():
        return [s for s in slots() if 100 <= s["slot"] <= 108]

    def grid_is_empty():
        return all(not s.get("item") for s in grid())

    def count(item):
        return run("inventory").get("counts", {}).get(item, 0)

    def place(item, targets):
        # One pickup per target: a max-stack-1 ingredient cannot fill two slots
        # from one held stack, while a stack of four can.
        for target in targets:
            slot = find(item)
            run("click-slot", str(slot))
            wait(1)
            run("click-slot", str(target), "right")
            wait(1)
            run("click-slot", str(slot))
            wait(1)

    def setup(ingredients):
        try:
            run("close")
        except SystemExit:
            pass
        wait(4)
        run("setblock", x, y, z, "minecraft:air")
        run("setblock", x, y, z, "minecraft:crafting_table")
        wait(2)
        run("command", f"tp @s {float(x) - 0.5} {y} {float(z) + 0.5}")
        wait(3)
        run("command", "clear @s")
        for item, n in ingredients:
            run("give", item, str(n))
        wait(3)
        run("interact", x, y, z)
        wait(5)

    failures = []

    def check(label, ingredients, placements, want_item, want_count, consumed_item):
        setup(ingredients)
        for item, targets in placements:
            place(item, targets)
        got = result()
        print(f"  {label} -> {json.dumps(got)}")
        if not got or got.get("item") != want_item or got.get("count") != want_count:
            print(f"    grid: {json.dumps(grid())}")
            failures.append(f"{label}: expected {want_count} {want_item}, got {got}")
            return
        run("click-slot", str(RESULT), "shift")
        wait(4)
        made = count(want_item)
        if made < want_count:
            failures.append(f"{label}: the crafted stack reached the inventory as {made}")
        if not grid_is_empty():
            failures.append(f"{label}: the grid still holds ingredients after the craft")
        left = count(consumed_item)
        if left != 0:
            failures.append(f"{label}: {consumed_item} was not consumed ({left} left)")
        run("command", "clear @s")
        wait(2)

    def empty_control(label, ingredients, placements):
        setup(ingredients)
        for item, targets in placements:
            place(item, targets)
        got = result()
        print(f"  {label} -> {json.dumps(got)}")
        if got is not None:
            failures.append(f"{label}: the control crafted {got['item']} x{got['count']}")

    def every(*slots):
        return list(slots)

    # --- the two recipes the package carries ---------------------------------
    # gold_fiber:  L N L   with L from #minecraft:leaves, N from #c:nuggets/gold
    #              N G N   and G = minecraft:short_grass; four at a time.
    #              L N L
    check(
        "gold fiber: 4 leaves, 4 gold nuggets and short grass around the centre, four at a time",
        [
            ("minecraft:oak_leaves", 4),
            ("minecraft:gold_nugget", 4),
            ("minecraft:short_grass", 1),
        ],
        [
            ("minecraft:oak_leaves", every(GRID["tl"], GRID["tr"], GRID["bl"], GRID["br"])),
            ("minecraft:gold_nugget", every(GRID["tm"], GRID["ml"], GRID["mr"], GRID["bm"])),
            ("minecraft:short_grass", every(GRID["mm"])),
        ],
        GOLD_FIBER, 4, "minecraft:gold_nugget",
    )
    # gold_brick: stone bricks + the fibre, shapeless.
    check(
        "gold brick: stone bricks and gold fiber, shapeless",
        [("minecraft:stone_bricks", 1), (GOLD_FIBER, 1)],
        [
            ("minecraft:stone_bricks", every(GRID["tl"])),
            (GOLD_FIBER, every(GRID["mm"])),
        ],
        GOLD_BRICK, 1, GOLD_FIBER,
    )

    # --- controls: a wrong ingredient leaves the result empty ----------------
    empty_control(
        "gold fiber with dirt where the short grass goes",
        [
            ("minecraft:oak_leaves", 4),
            ("minecraft:gold_nugget", 4),
            ("minecraft:dirt", 1),
        ],
        [
            ("minecraft:oak_leaves", every(GRID["tl"], GRID["tr"], GRID["bl"], GRID["br"])),
            ("minecraft:gold_nugget", every(GRID["tm"], GRID["ml"], GRID["mr"], GRID["bm"])),
            ("minecraft:dirt", every(GRID["mm"])),
        ],
    )
    empty_control(
        "gold brick with stone where the gold fiber goes",
        [("minecraft:stone_bricks", 1), ("minecraft:stone", 1)],
        [
            ("minecraft:stone_bricks", every(GRID["tl"])),
            ("minecraft:stone", every(GRID["mm"])),
        ],
    )

    if failures:
        for line in failures:
            print(f"  FAIL {line}", file=sys.stderr)
        return 1
    print("  both carried recipes craft their declared id and count, the grid empties,")
    print("  each distinctive ingredient is consumed, and two wrong-ingredient controls craft nothing")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
