# Nature's Aura for Horizon — slices 1–4

A native Horizon port of [Nature's Aura](https://github.com/Ellpeck/NaturesAura)
41.10 (MIT) by Ellpeck. It implements one complete aura/ritual loop, the carried
recipes and materials, and the grass-death drain-spot effect's grass and leaf
branches. Since 2026-11-18 it
is one **flat SDK package** (`naturesaura.hmod`, `api = "0.2"`), which carries the
code, `assets/` and `data/` together.

## What works

- **Aura drain spots.** A chunk carries `pos -> amount` spots (its base is
  1,000,000 aura), kept in a typed chunk attachment and summed over a radius, as
  the original's `AuraChunk` does.
- **The aura bloom** (`naturesaura:aura_bloom`), the natural source: a plant that
  stores 150,000 aura in its chunk the first time it ticks, once.
- **The nature's altar** (`naturesaura:nature_altar`) inside its 9x4x9
  multiblock: it pulls up to 300 aura a tick out of the area into its own
  500,000 container (in the Overworld and the Nether, as upstream) and
  transmutes one item at a time.
- **Three recipes**, every altar recipe that needs no catalyst:
  `iron_ingot -> infused_iron` (15,000 aura, 80 ticks),
  `stone -> infused_stone` (7,500 aura, 40 ticks),
  `gold_ingot -> tainted_gold` (3,000 aura, 300 ticks).
- **Materials**: the gold bricks the multiblock needs are craftable in-game
  (`gold nuggets + short grass + leaves -> gold_fiber`, then
  `stone_bricks + gold_fiber -> gold_brick`).
- **Grass death** (`GrassDieEffect`): once a running second, a spot whose amount
  is negative *and* whose radius-50 area aura is below zero turns grass into
  coarse dirt and vanilla leaves into `naturesaura:decayed_leaves`; a spot at zero
  or above leaves them alone. The carried `decayed_leaves` block drops itself only
  to silk touch, as upstream's loot table declares.

## What does not work yet (by design — see `FINDINGS.md` §3.1)

The mod's other blocks and items, the other drain-spot effects (plant boost, ore
spawn, nether decay and the bush/nether-grass branches of grass death, including
the decayed leaves' random-tick decay to air), the aura containers for items
(`aura_bottle`, `aura_cache`), the RF converter, the
other aura generators, the tree ritual, the offering table, the altars of
birthing, worldgen for the bloom, and the guide book. The altar's 31 catalyst
recipes need the catalyst blocks, which are a later slice; the altar has no
crafting recipe in this slice (upstream's is a tree ritual).

## Playing it

The mod is delivered as **one file**, `naturesaura.hmod`: content and code in one
package, as the SDK format defines. Install it once — there is no separate content
package to add. The launcher builds it against the exact Horizon / Mixins commits
the manifest pins.

In game: `give naturesaura:aura_bloom`, plant it on dirt or grass, then build the
altar's multiblock around where the altar will stand (`FINDINGS.md` §2.3 has the
four pattern layers: 20 planks, 16 stone bricks, 4 fancy bricks and 8 gold bricks
on a 9x4x9 frame, the altar one block above the bottom layer's centre), place the
altar, and right-click it with an iron ingot. 80 ticks later it holds infused
iron; a right click with an empty hand takes it off. A recipe input is not taken
back by hand — only a finished product is (a hopper can take either, as
upstream's `canExtract` refuses an input too).

## Commands

| Command | What it does |
|---|---|
| `/naturesaura aura <x> <y> <z> [radius]` | the area's aura (base + spots), its spot sum and count |
| `/naturesaura spot <x> <y> <z>` | one spot's amount |
| `/naturesaura altar <x> <y> <z>` | completeness, container, recipe, slot and area |
| `/naturesaura altar-build <x> <y> <z>` | *(op)* builds the multiblock frame, for tests |
| `/naturesaura drain <x> <y> <z> <amount>` | *(op)* drains a spot |
| `/naturesaura altar-aura <x> <y> <z> <amount>` | *(op)* sets the altar's container |

## Building

```
cargo test                                        # the pure rules, 26 tests
cargo clippy --all-targets -- -D warnings
cargo fmt --check
rm -f Cargo.lock                                  # see notes.md, migration trap 5
cargo horizon-mod build                           # -> target/horizon-mod/naturesaura.hmod
cp target/horizon-mod/naturesaura.hmod ./naturesaura.hmod
cargo horizon-mod check naturesaura.hmod
cargo horizon-mod fits naturesaura.hmod --build 0a875da
```

`cargo horizon-mod` fetches `.horizon/sdk/` on first use (gitignored, never packed).
`tools/assemble_content.py` rebuilds `assets/` and `data/` from the pack jar
(SHA-1 checked) and `tools/stamp_license.py` writes the licence copies the content
half carries; `tools/e2e-all.sh` is the 0.1.0 in-game scenario, kept unchanged —
it needs a launcher-built game (see `notes.md`, migration trap 15).
`VALIDATION.md` records every command and its result.

## Licence

MIT, as upstream. The port records upstream's copyright and licence in `LICENSE`
and `LICENSE-NOTICE.md` (both inside the package, and copied into the content half
as `data/naturesaura/license/`); it is local-only until the maintainer approves
publishing.
