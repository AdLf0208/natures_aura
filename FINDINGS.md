# Nature's Aura — jar inventory, upstream behaviour, slice 1 boundary

Written **before** any code, from the artefact and the upstream repository, as the
brief requires. Where the jar and a name disagree, the jar wins.

## 1. The artefact

| | |
|---|---|
| File | `ports/_shared/licence-cache/NaturesAura-41.10.jar` (**kept**, not deleted) |
| Size | 1,553,371 B |
| SHA-1 | `58e4f9a07e5320432d11aaa3d876337edeeeb8db` |
| Pack record | Modrinth `4cJkN3aF` version `kJ1hHmK0` (`NaturesAura 41.10`, published 2026-09-04); the recorded `sha1` is the same string (`ports/_shared/source-map/deps-cache/4cJkN3aF.versions.json`) |
| Licence in the jar | `META-INF/neoforge.mods.toml`: `license = "MIT"`; `LICENSE` in the upstream repo is MIT |
| Licence rank | **0** (permissive). Local-only until the maintainer's approval (§8) |
| Source repo | <https://github.com/Ellpeck/NaturesAura>, tag **`41.10`** = `70b93d37037f3559312834f0f42e824431090b64` ("41.10 (second try)") — cloned to `/tmp/opencode/naturesaura-upstream` (read-only analysis, not shipped) |
| Mod id / namespace | `naturesaura` (all ids below carry it) |

### What the jar contains (measured, not guessed)

`1870` entries. Namespaces: `assets/naturesaura` (1070 files), `data/naturesaura`
(391), `data/minecraft` (31), `data/c` (8), `data/curios` (7), `data/neoforge` (4).

| Family | Count | Examples |
|---|---:|---|
| blockstates (`assets/naturesaura/blockstates/`) | 78 (+1 blank entry) | `aura_bloom`, `nature_altar`, `gold_brick`, `infused_stone`, `aura_detector`, `offering_table`, `rf_converter`, `time_changer` … |
| item models (`assets/naturesaura/models/item/`) | 149 | `infused_iron`, `tainted_gold`, `gold_fiber`, `aura_bottle`, `eye`, `token_rage` … |
| `data/naturesaura/recipe/` | 246 | 36 `altar/`, 7 `offering/`, `tree_ritual/`, crafting |
| `data/naturesaura/loot_table/` | 79 | `blocks/*` |
| `data/naturesaura/advancement/` | 26 | |
| `data/naturesaura/worldgen/` | 15 | configured/placed features + biome modifiers for `aura_bloom`, `aura_cactus`, … |
| `data/naturesaura/patchouli_books/` | 3 | the in-game guide (out of any slice) |
| tags | `data/naturesaura/tags/block/` 4 | `altar_wood`, `altar_stone`, `altar_gold_brick`, `altar_fancy_brick` |
| curios tags | 7 (data) | trinket slots |

So the jar is a **content-heavy magic/tech mod**: 78 blockstates, 149 item models,
246 recipes — not a small data pack, and not a machine mod either (the altar and
the ritual blocks are the *only* GUI-less "machines" in the slice's family; the
mod's real machines are generators and the RF converter).

## 2. Upstream behaviour that the slice implements (read from tag `41.10`)

Files read (analysis only, rewritten in Rust): `chunk/AuraChunk.java`,
`api/aura/chunk/IAuraChunk.java`, `api/aura/type/IAuraType.java`,
`InternalHooks.java` (aura helpers), `blocks/tiles/BlockEntityImpl.java`
(`generateAura`, `canGenerateRightNow`), `blocks/tiles/BlockEntityAuraBloom.java`,
`blocks/BlockAuraBloom.java`, `blocks/tiles/BlockEntityNatureAltar.java`,
`blocks/BlockNatureAltar.java`, `blocks/multi/Multiblock.java`,
`blocks/multi/Multiblocks.java` (`ALTAR`), `data/BlockTagProvider.java`,
`recipes/AltarRecipe.java`, `reg/ModRegistry.java`, the jar's
`data/naturesaura/recipe/altar/*.json` and the `altar_*` block tags.

### 2.1 Aura is a per-chunk, per-block-position store

- A chunk holds **drain spots**: `pos -> int amount`, saved as
  `{drain_spots: [{pos: long, amount: int}]}` and marked dirty on every change
  (`AuraChunk.serializeNBT`).
- The area's aura is **`DEFAULT_AURA` (1,000,000) + the sum of the spots in
  radius** (`InternalHooks.getAuraInArea`). The base is implicit: an untouched
  chunk reads 1,000,000; a drained one can read less, even negative.
- `storeAura(pos, amount, aimForZero)` and `drainAura(pos, amount, aimForZero,
  simulate)` add/subtract at one spot, remove the spot at 0, and answer how much
  moved. `aimForZero` stops at 0 when the operation would cross it.
- Producers pick the **lowest** spot in radius 35 (most in debt gets filled
  first) and drainers the **highest** in radius 20 (the fullest is spent first);
  when no spot qualifies, the caller's own position is the default spot
  (`getLowestSpot`/`getHighestSpot`).
- Generation is capped: `canGenerateRightNow(toAdd)` is
  `area(radius 35) + toAdd <= 2 * DEFAULT_AURA`.

### 2.2 The natural source: the Aura Bloom

- `naturesaura:aura_bloom` is a `BushBlock` (cross model, no collision) allowed on
  grass block, dirt, podzol, coarse dirt and farmland.
- It has a block entity with **one saved flag, `just_generated`**. The worldgen
  feature sets it when the flower is generated; the block entity's **first tick**
  then calls `generateAura(150000)` and clears it (`BlockEntityAuraBloom`).
- `generateAura(amount)` loops: take the lowest spot in radius 35, `storeAura`
  what fits, subtract what was stored, repeat while aura is left.

### 2.3 The ritual: the Nature's Altar

- `naturesaura:nature_altar` is a 12/16-high block with one item slot (limit 1)
  and a **500,000 aura container**. There is no menu: a right click inserts one
  item (`Helper.putStackOnTile`), and a right click with an empty hand pops the
  slot's stack out.
- It is complete only inside the **9x4x9 altar multiblock** (`Multiblocks.ALTAR`),
  re-checked every 40 ticks. The altar sits at raw offset `(4, 1, 4)`, i.e. the
  pattern's start is `altar - (4, 1, 4)`:

  ```
  y=3  {"    M    ","         ","         ","         ","M       M","         ","         ","         ","    M    "}
  y=2  {"    B    ","         ","         ","         ","B       B","         ","         ","         ","    B    "}
  y=1  {"    B    ","         ","  M   M  ","         ","B   0   B","         ","  M   M  ","         ","    B    "}
  y=0  {"         ","   WBW   ","   WBW   "," WWCWCWW "," BBW WBB "," WWCWCWW ","   WBW   ","   WBW   ","         "}
  ```

  with `B` = the `naturesaura:altar_stone` tag (`#minecraft:stone_bricks` +
  `nether_bricks`), `W` = `naturesaura:altar_wood` (`#minecraft:planks`,
  `crimson_planks`, `warped_planks`), `C` = `altar_fancy_brick`
  (`red_nether_bricks`, `chiseled_stone_bricks`), `M` = `altar_gold_brick`
  (`naturesaura:gold_brick`, `naturesaura:gold_nether_brick`), `0` = the altar,
  and blank = anything. 48 matched positions + the altar.
- While complete, each tick (`BlockEntityNatureAltar.tick`):
  1. make room for up to **300** aura in its container;
  2. in the **Overworld or Nether aura type only**, drain
     `min(area aura in radius 20, room)` from the **highest spot** in radius 20
     into the container;
  3. if the slot holds an item with an aura container, fill that instead;
  4. else, with a matching altar recipe, spend `ceil(aura / time)` per tick and,
     after `time` ticks, replace the slot's item with the recipe's output.
- Recipes (`data/naturesaura/recipe/altar/*.json`, type `naturesaura:altar`): an
  input item, an output, an optional **catalyst** (a block matched against the
  four non-gold pillars at `altar ± (2, 0, ±2)`), `aura` and `time`.

### 2.4 Dimension aura types

`IAuraType.forLevel` picks the type of the level: `overworld`, `nether`, `end`,
or `other` (priority `MIN_VALUE`). The altar pulls aura only when the type is
overworld or nether; in the End a complete altar transmutes **only** from aura an
aura-container item hands it.

## 3. The slice (chosen from the inventory, recorded before coding)

**Slice 1 = one aura/ritual loop with its natural source:**

```
aura bloom (natural source, 150,000 aura once, when it appears)
        -> chunk drain spots (radius 35 store / radius 20 drain)
        -> the nature's altar's 500,000 container (300 aura a tick)
        -> altar recipes without a catalyst:
             minecraft:iron_ingot      -> naturesaura:infused_iron   (15000 aura,  80 ticks)
             minecraft:stone           -> naturesaura:infused_stone  ( 7500 aura,  40 ticks)
             minecraft:gold_ingot      -> naturesaura:tainted_gold   ( 3000 aura, 300 ticks)
```

Content carried with it (from the jar, MIT): the blocks `aura_bloom`,
`nature_altar`, `gold_brick`, `infused_stone`; the items `infused_iron`,
`tainted_gold`, `gold_fiber`; the two crafting recipes that make the multiblock's
gold bricks reachable from vanilla (`gold_fiber` from gold nuggets + short grass +
leaves, `gold_brick` from stone bricks + gold fiber); the four blocks' loot
tables in the 26.3 singular-typed shape; the jar's models, textures and lang
lines; the four `altar_*` block tags (carried as data, and mirrored as the
runtime's accepted-id sets because the pin has no tag-reading helper for a tick).

Why this is the smallest honest loop: the altar's *only* inputs are aura and one
item, and its only output is one item, so the loop closes without a menu, a
screen, a fluid, energy or a multiblock-machine API. Everything the loop touches
(the chunk store, the natural source, the ritual, the completion gate) is the
mod's own core, and each piece is observable from `horizon-ctl` (aura numbers,
the spot list, the altar's container, the slot's item).

### 3.1 What slice 1 deliberately leaves out (gaps, recorded in `notes.md`)

- **The other 33 altar recipes** (31 with a catalyst, 2 that produce an item with
  components): the catalyst blocks (`crushing_catalyst`, `conversion_catalyst`)
  and the item-component outputs are a later slice.
- **The aura drain-spot passive effects** (`chunk/effect/*`: plant boost, grass
  death, ore spawn, nether decay …): they change the world each second at every
  drained spot and are a slice of their own.
- **The other natural sources** (oak/moss/flower/potion/firework/chorus/animal
  generators), the **aura detector/timer**, the **aura containers for items**
  (`aura_bottle`, `aura_cache`, `aura_trove`), the **RF converter**, the **tree
  ritual**, the **offering table**, the **altars of birthing**, the staffs, the
  armour, the `depth`/`sky` tiers, the **worldgen features** (the bloom's natural
  placement — worldgen sorts last in this campaign) and the **guide book**.
- The altar's **waterlogging** (upstream `hasWaterlogging()`): the block templates
  the pin exposes have no waterlogged state in this slice.
- The bloom's **exact ground set**: the `minecraft:poppy` template allows
  mycelium, moss, mud and rooted dirt as well; upstream's bloom allows grass
  block, dirt, podzol, coarse dirt and farmland. Recorded as a template
  deviation; the five upstream grounds are covered.

### 3.2 Deviations that are API-shaped, not design changes

- Aura spots are saved in a **chunk attachment** (`Holder::Chunk`, codec compound
  `{spots: [{x,y,z,amount}]}`) instead of a NeoForge `AttachmentType` on the
  `LevelChunk`; the shape and the arithmetic are upstream's.
- `generateAura`/`drainAura` are pure functions over the spot map in `runtime.rs`;
  the glue only iterates the loaded chunks in radius and writes the attachment.
- The bloom's `just_generated` flag is set when its block entity is created, so a
  **player-planted** bloom also gives its 150,000 aura once. Upstream only sets it
  from its worldgen feature; without worldgen in the slice the loop needs a
  source a player can place, and the flag/once-only behaviour is upstream's.
- The altar's tick writes its container and progress to its block entity's saved
  compound; the pin reverts block-entity writes made *inside a tick* for the
  vanilla-BE path, so the work is done on the shared `ModBlockEntity` (as
  `charginggadgets` does) and the chunk is marked modified.
