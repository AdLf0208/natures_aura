# Nature's Aura — port notes (slice 1 · slice 2)

**Mod:** Nature's Aura (`naturesaura`) 41.10, by Ellpeck, MIT.
**Class / tier:** `content` / `deep` — but slice 1 is the **logic** shape of it:
one aura/ritual loop with its natural source, no menu, no screen, no fluids, no
energy.
**Approach** (`horizon/docs/modpacks.md`): an **agent port** of the behaviour
(`runtime.rs` + glue) with a **carry-over** of the slice's assets and data
(`content/`, script-generated from the jar).
**Licence:** MIT read from the jar (`META-INF/neoforge.mods.toml`), **rank 0**;
local-only until the maintainer's approval (ARCHITECTURE.md §8).
**Pin:** Horizon nightly **`0a875da91dbb0fdcb7ba8c42c11341aa229af51a`** (slice 2,
2026-11-21; slice 1 and the SDK migration ran at `0b14f8acfd97fa2c750e42795249dcc981178ca4`),
horizon-mixins **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, launcher 0.2.92; Minecraft 26.3.
Before the migration (2026-11-18) the campaign pin was
`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9` / mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`.
**Jar:** `ports/_shared/licence-cache/NaturesAura-41.10.jar`, SHA-1
`58e4f9a07e5320432d11aaa3d876337edeeeb8db` (the pack record's), **kept**.
**Upstream:** tag `41.10` = `70b93d37037f3559312834f0f42e824431090b64`; the
analysis clone was `/tmp/opencode/naturesaura-upstream` (not shipped).

**Format: the SDK package (2026-11-18).** This port was migrated from the retired
`.hnmod` container to the flat SDK package — one `naturesaura.hmod` carrying
`horizon-mod.toml`, `Cargo.toml`, `src/lib.rs` (one `impl Hooks` plus three Parts),
`src/runtime.rs`, `assets/` and `data/`. See
[`## Migration to the SDK format`](#migration-to-the-sdk-format) for the file map,
the hook table and every trap. `src/runtime.rs` was byte-identical to the retired
`mixins/naturesaura/runtime.rs` (sha256
`fa7273e5d8a238447f93fb43f5ef7df094941e44a78d1dc1c3412d0f6029a393`) at migration; the
2026-11-18 review fixes added `may_inspect` and `transfer_is_complete` (sha256
`0a11dfcc402170321a317421f7eaaebc5f0fd96f76799ba71bd8d489a35dc08d`, the same inside the
package).

## Slice 2 (2026-11-21) — the carried recipes, and the 1.21.1 dialect they arrived in

**Version 0.2.2**: 54,908 B, sha256 `be31234c0485c8873071b0a4ff2f1568dc23bbc3a35a96a1140fe61d8d978143`,
62 entries, no `Cargo.lock`. Pin **`0a875da`** / mixins `e59da909…`, launcher 0.2.92.
`src/runtime.rs` is unchanged (`0a11dfcc402170321a317421f7eaaebc5f0fd96f76799ba71bd8d489a35dc08d`,
the same inside the package); this slice is content, its proof, and the carry tool that made it.

### Why this slice

`FINDINGS.md` §3 carries "the two crafting recipes that make the multiblock's gold
bricks reachable from vanilla (`gold_fiber` from gold nuggets + short grass + leaves,
`gold_brick` from stone bricks + gold fiber)". The parity rule (2026-11-20) says a
carried recipe that is never crafted is dead data, and neither recipe had ever been
crafted. This is the **data half** of a `deep`-tier port: no hook, no machine, no
renderer, no new API — exactly the shape the tier allows for carried content.

### The defect: a carry error, not an upstream defect

The Java mod is correct (its 1.21.1 JSON is what 1.21.1 expects). The **carry
pipeline** was not: `tools/assemble_content.py`'s `upgrade_recipe` rewrote a shaped
recipe's `key` from the 1.21.1 object form (`{"item": …}` / `{"tag": …}`) to 26.3's
bare strings, but its shapeless branch turned only `{"tag": …}` into `"#tag"` and
left `{"item": …}` untouched:

```python
recipe["ingredients"] = [
    f"#{value['tag']}" if isinstance(value, dict) and "tag" in value else value
    for value in ingredients
]
```

So `data/naturesaura/recipe/gold_brick.json` shipped — and 0.2.0/0.2.1 delivered — in
the 1.21.1 dialect. 26.3 refuses that ingredient at load and the recipe returns
nothing, silently: `cargo horizon-mod check`, `fits` and all 21 unit tests are green
on the dead recipe. Same class as the `mekanism`, `modularrouters` and `xnet` fixes of
2026-11-21.

### Red, before the fix (verbatim)

Static, on the delivered 0.2.1 package (`evidence/naturesaura/data-shape-red.log`):

```
$ python3 ports/naturesaura/tools/data_shape.py naturesaura.hmod
  FAIL naturesaura.hmod:data/naturesaura/recipe/gold_brick.json: minecraft:crafting_shapeless ingredient(s) in the 1.21.1 object form: {"item": "minecraft:stone_bricks"}, {"item": "naturesaura:gold_fiber"}
  ok   naturesaura.hmod:data/naturesaura/loot_table/blocks/aura_bloom.json: loot pool uses the 26.3 condition form
  ...
EXIT:1
```

In game, on a launcher-built game carrying the pre-fix tree
(`evidence/naturesaura-e2e/e2e-red.log`, EXIT:1):

```
== G: the two carried recipes craft in a real crafting table ==
  the server rejected these ingredients at load:
  [04:11:19] [load-items/ERROR]: recipes: invalid ingredient (unresolved tag or malformed input): {"item":"minecraft:stone_bricks"}
  FAIL gold brick: stone bricks and gold fiber, shapeless: expected 1 naturesaura:gold_brick, got None
  gold fiber: 4 leaves, 4 gold nuggets and short grass around the centre, four at a time -> {"count": 4, "item": "naturesaura:gold_fiber", "slot": 200}
  gold brick: stone bricks and gold fiber, shapeless -> null
    grid: [{"count": 1, "item": "minecraft:stone_bricks", "slot": 100}, ..., {"count": 1, "item": "naturesaura:gold_fiber", "slot": 104}, ...]
FAIL: the crafting-table recipe check failed
```

The shaped recipe already crafted (it was in the right dialect); only the shapeless
one was dead, which is why the defect survived slice 1.

### The fix

- `tools/assemble_content.py`: one `ingredient(value)` helper used by both the shaped
  `key` and the shapeless `ingredients` list — `tag` → `"#tag"`, `item` → `"id"`, a
  list (26.3's alternative form) untouched. The generator is the data's source, so
  the fix is durable.
- Re-ran `assemble_content.py` from the kept jar (`SHA-1 58e4f9a0…`, checked by the
  tool) and `stamp_license.py`. **Exactly one of the 52 carried files changed**
  (`diff -rq` over the old and new trees): `data/naturesaura/recipe/gold_brick.json`.
  Every other file is byte-for-byte the same carry (`evidence/naturesaura/content-carry.log`).
- `tools/data_shape.py` (new): the dialect as a check over a package or a data dir,
  failing on a 1.21.1 ingredient and on a plural loot `conditions` array.
- `tools/craft_check.py` (new) and phase G of `tools/e2e.sh`: both carried recipes
  crafted in a real table, in survival, asserting id, count, the grid emptied, the
  distinctive ingredient consumed, and **two wrong-ingredient controls** that craft
  nothing (so the table cannot be passing on anything handed to it).
- Phase H/I of `tools/e2e.sh` close the rest of the package's parity gap: all seven
  items given and read back (unknown item refused), all four blocks placed and read
  back (unknown block refused), and all four loot tables dropping their block when
  broken, with vanilla stone as the harness's differential.

### Green, the delivered 0.2.2

| Check | Command | Result |
|---|---|---|
| Runtime tests | `cargo test` | **21 passed**, 0 failed (`evidence/naturesaura/sdk-test.log`) |
| Lints | `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| Formatting | `cargo fmt --check` | clean (`sdk-fmt.log`) |
| Package, twice | `rm -f Cargo.lock && cargo horizon-mod build` ×2 | 54,908 B, `cmp`-identical (`sdk-build.log`) |
| Registry check | `cargo horizon-mod check naturesaura.hmod` | `naturesaura 0.2.2 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, **54908 bytes**, sha256 `be31234c…8143`, exit 0 (`sdk-check.log`) |
| Fits the pin | `cargo horizon-mod fits … --build 0a875da` | `0a875da  made for` (`sdk-fits.log`) |
| Fits another build | `cargo horizon-mod fits … --build 0b14f8a` | `0b14f8a  fits` (`sdk-fits-other.log`) |
| Package gate | `verify-sdk-package.py naturesaura` | **1/1** |
| In-game | `launcher-game.sh build` + `build-slot.sh run --step e2e -- tools/e2e.sh` | `PASS` + **`EXIT:0`** (`evidence/naturesaura-e2e/e2e-launcher.log`) |
| Control | same run with `GAME=CONTROL_GAME CONTROL=1` | `Unknown or incomplete command` / `Unknown block type` → `FAIL` + **`EXIT:1`** (`e2e-launcher-control.log`) |
| Can fail | `SELFTEST=1` | `FAIL: area + container (selftest, must fail) …` + **`EXIT:1`** (`e2e-selftest.log`) |
| Log gate | `check-e2e-logs.py naturesaura-e2e --strict` | 4 logs, ok=4, exit 0 |
| Dialect | `data_shape.py` on the tree and the package | every recipe/loot file in the 26.3 dialect (`data-shape-green.log`) |

Green phase G, verbatim:

```
== G: the two carried recipes craft in a real crafting table ==
  gold fiber: 4 leaves, 4 gold nuggets and short grass around the centre, four at a time -> {"count": 4, "item": "naturesaura:gold_fiber", "slot": 200}
  gold brick: stone bricks and gold fiber, shapeless -> {"count": 1, "item": "naturesaura:gold_brick", "slot": 200}
  gold fiber with dirt where the short grass goes -> null
  gold brick with stone where the gold fiber goes -> null
  both carried recipes craft their declared id and count, the grid empties,
  each distinctive ingredient is consumed, and two wrong-ingredient controls craft nothing
```

### Coverage (parity rule, 2026-11-20)

| Family in the jar | In 0.2.2 | Status | Proof |
|---|---|---|---|
| crafting recipes (246 in the jar) | 2 carried: `gold_fiber` (shaped), `gold_brick` (shapeless) | **tested** | phase G: ids, counts, grid emptied, ingredients consumed; two wrong-ingredient controls craft nothing; pre-fix leg fails the same phase |
| loot tables (79 in the jar) | 4 carried | **tested** | phase I: each drops itself; vanilla stone drops as the differential |
| blockstates (78 in the jar) | 4 carried blocks | **tested** | phases B (bloom places, 150,000 aura), C/D (altar places/forms), H (all four place + read back); unknown block refused |
| item models (149 in the jar) | 7 carried items | **tested** | phase H: each given and read back; unknown item refused |
| altar recipes (36 in the jar) | the 3 catalyst-free (`iron_ingot`→`infused_iron`, `stone`→`infused_stone`, `gold_ingot`→`tainted_gold`) | **partly tested** | `infused_iron` crafted in game (phase D, 15,040 aura, 80 ticks, conservation); all three rules and costs unit-tested (`only_the_catalyst_free_altar_recipes_are_known`, `a_recipe_costs_its_aura_rounded_up_per_tick`); the other two are not individually in-game |
| multiblock (`Multiblocks.ALTAR`, 9x4x9) | 1 | **tested** | phase D: `placed 48 blocks`, `complete=1`, one ritual run with area+container conservation |
| commands | 6 (`aura`, `spot`, `altar`, `altar-build`, `drain`, `altar-aura`) | **tested** | phases A–F parse each parsed reply |
| chunk aura attachment | 1 (drain spots) | **tested** | phase B (150,000 once, not twice), phase F (deficit ⇒ no transmute) |
| block tags | 4 `altar_*` + `c:nuggets/gold` | **tested** | the multiblock forms from the tagged blocks; `gold_fiber` crafts from `#c:nuggets/gold` and `#minecraft:leaves` |
| creative tab | 1 | **not asserted** | registration only; no gameplay claim |
| the other 74 blocks, 142 items, 31 catalyst recipes, worldgen, guide book, RF converter, generators, client overlay | — | **not ported** | `FINDINGS.md` §3.1 (later slices) |

### Carry error fixed (recorded here, not as an upstream defect)

The Java mod is correct; the defect was introduced by this campaign's carry step. The
minimal fix is in the generator, the data is regenerated from the jar, and the red
leg above is the proof that would have stayed red without it. Nothing about the mod's
behaviour changed — the recipe now works as its author wrote it. Worth an upstream
report: no.

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

## Slice 3 (2026-11-24) — the grass-death drain-spot effect (0.2.4)

**Version 0.2.4** (patch of the 0.2.3 copyright fix on PR #1859). Pin unchanged:
`0a875da91dbb0fdcb7ba8c42c11341aa229af51a` / mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, launcher 0.2.92, for 26.3.

### The gap, and the choice

`FINDINGS.md` §3.1, verbatim: *"The aura drain-spot passive effects
(`chunk/effect/*`: plant boost, grass death, ore spawn, nether decay …): they
change the world each second at every drained spot and are a slice of their
own."* The artefact's `de/ellpeck/naturesaura/chunk/effect/` holds 14 effect
classes (`DrainSpotEffects` registers 14). The cheapest one expressible at this
pin was measured, not guessed:

| Effect | Needs | Verdict |
|---|---|---|
| **grass death** (`GrassDieEffect`) | a negative spot + a negative radius-50 area, `level_set_block` grass→coarse dirt | **chosen** — both helpers are in the pin's SDK (`Backend::level_set_block` 859, `LevelRef::set_block`) |
| plant boost | `BonemealableBlock.performBonemeal` on an arbitrary block | not ported — the pin stamps `bonemeal`/`perform_bonemeal` hooks but exposes no "bonemeal this position" helper |
| ore spawn | places an `naturesaura:*` ore block | not ported — no ore block is carried; a vanilla ore would change the mod's product |
| nether decay | nether blocks → nether-rack/ash | not ported — needs a dimension-specific block set the slice does not carry |
| grass death's other branches | `DECAYED_LEAVES`, bush→air, nether-grass→netherrack | not ported — the mod's decayed-leaves block is not carried |

This is the `deep` tier's logic shape: it shapes the already-ported chunk-aura
store (the spot amount the bloom and `/naturesaura spot` already expose) and
writes blocks through the pin's existing `level` handle; it adds no machine,
renderer, item or API.

### Upstream behaviour (read from tag `41.10`, the commit in this file's header)

Files read at
`https://raw.githubusercontent.com/Ellpeck/NaturesAura/41.10/src/main/java/de/ellpeck/naturesaura/…`:
`chunk/effect/DrainSpotEffects.java`, `chunk/effect/GrassDieEffect.java`,
`chunk/effect/PlantBoostEffect.java`, `chunk/AuraChunk.java`,
`api/aura/chunk/IAuraChunk.java`, `InternalHooks.java`. The rules:

- `AuraChunk.update()` calls every effect's `update(level, chunk, …)` at each
  drain spot; the mod ticks its active chunks once a second (20 ticks).
- `GrassDieEffect.calcValues`: active only when `spot < 0` **and** the radius-50
  area aura `InternalHooks.getAuraAndSpotAmountInArea` is negative. That area is
  `1_000_000 + the spot sum`; `spotCount` is the **count** of spots
  (`AuraChunk.getAuraAndSpotAmount` increments a counter per spot). Then
  `amount = min(300, ceil(|aura| / 100000 / spotCount))`, requiring `amount > 1`,
  and `dist = clamp(|aura| / 75000, 5, 75)`.
- `update` sets a `GrassBlock` to `COARSE_DIRT` (among other block branches) at
  `amount / 2 + random(amount / 2)` gaussian-scattered positions within `dist`.
- `appliesHere`: only in the Overworld and the Nether.

The port implements exactly the `GrassBlock → coarse_dirt` branch; the other
branches are recorded above and in the coverage table.

### Red, before the glue (verbatim)

`cargo test` on the stubbed rule (`ports/_shared/evidence/naturesaura/effect-red.log`):

```
running 24 tests
test runtime::tests::grass_die_concentrates_the_debt_over_the_spot_count ... FAILED
test runtime::tests::grass_die_fires_only_at_a_debt_whose_area_is_negative ... FAILED
test runtime::tests::grass_die_turns_grass_into_coarse_dirt ... FAILED

---- runtime::tests::grass_die_fires_only_at_a_debt_whose_area_is_negative stdout ----
thread 'runtime::tests::grass_die_fires_only_at_a_debt_whose_area_is_negative' (863269) panicked at src/runtime.rs:893:9:
assertion `left == right` failed
  left: None
 right: Some(GrassDie { amount: 15, dist: 20 })

test result: FAILED. 21 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out
```

Green: the same three tests pass after the rule is written, then
`grass_die_targets_start_at_the_spot_and_stay_within_dist` pins the deterministic
target set. `cargo test` is **25 passed, 0 failed**.

### Glue and deviations

`src/runtime.rs` owns the rule (`grass_die`, `grass_die_replacement`,
`grass_die_targets`), `src/lib.rs` adds one `server_tick` listener that runs the
effect once every 20 running ticks over each overworld/nether level, and the
per-spot pass calls `grass_die` then `level.set_block(..., NEIGHBORS | CLIENTS)`.

- **Deterministic scatter.** Upstream samples `level.random` (gaussian); the port
  uses a position-seeded LCG and makes the spot itself the first target.
  Upstream's pure scatter can miss the very spot that drives the effect, which
  makes it unobservable in a small game scenario; starting at the spot keeps the
  world change identical while being reproducible. Both the generator and the
  first-target choice are in `runtime.rs` and unit-tested.
- **Chunk discovery.** Upstream walks `LevelData.auraChunksWithSpots`, which the
  public SDK does not expose (`level_loaded_chunks` is a hidden backend method).
  The port scans the 8-chunk (128-block) window around each online player via
  the public `Host::chunk` / `Players::server_players` and reads each chunk's
  attachment. That window is wider than the effect's 75-block scatter and
  50-block area radius.
- **Only the grass branch** of `GrassDieEffect.update` is carried (see the table).

### In-game assertion (phase J, fail-capable)

Two `minecraft:grass_block`s 100 blocks apart in the Overworld. The first gets
`/naturesaura drain … 2500000` (spot `-2,500,000`, radius-50 area `-1,500,000`,
count 1 → `amount 15`, `dist 20`); the second stays at spot 0. After 45 ticks the
drained block reads `minecraft:coarse_dirt` and the control still reads
`minecraft:grass_block`. `SELFTEST=effect` asserts the drained block is *still*
grass and ends `EXIT:1`; the unmodded control cannot run `/naturesaura drain` at
all. Both the spot and the area are read with `/naturesaura spot` and
`/naturesaura aura` before and after, so the threshold crossing is in the log.

### Delivery and verification (0.2.4)

`naturesaura.hmod` (in the port root, never `target/`): **59,131 B**, SHA-256
`4e70a92adf6932e72f0de90393dd9fedd2019b663fa4d0e91140090c7522b5c9`, 62
entries, no packed `Cargo.lock`. Two `rm -f Cargo.lock && cargo horizon-mod build`
runs were `cmp`-identical; `check` printed `naturesaura 0.2.4 … 59131 bytes`,
`fits 0a875da` printed `made for` and `fits 0b14f8a` printed `fits`;
`verify-sdk-package.py` was **1/1** including *"package matches the tree"*.

| Check | Command | Result |
|---|---|---|
| Runtime tests | `cargo test` | **25 passed**, 0 failed (`evidence/naturesaura/sdk-test.log`, re-generated after the final build) |
| Lints / format | `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` | exit 0, 0 warnings / clean |
| Package, twice | `rm -f Cargo.lock && cargo horizon-mod build` ×2 | 59,131 B, `cmp`-identical, forwarded to `AdLf-PC` (`Compiling naturesaura v0.2.4`) |
| Registry check | `cargo horizon-mod check naturesaura.hmod` | `naturesaura 0.2.4 …`, 59,131 B, sha `4e70a92a…b5c9`, exit 0 (`sdk-check.log`) |
| Fits pin | `cargo horizon-mod fits … --build 0a875da` | **`0a875da  made for`** (`sdk-fits.log`) |
| Fits other build | `cargo horizon-mod fits … --build 0b14f8a` | **`0b14f8a  fits`** (`sdk-fits-other.log`) |
| Package gate | `verify-sdk-package.py naturesaura` | **1/1**, "package matches the tree" |
| Game build | `launcher-game.sh build naturesaura --port ports/naturesaura` | stale 0.2.3 tag removed; `Compiling naturesaura (1 of 1)`; new tag `native-0a875da-0fb4e08164b6` (`launcher-build-0.2.4.log`) |
| In-game default | `build-slot.sh run --step e2e -- bash tools/e2e.sh` | A–I plus J → `PASS` + **`EXIT:0`** (`e2e-launcher.log`) |
| Control | `CONTROL=1 GAME=$CONTROL_GAME` | `Unknown or incomplete command` → `FAIL` + **`EXIT:1`** (`e2e-launcher-control.log`) |
| Can fail | `SELFTEST=1` / `SELFTEST=effect` | conservation / inverted grass assertion → `FAIL` + **`EXIT:1`** (`e2e-selftest.log`, `e2e-selftest-effect.log`) |
| Log gate | `check-e2e-logs.py naturesaura-e2e --strict` | logs=5, ok=5, missing-exit=0, bad-exit=0 |

The exact upstream copyright line was checked first in `LICENSE`,
`LICENSE-NOTICE.md` and both packed content copies and left byte-for-byte as the
reviewer required. `E2E_TMP`/`SOCK` came from `launcher-game.sh env`, never
`/tmp` for the game root. No local `Block`/`Item` type or variant was introduced,
so `fits` was not false-failed.

### Coverage added by this slice

| Family | In 0.2.4 | Status | Proof |
|---|---|---|---|
| grass-die effect (`chunk/effect/GrassDieEffect`) | `GrassBlock → COARSE_DIRT`, `server_tick` every 20 ticks | **tested** | phase J: drained spot's grass → coarse dirt, undrained control stays grass; `SELFTEST=effect` inverts it |
| effect threshold (`calcValues`) | negative spot + negative radius-50 area, count divisor, `amount`/`dist` clamps | **tested** | 3 unit tests (`grass_die_*`), `/naturesaura spot`/`aura` readouts in phase J |
| grass-die leaf/bush/nether-grass branches, plant boost, ore spawn, nether decay | — | **not ported** | uncarried blocks/helpers (see the choice table) |

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

## The slice boundary (and why)

Implemented: the chunk aura drain spots (base 1,000,000 + spots), the aura bloom
(150,000 aura, once, when it appears), the nature's altar's 9x4x9 multiblock, its
500,000 container (300 aura a tick from the area, only in the Overworld and the
Nether), the ritual for the three catalyst-free recipes, the right-click
insert/take, and the materials that make the multiblock buildable in-game.

Why this is the smallest honest loop: the altar's only inputs are aura and one
item, its only output one item, so the loop closes with no GUI, no energy, no
fluid and no multiblock-machine API; yet every piece of it is the mod's own core
(the aura store, the natural source, the ritual, the completion gate) and every
piece is observable from `horizon-ctl` (aura numbers, the spot list, the altar's
container, the slot's item).

Not implemented (each is a later slice, listed in `FINDINGS.md` §3.1): the other
74 blocks and 142 items; the 31 catalyst altar recipes and the 2
component-output ones; the drain-spot passive effects; the item aura containers
(`aura_bottle`, `aura_cache`, `aura_trove`); the RF converter; the other
generators; the tree ritual, the offering table and the altars of birthing; the
bloom's worldgen (worldgen sorts last); the guide book; the client aura overlay
and `PacketAuraChunk` sync.

## Gaps and deviations (what the API or the slice changes)

| Gap | Shape | What the slice does instead |
|---|---|---|
| `IAuraType` per dimension | the pin has no aura-type registry | `game.native_dimension()`: the altar pulls in `minecraft:overworld` and `minecraft:nether`, and not in the End — upstream's rule, no type objects |
| Aura spot storage | NeoForge attaches to `LevelChunk`; the pin has `Holder::Chunk` attachments | a `ChunkAura` codec compound `{spots: [{x,y,z,amount}]}`; same arithmetic, same NBT spirit |
| `PacketAuraChunk` / client aura overlay | no client HUD in the slice | `sync: false`; the readouts are commands. A later slice adds the overlay and then needs `sync: true` |
| Per-item container rules | `ModContainer` has per-slot, not per-item, `inputs`/`outputs` | every side reaches the altar's one slot; the click refuses a non-recipe insert, and only a finished product leaves by hand (a hopper may take the input — recorded) |
| The click is asked more than once | the game asks `right_click_block` again for the second hand of one click (the probe logged `held=Some(iron)` then `held=None` on the same click) | taking only a **finished product** by hand (upstream's `canExtract` refuses a recipe input) keeps the second ask from undoing the insert; upstream's `putStackOnTile` would take an input back by hand, and here only automation can (recorded) |
| The altar's waterlogging | the slice's block templates have no waterlogged state | not carried (`BlockNatureAltar.hasWaterlogging`) |
| The bloom's ground set | `minecraft:poppy` allows mycelium, moss, mud and rooted dirt too | upstream's five grounds are covered; the four extra ones are a template deviation |
| Aura-container items | the altar's "fill the held item's aura" branch | not in the slice (no such items carried); the tick goes straight to the recipe |
| The generators' cap (`area + toAdd <= 2 * DEFAULT_AURA`) | upstream `canGenerateRightNow` on generators | the bloom is **uncapped** upstream (it calls `generateAura` directly), so the slice's only generator needs no cap; the rule belongs to the moss/flower/oak generators of the next slice |
| The altar's own recipe | upstream has none (the tree ritual makes it) | given by the harness in the e2e; a player gets it from the tree ritual, which is a later slice |
| Block-entity writes in a tick | the altar writes its own state each tick | it writes the block entity's saved compound the tick part is handed (`event.entity.nbt`, the SDK's checked view) and leaves the reply `Changed`, so the chunk saves what the tick did; the 0.1.0 e2e proved the state survives across ticks and the ritual completes |

## Verification (evidence in `VALIDATION.md` and `ports/_shared/evidence/naturesaura/`)

| Step | Command | Result |
|---|---|---|
| Runtime tests | `cargo test --locked --offline` | red 3/19, then 18 passed, 0 failed |
| Lints | `cargo clippy --all-targets -- -D warnings`, `cargo fmt` | clean |
| Stage | `python3 tools/horizon.py stage` | `stage_changed: true` |
| Game check | `python3 tools/horizon.py check` | `cargo_exit: 0`, 0 warnings |
| Embedded tests | `python3 tools/horizon.py test` | `game::naturesaura::tests` 18 passed, 0 warnings |
| Content build | `tools/package.sh` → `cargo horizon-mod build/check` | 21,529 B, 53 entries, `no code: content only`, same SHA-256 across two builds |
| Bundle | `tools/package.py` | 32,749 B, 13 entries, embedded content `cmp`-identical |
| e2e (content dev mod) | `build-slot.sh e2e … -- tools/e2e-all.sh` | `PASS` + `EXIT:0` (`e2e-slice1.log`) |
| e2e (one file installs both) | `EMBEDDED=1 CONTROL=1` | `PASS` + `EXIT:0`; the control shows the ids unknown without the embedded file (`e2e-embedded.log`) |
| e2e (can fail) | `SELFTEST=1` | `FAIL` + `EXIT:1` (`e2e-selftest.log`) |
| Log checker | `check-e2e-logs.py naturesaura --strict` | 3 logs, ok=3, exit 0 |
| Registry gate | `upstream_ready.py naturesaura` | blocked only on `status=pending` and the post-run rebuild, both the orchestrator's; every other gate holds |

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port

## Reviewer repairs (2026-11-11) — PR review items and what changed


Each item below is quoted from `ports/_shared/horizon-mods/reviews/pr-*-naturesaura.md` (the review is the specification), with what this repair did about it.

- **Reviewer:** restored recipe index out of bounds panics the altar tick
  **Repair:** Fixed: `runtime::recipe_at` bounds-checks the table, `Altar::step` drops a run whose index is gone, and the restore path (`recipe_at_index`) filters invalid saves. Test: `a_saved_recipe_index_past_the_table_is_dropped_not_indexed`.
- **Reviewer:** description: ',000 container' corruption, relative link, stray bullets
  **Repair:** Rewritten: the altar is stated as **500,000 aura**, the table is a real table, and the link is absolute.
- **Reviewer:** licence notice
  **Repair:** `LICENSE` + `LICENSE-NOTICE.md` in the `.hnmod`, `data/naturesaura/license/` in the carried `.hmod`.
- **Reviewer:** registry artifact delivery
  **Repair:** **Staging (orchestrator, not ours)**: the artifact is not in registry release/staging as `<sha256>.<ext>`; the orchestrator answers it with `horizon-mods#686` (no write access to the registry's releases). Every repackaged artifact below has a fresh sha256 for that re-push, and the author release must be re-uploaded from the port repository.

Skills: superpowers/using-superpowers,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

### Repair verification (2026-11-11/12)

- Staged `check`: check exit 0 (`ports/_shared/evidence/repair-2026-11-11/naturesaura-check.log`)
- Embedded tests: embedded **19 passed; 0 failed** (`ports/_shared/evidence/repair-2026-11-11/naturesaura-test.log`, the new bounds test included)
- In-game: e2e **PASS** + `EXIT:0` (`ports/_shared/evidence/repair-2026-11-11/naturesaura-e2e.log`): bloom, altar multiblock, aura accounting
- `cargo test` (standalone lib), `cargo clippy --all-targets -- -D warnings` and
  `cargo fmt --check` are clean in the port; the artifacts were built twice and
  hash identically (`registry.toml` carries the sha256 above).
- Skills: superpowers/using-superpowers,systematic-debugging,test-driven-development,verification-before-completion; BeautifulRust/rust-best-practices,rust-port


## Migration to the SDK format (2026-11-18)

Migrated from the retired `.hnmod` container (Horizon's mod-API merge #16941, launcher
≥ 0.2.70; PR #17992 cleans up the old files) to the flat SDK package at nightly
**`0b14f8acfd97fa2c750e42795249dcc981178ca4`** / mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, launcher 0.2.78. The registry closed this
port's former submission **#837** on 2026-11-18 with the format purge, so 0.2.0 supersedes
it. The reference recipe was `ports/imfast/notes.md` → "Migration to the SDK format", and
`external/botania/notes.md` for a port with block entities and a quantity; this port is the
same shape as botania plus the chunk attachment.

**Done in this order:** the old layout was moved into
`ports/_shared/evidence/naturesaura/retired-hnmod-0.1.0/` **before** the first
`cargo horizon-mod build` (the tool refuses a root holding retired package paths), then
`src/runtime.rs` was copied with `cp` and proved with `cmp` + `sha256sum`, `cargo test`
ran on the new crate **before any glue existed**, and only then was `src/lib.rs` written.

### File map

| Old `.hnmod` layout | New SDK package |
|---|---|
| `horizon-mixin.toml` (`[mod]`, `[compatibility]`, 2 `[[module]]`, 8 `[[injection]]`) | `horizon-mod.toml` (id/name/version/api/side/capabilities + `[native]` game/horizon/mixins) |
| `Cargo.toml` (`[lib] path = "mixins/naturesaura/runtime.rs"`, `[workspace] members = ["tools/audit"]`) | `Cargo.toml` (`[lib] path = "src/lib.rs"`, one dependency: `horizon-mod-api` from `.horizon/sdk/`) |
| `mixins/naturesaura/runtime.rs` (module + crate lib) | `src/runtime.rs` — **byte-identical**, `pub mod runtime;` at the crate root |
| `mixins/naturesaura/support.rs` (`naturesaura_state` module, 752 lines) | `src/lib.rs` — one `impl Hooks`, three Parts, the attachment and the helpers |
| 8 per-hook helper `.rs` files (`block_entity.rs`, `block_entity_ticks.rs`, `block_entity_container.rs`, `block_entity_tick.rs`, `right_click_block.rs`, `register_commands.rs`, `perform_command.rs`, `embedded_content.rs`) | methods of `impl Hooks`, of the Parts, or a `Registrar` call; no per-hook files |
| `content/` (`assets/`, `data/`, its own `horizon-mod.toml`, `naturesaura.hmod`) | package root: `assets/`, `data/` — the content half of the same `.hmod` |
| `mixins/naturesaura/data/naturesaura-content.hmod` + `embedded_content.rs` (the AE2 write-out) | **gone**: code and content are one package, so there is nothing to embed |
| `compatibility.json`, `rust-toolchain.toml`, `horizon-native.json` | **gone**: `[native]` in `horizon-mod.toml` is the pin, and the launcher's toolchain is the one that builds |
| `tools/horizon.py` (stage/check/test/build), `tools/package.py`/`package.sh` (.hnmod), `tools/audit/` | **gone** (moved to the evidence folder); the SDK ladder is `cargo test` + `cargo horizon-mod build/check/fits` |
| `naturesaura.hnmod` (15 entries, 32,749 B) | `naturesaura.hmod` (62 entries, 53,558 B) |

**Kept and updated:** `tools/assemble_content.py` (now writes `assets/` + `data/` at the
package root), `tools/stamp_license.py` (now writes `data/naturesaura/license/` at the
root), `tools/e2e.sh`, `tools/e2e-all.sh`, `FINDINGS.md`, `notes.md`, `VALIDATION.md`,
`LICENSE`, `LICENSE-NOTICE.md`. **Retired into evidence:** `mixins/`, `naturesaura.hnmod`,
`compatibility.json`, `rust-toolchain.toml`, `Cargo.lock`, `PLAN.md`, the old repo
`AGENTS.md` (it tells the reader to open `compatibility.json` and to run `tools/horizon.py`),
`tools/horizon.py`, `tools/package.py`, `tools/package.sh`, `tools/audit/`, the old
`content/horizon-mod.toml` and `content/naturesaura.hmod`.

### Manifest mapping

- identity: `id = "naturesaura"` (also the resource namespace), `name`, `version = "0.2.0"`
  (the port's version, not upstream's 41.10), `license = "MIT"` in `Cargo.toml`,
  `authors = ["Ellpeck", "AdLf0208"]`.
- `api = "0.2"` (exact), `side = "both"` (an integrated server belongs to the client
  installation, so a LAN host and single player need it), `capabilities = ["registries"]`
  (the schema's one value — this package really does declare blocks and block entities).
- `[native] game = "26.3"`, `horizon = "0b14f8a…"`, `mixins = "e59da90…"` replace
  `[compatibility] horizon_revision` and the bundle's mixins revision.
- `[mod] priority = 100` and the plan's ordering are gone; nothing in this slice needed
  ordering (`#[listener(priority = …)]` is the new lever).

### Hook → SDK table

| Old `[[injection]]` / helper | New place | Mapping notes |
|---|---|---|
| `block_entity` (`block_entity.rs`, returned the initial compound) | `BlockEntityPart::block_entity` on `BloomPart` / `AltarEntityPart` | a declared type would start with an empty compound; the Parts answer the same initial tags the hook did (`just_generated = 1`; `aura/timer/recipe/complete`) |
| `block_entity_ticks` (`block_entity_ticks.rs`) | `BlockEntityPart::block_entity_ticks` | `event.ticks = true` |
| `block_entity_tick` (`block_entity_tick.rs`, bindings `game`/`pos`/`entity`) | `BlockEntityPart::block_entity_tick` | `event.pos`, `event.entity.nbt`/`.items` (a checked view; `Deref`s to the fields); a Part that changed nothing answers `BlockEntityTickReply::Unchanged` |
| `block_entity_container` (`block_entity_container.rs`, built a `ModContainer`) | `Registrar::block_container(&altar, ModContainer { … })` in `Hooks::register` | a default-capability declaration, not a hook: one slot, every face reaches it, input and output |
| `right_click_block` (`right_click_block.rs`, `game.held()`, `*canceled = true`) | `BlockPart::use_item_on` + `use_without_item` on `AltarClickPart` (attached with `Registrar::block_part`) | the SDK splits the two hands into two methods; an answer replaces the block's behaviour, which is what `canceled` meant. The old two-asks-per-click fix is preserved: only a **finished product** is taken by hand |
| `register_commands` (`register_commands.rs`) | `Hooks::register_commands` (`RegisterCommandsEvent.commands`) | same six usages |
| `perform_command` (`perform_command.rs`, string words) | `Hooks::run_mod_command` (`CommandInvocation`) | typed arguments: `<pos:block_pos>` → `CommandArgumentValue::BlockPos([i32; 3])`, `<amount:integer>` → `Integer`; `Flow::Cancel` = handled |
| `embedded-content` (`content_root` HEAD) | **none** | the SDK `.hmod` is one package: `assets/`+`data/` are loaded because they are in it |
| the chunk attachment (`mod_api::AttachmentType` + `native_attachments`) | the same `AttachmentType` static, read through `host.attachments()` (`Host::attachments`, `#[access(WorldOnly)]`) and `Holder::Chunk([cx, cz])` | `get_existing` for reads that must not create a save entry, `get`/`set` for a write; the codec is unchanged |
| `game.native_block(pos)` | `level.block(host, pos)` → `BlockKey::id(host)` | `LevelRef` handle methods |
| `game.native_dimension()` (the altar's Overworld/Nether rule) | the level in place compared with `host.level("minecraft:overworld")` / `("minecraft:nether")` | a block-entity tick is not told its level; `LevelRef` is `Eq`, so equality answers it without a dimension string |
| `game.world.game_time` | `level.game_time(host)` | `i64` |
| `game.native_block_entity_sync(pos)` | `host.block_entity_sync(pos)` | after a craft |
| `game.native_with_mod_block_entity` (read/write) | `host.block_entity(level, pos)` → `BlockEntityRef::data` / `merge_data` / `item_slot` / `set_item_slot` | the slot writes go through `host.transaction(…)` so the insert and the player's hand move together |
| `game.held()`, `game.native_take_items`, `game.native_give_stack`, `game.creative` | `event.click.stack`, `ItemPlace::PlayerHand{player, hand}.extract(…)` / `.insert(…)` | `extract` already gives without losing any for a creative player, so the separate creative check is gone |
| `game.pack.assets.registry.default_state(id)` | `host.block_state_id(id)` | `/setblock`-style parsing; the harness builds the frame with default states |
| `game.native_set_block(…)` | `level.set_block(host, pos, state, BlockUpdateFlags::NEIGHBORS \| CLIENTS)` | the old `flags::UPDATE_ALL` is those two bits |
| `NativeCommand::new` / `::op` | `Command::new` / `Command::op` | same usage strings |
| `game.native_feedback(Text::literal(…))` | `host.commands().feedback(Text::literal(…))` | `ServicesServerHost::commands` |

### Build and verify (measured 2026-11-18)

Every command ran in a `ports/tools/build-slot.sh` lease with `BUILD_LOCAL=1` (the job id
was `sdk-na`; steps `test`, `lint`, `fmt`, `build`, `check`), **directly in the port** —
`build-slot.sh` speaks the old `tools/horizon.py` and does not by itself know the SDK
ladder, so it is used as the slot guard, not as the driver. `CARGO_INCREMENTAL=0`, no
`--target-dir`.

```
cargo horizon-mod build                        # fetches .horizon/sdk/ first
cargo test                                     # 19 passed; 0 failed
cargo clippy --all-targets -- -D warnings      # exit 0, 0 warnings
cargo fmt --check                              # clean
rm -f Cargo.lock && cargo horizon-mod build    # -> target/horizon-mod/naturesaura.hmod
cargo horizon-mod check naturesaura.hmod       # exit 0
cargo horizon-mod fits naturesaura.hmod --build nightly   # exit 0
```

`check` output:

```
naturesaura 0.2.0 (native-mod, format native-1/0b14f8acfd97fa2c750e42795249dcc981178ca4/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  53558 bytes, SHA-256 a64a5d8ff02b87b02c79787852e6f7c13f5ac91eee04e560ba03c236cf9c4f3c, unsigned
  native source: unsandboxed; not executed by the registry check. Compile and test against the exact pinned Horizon and Mixins revisions before review.
```

`fits` output:

```
Nature's Aura:
0b14f8a  made for
```

The archive is 62 entries: `horizon-mod.toml`, `Cargo.toml`, `LICENSE`,
`LICENSE-NOTICE.md`, `README.md`, `assets/…` (31), `data/…` (23, the two licence
copies included), `src/lib.rs`, `src/runtime.rs`, `uses.json`. **No `Cargo.lock`**
(trap 5). `unzip -p naturesaura.hmod src/runtime.rs | sha256sum` is
`fa7273e5…a393`, the retired file byte for byte.

`uses.json` is the migration's best summary: **358 surface entries** — the seven hooks
(`block_entity`, `block_entity_ticks`, `block_entity_tick`, `use_item_on`,
`use_without_item`, `register_commands`, `run_mod_command`), `Hooks::register` with
`DefaultCapabilitiesRegistrar::block_container` and `BlockEntityTypeBuilder`, the
`AttachmentType` handles (`get`/`get_existing`/`set`/`remove`), the `BlockEntityRef`
handles (`data`, `merge_data`, `item_slot`, `set_item_slot`, `kind`, `position`,
`level`), `HostTransaction::transaction`, `Commands::feedback`, both macros
(`horizon_mod`, `horizon_part`) and the `registries` capability.

### Traps this migration hit (the next port must know)

1. **`tools/audit/` is a retired package path.** The first build failed with
   `error: unexpected package path: tools/audit/src/main.rs` — the old `Cargo.toml`'s
   `[workspace] members = ["tools/audit"]` left a second crate in the tree. Anything
   that is not a package path (`src/`, `assets/`, `data/`, `horizon-mod.toml`,
   `Cargo.toml`, root files) is refused; move it to evidence **before** the first build.
2. **`ItemPlace` carries the item verbs, not `ServerHost`.** `game_insert`/`game_extract`
   live on the *backend* trait (`ItemsBackendMut`); a mod calls
   `ItemPlace::PlayerHand{player, hand}.extract(host, tx, &resource, n)` and `.insert(…)`.
   `ServerHost` offers neither.
3. **`runtime::altar_complete` takes `impl Fn([i32;3]) -> String`.** The old glue passed
   `game.native_block(at)` (a `String`); `level.block(host, at)` gives a `BlockKey`, so the
   closure answers `block_id(host, level, at).unwrap_or_default()`.
4. **A block-entity tick is not told its level.** Find it with
   `host.levels().list_levels().into_iter().find(|level| level.in_place(host))`, and
   answer "which dimension" by equality with `host.level("minecraft:overworld")` /
   `("minecraft:nether")` (`LevelRef` is `Eq`); do not guess the overworld first.
5. **A fresh declared block entity starts with an empty compound.** Without the
   `block_entity` Part method the altar would read `recipe = 0` (a real recipe!) and the
   bloom would never generate, because `read_int` answers 0 for an absent key. The Parts
   answer the old initial tags, and every read of a key whose `0` is meaningful uses the
   sentinel `UNSET` (`-1`).
6. **A tick may answer `Unchanged`.** The bloom does once it has generated, so a spent
   bloom stops dirtying its chunk; the altar writes every tick (as the 0.1.0 subject did)
   and keeps the default `Changed`.
7. **`Compound` is a `BTreeMap<JavaString, Tag>` and `JavaString` is not exported.**
   Write with `nbt.insert(key.into(), …)` and read with the UTF-16 key comparison helper
   (`key.encode_utf16()` against `name.0`); do not try to name the key type.
8. **`ItemStack::new` is fallible and takes an `ItemKey`.** `ItemStack::of(item)` for one,
   `ItemStack::new(key, count) -> Option<_>`; the key comes from
   `Registrar::item_key(id)` at registration (the recipe table names items by id only, so
   the outputs are resolved once into a `OnceLock`), and an id from a stack is
   `stack.item().id(host)`.
9. **`BlockPart` vs `BlockEntityPart`.** `use_item_on`/`use_without_item` are block hooks;
   attaching the click Part with `Registrar::block_part(&altar, …)` keeps the block table
   unambiguous, while the two entity Parts are attached with
   `Registrar::block_entity_part(&type, …)` to types built by
   `block_entity_type("…").blocks(&["naturesaura:…"]).build()`. A block-entity Part also
   needs an empty `impl BlockPart` beside it (`#[horizon_part]` derives subscriptions, it
   does not implement the supertrait).
10. **The content half moves to the package root.** `assets/` and `data/` are no longer
    under `content/`; `tools/assemble_content.py` now writes them there and no longer
    writes a manifest (the root `horizon-mod.toml` is the only one). Re-running it
    reproduced all 52 carried files byte for byte (`sha256sum` diff empty); only
    `data/naturesaura/license/{license,notice}.txt` are added by `tools/stamp_license.py`,
    which is what the 0.1.0 artifact shipped.
11. **`cargo fmt --check`, not `--all`.** `.horizon/sdk` is a path dependency; `--all`
    reports the SDK's own `build.rs` diffs.
12. **`Cargo.lock` changes the artifact.** `cargo test`/`clippy` write one; remove it right
    before `cargo horizon-mod build` (`/Cargo.lock` is in `.gitignore`). With it the
    package would pack a lock the SDK was not built against.
13. **No `rust-toolchain.toml`.** The launcher and the pinned SDK decide the toolchain
    (`sdk.json` declares rust 1.99.1; this crate was compiled here with rustc 1.97.0,
    edition 2024).
14. **`cargo horizon-mod check` with no file checks the wasm target.** Use `check
    <file.hmod>` for the package and `cargo check`/`cargo test` for the native build.
15. **The 0.1.0 e2e cannot run the new package.** `tools/e2e.sh`/`e2e-all.sh` drive a game
    built by `tools/horizon.py` with the bundle compiled in; the SDK package is built and
    run by `horizon-launcher native-build .`, which is not installed here. The scripts are
    kept unchanged (their assertions are the behaviour's proof) and no 0.2.0 in-game leg is
    claimed; the 0.1.0 runs in `ports/_shared/evidence/naturesaura/` stand as the
    behaviour's evidence for a `runtime.rs` that did not change.

### Gaps (unchanged by the migration)

`ported-partial`, as before: the slice boundary above is the port's scope. The migration
adds no new gap and closes none of the recorded ones (`IAuraType` per dimension, the
per-item container rules, `PacketAuraChunk`/client overlay, waterlogging, the altar's own
recipe). One *new* recorded difference: the `/naturesaura altar` readout names the slot's
item by **id** where the 0.1.0 code printed the item field directly; the SDK exposes
`ItemKey::id(host)`, so the string is the same and no API gap is implied.

`## Upstream defects fixed`: none found in this migration. The 0.1.0 repair (a saved
recipe index past the table panicked the tick) was a port defect, already fixed and
covered by `a_saved_recipe_index_past_the_table_is_dropped_not_indexed`.

### Changelog for the resubmission (supersedes #837)

> Supersedes #837, closed on 2026-11-18 because Horizon retired the `.hnmod` container
> (#17941; PR #17992 cleans up the old files). 0.2.0 is the flat SDK package: `api = "0.2"`,
> `[native]` pinned to nightly `0b14f8acfd97fa2c750e42795249dcc981178ca4` and mixins
> `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, launcher 0.2.78, for 26.3. The eight old
> injections became seven typed hooks (`block_entity`, `block_entity_ticks`,
> `block_entity_tick`, `use_item_on`, `use_without_item`, `register_commands`,
> `run_mod_command`) plus a `Registrar::block_container` declaration and one typed chunk
> attachment; the content half now travels in the same package instead of an embedded
> `.hmod`; `src/runtime.rs` and its 19 tests are unchanged, byte for byte. Package:
> 62 entries, **53,558 B**, sha256
> `a64a5d8ff02b87b02c79787852e6f7c13f5ac91eee04e560ba03c236cf9c4f3c`; `check` and `fits`
> exit 0. No 0.2.0 in-game run: the launcher is not installed here (trap 15).

### Publication route (campaign ruling 2026-11-18)

The submission form is gone; publishing is now `horizon-rust/horizon-mods#1147` — the artifact
is attached to a release of the author's own repository under the entry's exact `file` name, a
PR adds the `[[versions]]` record and links the file, and a **maintainer** runs
`scripts/stage.sh` (a red `check` before that is the expected `fetch.sh` failure). The
orchestrator's `python3 ports/tools/publish-ready.py naturesaura` does the bookkeeping; this
job owns no submission.

### Migration verification

| Check | Command | Result |
|---|---|---|
| Runtime tests in the new crate, before the glue | `cargo test` | **19 passed**, 0 failed (`evidence/naturesaura/sdk-test.log`) |
| Lints | `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| Formatting | `cargo fmt --check` | clean (`sdk-fmt.log`) |
| Package | `cargo horizon-mod build` | `target/horizon-mod/naturesaura.hmod`, exit 0 (`sdk-build.log`) |
| Registry check | `cargo horizon-mod check naturesaura.hmod` | `naturesaura 0.2.0 (native-mod, format native-1/0b14f8a…/e59da90…, for 26.3)`, **53,558 B**, sha256 `a64a5d8f…5cf3c`, **exit 0** (`sdk-check.log`) |
| Fits the pinned build | `cargo horizon-mod fits naturesaura.hmod --build nightly` | `0b14f8a made for`, exit 0 (`sdk-fits.log`) |
| Same package twice | `cargo horizon-mod build` again | 53,558 B, `a64a5d8f…5cf3c` both times (`sdk-reproducible.txt`, `sdk-build-second.log`) |
| `runtime.rs` untouched | `unzip -p naturesaura.hmod src/runtime.rs \| sha256sum` | `fa7273e5…a393` = the retired file (`sdk-runtime-sha256.txt`) |
| Content carry unchanged | re-ran `assemble_content.py` + `stamp_license.py` | the 52 carried files byte for byte, plus the two licence copies (`sdk-content-carry.log`) |

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

## Review fixes (2026-11-18)

`var/review-all/1210-naturesaura.md`, both findings fixed; version **0.2.1** (54,918 B, sha256
`7ff4203525ca1d9ff54cb42d324130d08b6ae2fb3a975a830dda5a2f9dbc3c0b`).

- **Output destroyed by an incompatible hand (src/lib.rs:507).** The take path cleared the
  altar's slot first and ignored what `ItemPlace::insert` took. It now fills the hand first and
  clears the slot only for a full transfer, both in one transaction: a partial or zero insert
  returns `Err`, the journal rolls the insert back, and the finished output stays. Rule:
  `runtime::transfer_is_complete`, pinned by `only_a_full_transfer_clears_the_output` (a full
  transfer, an incompatible hand, a partial insert, and an empty slot).
- **Altar readout disclosure (src/lib.rs:570, :745).** `/naturesaura altar` named the altar's
  current item and count at any loaded coordinates. It now needs an operator (permission ≥ 2) or
  a player in the same level within 8 blocks; any other caller gets a generic refusal. Rule:
  `runtime::may_inspect`, pinned by `an_altar_readout_needs_an_operator_reach_or_the_same_level`.

Ladder: `cargo test` 21 passed; `cargo clippy --all-targets -- -D warnings` exit 0;
`cargo fmt --check` clean; `cargo horizon-mod build` twice byte-identical; `check` and `fits`
exit 0. In-game: `launcher-game.sh run naturesaura` → `PASS` + `EXIT:0`
(`ports/_shared/evidence/naturesaura-e2e/e2e-launcher.log`); `GAME=$CONTROL_GAME` →
`FAIL` (`naturesaura aura … Unknown or incomplete command`) + `EXIT:1`.

## Reviewer follow-up on #1530 (2026-11-24) — version 0.2.3

**The reviewer's item, verbatim:** "`LICENSE` omits 2021 from Ellpeck's upstream copyright line and attributes the port to other contributors. Please keep upstream's exact copyright line and correct the port attribution."

The merged PR carried **0.2.2**, so the fix is a **new version**; the published record is not
rewritten. The publisher appends 0.2.3 to the retained PR branch (orchestrator's step).

**Exact upstream line, verified from the artefact** — `Ellpeck/NaturesAura`'s `LICENSE` at
tag `41.10` = commit `70b93d37037f3559312834f0f42e824431090b64` (the commit recorded in this
file's header), fetched from the upstream raw file at that commit and read back:

```
Copyright (c) 2021 Ellpeck
```

**What changed (all packed bytes, so a product change with no new behaviour family):**
- `LICENSE` line 3 is now that exact upstream line; line 4 is the port's own credit
  `Copyright (c) 2026 AdLf0208 (Horizon port)`, following the industrialforegoing 0.2.13
  precedent (upstream line 3, port line 4). The refused `Copyright (c) Ellpeck` /
  `Copyright (c) 2026 Horizon Veinminer contributors` first credit line is gone.
- `LICENSE-NOTICE.md` quotes the same verbatim line, with the port attribution in a separate
  block below it; the stale `Copyright of the original: Copyright (c) Ellpeck.` bullet is gone.
- `tools/stamp_license.py` re-ran, so the two packed copies under `data/naturesaura/license/`
  carry the corrected texts.
- `horizon-mod.toml` and `Cargo.toml`: `0.2.2` → `0.2.3`. `src/runtime.rs` and every game
  behaviour are untouched (`0a11dfcc…`, still 21 tests).

**Delivered 0.2.3:** 55,145 B, sha256
`4687233c2c7d2af296a0cfd775300262421ff2a2e9165d6526af43de81529c7f`, 62 entries, no packed
`Cargo.lock`; `LICENSE`, `LICENSE-NOTICE.md` and both content copies are the corrected bytes.

| Check | Command | Result |
|---|---|---|
| Package, twice | `build-slot.sh run --step build -- bash -c 'rm -f Cargo.lock && cargo horizon-mod build && …'` ×2 | 55,145 B, `cmp`-identical, forwarded to `AdLf-PC` (`sdk-build.log`) |
| Registry check | `cargo horizon-mod check naturesaura.hmod` | `naturesaura 0.2.3 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, 55,145 B, sha256 `4687233c…29c7f`, exit 0 (`sdk-check.log`) |
| Fits pin | `cargo horizon-mod fits naturesaura.hmod --build 0a875da` | **`0a875da  made for`** (`sdk-fits.log`) |
| Fits other build | `cargo horizon-mod fits naturesaura.hmod --build 0b14f8a` | **`0b14f8a  fits`** (`sdk-fits-other.log`) |
| Runtime tests (after the build) | `cargo test` | **21 passed**, 0 failed (`sdk-test.log`) |
| Lints / format | `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` | exit 0, 0 warnings / clean |
| Package gate | `verify-sdk-package.py naturesaura` | **1/1**, including `ok package matches the tree — every packed file is the tree's bytes` |
| Game build | `launcher-game.sh build naturesaura --port ports/naturesaura` | native tag `native-0a875da-9a0ab846863a`, exported game carries version 0.2.3 and the corrected `LICENSE` (`launcher-build-0.2.3.log`) |
| In-game default | `build-slot.sh run --step e2e -- bash tools/e2e.sh` | slice-1 A–F, recipes G, items/blocks H, loot I all green → `PASS` + **`EXIT:0`** (`e2e-launcher.log`) |
| Control | `CONTROL=1 GAME=$CONTROL_GAME` | unknown command/block, `FAIL` + **`EXIT:1`** (`e2e-launcher-control.log`) |
| Can fail | `SELFTEST=1` | `FAIL: area + container (selftest, must fail) is 1134960, expected 1134961` + **`EXIT:1`** (`e2e-selftest.log`) |
| Log gate | `check-e2e-logs.py naturesaura-e2e --strict` | logs=4, ok=4, missing-exit=0, bad-exit=0 |

No stale `native-*` tag carrying naturesaura was found before the game build (defensive scan of
the launcher's versions dir found none to remove). `E2E_TMP`/`SOCK` came from
`launcher-game.sh env`, never `/tmp` for the game root. No `Block`/`Item` local type or variant
was introduced, so `fits` was not false-failed.

Skills: superpowers/using-superpowers,writing-plans,verification-before-completion; BeautifulRust/rust-best-practices

## Next slice

- **Family:** the leaf branch of the same `GrassDieEffect` — `LeavesBlock →
  naturesaura:decayed_leaves` — from the gap line recorded in this slice's choice
  table and `FINDINGS.md` §3.1's "the aura drain-spot passive effects
  (`chunk/effect/*`: plant boost, grass death, ore spawn, nether decay …)".
  It is the cheapest remaining family: the threshold, the once-a-second driver
  and the target set are already in `runtime.rs`/`lib.rs`, and `decayed_leaves`
  is a block in the kept jar that `assemble_content.py` can carry. Plant boost
  stays blocked: the pin exposes `level_bone_meal` only on the hidden
  `LevelsBackendMut`, with no public `ServerHost` wrapper (`world_edit.rs` has
  none), so a mod cannot bonemeal an arbitrary position.
- **Why it fits:** `deep` tier, no new machine/renderer/API — one more block
  branch on the existing drained-spot effect, with its block carried in the
  same flat `.hmod`.
- **In-game assertion:** `ctl setblock` a `minecraft:oak_leaves` block at a
  drained spot (`/naturesaura drain … 2500000`, radius-50 area `-1,500,000`),
  tick 45, and `ctl block` it back as `naturesaura:decayed_leaves`; a control at
  spot ≥ 0 under `minecraft:oak_leaves` must stay `minecraft:oak_leaves`.
  `SELFTEST=effect-leaf` inverts the drained assertion to `EXIT:1`, and the
  `/naturesaura spot`/`aura` readouts prove the threshold crossing.
